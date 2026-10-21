# core — the phone's brain

Rust, shared by the iOS and Android apps. Everything that decides — keys, addresses,
transactions, the protocol's checks, every screen and sentence a person sees — is here; the
shells only draw and operate the camera and screen.

```
wallet/    airkeys-wallet: pure functions, no I/O
  keys       BIP-39 (phrase, SeedQR), BIP-32 hardened; Wallet = the two account roots
  bitcoin    P2WPKH accounts, recipient parsing, previous-tx checks, BIP-143 signing
  evm        EIP-55, RLP, EIP-1559 / legacy EIP-155 transactions, ERC-20 transfer
  protocol   the JSON messages of docs/PROTOCOL.md, strict
  review     every check of PROTOCOL §7 → a Review that only it can sign
  registry   registry/networks.json, compiled in
app/       airkeys-app (lib `airkeys`): the Crux app + the runtime the shells call (uniffi)
  app        events → phases → view model; platform requests derived from the phase
  view, ops  what crosses the FFI: screens, actions, requests
  runtime    AirkeysCore: dispatch / scan camera frames / code frames to show (Lumen Code, QR)
bindgen/   the uniffi-bindgen binary (Kotlin, Swift)
e2e/       signing and recovery checked against real chain software (Node)
scripts/   build-android.sh, build-ios.sh
```

Dependencies are few and mainstream: RustCrypto (`k256`, `sha2`, `sha3`, `hmac`, `pbkdf2`,
`ripemd`), `bs58`, `bech32`, `serde`, `zeroize` in the wallet;
`crux_core`, `uniffi`, `lumen-code` (pinned commit) and `rqrr` (QR reading) in the app.

## Commands

```sh
cargo test --workspace --release
cargo clippy --workspace --all-targets -- -D warnings

# the phone's signing path as a command line (reads a request on stdin, prints the reply)
AIRKEYS_PHRASE="…" cargo run --release -p airkeys-wallet --example sign < request.json
AIRKEYS_PHRASE="…" cargo run --release -p airkeys-wallet --example sign -- accounts evm 0 1 2

cargo run --release -p airkeys-wallet --example bench        # keys, review, signing
cargo run --release -p airkeys-app --example scan_bench      # camera-frame decoding
cargo run --release -p airkeys-wallet --example vectors      # regenerates the web app's test vectors
```

Other `app` examples: `phone` (the app without a phone: films its code for a fake webcam),
`render_code` / `read_screens` (codes to PNG and back), `capacity` (bytes per code frame).

## End-to-end checks (`e2e/`)

| Script | Against |
|---|---|
| `evm.mjs` | an anvil fork of Ethereum mainnet (`EVM_FORK_URL`); needs Foundry |
| `bitcoin.mjs` | Bitcoin Core regtest (`BITCOIND=/path/to/bitcoind`) |
| `recovery.mjs` | docs/RECOVERY.md with Bitcoin Core (`BITCOIND`) and `cast` |

Run them from `core/`: `node e2e/evm.mjs`.

## Apps

`scripts/build-android.sh` (run by Gradle) and `scripts/build-ios.sh` (run by hand after a
change here) build the libraries and generate the Kotlin and Swift bindings; see the apps'
READMEs.
