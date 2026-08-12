#!/usr/bin/env node
/**
 * Generate every downstream i18n artifact from the hand-edited corpus.
 *
 *   rust/crates/airkeys-core/i18n/locales/<lng>.json     <- the ONLY hand-edited files
 *         ├─► rust/crates/airkeys-core/src/i18n_catalogs/<lng>.rs   compiled-in tables
 *         ├─► rust/crates/airkeys-core/src/i18n_catalogs/mod.rs     feature-gated dispatch
 *         └─► app-web/airkeys.app/static/i18n/<lng>.json            fetched at runtime by web
 *
 * Usage:
 *   node rust/scripts/gen-i18n.mjs           regenerate
 *   node rust/scripts/gen-i18n.mjs --check   fail if the committed output is stale (CI)
 *
 * The generated files are COMMITTED. That is not redundancy: a corpus edit that
 * forgets the regeneration leaves the app rendering the old string while the
 * Rust suite renders the new one, and both would be green. `--check` is what
 * turns that into a failing build.
 */

import { readFileSync, writeFileSync, mkdirSync, readdirSync, existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const RUST_DIR = dirname(dirname(fileURLToPath(import.meta.url)));
const REPO_ROOT = dirname(RUST_DIR);
const CRATE_DIR = join(RUST_DIR, 'crates', 'airkeys-core');
const LOCALES_DIR = join(CRATE_DIR, 'i18n', 'locales');
const CATALOGS_DIR = join(CRATE_DIR, 'src', 'i18n_catalogs');
const WEB_DIR = join(REPO_ROOT, 'app-web', 'airkeys.app', 'static', 'i18n');

const CHECK_ONLY = process.argv.includes('--check');

/** The fallback language. Every other locale is compared against it. */
const FALLBACK = 'en';

/**
 * Which plural forms each language must supply, mirroring
 * `src/i18n/plural.rs`. Kept here as a literal rather than parsed out of the
 * Rust: two independent statements of the same rule catch the case where one
 * of them is edited alone.
 */
const PLURAL_FORMS = {
  en: ['_one', '_other'],
  zh: ['_other'],
  ja: ['_other'],
};

const PLURAL_SUFFIXES = ['_zero', '_one', '_two', '_few', '_many', '_other'];

function fail(message) {
  console.error(`gen-i18n: ${message}`);
  process.exit(1);
}

/** `{"a":{"b":"c"}}` -> `[["a.b", "c"]]`, sorted, rejecting non-string leaves. */
function flatten(value, path, out, lng) {
  if (value !== null && typeof value === 'object' && !Array.isArray(value)) {
    for (const key of Object.keys(value)) {
      flatten(value[key], path ? `${path}.${key}` : key, out, lng);
    }
    return out;
  }
  if (typeof value !== 'string') {
    fail(`${lng}: \`${path}\` is ${Array.isArray(value) ? 'an array' : typeof value}, expected a string`);
  }
  out.push([path, value]);
  return out;
}

/** Strip a trailing plural suffix: `a.b_one` -> `a.b`. */
function baseKey(key) {
  const suffix = PLURAL_SUFFIXES.find((s) => key.endsWith(s));
  return suffix ? key.slice(0, -suffix.length) : key;
}

function loadLocale(lng) {
  const raw = readFileSync(join(LOCALES_DIR, `${lng}.json`), 'utf8');
  const entries = flatten(JSON.parse(raw), '', [], lng);
  entries.sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0));
  return entries;
}

/**
 * The corpus invariants. Every one of these was a real class of bug in a
 * previous life: a key added to en only (silent English on a Chinese screen), a
 * plural form written for a language that has no plural (dead entry that
 * shadows nothing), a `{{var}}` renamed in one locale (a sentence with a hole).
 */
function validate(locales) {
  const fallback = locales.get(FALLBACK);
  if (!fallback) fail(`no \`${FALLBACK}.json\` in ${LOCALES_DIR}`);

  const fallbackBases = new Set(fallback.map(([key]) => baseKey(key)));

  for (const [lng, entries] of locales) {
    const forms = PLURAL_FORMS[lng];
    if (!forms) fail(`${lng}: no plural rule declared — add it to PLURAL_FORMS and to src/i18n/plural.rs`);

    const bases = new Set(entries.map(([key]) => baseKey(key)));
    for (const base of fallbackBases) {
      if (!bases.has(base)) fail(`${lng}: missing \`${base}\` (present in ${FALLBACK})`);
    }
    for (const base of bases) {
      if (!fallbackBases.has(base)) fail(`${lng}: \`${base}\` has no ${FALLBACK} counterpart`);
    }

    // Plural keys: exactly the forms this language's rules use, no more.
    const pluralBases = new Set(
      entries.map(([key]) => key).filter((key) => key !== baseKey(key)).map(baseKey),
    );
    for (const base of pluralBases) {
      for (const form of forms) {
        if (!entries.some(([key]) => key === `${base}${form}`)) {
          fail(`${lng}: \`${base}\` is plural but has no \`${form}\` form`);
        }
      }
      for (const [key] of entries) {
        if (baseKey(key) === base && key !== base && !forms.includes(key.slice(base.length))) {
          fail(`${lng}: \`${key}\` is a plural form this language does not use (allowed: ${forms.join(', ')})`);
        }
      }
    }

    // Interpolation variables must match the fallback's, per base key.
    const varsOf = (text) => [...text.matchAll(/\{\{\s*([^}\s]+)\s*\}\}/g)].map((m) => m[1]).sort();
    const fallbackVars = new Map(fallback.map(([key, value]) => [baseKey(key), varsOf(value).join(',')]));
    for (const [key, value] of entries) {
      const expected = fallbackVars.get(baseKey(key));
      const actual = varsOf(value).join(',');
      if (expected !== undefined && expected !== actual) {
        fail(`${lng}: \`${key}\` uses {{${actual || 'none'}}}, ${FALLBACK} uses {{${expected || 'none'}}}`);
      }
    }
  }
}

