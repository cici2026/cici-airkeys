#!/usr/bin/env bash
#
# Replay the kernel checks through the uniffi-generated Swift bindings.
#
#   rust/scripts/smoke-swift.sh
#
# The fourth surface. See smoke-kotlin.sh for why this is a separate claim from
# `cargo test`. macOS only.
set -euo pipefail

RUST_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$RUST_DIR"

if [ "$(uname -s)" != "Darwin" ]; then
  echo "smoke-swift: macOS only" >&2
  exit 1
fi

echo "smoke-swift: building the dylib"
cargo build --release -p airkeys-core-uniffi
LIB_FILE="target/release/libairkeys_core_uniffi.dylib"

echo "smoke-swift: generating bindings from the compiled library"
rm -rf bindings/swift
cargo run --quiet --release -p airkeys-core-uniffi --bin uniffi-bindgen -- generate \
  --library "$LIB_FILE" --language swift --out-dir bindings/swift

OUT="target/swift-harness"
mkdir -p "$OUT"

# The generated Swift reaches the C shim through a module map, and swiftc only
# finds it under the clang-conventional name. The harness and the generated
# Swift compile into ONE module — there is no `import airkeys_core_uniffi`,
# because that module is what these two files together become.
cp bindings/swift/airkeys_core_uniffiFFI.h "$OUT/"
cp bindings/swift/airkeys_core_uniffiFFI.modulemap "$OUT/module.modulemap"

echo "smoke-swift: compiling and running the harness"
swiftc \
  -swift-version 5 \
  -I "$OUT" \
  -L target/release -lairkeys_core_uniffi \
  -Xlinker -rpath -Xlinker "$RUST_DIR/target/release" \
  bindings/swift/airkeys_core_uniffi.swift \
  harness/swift/main.swift \
  -o "$OUT/harness" \
  -suppress-warnings

"$OUT/harness"
