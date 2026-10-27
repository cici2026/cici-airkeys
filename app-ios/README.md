# Airkeys for iOS

A SwiftUI shell around the Rust core (`core/app`): it draws the core's view model, sends it
the person's actions, camera frames and radio states, and does what it asks (camera on/off,
keep the screen on, protect the screens that show a phrase, haptics).

```
project.yml          XcodeGen spec: the Airkeys app and its UI tests (iOS 17+), and the one
                     source of Airkeys/Resources/Info.plist
Config/              Base.xcconfig; your signing team goes in Local.xcconfig (not committed)
Airkeys/App/         AirkeysApp.swift (scene, cover, keyboards), Shell.swift (the core, radios),
                     Camera.swift
Airkeys/UI/          Screens.swift, Review.swift, Parts.swift, PhraseEditor.swift, Theme.swift
Airkeys/Resources/   assets, Info.plist, PrivacyInfo.xcprivacy (nothing tracked or collected)
AirkeysCoreKit/      Swift package: the core's xcframework + generated Swift (both built, not committed)
AirkeysUITests/      import → connect → review → sign, on a simulator or a phone
scripts/             make-icon.swift (the app icon)
```

Radios: Wi-Fi from `NWPathMonitor`, the cellular radio from `CTTelephonyNetworkInfo`, Bluetooth
from CoreBluetooth (the one permission prompt besides the camera's). iOS has no API for a Wi-Fi
radio that is on but not joined to a network; see [DESIGN.md](../docs/DESIGN.md#air-gap-enforced).

The app refuses third-party keyboards, so a phrase or passphrase is typed on Apple's keyboard.
Screens showing one are covered in the app switcher and while the screen is recorded or
mirrored; iOS does not let apps block screenshots.

## Build

Needs a Mac with Apple silicon, Xcode 26, Rust (see `core/rust-toolchain.toml`) and
`brew install xcodegen`.

```sh
../core/scripts/build-ios.sh     # after every change in core/: xcframework + Swift bindings
xcodegen                         # Airkeys.xcodeproj from project.yml
open Airkeys.xcodeproj
```

`AIRKEYS_IOS_SIM_ONLY=1 ../core/scripts/build-ios.sh` builds the simulator slice only. To run on
a phone, copy `Config/Local.xcconfig.example` to `Config/Local.xcconfig` and set your team and a
bundle ID of your own.

## Testing

Grant the simulator's camera first, then run the UI tests:

```sh
xcrun simctl privacy booted grant camera app.airkeys
xcodebuild -project Airkeys.xcodeproj -scheme Airkeys \
  -destination 'platform=iOS Simulator,name=iPhone 17 Pro' test
```

`TEST_RUNNER_SCREENSHOTS=<dir>` saves the screenshots, codes included
(`core/app/examples/read_screens` decodes them); on a phone they go into the test result, and
every radio must really be off: the shortcut below is for the simulator only.

**Install while online.** iOS verifies a development-signed app online on its first launch
after an install; offline, it refuses to open it ("Developer App Certificate is not trusted").
Install (with Xcode, or `xcodebuild test` once) while the phone is online, open the app once,
then take the phone offline. `xcodebuild test` reinstalls its test runner on every run, so on a
real phone the UI tests that need an offline phone (import, sign) pass only up to the offline
screen; the rest is checked by hand.

Debug builds only: `AIRKEYS_DELIVER=<base64 message>` hands the scan screen a message as if
scanned, and on the simulator `AIRKEYS_ASSUME_OFFLINE=1` treats the radios as off (the
simulator shares the Mac's network). Never open a wallet that holds funds in a debug build.
