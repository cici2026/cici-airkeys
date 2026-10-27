# Store submission

What Google Play and the App Store ask for, ready to paste or upload, and how a release is made.

| File | Use |
|---|---|
| `app-store/listing-en.md`, `app-store/listing-zh-Hans.md` | name, subtitle, promotional text, keywords, description, URLs |
| `google-play/listing-en-US.md`, `google-play/listing-zh-CN.md` | name, short and full description |
| `review-notes.md` | notes for App Review and Play's "App access" |
| `privacy-and-compliance.md` | App Privacy, Data safety, ratings, export compliance, pricing, availability |
| `app-store/icon-1024.png`, `google-play/icon-512.png` | store icons |
| `google-play/feature-graphic-{en,zh}.png` | 1024×500 feature graphic (source: `feature-graphic.html`) |
| `*/screenshots/{en,zh}/` | raw screenshots: App Store 1320×2868 (6.9-inch), Play 1080×2160 |
| `*/framed/{en,zh}/` | the same with captions, for upload (source: `screenshot-frame.html`) |

## Before the first submission

- Both developer accounts in the name of MONDAY LABS LTD, as an organisation (a D-U-N-S number
  for each). Apple accepts cryptocurrency wallets only from organisations (guideline
  3.1.5(b)). On Apple, accept the Paid Apps Agreement and add banking and tax details; on
  Google, set up a payments profile.
- airkeys.app deployed, with `/privacy/` and `/terms/` reachable: both stores link to them.
- A review phrase holding a little Sepolia ETH, and a screen recording of a transfer, for
  `review-notes.md`.

## Google Play

1. Create an upload key once, and keep it with a backup offline:
   `keytool -genkeypair -keystore airkeys-upload.jks -alias upload -keyalg RSA -keysize 4096 -validity 10000`.
   Set `airkeysKeystore`, `airkeysKeystorePassword`, `airkeysKeyAlias` and
   `airkeysKeyPassword` in `~/.gradle/gradle.properties` (see
   [app-android/README.md](../app-android/README.md#build)), and enrol in Play App Signing.
2. `cd app-android/Airkeys && ./gradlew :app:bundleRelease` →
   `app/build/outputs/bundle/release/app-release.aab`.
3. After the first upload, copy the SHA-256 of the app signing certificate from Play Console →
   Test and release → App integrity and publish it in the main README, so anyone can check an
   APK they were given.

## App Store

1. `core/scripts/build-ios.sh`, then `xcodegen` in `app-ios/`, with the MONDAY LABS team in
   `Config/Local.xcconfig` and the bundle ID left at `app.airkeys`.
2. In Xcode: Product → Archive (Release), then upload from the Organizer.
3. Fill in the listing, the privacy answers and the review notes; attach the 6.9-inch framed
   screenshots.

## Screenshots and graphics

The raw screenshots come from the debug builds, driven through the same flow in English and
Chinese with the test phrase and a sample request:

```sh
# iOS: an iPhone 17 Pro Max simulator (1320×2868), with a clean, offline-looking status bar
xcrun simctl status_bar booted override --time 9:41 --batteryState discharging --batteryLevel 100 \
  --dataNetwork hide --wifiMode failed --wifiBars 0 --cellularMode notSupported
xcrun simctl privacy booted grant camera app.airkeys
cd app-ios && TEST_RUNNER_SCREENSHOTS=/tmp/shots xcodebuild -project Airkeys.xcodeproj \
  -scheme Airkeys -destination 'platform=iOS Simulator,name=iPhone 17 Pro Max' test \
  -only-testing:AirkeysUITests/FlowTests/testImportConnectReviewSign \
  -only-testing:AirkeysUITests/FlowTests/testChinese

# Android: an emulator with the debug APK installed
ANDROID_SERIAL=emulator-5554 store/capture-android.sh
```

Copy the iOS shots into `app-store/screenshots/` as `01-welcome` (`i01-welcome`,
`zh01-welcome`), `02-home` (`i04-home`, `zh03-home`), `03-review` (`i05-review`,
`zh06-review`) and `04-code` (one `code-accounts-*`, `zh05-code`). Then
`node store/render.mjs` frames all of them, renders the feature graphics and copies the icons.
The captions are in `render.mjs`.

## Releasing a version

1. Set the version (semantic, like `0.2.0`) in `core/Cargo.toml`,
   `app-android/Airkeys/app/build.gradle.kts` (`versionName`), `app-ios/project.yml`
   (`MARKETING_VERSION`) and `app-web/airkeys.app/package.json`; CI checks they agree. Raise
   `versionCode` and `CURRENT_PROJECT_VERSION` by one: each store refuses a build number it has
   seen.
2. Move the CHANGELOG's Unreleased entries under the new version and date. Its user-facing
   lines become the stores' "What's New".
3. Build both apps from a clean checkout of that commit, upload them, and tag it once both
   builds are uploaded: `git tag -a v0.2.0 -m v0.2.0 && git push origin v0.2.0`. The tag is
   how anyone matches a store version to its source. Pushing it also makes the GitHub release,
   with the browser extension attached (`.github/workflows/release.yml`); the site's download
   button always points at the latest one.
4. Deploy the web companion from the same commit.
5. After a store approves for the first time, set its listing URL in
   `app-web/airkeys.app/src/lib/site/site.ts` (`stores.googlePlay`, `stores.appStore`); the
   site then links to it instead of saying "Coming soon".

A phone updates only while online, so the What's New of every release ends with the same
instruction: close the wallet, go online, update, and take the phone offline again before
opening a wallet.
