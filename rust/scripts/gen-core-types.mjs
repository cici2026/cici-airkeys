#!/usr/bin/env node
/**
 * Emit the TypeScript mirrors of the Core <-> Shell wire types.
 *
 *   node rust/scripts/gen-core-types.mjs           regenerate
 *   node rust/scripts/gen-core-types.mjs --check   fail on drift (CI)
 *
 * Wraps `cargo run -p airkeys-core --features bindings --bin generate_bindings`,
 * which is the thing that actually knows the types. This script exists for the
 * `--check` gate: the generated files are committed, and a hand-edit or a
 * forgotten regeneration has to fail the build rather than fail at run time in
 * a branch the end-to-end suites never reach.
 */

import { execFileSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, readdirSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const RUST_DIR = dirname(dirname(fileURLToPath(import.meta.url)));
const REPO_ROOT = dirname(RUST_DIR);
const OUT_DIR = join(REPO_ROOT, 'app-web', 'airkeys.app', 'src', 'lib', 'core', 'generated');

const CHECK_ONLY = process.argv.includes('--check');

function generateInto(dir) {
  execFileSync(
    'cargo',
    ['run', '--quiet', '-p', 'airkeys-core', '--features', 'bindings', '--bin', 'generate_bindings', '--', dir],
    { cwd: RUST_DIR, stdio: 'inherit' },
  );
}

function listFiles(dir) {
  if (!existsSync(dir)) return [];
  return readdirSync(dir).filter((name) => name.endsWith('.ts')).sort();
}

if (!CHECK_ONLY) {
  // Wipe first: a type deleted in Rust must not leave its .ts behind, still
  // importable and still compiling.
  rmSync(OUT_DIR, { recursive: true, force: true });
  generateInto(OUT_DIR);
  console.log(`gen-core-types: wrote ${listFiles(OUT_DIR).length} files to ${OUT_DIR}`);
  process.exit(0);
}

const staging = mkdtempSync(join(tmpdir(), 'airkeys-core-types-'));
try {
  generateInto(staging);
  const fresh = listFiles(staging);
  const committed = listFiles(OUT_DIR);

  const drift = [];
  for (const name of new Set([...fresh, ...committed])) {
    const a = fresh.includes(name) ? readFileSync(join(staging, name), 'utf8') : null;
    const b = committed.includes(name) ? readFileSync(join(OUT_DIR, name), 'utf8') : null;
    if (a !== b) drift.push(name);
  }

  if (drift.length) {
    console.error(
      `gen-core-types: the committed bindings are stale — run \`node rust/scripts/gen-core-types.mjs\` and commit the result:\n  ${drift.join('\n  ')}`,
    );
    process.exit(1);
  }
  console.log(`gen-core-types: up to date (${committed.length} files)`);
} finally {
  rmSync(staging, { recursive: true, force: true });
}