const rustString = (text) =>
  `"${text.replace(/\\/g, '\\\\').replace(/"/g, '\\"').replace(/\n/g, '\\n')}"`;

function catalogSource(lng, entries) {
  const rows = entries.map(([key, value]) => `    (${rustString(key)}, ${rustString(value)}),`).join('\n');
  return `//! GENERATED by rust/scripts/gen-i18n.mjs from i18n/locales/${lng}.json — DO NOT EDIT.
//!
//! Edit the corpus and re-run the generator; \`gen-i18n.mjs --check\` fails the
//! build when this file and the corpus disagree.

// \`rustfmt::skip\` because two tools would otherwise both claim this file:
// rustfmt re-wraps the rows, the drift gate then sees output that does not
// match the generator, and \`cargo fmt && gen-i18n --check\` can never both pass.
// The generator owns the formatting here.
#[rustfmt::skip]
pub(crate) const ENTRIES: &[(&str, &str)] = &[
${rows}
];
`;
}

function modSource(langs) {
  const modules = langs
    .map((lng) => `#[cfg(feature = "i18n-${lng}")]\nmod ${lng.replace(/-/g, '_')};`)
    .join('\n');
  const arms = langs
    .map(
      (lng) =>
        `        #[cfg(feature = "i18n-${lng}")]\n        "${lng}" => Some(${lng.replace(/-/g, '_')}::ENTRIES),`,
    )
    .join('\n');
  return `//! GENERATED by rust/scripts/gen-i18n.mjs — DO NOT EDIT.
//!
//! One module per shipped locale, each behind its own \`i18n-<lng>\` feature.
//! The default feature set is EMPTY: a build opts into exactly the catalogs it
//! ships, and \`embedded\` answers \`None\` for the rest rather than pretending
//! they are missing from the corpus.

${modules}

/// The compiled-in table for \`language\`, or \`None\` when its feature is off.
pub(crate) fn embedded(language: &str) -> Option<&'static [(&'static str, &'static str)]> {
    #[allow(clippy::match_single_binding, unused_variables)]
    match language {
${arms}
        _ => None,
    }
}
`;
}

function emit(path, contents, stale) {
  if (CHECK_ONLY) {
    const current = existsSync(path) ? readFileSync(path, 'utf8') : null;
    if (current !== contents) stale.push(path);
    return;
  }
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, contents);
}

function main() {
  const langs = readdirSync(LOCALES_DIR)
    .filter((name) => name.endsWith('.json'))
    .map((name) => name.slice(0, -'.json'.length))
    .sort();
  if (!langs.length) fail(`no locale files in ${LOCALES_DIR}`);

  const locales = new Map(langs.map((lng) => [lng, loadLocale(lng)]));
  validate(locales);

  const stale = [];
  for (const [lng, entries] of locales) {
    emit(join(CATALOGS_DIR, `${lng}.rs`), catalogSource(lng, entries), stale);
    // The web asset keeps the corpus's own nesting — it is fetched and handed
    // to `Catalog::from_json`, which flattens it the same way this script does.
    emit(join(WEB_DIR, `${lng}.json`), readFileSync(join(LOCALES_DIR, `${lng}.json`), 'utf8'), stale);
  }
  emit(join(CATALOGS_DIR, 'mod.rs'), modSource(langs), stale);

  if (CHECK_ONLY) {
    if (stale.length) {
      fail(
        `these generated files are stale — run \`node rust/scripts/gen-i18n.mjs\` and commit the result:\n  ${stale.join('\n  ')}`,
      );
    }
    console.log(`gen-i18n: up to date (${langs.length} locales, ${locales.get(FALLBACK).length} keys)`);
    return;
  }

  console.log(
    `gen-i18n: wrote ${langs.length} catalogs (${locales.get(FALLBACK).length} keys each) to ${CATALOGS_DIR} and ${WEB_DIR}`,
  );
}

main();
