# Airkeys on the web

The site and the online half of the Airkeys cold wallet: it reads balances, prepares transfer
requests, shows them as a code for the phone, scans the phone's signed reply and broadcasts it.
It only ever holds addresses. A static SvelteKit build — no server — also packaged as a
browser extension.

```
src/routes/+page.svelte     the landing page (/)
src/routes/app/             the wallet (/app/)
src/extension/              the same wallet as the extension's page (hash routing)
src/lib/
  app/                      the wallet's screens: Wallet, AccountPage, Send, Connect, Settings
  chains/                   per network: balances, building requests, broadcasting
  lumen/                    showing and scanning Lumen Codes (scanning in a worker)
  protocol.ts, addresses.ts, units.ts, registry.ts   the protocol, checked against the core's vectors
  state.svelte.ts           accounts and settings, in localStorage — nothing else is kept
```

Chain access goes straight from the browser to the endpoints in Settings (defaults in
[`registry/networks.json`](../../registry/networks.json)); point them at your own nodes.
Settings also adds any EVM network from [ethereum-data.getvela.app](https://ethereum-data.getvela.app)
(its endpoint must answer for its chain id; token decimals are read from the chain), and a
Mainnet/Testnet switch keeps test coins apart from real ones.

## Commands

Needs [bun](https://bun.sh).

```sh
bun install
bun run dev            # http://localhost:5173
bun run build          # → build/: copy it to any static host, or open it from disk
bun run extension      # → extension/: chrome://extensions → Load unpacked
bun run test           # unit tests, against vectors made by the Rust core
bun run check          # types
node scripts/e2e.mjs   # the whole system (after `bun run build`; needs Chrome, Foundry, cargo)
BITCOIND=… bun scripts/chains/bitcoin.ts   # the Bitcoin client against Bitcoin Core regtest
```

`scripts/e2e.mjs` runs the built app in headless Chrome whose webcam is a video of the phone's
screen (the real Rust core), against an anvil fork of Ethereum mainnet: it connects, sends USDC
with the rest moved to a fresh account, signs, broadcasts and checks the fork's balances.
`PHONE=1` runs it all in a phone-sized browser, where the code fills the screen; `SHOTS=dir`
saves screenshots; `CHROME=…` points at Chrome.
