# Notes for app review

Paste into App Store Connect (App Review Information → Notes) and Play Console (App content →
App access → "All or some functionality is restricted", with these instructions).

Before each submission, replace `<REVIEW PHRASE>` with the 12 words of a phrase kept for
review only, whose account #0 holds a little Sepolia ETH (Ethereum testnet, no value). Check
it still does. Never put a phrase that holds real funds here. Attach a screen recording of the
whole flow (step 4 needs a computer with a camera, which reviewers may not have at hand).

```
Airkeys is a cold wallet: it signs transfers on a phone that stays offline, and works only
while the phone has no network connection. This is deliberate. A companion website,
https://airkeys.app/app/, opened on a computer, reads balances and prepares transfers; the
phone and the website exchange animated codes through their cameras.

Airkeys does not create wallets, holds no funds on any server, has no account and collects
no data. The recovery phrase stays in memory and is forgotten when the wallet is closed.

To test on the phone alone:
1. Turn on Airplane Mode and turn off Wi-Fi and Bluetooth. Until then the app shows only
   "Take this phone offline".
2. Tap "Enter recovery phrase" and type this test phrase (it holds nothing):
   abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about
   Typing the first four letters of a word and a space completes it.
3. Tap Continue, leave the passphrase empty, tap "Open wallet". The account screen shows a
   Bitcoin and an Ethereum address. "Connect" → "Show code" shows the animated code that the
   website reads. "Close wallet" forgets the phrase.

To test a transfer (needs a computer with a camera; a screen recording is attached):
4. On the computer, open https://airkeys.app/app/ and choose Testnet at the top. On the phone,
   open a wallet with this review phrase, which holds Sepolia test ETH (no value):
   <REVIEW PHRASE>
   Phone: Connect → Show code. Website: "Scan phone", and hold the phone up to the camera.
5. Website: open account #0, Send next to ETH, enter any address and a small amount, then
   Review → "Show to the phone". Phone: "Scan request" and point it at that code. The phone
   shows the recipient, amount and network fee; hold the yellow button to sign. It then shows
   a code. Website: "Scan the phone's reply" and hold the phone up to the camera; the website
   broadcasts the transfer and shows Sent.

The source code and the protocol are public: https://github.com/mondaylabsltd/airkeys
```
