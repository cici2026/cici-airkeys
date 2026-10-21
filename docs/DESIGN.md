# Airkeys — design

What Airkeys does, why each decision is what it is, what is verified, and what is not.
The wire format is in [PROTOCOL.md](PROTOCOL.md); getting funds back without Airkeys is in
[RECOVERY.md](RECOVERY.md).

## Goal and threat model

Airkeys protects Bitcoin and Ethereum-family funds held for the long term ("bunker mode")
against:

| Adversary | What Airkeys does |
|---|---|
| A **future quantum computer** that derives private keys from public keys | Never reveals a public key until an account spends; keeps every account's key independent (hardened derivation); by default moves what stays behind to a fresh account when one spends. |
| A **compromised online computer** (the web app, its browser, its endpoints) | It only ever sees addresses. It can propose transfers, but the phone rebuilds every byte from readable fields, checks them, and shows recipient, amount, chain and fee before you hold to sign. |
| A **thief of the phone** | Nothing is stored: no phrase, no key, no file. A phone whose app was closed (or that restarted) holds nothing. While the wallet is open — also in the background — the phrase is in memory, guarded only by the lock screen: close the wallet when done. |
| **Network exposure of the signing phone** | The app refuses to work while Wi-Fi, Bluetooth or mobile data is on. The Android app has no internet permission at all. |

