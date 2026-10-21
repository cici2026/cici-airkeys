#!/bin/sh
# The core for Android: app-android/Airkeys/app/src/main/jniLibs/<abi>/libairkeys.so (cargo-ndk)
# and the Kotlin bindings in app-android/Airkeys/app/build/generated/uniffi. Gradle runs this
# before each build (task cargoBuild; up to date when nothing under core/ changed).
#   one time: cargo install cargo-ndk
set -eu
CORE="$(cd "$(dirname "$0")/.." && pwd)"
ANDROID="$CORE/../app-android/Airkeys"
LOCAL_SDK="$(sed -n 's/^sdk.dir=//p' "$ANDROID/local.properties" 2>/dev/null || true)"
SDK="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-${LOCAL_SDK:-$HOME/Library/Android/sdk}}}"
if [ -z "${ANDROID_NDK_HOME:-}" ]; then
  ANDROID_NDK_HOME="$(ls -d "$SDK/ndk/"*/ 2>/dev/null | sort -V | tail -1)"
  export ANDROID_NDK_HOME
fi
ABIS="${AIRKEYS_ABIS:-arm64-v8a armeabi-v7a x86_64}"
TARGETS=""
for a in $ABIS; do TARGETS="$TARGETS -t $a"; done
# 16 KB pages (Android 15+ devices, required by Google Play)
PAGE16K="-C link-arg=-Wl,-z,max-page-size=16384"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_RUSTFLAGS="${CARGO_TARGET_AARCH64_LINUX_ANDROID_RUSTFLAGS:-} $PAGE16K"
export CARGO_TARGET_X86_64_LINUX_ANDROID_RUSTFLAGS="${CARGO_TARGET_X86_64_LINUX_ANDROID_RUSTFLAGS:-} $PAGE16K"

cd "$CORE"
# shellcheck disable=SC2086
cargo ndk $TARGETS --platform 26 -o "$ANDROID/app/src/main/jniLibs" build --release -p airkeys-app
# the Lumen Code crate's own C-ABI library comes along as a by-product; the app links it statically
rm -f "$ANDROID"/app/src/main/jniLibs/*/liblumen*.so
# bindings come from the compiled host library: uniffi checks they match it at run time
cargo build --release -p airkeys-app
HOSTLIB="target/release/libairkeys.dylib"
[ -f "$HOSTLIB" ] || HOSTLIB="target/release/libairkeys.so"
OUT="$ANDROID/app/build/generated/uniffi"
rm -rf "$OUT"
cargo run --release -q -p airkeys-bindgen --bin uniffi-bindgen -- generate --library "$HOSTLIB" \
  --language kotlin --out-dir "$OUT" --no-format
echo "Android core libraries and Kotlin bindings updated"
