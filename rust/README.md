# airkeys-core — the shared computation core

Pure, deterministic, correctness-critical computation for Airkeys, plus the
portable state machines that own what a flow decides. No I/O, no network, no UI,
no clock, no randomness. **One implementation, four surfaces.**

## Why this exists

A credential manager is four clients that must agree exactly. If the desktop
rates a password Strong and the phone rates it Weak, the rating is worthless. If
two devices derive different ids for the same entry, a sync duplicates it. If
the web app clears the clipboard after 30 seconds and iOS after 90, one of them
is a security bug and nobody can say which.

Every one of those is a rule, and a rule implemented four times is four rules.
This crate is the one implementation, shared to every platform.

## Layout

```
crates/airkeys-core           pure logic, zero FFI dependencies
  primitives                  hex / base64url / sha256 / stable item ids
  strength                    secret-strength verdicts
  types                       the value types that cross every boundary
  i18n                        the translation engine
  i18n_catalogs               GENERATED per-locale tables
  app/                        Crux state machines — see below
crates/airkeys-core-uniffi    uniffi shell → Kotlin (Android) + Swift (iOS)
crates/airkeys-core-wasm      wasm-bindgen shell → the web app
harness/{kotlin,swift}        conformance replay through the generated bindings
scripts/                      build, generate, smoke
```

The desktop app (`../app-desktop/airkeys-wallet`) links `airkeys-core` **directly**
— it is a Rust runtime, so it needs no shell crate at all.

**Never hand-roll a primitive.** Hashing comes from `sha2`, base64 from
`base64`. What this crate adds is strictness: junk input errors instead of being
silently coerced.

## Commands

| What | Command |
|---|---|
| Test everything | `cargo test --workspace --features airkeys-core/i18n-all,airkeys-core/crux` |
| Lint | `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings` |
| Regenerate the i18n catalogs | `node scripts/gen-i18n.mjs` |
| Gate i18n drift (CI) | `node scripts/gen-i18n.mjs --check` |
| Regenerate the TypeScript wire types | `node scripts/gen-core-types.mjs` |
| Gate TypeScript drift (CI) | `node scripts/gen-core-types.mjs --check` |
| Build the web artifact | `node scripts/build-web.mjs` |
| Gate the web artifact (CI) | `node scripts/build-web.mjs --check` |
| Android `.so` + Kotlin bindings | `scripts/build-android.sh` |
| iOS XCFramework + Swift sources | `scripts/build-ios-xcframework.sh` |
| Kotlin bindings conformance | `scripts/smoke-kotlin.sh` |
| Swift bindings conformance (macOS) | `scripts/smoke-swift.sh` |

---

## The five rules

Everything in this repository follows from these. They are enforced by tests and
by the type system wherever that was possible; where it was not, they are stated
here so a reviewer can hold the line.

### 1. The core decides; it never does

```text
Event ─► update(Model) ─► Model' + Command<Effect>
                                      │
ViewModel = view(Model')              └─► the shell performs I/O and answers
```

No network, no storage, no clipboard, no clock, no randomness in
`airkeys-core::app`. Wall-clock instants arrive as **fields on the results the
shell returns** (`DeviceAuthenticated { now_iso }`), and elapsed time is counted
from answered `Wait` operations rather than read from a clock.

That is what makes every rule testable in microseconds with no device and no
browser — `tests/app_unlock.rs` asserts the "waited 9 seconds" behaviour without
waiting nine seconds — and it is what lets the same machine drive GPUI, SwiftUI,
Compose and React.

### 2. A decrypted secret never enters the core

`Item` has no secret field. The core addresses secrets **by reference**:
`CopyField { request: CopyRequest { item_id, field } }`. The shell resolves the
plaintext from its own keystore, puts it on the clipboard, and answers `Copied`.

If a plaintext password can never be in the model, it can never be in a view
model, a serialized effect, a log line or a crash dump. A `StrengthReport` is
allowed to cross — it is a verdict, not a secret.

### 3. Feature gates are load-bearing, so verify them

