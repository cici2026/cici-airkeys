#!/bin/sh
# The core for Android: app-android/Airkeys/app/build/rust/jniLibs/<abi>/libairkeys.so (cargo-ndk)
# and the Kotlin bindings in app-android/Airkeys/app/build/generated/uniffi. Gradle runs this
# before each build (task cargoBuild; up to date when nothing under core/ changed).
#   one time: cargo install cargo-ndk
#   AIRKEYS_ABIS="arm64-v8a" builds one ABI (the default is all three)
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

cd "$CORE"
# No path of this machine in the libraries: builds are the same anywhere, and say nothing about
# who built them. 16 KB pages: Android 15+ devices, required by Google Play.
REMAP="--remap-path-prefix=$HOME/.cargo=/cargo --remap-path-prefix=$(rustc --print sysroot)=/rustc --remap-path-prefix=$CORE=/airkeys/core"
PAGE16K="-C link-arg=-Wl,-z,max-page-size=16384"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_RUSTFLAGS="${CARGO_TARGET_AARCH64_LINUX_ANDROID_RUSTFLAGS:-} $REMAP $PAGE16K"
export CARGO_TARGET_X86_64_LINUX_ANDROID_RUSTFLAGS="${CARGO_TARGET_X86_64_LINUX_ANDROID_RUSTFLAGS:-} $REMAP $PAGE16K"
export CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_RUSTFLAGS="${CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_RUSTFLAGS:-} $REMAP"

# a fresh directory: an ABI left out of this build must not ship an older library
LIBS="$ANDROID/app/build/rust/jniLibs"
rm -rf "$LIBS"
# shellcheck disable=SC2086
cargo ndk $TARGETS --platform 26 -o "$LIBS" build --release --locked -p airkeys-app
# the Lumen Code crate's own C-ABI library comes along as a by-product; the app links it statically
rm -f "$LIBS"/*/liblumen*.so
# bindings come from the compiled host library: uniffi checks they match it at run time
cargo build --release --locked -p airkeys-app
HOSTLIB="target/release/libairkeys.dylib"
[ -f "$HOSTLIB" ] || HOSTLIB="target/release/libairkeys.so"
OUT="$ANDROID/app/build/generated/uniffi"
rm -rf "$OUT"
cargo run --release --locked -q -p airkeys-bindgen --bin uniffi-bindgen -- generate --library "$HOSTLIB" \
  --language kotlin --out-dir "$OUT" --no-format
echo "Android core libraries and Kotlin bindings updated"
