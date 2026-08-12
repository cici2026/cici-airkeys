#!/usr/bin/env bash
#
# Cross-compile libairkeys_core_uniffi.so for the Android app, and generate the
# Kotlin bindings that call it.
#
#   rust/scripts/build-android.sh
#
# Output:
#   app-android/Airkeys/app/src/main/jniLibs/<abi>/libairkeys_core_uniffi.so
#   app-android/Airkeys/app/src/main/java/uniffi/airkeys_core_uniffi/…kt
#
# Both are gitignored — they are build outputs, and committing a .so is how a
# repo ends up shipping a binary nobody can reproduce.
#
# Prerequisites (one-time):
#   rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
#   cargo install cargo-ndk
set -euo pipefail

RUST_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROOT="$(dirname "$RUST_DIR")"
APP="$ROOT/app-android/Airkeys/app/src/main"
JNI_OUT="$APP/jniLibs"
KT_OUT="$APP/java"

SDK="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-$HOME/Library/Android/sdk}}"
if [ -z "${ANDROID_NDK_HOME:-}" ]; then
  ANDROID_NDK_HOME="$(ls -d "$SDK/ndk/"*/ 2>/dev/null | sort -V | tail -1)"
  ANDROID_NDK_HOME="${ANDROID_NDK_HOME%/}"
fi
if [ -z "$ANDROID_NDK_HOME" ] || [ ! -d "$ANDROID_NDK_HOME" ]; then
  echo "error: no Android NDK found (set ANDROID_NDK_HOME, or install one under $SDK/ndk)" >&2
  exit 1
fi
export ANDROID_NDK_HOME

if ! command -v cargo-ndk >/dev/null 2>&1; then
  echo "error: cargo-ndk not installed — run: cargo install cargo-ndk" >&2
  exit 1
fi

cd "$RUST_DIR"

# --platform must match minSdk in app-android/Airkeys/app/build.gradle.kts. A
# higher value here links against symbols the minimum device does not have, and
# the failure is a dlopen crash on that device only.
echo "build-android: cross-compiling for arm64-v8a, armeabi-v7a, x86_64"
cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 --platform 31 -o "$JNI_OUT" \
  build --release -p airkeys-core-uniffi

# The bindings are generated FROM THE COMPILED LIBRARY, not from source: that is
# what guarantees the checksums in the Kotlin match the ones in the .so. A
# mismatch does not fail the build — it panics on the first FFI call at run time.
echo "build-android: generating Kotlin bindings"
cargo run --quiet --release -p airkeys-core-uniffi --bin uniffi-bindgen -- generate \
  --library "target/release/libairkeys_core_uniffi.dylib" \
  --language kotlin --out-dir "$KT_OUT" 2>/dev/null ||
  cargo run --quiet --release -p airkeys-core-uniffi --bin uniffi-bindgen -- generate \
    --library "target/release/libairkeys_core_uniffi.so" \
    --language kotlin --out-dir "$KT_OUT"

echo "build-android: OK — $(find "$JNI_OUT" -name 'libairkeys_core_uniffi.so' | wc -l | tr -d ' ') ABIs in $JNI_OUT"
