# Airkeys for Android

A Jetpack Compose shell around the Rust core (`core/app`): it draws the core's view model,
sends it the person's actions, camera frames and radio states, and does what it asks (camera
on/off, keep the screen on, block screenshots with `FLAG_SECURE`, haptics).

```
Airkeys/app/src/main/java/app/airkeys/
  MainActivity.kt   the activity: back button, camera permission, debug test hook
  Shell.kt          the core (one AirkeysCore per process) and what it asks of the window
  Radios.kt         whether the phone is offline: switches, and what the radios are seen doing
  Camera.kt         CameraX analysis frames (Y′CbCr) → the core
  ui/               Compose: Screens.kt, Parts.kt (shared pieces), Code.kt (the animated code),
                    Wheel.kt (the account wheel), Icons.kt, Theme.kt
```

The app has no `INTERNET` permission. It uses `CAMERA`; `ACCESS_WIFI_STATE` and
`ACCESS_NETWORK_STATE`, to see that Wi-Fi and every network are off; and `BLUETOOTH` up to
Android 11, to read the adapter. All but the camera are granted at install. Nothing is backed
up or transferred to a new phone. Minimum Android 8 (API 26).

## Build

Needs JDK 17 and 21 (Gradle runs on 21 and fetches it if missing), the Android SDK with an NDK,
Rust (see `core/rust-toolchain.toml`) and `cargo install cargo-ndk`.

```sh
cd Airkeys
./gradlew :app:installDebug          # builds the core (core/scripts/build-android.sh), then the app
./gradlew :app:lintDebug             # Android lint
./gradlew :app:assembleRelease       # app/build/outputs/apk/release/
```

Gradle runs the Rust build first (task `cargoBuild`, up to date when nothing in `core/`
changed); `-PairkeysSkipRust` skips it. `AIRKEYS_ABIS="arm64-v8a"` builds one ABI. The core's
libraries go to `app/build/rust/jniLibs` and its Kotlin bindings to `app/build/generated/uniffi`.

Release builds are signed with your key when `airkeysKeystore`, `airkeysKeystorePassword`,
`airkeysKeyAlias` and `airkeysKeyPassword` are set in `~/.gradle/gradle.properties` (or as
`AIRKEYS_KEYSTORE`, `AIRKEYS_KEYSTORE_PASSWORD`, `AIRKEYS_KEY_ALIAS`, `AIRKEYS_KEY_PASSWORD`);
without them the APK is unsigned. R8 is off so the bytecode stays as written, which is easier
to audit.

## Testing

**Debug builds are for testing only**: they accept injected messages and, unlike release
builds, allow screenshots of the phrase and passphrase. Never open a wallet that holds funds in
one.

A debug build takes a message as if scanned, so flows run without a camera:

```sh
# a request, for example from the web app's "What this code says", or core/e2e
adb shell am start -n app.airkeys/.MainActivity --es airkeys.deliver "$(base64 < request.json | tr -d '\n')"
```

Until every radio is off, the app shows only its offline screen. On an emulator:

```sh
adb shell cmd connectivity airplane-mode enable
adb shell svc wifi disable
adb shell cmd bluetooth_manager disable
adb shell settings put global wifi_scan_always_enabled 0   # Wi-Fi scanning keeps the radio active
```
