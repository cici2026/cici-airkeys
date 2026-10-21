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
