//! Airkeys shared computation core.
//!
//! Pure, deterministic, correctness-critical computation only: parsing,
//! encoding, hashing, data assembly and validation, plus the portable state
//! machines that own what a flow decides. No I/O, no network, no UI, no clock,
//! no randomness. Every fallible function returns `Result<_, CoreError>` —
//! never a default value on bad input.
//!
//! One implementation, four surfaces: this crate (`cargo test`), the wasm shell
//! (web), the uniffi shell (iOS + Android), and the desktop app, which links it
//! natively. When they disagree, one of them is shipping a bug.

#![forbid(unsafe_code)]
// A panic in this crate is a crash in four apps at once, so the escape hatches
// are denied rather than discouraged. Fallible operations return
// `Result<_, CoreError>`; there is no `unwrap` path to production.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// …except in tests, where a panic IS the failure report. Denying them there
// only buys `match { Err(e) => panic!(...) }` written out by hand, which is the
// same panic with more code around it.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

/// Portable state machines. Feature-gated (`--features crux`) so the uniffi
/// bindings — and therefore the iOS/Android binaries — never link the
/// state-machine framework.
#[cfg(feature = "crux")]
pub mod app;
pub mod error;
pub mod i18n;
mod i18n_catalogs;
pub mod primitives;
pub mod strength;
pub mod types;

pub use error::CoreError;
pub use i18n::{resolve_language, Catalog, I18n, LanguageState, Options, Var};
pub use strength::{secret_strength, Strength, StrengthReport};
pub use types::{Item, ItemKind, VaultMeta};
