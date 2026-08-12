#!/usr/bin/env bash
#
# Replay the kernel checks through the uniffi-generated Kotlin bindings.
#
#   rust/scripts/smoke-kotlin.sh
#
# The third of the four surfaces. `cargo test` proves the Rust is right; this
# proves the KOTLIN CALLING IT gets the same answers — which is a different
# claim, and the one that breaks when uniffi is bumped without regenerating.
#
# Requires: cargo, kotlinc, java. JNA is fetched into rust/target/harness-libs
# on first run (the generated bindings need it at compile and run time).
set -euo pipefail

RUST_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$RUST_DIR"

LIB_DIR="target/harness-libs"
JNA_VERSION="5.14.0"
JNA_JAR="$LIB_DIR/jna-$JNA_VERSION.jar"

mkdir -p "$LIB_DIR"
if [ ! -f "$JNA_JAR" ]; then
  echo "smoke-kotlin: fetching JNA $JNA_VERSION"
  curl -fsSL -o "$JNA_JAR" \
    "https://repo1.maven.org/maven2/net/java/dev/jna/jna/$JNA_VERSION/jna-$JNA_VERSION.jar"
fi

echo "smoke-kotlin: building the cdylib"
cargo build --release -p airkeys-core-uniffi

# .dylib on macOS, .so on Linux — JNA resolves by name from jna.library.path, so
# only the directory matters at run time.
case "$(uname -s)" in
  Darwin) LIB_FILE="target/release/libairkeys_core_uniffi.dylib" ;;
  *)      LIB_FILE="target/release/libairkeys_core_uniffi.so" ;;
esac

echo "smoke-kotlin: generating bindings from the compiled library"
rm -rf bindings/kotlin
cargo run --quiet --release -p airkeys-core-uniffi --bin uniffi-bindgen -- generate \
  --library "$LIB_FILE" --language kotlin --out-dir bindings/kotlin

echo "smoke-kotlin: compiling the harness"
rm -rf harness/kotlin/build
mkdir -p harness/kotlin/build
kotlinc -classpath "$JNA_JAR" \
  bindings/kotlin/uniffi/airkeys_core_uniffi/airkeys_core_uniffi.kt \
  harness/kotlin/Harness.kt \
  -include-runtime -d harness/kotlin/build/harness.jar

echo "smoke-kotlin: running"
java -Djna.library.path="target/release" \
  -classpath "harness/kotlin/build/harness.jar:$JNA_JAR" HarnessKt
