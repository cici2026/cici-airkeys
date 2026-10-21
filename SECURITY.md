# Security

Airkeys holds the keys to people's money. Please report anything that could cost someone
their funds or their privacy privately, and give us time to fix it before it is public.

## Reporting a vulnerability

Use GitHub's private reporting: **Security → Report a vulnerability** on
[mondaylabsltd/airkeys](https://github.com/mondaylabsltd/airkeys/security/advisories/new).
Do not open a public issue.

Include what you found, how to reproduce it (a request message, a screenshot of a code, the
steps on the phone) and what an attacker gains. **Never send a recovery phrase that holds
funds** — use the standard test phrase (`abandon` eleven times, then `about`) or a fresh one.

You will get an answer within a week. Fixes are released as soon as they are ready; the
advisory follows, crediting you unless you prefer otherwise.

## What counts

Anything that breaks a promise in [docs/DESIGN.md](docs/DESIGN.md), for example:

- a key, the recovery phrase or a public key leaving the phone — through a code, a
  screenshot, a log or storage;
- the phone signing anything other than the transfers it showed, or showing them wrongly
  (recipient, amount, fee, network);
- a request that passes the checks of [PROTOCOL §7](docs/PROTOCOL.md) when it should not;
- the phone running while a radio is on;
- the web app preparing or broadcasting something other than what its review showed;
- a recipe in [docs/RECOVERY.md](docs/RECOVERY.md) that does not find the same accounts.

Out of scope: a phone or computer already controlled by an attacker, physical access to an
unlocked phone with the wallet open, and approving a request without reading it.

## Status

Airkeys is young (0.x) and has had no third-party audit. Its own review and tests are
described in [docs/DESIGN.md § What is verified](docs/DESIGN.md#what-is-verified). Only the
latest release is supported.
