#!/bin/sh
# The core for iOS: app-ios/AirkeysCoreKit/Artifacts/AirkeysFFI.xcframework (device and
# simulator, arm64) and the Swift bindings in app-ios/AirkeysCoreKit/Sources/AirkeysCore.
# Run it after every change under core/; Xcode never builds the Rust.
set -eu
CORE="$(cd "$(dirname "$0")/.." && pwd)"
KIT="$CORE/../app-ios/AirkeysCoreKit"
export IPHONEOS_DEPLOYMENT_TARGET=17.0
cd "$CORE"

cargo build --release -p airkeys-app
cargo run --release -q -p airkeys-bindgen --bin uniffi-bindgen -- generate \
  --library target/release/libairkeys.dylib --language swift --out-dir target/bindings/swift

if [ "${AIRKEYS_IOS_SIM_ONLY:-0}" != 1 ]; then
  cargo build --release -p airkeys-app --target aarch64-apple-ios
fi
cargo build --release -p airkeys-app --target aarch64-apple-ios-sim

HDRS="target/xcframework-headers"
rm -rf "$HDRS" "$KIT/Artifacts/AirkeysFFI.xcframework"
for SLICE in ios ios-sim; do
  mkdir -p "$HDRS/$SLICE"
  cp target/bindings/swift/AirkeysFFI.h "$HDRS/$SLICE/"
  cp target/bindings/swift/AirkeysFFI.modulemap "$HDRS/$SLICE/module.modulemap"
done
ARGS=""
if [ "${AIRKEYS_IOS_SIM_ONLY:-0}" != 1 ]; then
  ARGS="-library target/aarch64-apple-ios/release/libairkeys.a -headers $HDRS/ios"
fi
mkdir -p "$KIT/Artifacts" "$KIT/Sources/AirkeysCore"
# shellcheck disable=SC2086
xcodebuild -create-xcframework $ARGS \
  -library target/aarch64-apple-ios-sim/release/libairkeys.a -headers "$HDRS/ios-sim" \
  -output "$KIT/Artifacts/AirkeysFFI.xcframework" >/dev/null
cp target/bindings/swift/AirkeysCore.swift "$KIT/Sources/AirkeysCore/AirkeysCore.swift"
echo "AirkeysFFI.xcframework and AirkeysCore.swift updated"
