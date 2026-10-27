# Airkeys Constitution

The rules every change to Airkeys keeps. They come from [docs/DESIGN.md](../../docs/DESIGN.md),
which explains each; when the two disagree, fix whichever is wrong in the same change.

## Core Principles

### I. Addresses only

Nothing leaves the phone but addresses and signed transactions. A public key, an extended key
or a recovery phrase never does — not in a code, a screenshot, a log or a file. Every
derivation step is hardened, so one address cannot lead to another.

### II. Nothing created, nothing stored

Airkeys does not generate recovery phrases or randomness for them. A phrase is typed or
scanned, held in memory only, and gone when the wallet or the app closes. Nothing about a
wallet is written to storage.

### III. The air gap is enforced

The phone app does nothing until every radio is off, judged by what the phone observes, not
only by its switches. A radio coming back pauses everything. Unknown counts as on.

### IV. Clear signing, transfers only

The phone signs only transfers it builds itself, after showing every recipient, amount, fee and
network, and only after a deliberate hold. A request that fails a check of
[PROTOCOL §7](../../docs/PROTOCOL.md) is refused, never signed with a warning.

### V. The protocol does not break (NON-NEGOTIABLE)

[docs/PROTOCOL.md](../../docs/PROTOCOL.md) is a contract between phones and web apps updated
at different times. Within a protocol version, a message accepted today is accepted tomorrow
and means the same; derivation, addresses, signing, the checks of §7 and the code settings a
reader relies on do not change. Anything else is a new protocol version, agreed first, with
both sides and the document changed together.

### VI. One core, thin shells

Every decision — which screen, what it says, what may be signed — lives in the Rust core and
is shared by iOS and Android. The shells draw the core's view and perform what it asks; the two
phones never disagree. The web app holds addresses only and never needs a key.

### VII. Every claim is verified

A promise in the documentation has a test behind it, against independent implementations
where one exists (Bitcoin Core, Foundry, libsecp256k1). What is not verified is written down
as a limit, not left out.

## Security requirements

- Untrusted input (codes, requests, addresses, chain data) is bounded and checked before use;
  it may be refused, never crash or hang the app.
- Secrets in memory are wiped when no longer needed; screens holding them block screenshots in
  release builds.
- No analytics, no telemetry, no network access on the phone. The web app talks only to the
  endpoints in its settings and loads no analytics; only the website's own pages count visits.
- Vulnerabilities are handled privately ([SECURITY.md](../../SECURITY.md)).

## Development workflow

- The checks in [CONTRIBUTING.md](../../CONTRIBUTING.md) pass before a change is merged; CI runs
  them on every pull request.
- A change to signing, requests or the codes also runs the end-to-end checks against Bitcoin
  Core, an Ethereum fork and the web app in Chrome.
- User-visible changes are recorded in [CHANGELOG.md](../../CHANGELOG.md).

## Governance

This constitution overrides other practices. Amending it takes a pull request that states the
change and why, updates DESIGN.md to match, and bumps the version below (major: a principle
removed or redefined; minor: one added; patch: wording).

**Version**: 1.0.0 | **Ratified**: 2026-10-21 | **Last Amended**: 2026-10-21
