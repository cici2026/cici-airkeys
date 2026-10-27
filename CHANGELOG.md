# Changelog

All notable changes to Airkeys. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
versions follow [Semantic Versioning](https://semver.org/). The wire protocol has its own
version (`"airkeys": 1`, [docs/PROTOCOL.md](docs/PROTOCOL.md)), which changes only when a
message changes.

## [Unreleased]

### Added

- The phone apps (Android, iOS) on a shared Rust core: import a recovery phrase by typing or
  SeedQR, keep it in memory only, show addresses for accounts #0–#99, review and sign
  Bitcoin and EVM transfers (ETH and ERC-20), all behind an enforced air gap.
- The web app and browser extension: balances across Bitcoin and EVM chains, mainnet and
  testnet apart, transfer requests with the rest moved to a fresh account (bunker mode) or
  kept, broadcasting, and networks added from ethereum-data.getvela.app.
- Lumen Code between them, filling the screen; every message readable JSON.
- Protocol v1, the design and threat model, and recovery without Airkeys
  ([docs/](docs/)).
- The web app opens on Home: the total, split into Bunker and Exposed; what each exposed
  account holds per chain, with a Move button (everything on that chain to a fresh account, in
  one signing); every asset across accounts and chains, by worth. Prices from Chainlink feeds;
  other currencies through Vela's exchange-rate service.
- Before a first send on an EVM chain, the web app says what the account holds on the other
  EVM chains: that send reveals the key there too.

### Changed

- Every derivation path starts with Airkeys' own step `65738275'`
  (`m/65738275'/84'/0'/0'/0'/i'`, `m/65738275'/44'/60'/0'/0'/i'`), so no extended public key a
  standard wallet exported for the same phrase is an ancestor of an Airkeys account. Addresses
  derived before this change are on the old paths: move their funds first.
