# Contributing to Airkeys

Airkeys guards real money. A change must first keep the promises in
[docs/DESIGN.md](docs/DESIGN.md), then keep things simple. Report security problems privately
([SECURITY.md](SECURITY.md)), not in an issue.

## Before you start

- Beyond a small fix, first open an issue saying what you want to change and why, so no work
  goes into something that cannot be merged.
- Never put a recovery phrase that holds funds in an issue, a test or a screenshot. Use the
  standard test phrase (`abandon` eleven times, then `about`).

## The protocol does not break

[docs/PROTOCOL.md](docs/PROTOCOL.md) is a contract between phones and web apps updated at
different times. Within protocol version 1:

- a message the other side accepts today must still be accepted, and mean the same;
- derivation paths, addresses, signing and the checks of §7 do not change;
- the Lumen Code settings a reader relies on (colours, parity, quiet zone) do not change.

Anything else needs a new protocol version, agreed in an issue first, with both sides and the
document updated together.

## Where things go

- Decisions live in `core/`: which screen comes next, every string a person reads, what may be
  signed. The Android and iOS shells only draw the core's view and do what it asks; the two
  phones behaving differently is a bug.
- The web app holds addresses only and must never need a key or a public key.
- `registry/networks.json` is shared by the phone and the web app; change it in one place.

## Checks

Run what you touched; CI runs all of it on every pull request.

```sh
# core
cd core
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --release
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps

# web app
cd app-web/airkeys.app
bun install
bun run format:check && bun run check && bun run test

# Android (builds the core with cargo-ndk)
cd app-android/Airkeys && ./gradlew :app:assembleDebug :app:lintDebug

# iOS (signing for a phone: app-ios/Config/Local.xcconfig.example)
core/scripts/build-ios.sh
cd app-ios && xcodegen && xcodebuild -scheme Airkeys \
  -destination 'platform=iOS Simulator,name=iPhone 17 Pro' test
```

The end-to-end checks (`core/e2e/`, `app-web/airkeys.app/scripts/e2e.mjs`) need Bitcoin Core,
Foundry and Chrome (see each README); run them after changing signing, requests or the codes.

After changing how the core parses addresses, regenerate the web app's test vectors (CI checks
them):

```sh
cd core && cargo run -q --release -p airkeys-wallet --example vectors \
  > ../app-web/airkeys.app/src/lib/vectors.json
```

A new version goes in four places, which CI checks agree: `core/Cargo.toml`,
`app-android/Airkeys/app/build.gradle.kts` (`versionName`, and `versionCode` + 1),
`app-ios/project.yml` (`MARKETING_VERSION`, `CURRENT_PROJECT_VERSION`) and
`app-web/airkeys.app/package.json`.

## Style

- Formatting: `cargo fmt`, Prettier (`bun run format`), the IDE defaults for Kotlin and Swift,
  and `.editorconfig` for the rest.
- Comments say why, not what. Match the comments and names around you.
- One change per commit; the message says what and why (`fix: …`, `feat: …`, `docs: …`).
- Anything a user would notice gets a line in [CHANGELOG.md](CHANGELOG.md) under *Unreleased*.

## License

By contributing you agree that your work is released under the [MIT License](LICENSE).
