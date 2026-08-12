# Airkeys

A credential manager on four clients and **one** shared Rust core.

```
airkeys/
├─ rust/                       the shared core — every decision lives here
│  ├─ crates/airkeys-core            pure kernels + Crux state machines
│  ├─ crates/airkeys-core-uniffi     → Kotlin (Android) + Swift (iOS)
│  ├─ crates/airkeys-core-wasm       → the web app
│  ├─ scripts/                       generate, build, smoke
│  └─ harness/{kotlin,swift}         conformance replay through the bindings
├─ app-desktop/airkeys-wallet   GPUI — links the core natively, no FFI
├─ app-web/airkeys.app          SvelteKit — drives the core through wasm
├─ app-ios/Airkeys              SwiftUI
├─ app-android/Airkeys          Compose
└─ design/                      mocks and tokens
```

## Where does this code go?

The one question worth answering before every change.

| The change is about… | It goes in |
|---|---|
| what *should happen* — a rule, a state transition, a validation | `rust/crates/airkeys-core` |
| performing something — storage, network, biometrics, clipboard, timers | the platform shell |
| how it *looks* — colour, spacing, type | that client's theme module |
| what it *says* | `rust/crates/airkeys-core/i18n/locales/*.json` |

A rule in a UI layer is a rule three other clients do not have. When something
feels like UI but is really a decision — "is a second unlock press allowed?",
"when is the clipboard wiped?" — it belongs in the core, where it is tested in
microseconds and shared by everyone.

## Start here

- **[rust/README.md](rust/README.md)** — the five rules the whole repo follows,
  the state-machine boundary, and every command.
- **[app-desktop/airkeys-wallet/README.md](app-desktop/airkeys-wallet/README.md)** —
  the GPUI layering, the env pins, and the gpui quirks that will bite you.

## Verify it end to end

```bash
cd rust
cargo test --workspace --features airkeys-core/i18n-all,airkeys-core/crux
cargo clippy --workspace --all-targets --features airkeys-core/i18n-all,airkeys-core/crux -- -D warnings
node scripts/gen-i18n.mjs --check          # generated catalogs match the corpus
node scripts/gen-core-types.mjs --check    # generated TypeScript matches the Rust
node scripts/build-web.mjs --check         # the committed wasm matches the source
scripts/smoke-swift.sh                     # the same facts, through the Swift bindings
scripts/smoke-kotlin.sh                    # …and the Kotlin ones
cargo tree -p airkeys-core-uniffi | grep -c crux   # must be 0

cd ../app-desktop/airkeys-wallet
cargo test && cargo run
AIRKEYS_GALLERY=1 cargo run                # every screen state, listed
```
