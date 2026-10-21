# Contributing to Airkeys

Thanks for helping. Airkeys guards real money, so changes are judged first by whether they
keep its promises ([docs/DESIGN.md](docs/DESIGN.md)), then by whether they keep it simple.

Found a security problem? Do not open an issue — see [SECURITY.md](SECURITY.md).

## Before you start

- For anything beyond a small fix, open an issue first and describe what you want to change
  and why. It saves you work on something that cannot be merged.
- Never paste a recovery phrase that holds funds into an issue, a test or a screenshot. Use the
  standard test phrase (`abandon` eleven times, then `about`).

## The protocol does not break

[docs/PROTOCOL.md](docs/PROTOCOL.md) is a contract between phones and web apps that are
updated at different times. Within protocol version 1:

- a message the other side accepts today must still be accepted, and mean the same;
- derivation paths, addresses, signing and the checks of §7 do not change;
- the Lumen Code settings a reader relies on (colours, parity, quiet zone) do not change.

Anything else needs a new protocol version, agreed in an issue first, with both sides and the
document updated together.

## Where things go

- **Decisions live in `core/`.** Which screen comes next, every string a person reads, what may
  be signed — the Android and iOS shells only draw the core's view and perform what it asks.
  A change that makes the two phones behave differently is a bug.
- **The web app holds addresses only.** It must never need a key or a public key.
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
Foundry and Chrome; see each README. Run them when you change signing, requests or the codes.

If you change how the core parses addresses, regenerate the web app's test vectors (CI checks
that they match):

```sh
cd core && cargo run -q --release -p airkeys-wallet --example vectors \
  > ../app-web/airkeys.app/src/lib/vectors.json
```

A new version is set in four places, and CI checks they agree: `core/Cargo.toml`,
`app-android/Airkeys/app/build.gradle.kts` (`versionName`, and `versionCode` + 1),
`app-ios/project.yml` (`MARKETING_VERSION`, `CURRENT_PROJECT_VERSION`) and
`app-web/airkeys.app/package.json`.

## Style

- Formatting is automatic: `cargo fmt`, Prettier (`bun run format`), and the IDE defaults for
  Kotlin and Swift; `.editorconfig` covers the rest.
- Write for the person reading the code next: say why, not what. Match the comments and names
  around you.
- Commits: one change each, with a message that says what it does and why
  (`fix: …`, `feat: …`, `docs: …`).
- Add a line to [CHANGELOG.md](CHANGELOG.md) under *Unreleased* for anything a user would
  notice.

## License

By contributing you agree that your work is released under the [MIT License](LICENSE).
