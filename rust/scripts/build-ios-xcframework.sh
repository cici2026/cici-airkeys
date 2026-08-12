#!/usr/bin/env bash
#
# Build the iOS XCFramework + Swift sources for the AirkeysCoreKit package.
#
#   rust/scripts/build-ios-xcframework.sh
#
# Builds the host dylib, generates the Swift bindings from it, cross-compiles
# the static library for device + simulator, and assembles
# app-ios/AirkeysCoreKit/Artifacts/AirkeysCoreFFI.xcframework.
#
# macOS only. Requires: cargo, xcodebuild, and the rustup targets
# aarch64-apple-ios + aarch64-apple-ios-sim.
set -euo pipefail

RUST_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROOT="$(dirname "$RUST_DIR")"
KIT_DIR="$ROOT/app-ios/AirkeysCoreKit"
cd "$RUST_DIR"

if [ "$(uname -s)" != "Darwin" ]; then
  echo "build-ios-xcframework: macOS only (needs xcodebuild and the Apple toolchains)" >&2
  exit 1
fi

echo "build-ios-xcframework: building the host dylib"
cargo build --release -p airkeys-core-uniffi
LIB_FILE="target/release/libairkeys_core_uniffi.dylib"

# From the compiled library, not from source — see build-android.sh on why.
echo "build-ios-xcframework: generating Swift bindings"
cargo run --quiet --release -p airkeys-core-uniffi --bin uniffi-bindgen -- generate \
  --library "$LIB_FILE" --language swift --out-dir bindings/swift

echo "build-ios-xcframework: building device + simulator static libraries"
cargo build --release -p airkeys-core-uniffi --target aarch64-apple-ios
cargo build --release -p airkeys-core-uniffi --target aarch64-apple-ios-sim

# Each xcframework slice needs its own copy of the C headers, with the modulemap
# renamed to the clang-conventional module.modulemap.
HDRS="target/xcframework-headers"
rm -rf "$HDRS"
for SLICE in ios ios-sim; do
  mkdir -p "$HDRS/$SLICE"
  cp bindings/swift/*.h "$HDRS/$SLICE/"
  cp bindings/swift/*.modulemap "$HDRS/$SLICE/module.modulemap"
done

OUT="$KIT_DIR/Artifacts/AirkeysCoreFFI.xcframework"
rm -rf "$OUT"
mkdir -p "$(dirname "$OUT")"
xcodebuild -create-xcframework \
  -library target/aarch64-apple-ios/release/libairkeys_core_uniffi.a \
  -headers "$HDRS/ios" \
  -library target/aarch64-apple-ios-sim/release/libairkeys_core_uniffi.a \
  -headers "$HDRS/ios-sim" \
  -output "$OUT"

# The Swift source IS committed (unlike the .so on Android): Xcode needs it in
# the package to compile, and it is readable text that reviews like code.
mkdir -p "$KIT_DIR/Sources/AirkeysCore"
cp bindings/swift/airkeys_core_uniffi.swift "$KIT_DIR/Sources/AirkeysCore/"

echo "build-ios-xcframework: OK — $OUT"
