//! The one error type the whole core returns.
//!
//! Flat and enumerated on purpose: the uniffi shell mirrors these variants 1:1
//! so Kotlin and Swift branch on the same vocabulary the Rust tests do, and the
//! wasm shell stringifies the same `Display`. A new variant is a wire change —
//! add it here, mirror it in `airkeys-core-uniffi`, regenerate.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CoreError {
    #[error("invalid hex: {0}")]
    InvalidHex(String),
    #[error("invalid base64url: {0}")]
    InvalidBase64Url(String),
    #[error("invalid item: {0}")]
    InvalidItem(String),
    #[error("invalid vault: {0}")]
    InvalidVault(String),
    /// No catalog on the fallback chain has this key. i18next's contract is to
    /// echo the key rather than render nothing; that decision belongs to the
    /// shell, so the core reports the miss instead of papering over it.
    #[error("i18n: missing key `{0}`")]
    I18nMissingKey(String),
    #[error("i18n: unsupported option: {0}")]
    I18nUnsupportedOption(String),
    #[error("i18n: catalog unavailable: {0}")]
    I18nCatalogUnavailable(String),
    #[error("i18n: catalog parse failed: {0}")]
    I18nCatalogParse(String),
    #[error("internal: {0}")]
    Internal(String),
}