Two features default to **off**, and the reason is the same in both cases: with
the feature off, the dependency is not in the graph at all.

```bash
cargo tree -p airkeys-core-uniffi | grep -c crux   # must be 0
cargo tree -p airkeys-core-wasm   | grep -c crux   # must be > 0
```

- `crux` — the state machines. iOS and Android do not link the framework, so the
  static library and the `.so` cannot change when a machine does.
- `i18n-<lng>` — one feature per locale, **default zero**. Web fetches
  `/i18n/<lng>.json` at runtime; desktop compiles all of them in; native ships
  the fallback and fetches the rest. Embedding a catalog costs more over the wire
  than fetching it, because base64-wrapping defeats compression on exactly the
  data type that compresses best.

### 4. Generated artifacts are committed and drift-gated

| Generated | From | Gate |
|---|---|---|
| `src/i18n_catalogs/*.rs` | `i18n/locales/*.json` | `gen-i18n.mjs --check` |
| `app-web/…/static/i18n/*.json` | the same corpus | the same |
| `app-web/…/src/lib/core/generated/*.ts` | the Rust wire types | `gen-core-types.mjs --check` |
| `pkg-web/` | `airkeys-core-wasm` | `build-web.mjs --check` |

`i18n/locales/*.json` is **the only hand-edited file set.** Edit a string, run
the generator, commit the regeneration *with* the corpus change — a corpus edit
without it leaves the app rendering the old string while the Rust suite renders
the new one, and both would be green.

### 5. Bindings are generated from the compiled library

`build-android.sh` and `build-ios-xcframework.sh` generate from
`--library target/release/libairkeys_core_uniffi.*`, never from source. uniffi
bindings are checksum-coupled to the version that produced them, and a mismatch
does not fail the build — it panics on the first FFI call at run time.

**Never bump uniffi alone.** Bump the workspace pin, regenerate both languages,
and run both smoke harnesses before committing.

---

## `app/` — the state machines

Everything outside `app/` is a *kernel*: a pure function the app calls
(`secret_strength`, `item_fingerprint`, `t`). `app/` is different in kind — it
holds machines that own what a flow decides, built on
[Crux](https://redbadger.github.io/crux/).

- [`shell`](crates/airkeys-core/src/app/shell.rs) — the operation/result
  vocabulary every machine speaks. Shared, so each platform writes **one**
  executor.
- [`unlock`](crates/airkeys-core/src/app/unlock.rs) — machine A, a linear
  ceremony: support → authenticate → open → enter.
- [`item_list`](crates/airkeys-core/src/app/item_list.rs) — machine B, the shape
  most product screens have: several operations in flight over a list the user
  is editing.

Adding a machine is: the `App` impl, three lines of `SplitEffect`, one
`bridge_class!` line in the wasm crate, and one line in
`generate_bindings.rs`. The bridge, the desktop host and the test driver are
written once, against `SplitEffect`, so none of them can fork per machine.

### The rules each machine encodes

Read the module docs — every rule is stated with the failure it prevents. The
short version:

**unlock** — read the vault before raising the OS sheet (never prompt for a
fingerprint and *then* say there is no vault); a cancel is not an error; one
ceremony at a time, and a superseded attempt's answer is dropped; a failed
*read* is not an absent vault.

**item_list** — the visible list is derived, never cached; a favourite toggle is
optimistic and reverts on failure; and a newer copy supersedes the older
clipboard wipe, which is what the `generation` counters are for.

---

## Testing

`tests/support/mod.rs` is a driver written against `SplitEffect`, so it drives
any machine: dispatch an event, collect the operations, resolve them one at a
time. It is the same plumbing the wasm bridge and the desktop host use, which is
the point — the tests exercise the real correlation logic rather than a
simulation of it.

Two habits worth keeping:

- **`resolve_matching`, not `resolve`, when more than one operation is in
  flight.** Answering the oldest can feed a vault result to the elapsed ticker;
  the machine correctly ignores it and the test then asserts nothing.
- **Assert the operations, not just the view.** "Did it prompt?" and "did it
  raise the OS sheet?" are the questions that catch the interesting bugs.
