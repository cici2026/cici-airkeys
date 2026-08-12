//! The uniffi bindgen CLI, built from this crate so the generator and the
//! library are always the same uniffi version.
//!
//! Generating with a mismatched bindgen produces bindings whose checksums do
//! not match the compiled library, and the failure surfaces as a runtime panic
//! on the first FFI call rather than at build time.

fn main() {
    uniffi::uniffi_bindgen_main()
}
