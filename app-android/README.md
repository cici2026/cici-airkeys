# Airkeys for Android

A Jetpack Compose shell around the Rust core (`core/app`). It draws the core's view model,
sends it the person's actions, camera frames and radio states, and performs what it asks:
camera on/off, keep the screen on, block screenshots (`FLAG_SECURE`), haptics.

```
Airkeys/app/src/main/java/app/airkeys/
  MainActivity.kt   the activity: radios, back button, debug test hook
  Shell.kt          the core (one AirkeysCore), requests, radio watching
  Camera.kt         CameraX analysis frames (YUV) → the core
  ui/               Compose: Screens.kt (one function per screen), Parts.kt, Code.kt, Theme.kt
```

No `INTERNET` permission. Permissions: `CAMERA`, `ACCESS_WIFI_STATE` and `BLUETOOTH` (read
the radios' state; `BLUETOOTH` only up to Android 11, where reading the adapter needs it).
Minimum Android 8 (API 26).

## Build

Needs JDK 17+, the Android SDK with an NDK, Rust (see `core/rust-toolchain.toml`) and
`cargo install cargo-ndk`.

```sh
cd Airkeys
./gradlew :app:installDebug          # builds the core (core/scripts/build-android.sh), then the app
./gradlew :app:assembleRelease       # app/build/outputs/apk/release/
```

Gradle runs the Rust build before every build (task `cargoBuild`, up to date when nothing in
`core/` changed); `-PairkeysSkipRust` skips it. `AIRKEYS_ABIS="arm64-v8a"` builds one ABI.

The release build is signed with the debug key as a placeholder: configure your own
`signingConfig` before distributing it.

## Testing

Debug builds accept a message as if scanned, so flows run without a camera:

```sh
adb shell am start -n app.airkeys/.MainActivity --es airkeys.deliver "$(base64 < request.json | tr -d '\n')"
```

The emulator's radios are on by default; turn on airplane mode and Wi-Fi/Bluetooth off
(`adb shell cmd connectivity airplane-mode enable`, `adb shell svc wifi disable`) — the app shows
nothing else until they are off. Debug builds allow screenshots; release builds block them while
the phrase or passphrase is on screen.
