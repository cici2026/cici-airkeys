# Privacy, rating and compliance answers

The answers both stores ask for, and why each is true. Check them against the code when the
apps change.

## App Store Connect

- **App Privacy → Data collection:** "No, we do not collect data from this app." The app makes
  no network requests (`Network` is imported only to watch whether a path exists); camera
  frames are read in memory and never stored.
- **Privacy manifest:** `app-ios/Airkeys/Resources/PrivacyInfo.xcprivacy` declares no tracking,
  no collected data types and its required-reason APIs.
- **Export compliance:** `ITSAppUsesNonExemptEncryption = NO` (set in `project.yml`). The app
  hashes (SHA-256, SHA-512, RIPEMD-160, Keccak, HMAC, PBKDF2) and signs (ECDSA); it encrypts
  nothing.
- **Age rating:** "None" to every content question; "No" to gambling and contests → 4+. Apple
  may raise it for a finance app; accept what it gives.
- **Category:** Finance (secondary: Utilities).
- **Price:** US$29.99 in the United States; let Apple derive the other storefronts' prices
  (no regional prices).
- **Guideline 3.1.5(b), cryptocurrency wallets:** allowed only from developers enrolled as an
  organisation. Publish under MONDAY LABS LTD, not a personal account.
- **Availability:** every storefront except China mainland (wallet apps there need a licence
  Airkeys does not have). iPhone only: `TARGETED_DEVICE_FAMILY` is 1; turn off "Make this app
  available" for Mac (Designed for iPhone) and Apple Vision Pro in Pricing and Availability.
- **Camera and Bluetooth prompts:** the purpose strings are in `project.yml`. Bluetooth is only
  read, to check that it is off.

## Google Play Console

- **Data safety:** "No" to "Does your app collect or share any of the required user data
  types?" The app has no `INTERNET` permission, so it cannot send anything.
- **Permissions:** `CAMERA` (codes and SeedQR), `ACCESS_WIFI_STATE` and `ACCESS_NETWORK_STATE`
  (to check the phone is offline), `BLUETOOTH` up to Android 11 (to check it is off).
- **Content rating (IARC):** category "Utility, productivity, communication or other"; "No" to
  every content question → Everyone / PEGI 3.
- **Target audience:** 18 and over (crypto; keeps the app out of the Families policy).
- **Ads:** no.
- **App access:** see `review-notes.md`.
- **Financial features declaration:** choose cryptocurrency → non-custodial wallet (software
  wallet). Airkeys holds no funds and no keys on any server, offers no exchange, and signs only
  transfers the user prepares; Play exempts non-custodial wallets from its licensing
  requirement.
- **Category:** Finance.
- **Price:** US$29.99; accept Google's converted local prices. A paid app needs a payments
  (merchant) profile.
- **Countries:** all except China (Play is not available there anyway).
- **Developer account:** an organisation account (MONDAY LABS LTD, with a D-U-N-S number).
  Personal accounts created after November 2023 must first run a closed test with 12 testers
  for 14 days.
- **App signing:** enrol in Play App Signing; the upload key stays with us (see README).
