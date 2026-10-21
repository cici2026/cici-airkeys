# Airkeys

A cold wallet for a phone that stays offline. It shows the world **addresses — never a
public key** — so the funds behind them stay out of reach, even of a quantum computer.

- **Phone apps** (Android, iOS) hold your recovery phrase in memory only, refuse to run while
  any radio is on, and sign transfers only, after showing every recipient, amount and fee.
- **Web app** (a static site, also a browser extension) reads balances, prepares transfers and
  broadcasts them. It only ever holds addresses.
- They talk through animated codes ([Lumen Code](https://github.com/atshelchin/lumen-code)):
  one screen shows, the other camera reads. Every message is readable JSON
  ([protocol](docs/PROTOCOL.md)).

Networks: **Bitcoin** (BTC) and **Ethereum and EVM chains** (ETH and ERC-20 tokens on
Ethereum, Base, Arbitrum, OP, Polygon, BNB, Avalanche, Sepolia, and any chain you add).

## Why

A quantum computer that breaks elliptic curves recovers a private key from its public key.
Bitcoin and Ethereum addresses are hashes of public keys: an address that has never sent
keeps its key unknown. But every mainstream way of connecting a hardware wallet hands the
computer public keys — Ledger with every address, MetaMask by taking Trezor's extended key,
QR wallets by syncing extended keys. Airkeys sends addresses only, derives every account on
a fully hardened path, and when you spend, moves what stays behind to a fresh account in the
same signing. → [Design](docs/DESIGN.md)

## Repository

```
docs/            PROTOCOL.md (the wire protocol), DESIGN.md (decisions, threat model, limits),
                 RECOVERY.md (getting your funds back without Airkeys)
registry/        networks.json — chains and tokens, shared by the phone's core and the web app
core/            Rust: the phone's whole brain, shared by iOS and Android
  wallet/          keys, addresses, transactions, protocol checks — pure functions
  app/             the Crux app (every screen and rule) + the runtime the shells call (uniffi)
  e2e/             signing and recovery checked against Bitcoin Core and an Ethereum fork
app-android/     Jetpack Compose shell
app-ios/         SwiftUI shell
app-web/         SvelteKit static site: landing page, web app, browser extension
```

The shells draw the core's view and perform what it asks (camera, screen); every decision —
which screen, what it says, what may be signed — is in `core/`.

## Build

| What | Needs | Command |
|---|---|---|
| Core tests | Rust (pinned in `core/rust-toolchain.toml`) | `cd core && cargo test --workspace --release` |
| Android app | JDK 17+, Android SDK + NDK, `cargo install cargo-ndk` | `cd app-android/Airkeys && ./gradlew :app:assembleRelease` |
| iOS app | Xcode 26, `brew install xcodegen` | `core/scripts/build-ios.sh && cd app-ios && xcodegen && open Airkeys.xcodeproj` |
| Web app | [bun](https://bun.sh) (or npm) | `cd app-web/airkeys.app && bun install && bun run build` → `build/` |
| Browser extension | the same | `bun run extension` → `extension/` (load unpacked) |

Self-hosting the web app is copying `build/` to any static host — or opening it from disk.
Its chain endpoints are in Settings; point them at your own nodes.

## Verification

Everything below runs, and passes, on this tree:

```sh
cd core
cargo test --workspace --release        # 47 tests: BIP-39/32, addresses vs. independent tools, hostile requests,
                                        # protocol checks, app flows through the real FFI object
cargo clippy --workspace --all-targets -- -D warnings
node e2e/evm.mjs                        # anvil fork of mainnet: USDC, USDT, ETH, EIP-1559 + legacy
BITCOIND=… node e2e/bitcoin.mjs         # Bitcoin Core regtest: P2TR/P2PKH/P2SH outputs, fee-attack refusal
BITCOIND=… node e2e/recovery.mjs        # docs/RECOVERY.md's recipes find the same accounts

cd ../app-web/airkeys.app
bun run test                            # web logic vs. the core's vectors
BITCOIND=… bun scripts/chains/bitcoin.ts   # the web app's Bitcoin client against Bitcoin Core
node scripts/e2e.mjs                    # the whole system: web app in Chrome with a fake webcam showing
                                        # the phone's real screen, against an Ethereum fork
cd ../../app-ios && xcodebuild -scheme Airkeys -destination 'platform=iOS Simulator,name=iPhone 17 Pro' test
```

See [docs/DESIGN.md § What is verified](docs/DESIGN.md#what-is-verified) for what each check
proves and what remains untested.

## License

MIT.