Out of scope: a compromised phone operating system (it sees the phrase as it is typed), a
person who signs what the phone shows without reading it, a quantum computer that attacks a
transaction while it waits in the mempool (its key is public from the moment it is broadcast),
and privacy from the web app's endpoints: they see which addresses are asked about together
(see [Many accounts](#many-accounts-in-the-web-app)).

## Decisions

### Two networks
Bitcoin (native SegWit) and EVM chains (Ethereum and any chain with the same addresses, ERC-20
tokens included). Both addresses are hashes of a public key, which is the property the whole
design rests on. Every EVM chain shares one address per account; the registry lists Ethereum,
Base, Arbitrum One, OP Mainnet, Polygon, BNB Smart Chain, Avalanche C-Chain and Sepolia, and the
web app takes any other chain by hand.

### Addresses only
The phone shares `{ network, index, address }` and nothing else (PROTOCOL §4) — no public
key, no extended key, no path (the path is fixed by network and index). The web app learns the
accounts named in a reply the same way (PROTOCOL §6). This is the whole point: Ledger,
Trezor-via-MetaMask and QR wallets all hand out public keys at connection time.

### Every derivation step hardened
`m/84'/0'/0'/0'/i'` and `m/44'/60'/0'/0'/i'`. With non-hardened steps, a child private key
plus its parent's extended public key gives the parent private key — so one key broken
through its revealed public key could open every sibling. Hardened steps contain the damage to
the one account. The cost: these are not the wallets' default paths; [RECOVERY.md](RECOVERY.md)
shows how standard tools still recover them. Derivation is checked against the official
BIP-39/BIP-32 vectors and independent implementations (Foundry, @scure/bip32, Bitcoin Core
descriptors).

### Transfers only, built by the phone
A request carries readable transfer fields — recipient, amount, token, fees, nonce — never
transaction bytes, calldata or scripts (PROTOCOL §5). The phone builds every byte it signs, so
it can sign nothing but the transfer it shows: BTC, ETH (or a chain's native coin) and ERC-20
`transfer`. No message signing, no other contract calls, no NFTs. Unknown fields, versions or
types are refused, never ignored.

### Bunker mode, by default
Sending reveals the key of the account that sends. So:
- **EVM**: one request may hold up to 16 transactions. By default the web app adds, after the
  payment, transfers of every other token and the remaining native coin on that chain to a
  fresh account — one signing, one scan each way. The person can choose "Leave it here" and
  send only the amount entered. The phone refuses transfers back to the paying address.
- **Bitcoin**: a transaction spends whole coins, so the web app spends every coin of the
  account and the change goes to a fresh account. Any amount can be sent; the change never
  goes back to the paying address (the phone refuses that).
- The fresh account is the lowest account number, other than the payer, that has never been
  used. Accounts named by number (`{ "index": 1 }`) are derived by the phone itself, so the web
  app cannot redirect "your account #1".
- What remains: EVM gas that is reserved but not used is refunded to the old account (dust).

### Bitcoin inputs prove their amounts
Each input carries its whole previous transaction; the phone checks its txid, the amount and
that it pays this account. Without it, a malicious web app could lie about input amounts
across two signings and assemble a transaction with a huge hidden fee (the segwit-v0 fee
attack).

### Names and amounts the phone can trust
Chain names, native coins and well-known tokens come from [`registry/networks.json`](../registry/networks.json),
compiled into the phone (verified against the chains: symbol, decimals). A listed token whose
request metadata differs is refused; an unlisted token or EVM chain is shown with a warning and
its contract or chain id. Amounts are exact integers end to end.

### Nothing created, nothing stored
Airkeys never generates a recovery phrase — no randomness on the phone, no responsibility for
it. The phrase (typed, or from a SeedQR / CompactSeedQR / text QR) and the optional passphrase
live in the core's memory until the person closes the wallet or the app is closed; the app
keeps them while it is merely in the background. Nothing is written to storage, backups or the
keychain. Secrets are held in zeroizing buffers in Rust; the platform text fields and the FFI
copies are ordinary memory, freed but not wiped — an OS-level attacker is out of scope.

### Air gap, enforced
The core reports a "take this phone offline" screen instead of anything else until Wi-Fi,
Bluetooth, mobile data and every network connection are off. A radio coming back pauses
everything (camera off, codes hidden; a phrase stays protected) and the open wallet waits.

A switch is not proof, so each row is also checked by what the phone observes, and a radio
switched off but seen working is "still active":
- **Any network** (Wi-Fi, cellular, USB or Bluetooth tethering, Ethernet, VPN) must be absent.
- **Bluetooth** counts as off only when every signal says so — the adapter, its state and the
  system setting — and no Bluetooth Low Energy scanner exists (it exists only while the radio
  is powered). On a Redmi K40 the adapter API answered "off" while Bluetooth was on, in MIUI's
  "half-on" mode; the system setting and the scanner told the truth.
- **Mobile data**: Airplane mode, and no cellular signal (Android shows apps the signal from
  version 10; on 8 and 9, Airplane mode alone).
- **Wi-Fi**: switched off, and not scanning in the background ("Wi-Fi scanning" with Location
  on keeps the radio searching).

On Android each row that is not off opens the system screen that turns it off — for a radio
"still active", the Location scanning screen with Wi-Fi and Bluetooth scanning's own switches.

All of this works without a permission on Android. Seeing Wi-Fi scan results or nearby
Bluetooth devices themselves would need the "Nearby devices" permission, whose prompt
("find, connect to and determine the position of nearby devices") has no place in a cold
wallet; the checks above observe the same radios. iOS has no public API for the Wi-Fi radio:
it sees Wi-Fi when a Wi-Fi interface exists, the cellular radio when it is registered on a
network, Bluetooth through CoreBluetooth (a one-time permission; until granted it counts as
on), and any network through NWPathMonitor. Unknown always counts as on.

### Typing a phrase
One text field; finished words are completed from their first letters (every BIP-39 word is
unique by four) — `aban␣` becomes `abandon␣`. Completion happens only on a space or on
Continue, never mid-word, never while deleting; the field takes the core's text back inside
the same edit, so fast typing cannot race it. A numbered grid mirrors the paper and marks
unknown words; tapping a word there selects it in the field, so typing replaces it (and
completes it, like any finished word). No autocorrect, no keyboard learning; screenshots are
blocked on Android while a phrase is on screen.

### Choosing an account
The home screen picks the account number with a wheel, #0 to #99, like the iOS Timer's: a
haptic tick for every number that passes, the account chosen when the wheel comes to rest.
iOS uses the system wheel; Android draws the same drum in Compose and ticks with
`SEGMENT_FREQUENT_TICK` (Android 14+) or `CLOCK_TICK`. The core owns the range and ignores
anything outside it. Connect shows one account or all 100 at once (both networks: 200
addresses), all 100 by default.

### One core, thin shells
`core/app` is a [Crux](https://github.com/redbadger/crux) app: events in, a view model and
requests out, no I/O. Every decision — which screen, what is allowed, what may be signed — and
everything a person reads about the wallet, a request and the checks comes from it, so iOS and
Android cannot disagree; the shells add only fixed labels and explanations, the same on both. The runtime around it runs Lumen Code (send and receive)
and QR decoding in Rust, and answers every call synchronously: no callbacks, no threads.
Platform requests are notifications only (camera on, keep screen on, block screenshots,
haptics), and they are derived from the current screen by one function, so no path can leave
the camera on or a phrase unprotected. The shells are about 1,300–1,450 lines each.

### The channel
[Lumen Code](https://github.com/atshelchin/lumen-code) (fountain-coded colour codes, MIT, pinned
by commit in both Rust and JavaScript). Messages are compact JSON, readable with any Lumen
Code reader — so anyone can check that a code from the phone holds no key.

A code fills the screen: the bigger the code, the bigger its cells in the camera's image, and
the faster and farther away it reads. On a phone it runs edge to edge from the title to the
button, taller than wide (Lumen codes take any shape up to 2.4 : 1), and so carries about
twice what a square of the same cells does. Its cells (4 colours, 22 % parity) are large
because a laptop webcam across a desk has to read them: 44, 52 or 60 across, the first with
which a reader catching every frame has the message within four — counted by playing the
sender's own schedule into a receiver. In a simulated 720p webcam (blur, sensor noise), codes
44 and 52 across still read when 200–260 px wide in its image; 72 and 84 across needed 340 px.
A signed reply takes 44 across and two to four frames; all 100 accounts (8.8 KB compressed),
60 across and about ten (a second).

The web app shows a request as large as the window allows: a square of up to 796 px on a
computer (what a phone held upright frames best), the whole screen in a phone's browser. A
phone's camera close to a screen reads smaller cells than a webcam, so the web takes the
largest cells, 52 to 84 across, with which the phone reads the request within two frames.

### The web app
A static site — no backend. It reads chains directly from the browser (endpoints configurable
per chain, including EVM chains added by hand), keeps only addresses and settings in
`localStorage`, and builds without a server: put `build/` on any static host (over HTTPS, which
the camera needs), or load the extension build. Dependencies: Svelte, paulmillr's audited
`@noble`/`@scure` primitives, Lumen Code, a tiny QR encoder.

### Many accounts in the web app
Public endpoints limit how fast one visitor may ask, so balances are read in bulk:
- **EVM**: one JSON-RPC batch carries the balances, token balances and nonces of several
  addresses (50 calls per batch, batches 300 ms apart per chain); a nonce above zero marks the
  key as exposed on every EVM chain. 100 accounts take about 13 requests per chain.
- **Bitcoin**: Esplora answers one address per request, so accounts are read in order, three
  at a time, and reading stops after 10 unused accounts in a row (the usual gap limit) — bunker
  mode fills the lowest unused account, so used accounts sit at the low numbers. The rest are
  marked "not checked", with a button to check them.
- A server that answers HTTP 429 is asked again after 1, 3 and 9 s. Bitcoin falls back from
  mempool.space to Blockstream's Esplora when the first does not answer (unless Settings name
  an endpoint).
- Accounts that hold nothing fold into one line; the first account always shows.

The price is privacy, not keys: the endpoints see these addresses asked about together and can
link them. Pointing Settings at your own nodes removes that; connecting one account at a time
reduces it.

The fresh account a send moves the rest to is the lowest connected account the web app has
read and seen never used — never one it has not seen.

### Mainnet and testnet, never mixed
A switch in the web app shows either the real networks or the test networks (Sepolia, and any
test network added), never both: balances, sends and totals follow it, an amber strip marks
test mode, and Bitcoin (mainnet only) steps aside. The phone labels a listed test network
"Testnet" on its review. Key exposure counts every chain either way: a signature on a test
network reveals the same key.

### Adding a network
Settings searches the chains of [ethereum-data.getvela.app](https://ethereum-data.getvela.app)
(2,897 of them): picking one fills in its name, coin and explorer, offers its stablecoins, and
marks a test network. Nothing from the list is trusted blindly: the endpoint chosen must answer
with the chain id asked for, and each token's decimals are read from the chain. Chains can still
be entered by hand.

### Fees on layer 2
On OP-stack chains (Base, OP Mainnet) every transaction also pays a layer-1 data fee from the
same balance — on Base about six times its gas — and on Arbitrum a plain transfer needs more
than 21 000 gas. The web app reads both from the chain (the fee oracle's upper bound, with half
again as margin; the gas estimate) so the last transfer of a bunker sweep is not refused for
want of fees.

## What is verified

| Claim | How |
|---|---|
| BIP-39 and BIP-32 (hardened) | Official vectors (`core/wallet/src/keys.rs`) |
| Addresses on the Airkeys paths | Foundry `cast`, @scure/bip32, Bitcoin Core descriptors (`core/wallet/tests/derivation.rs`, `core/e2e/recovery.mjs`) |
| EVM signing | EIP-155 example byte-for-byte; mainnet fork executes USDC, USDT (non-standard), ETH, EIP-1559 and legacy transactions (`e2e/evm.mjs`) |
| Bitcoin signing | Bitcoin Core 31.1 regtest accepts and mines a 3-input spend to P2TR, P2PKH, P2SH and change (`e2e/bitcoin.mjs`) |
| Recovery without Airkeys | Each recipe of RECOVERY.md, run with Bitcoin Core 31.1 and `cast` 1.5 (its root-key script taken from the document), derives the phone's addresses for three accounts per network, with and without a passphrase; a Bitcoin Core wallet built from the descriptor sweeps coins on regtest (`e2e/recovery.mjs`) |
| Every refusal in PROTOCOL §7 | One check each across the e2e scripts and unit tests (wrong wallet, checksum, dust, fee attack, nonce gap, token metadata, …) |
| An independent review | A separate audit of the core, the protocol and the web app's parser and builders: signing re-verified with libsecp256k1 and alloy, 229,000 mutated camera frames and 3,000 crafted code packets without a panic. Its findings — a crash and a minutes-long hang from crafted addresses, paying `from` back under another spelling, fee warnings weighed against change, an underpaying Bitcoin build, a decimal comma read as a thousands separator, layer-2 fees — are fixed, each with a test (`wallet/tests/adversarial.rs`, `scripts/chains/bitcoin.ts`, `web.test.ts`) |
| A real camera, both ways | A Redmi K40's camera read a request code from a MacBook's screen in about 3 seconds (15 frames/s, ~11 ms per frame) and showed exactly the request. A MacBook's FaceTime HD camera (1080p), in the web app, read all 100 accounts (200 addresses) from the K40's full-screen code, held by hand under glare, about a second after it first saw the code |
| The air gap on real phones | Redmi K40: Bluetooth on in MIUI's half-on mode detected (the API alone said off), Wi-Fi scanning detected, every row off with Airplane mode, Bluetooth and Location off. iPhone 11: Wi-Fi, Bluetooth and a network shown as on |
| App flows | The real FFI object driven through import (a word fixed in the middle too), SeedQR/CompactSeedQR images, passphrases, radios and "still active", the account range, connect (one account and all 100, read back from the code), review, signing (`core/app/tests/flows.rs`); the iOS UI tests fix a word from the grid and sign |
| Codes on screen are readable | Screenshots of the full-screen code on the Android emulator and the iOS simulator decode to exactly the JSON shown — all 100 accounts (17 KB) from 11 screenshots on each, the signed reply from 2 |
| Code sizes | A simulated 720p webcam (blur, sensor noise) filming codes 200–420 px wide in its image, with the real scanner: which grids read, and in how many frames (`core/app/examples/capacity.rs` counts the frames for real messages); `core/app/tests/flows.rs` checks the code's shape for an area |
| The account wheel | Android emulator: a slow drag and a fling settle on the number in the band, the addresses switch to that account, and the system vibration log shows one tick per number passed; iOS simulator UI test turns the wheel to 7 and back |
| The web app's Bitcoin client | Against Bitcoin Core regtest behind an Esplora shim: balances, exposure, fee as shown, change to a fresh account, send-all, four refusals, and the phone refusing a forged input (`scripts/chains/bitcoin.ts`) |
| The whole system | Headless Chrome running the web app, its webcam a video of the phone core's screen: all 200 addresses read through the camera, a request read from the web page's pixels, signed, reply scanned, broadcast to a mainnet fork, executed — in a desktop window and in a phone-sized browser (`app-web/airkeys.app/scripts/e2e.mjs`, `PHONE=1`) |
| Many accounts against public endpoints | Measured, not a test: 200 addresses read in 72 s with 125 requests on the default endpoints, every account resolved (most of the time is Arbitrum's public node answering 429) |
| Speed on a phone | Xiaomi Snapdragon 870: open a wallet 2.7 ms, an address about 0.19 ms (all 200 for Connect: about 40 ms), check a 16-transaction request 3.3 ms, sign it 3.3 ms; scan a 1280×960 camera frame 2.3–13.5 ms. Measured with four networks, before Airkeys dropped two; the work per address and request is unchanged |

## Limits

- **Cameras are tested on one pair of devices** (Redmi K40, MacBook), both ways. Try your own
  phone and computer before trusting them with funds.
- The iOS camera path has not read a code on a device yet (the simulator has none); the app
  runs on an iPhone 11 (iOS 26.5.2). Android ran on a Redmi K40 (Android 13), camera included.
- Haptics were checked in the Android emulator's vibration log and are the system wheel's own on
  iOS; how they feel was not judged on a real phone.
- Bitcoin: accounts are native SegWit only, mainnet only; spending is all-coins (no coin
  control), so inscriptions or runes held by an account would be spent as plain coins.
- Reading many accounts on free public endpoints is slow where they throttle (Arbitrum) and
  links the addresses for them; run your own nodes for both.
- The layer-2 fee reserve is computed from live chain data but has not executed a sweep on a
  real layer-2 chain (a local fork does not charge layer-1 fees).
- iOS cannot see a Wi-Fi radio that is on but not joined to a network.
- Releases: the Android release is not minified and is signed with the debug key; the iOS app is
  not distributed. Set up your own signing.
