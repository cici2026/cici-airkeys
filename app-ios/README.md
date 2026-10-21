# Airkeys for iOS

A SwiftUI shell around the Rust core (`core/app`). It draws the core's view model, sends it
the person's actions, camera frames and radio states, and performs what it asks: camera on/off,
keep the screen on, cover the screen while it is captured or the app is inactive, haptics.

```
project.yml          XcodeGen spec: the Airkeys app and its UI tests (iOS 17+)
Airkeys/App/         AirkeysApp.swift (scene, cover), Shell.swift (the core, radios), Camera.swift
Airkeys/UI/          Screens.swift, Review.swift, Parts.swift, PhraseEditor.swift, Theme.swift
AirkeysCoreKit/      Swift package: the core's xcframework + generated Swift (both built, not committed)
AirkeysUITests/      import → connect → review → sign, on the simulator
```

Radios: Wi-Fi from `NWPathMonitor`, the cellular radio from `CTTelephonyNetworkInfo`, Bluetooth
from CoreBluetooth (the one permission prompt besides the camera's). iOS has no API for a Wi-Fi
radio that is on but not joined to a network — see [DESIGN.md](../docs/DESIGN.md#air-gap-enforced).

## Build

Needs Xcode 26, Rust (see `core/rust-toolchain.toml`) and `brew install xcodegen`.

```sh
../core/scripts/build-ios.sh     # after every change in core/: xcframework + Swift bindings
xcodegen                         # Airkeys.xcodeproj from project.yml
open Airkeys.xcodeproj
```

`AIRKEYS_IOS_SIM_ONLY=1 ../core/scripts/build-ios.sh` builds the simulator slice only. To run on
a device, set your team in `project.yml` (`DEVELOPMENT_TEAM`).

## Testing

```sh
xcodebuild -project Airkeys.xcodeproj -scheme Airkeys \
  -destination 'platform=iOS Simulator,name=iPhone 17 Pro' test
```

Debug builds only: `AIRKEYS_DELIVER=<base64 message>` hands the scan screen a message as if
scanned, and on the simulator `AIRKEYS_ASSUME_OFFLINE=1` treats the radios as off (the
simulator shares the Mac's network). Grant the camera beforehand with
`xcrun simctl privacy booted grant camera app.airkeys`.
