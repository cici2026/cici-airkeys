# Airkeys on the web

The site and the online half of the Airkeys cold wallet: it reads balances, prepares transfer
requests, shows them as a code for the phone, scans the phone's signed reply and broadcasts it.
It only ever holds addresses. A static SvelteKit build — no server — also packaged as a
browser extension.

```
src/routes/+page.svelte     the landing page (/)
src/routes/app/             the wallet (/app/)
src/routes/+error.svelte    a page that is not there
src/extension/              the same wallet as the extension's page (hash routing)
src/lib/
  app/                      the wallet: App (header, views), Wallet, AccountPage, Send, Connect,
                            Settings, Exposure (the key hidden/exposed badge)
  ui/                       small pieces: Address, Badge, Mark, Qr, Json
  chains/                   per network: balances, building requests, broadcasting
  lumen/                    showing and scanning Lumen Codes (scanning in a worker)
  protocol.ts, addresses.ts, units.ts, registry.ts   the protocol, checked against the core's vectors
  chainData.ts              adding an EVM chain from ethereum-data.getvela.app
  state.svelte.ts           accounts and settings, in localStorage — nothing else is kept
  holdings.svelte.ts        balances and key exposure per address, for the session
  web.test.ts, vectors.json unit tests; vectors.json is made by the Rust core (`bun run vectors`)
scripts/
  extension.mjs             builds the browser extension (`bun run extension`)
  e2e.mjs                   the whole system, end to end
  chains/bitcoin.ts         the Bitcoin client against Bitcoin Core regtest
```

Chain access goes straight from the browser to the endpoints in Settings (defaults in
[`registry/networks.json`](../../registry/networks.json)); point them at your own nodes.
Settings also adds any EVM network from [ethereum-data.getvela.app](https://ethereum-data.getvela.app)
(its endpoint must answer for its chain id; token decimals are read from the chain), and a
Mainnet/Testnet switch keeps test coins apart from real ones. The pages carry a Content
Security Policy: scripts only from the site itself, and connections only over HTTPS or to a
node on this computer (`http://localhost`, `http://127.0.0.1`).

## Commands

Needs [bun](https://bun.sh), and Node.js 22.12 or later (Vite runs on it).

```sh
bun install
bun run dev            # http://localhost:5173
bun run build          # → build/: serve it from any static host over HTTPS
bun run preview        # serves build/ at http://localhost:4173
bun run extension      # → extension/: the browser extension (below)
bun run test           # unit tests, against vectors made by the Rust core
bun run check          # types
bun run format         # Prettier; `bun run format:check` only checks, as CI does
bun run vectors        # remakes src/lib/vectors.json with the core (needs cargo)
node scripts/e2e.mjs   # the whole system (after `bun run build`; needs Chrome, Foundry, cargo)
BITCOIND=… bun scripts/chains/bitcoin.ts   # the Bitcoin client against Bitcoin Core regtest
```

`build/` is plain files, but it cannot be opened from disk: browsers run no module scripts
from `file://`, and give the camera only to secure pages. Serve it over HTTPS from any static
host (`localhost` counts as secure, so `bun run preview` works too), or load the extension.

The extension runs in Chromium browsers (Chrome, Edge, Brave; version 111 or later): open
`chrome://extensions`, turn on **Developer mode**, choose **Load unpacked** and pick the
`extension/` folder. Its toolbar button opens the wallet in a tab. Firefox is not supported.

`scripts/e2e.mjs` runs the built app in headless Chrome whose webcam is a video of the phone's
screen (the real Rust core), against an anvil fork of Ethereum mainnet: it connects, sends USDC
with the rest moved to a fresh account, signs, broadcasts and checks the fork's balances.
`PHONE=1` runs it all in a phone-sized browser, where the code fills the screen; `SHOTS=dir`
saves screenshots; `CHROME=…` points at Chrome.
