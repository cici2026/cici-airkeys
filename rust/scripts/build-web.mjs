#!/usr/bin/env node
/**
 * Build airkeys-core for the web app.
 *
 * Pipeline: wasm-pack (--target web) → size gate → rust/pkg-web/.
 *
 *   node rust/scripts/build-web.mjs           build and write rust/pkg-web/
 *   node rust/scripts/build-web.mjs --check   verify the committed output is
 *                                             current (CI); non-zero on drift
 *
 * `pkg-web/` is COMMITTED. That is deliberate: it means a web-only contributor
 * can build and run the app with no Rust toolchain, and it means a change to
 * the core shows up as a reviewable diff in the artifact the browser actually
 * loads. `--check` is what keeps the two in step.
 *
 * The output is a plain ESM module plus a `.wasm` asset — Vite fingerprints and
 * serves it, and the module is fetched asynchronously on first use. (If a
 * bundler ever lands here that cannot handle wasm-as-asset, the fallback is
 * base64-embedding the module and initialising synchronously. Do not reach for
 * that pre-emptively: it costs roughly a third more over the wire, because
 * base64 defeats compression on exactly the data type that compresses best.)
 */

import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { homedir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const RUST_DIR = dirname(dirname(fileURLToPath(import.meta.url)));
const CRATE_DIR = join(RUST_DIR, 'crates', 'airkeys-core-wasm');
const STAGING_DIR = join(RUST_DIR, 'target', 'pkg-web-staging');
const OUT_DIR = join(RUST_DIR, 'pkg-web');

/**
 * Hard ceiling on the wasm-opt'd module, in bytes.
 *
 * The budget is network transfer: at 1.5 MB raw the brotli'd wire cost is
 * roughly 500 KB, fetched once and cached immutably. If a change pushes past
 * this, the question to answer is *what got linked in* — not whether to raise
 * the number. The usual culprit is a locale catalog: the wasm crate ships zero
 * of them on purpose (see its Cargo.toml).
 */
const MAX_WASM_BYTES = 1_500_000;

const CHECK_ONLY = process.argv.includes('--check');

function build() {
  if (existsSync(STAGING_DIR)) rmSync(STAGING_DIR, { recursive: true, force: true });

  // Panic-location strings from registry crates otherwise embed the builder's
  // absolute $CARGO_HOME. That makes the artifact machine-dependent — so
  // `--check` could never pass on a CI runner — and ships a developer's home
  // directory to production. Remap so any machine produces the same bytes.
  const cargoHome = process.env.CARGO_HOME ?? join(homedir(), '.cargo');
  const rustflags = [process.env.RUSTFLAGS, `--remap-path-prefix=${cargoHome}=/cargo`]
    .filter(Boolean)
    .join(' ');

  execFileSync(
    'wasm-pack',
    [
      'build',
      '--release',
      '--target', 'web',
      '--out-dir', STAGING_DIR,
      '--out-name', 'airkeys_core',
      CRATE_DIR,
    ],
    { cwd: RUST_DIR, stdio: 'inherit', env: { ...process.env, RUSTFLAGS: rustflags } },
  );
}

function gateSize() {
  const wasm = join(STAGING_DIR, 'airkeys_core_bg.wasm');
  const bytes = statSync(wasm).size;
  if (bytes > MAX_WASM_BYTES) {
    throw new Error(
      `build-web: the module is ${bytes.toLocaleString()} bytes, over the ${MAX_WASM_BYTES.toLocaleString()} ceiling.\n` +
        `Run \`cargo tree -p airkeys-core-wasm\` and find what was linked in — do not just raise the ceiling.`,
    );
  }
  console.log(`build-web: ${bytes.toLocaleString()} bytes (ceiling ${MAX_WASM_BYTES.toLocaleString()})`);
}

/** Files worth shipping. wasm-pack also writes .gitignore and a package README. */
const SHIPPED = (name) =>
  name.startsWith('airkeys_core') && (name.endsWith('.js') || name.endsWith('.ts') || name.endsWith('.wasm'));

function digest(dir) {
  const files = existsSync(dir) ? readdirSync(dir).filter(SHIPPED).sort() : [];
  const hash = createHash('sha256');
  for (const name of files) {
    hash.update(name);
    hash.update(readFileSync(join(dir, name)));
  }
  return { files, hash: hash.digest('hex') };
}

build();
gateSize();

const fresh = digest(STAGING_DIR);
const committed = digest(OUT_DIR);

if (CHECK_ONLY) {
  if (fresh.hash !== committed.hash) {
    console.error(
      'build-web: rust/pkg-web/ is stale — run `node rust/scripts/build-web.mjs` and commit the result.\n' +
        `  fresh:     ${fresh.files.join(', ')}\n  committed: ${committed.files.join(', ')}`,
    );
    process.exit(1);
  }
  console.log(`build-web: rust/pkg-web/ is up to date (${committed.files.length} files)`);
  process.exit(0);
}

rmSync(OUT_DIR, { recursive: true, force: true });
mkdirSync(OUT_DIR, { recursive: true });
for (const name of fresh.files) {
  writeFileSync(join(OUT_DIR, name), readFileSync(join(STAGING_DIR, name)));
}
// A tiny manifest so a bug report can say which core a browser was running.
writeFileSync(
  join(OUT_DIR, 'build-info.json'),
  `${JSON.stringify({ files: fresh.files, sha256: fresh.hash }, null, 2)}\n`,
);
console.log(`build-web: wrote ${fresh.files.length} files to ${OUT_DIR}`);
