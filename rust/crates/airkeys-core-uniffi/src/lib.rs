//! uniffi shell over airkeys-core → Kotlin (Android) + Swift (iOS).
//!
//! **Thin by design: FFI mirror types and `#[uniffi::export]` wrappers, zero
//! logic.** Every function here is one line of conversion around a call into
//! `airkeys_core`. The moment a rule lives in this file it exists on two
//! platforms and not on the other two, which is the exact failure this
//! architecture is built to prevent.
//!
//! The mirrors are hand-written rather than derived on the core's own types on
//! purpose: it keeps `uniffi` out of `airkeys-core`'s dependency graph, so the
//! pure crate stays buildable — and testable — with no FFI toolchain at all.

uniffi::setup_scaffolding!();

// ---------------------------------------------------------------------------
// Error (flat: the foreign side sees the variant plus the Display message)
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error, uniffi::Error)]
#[uniffi(flat_error)]
pub enum CoreError {
    #[error("{0}")]
    InvalidHex(String),
    #[error("{0}")]
    InvalidBase64Url(String),
    #[error("{0}")]
    InvalidItem(String),
    #[error("{0}")]
    InvalidVault(String),
    #[error("{0}")]
    I18nMissingKey(String),
    #[error("{0}")]
    I18nUnsupportedOption(String),
    #[error("{0}")]
    I18nCatalogUnavailable(String),
    #[error("{0}")]
    I18nCatalogParse(String),
    #[error("{0}")]
    Internal(String),
}

impl From<airkeys_core::CoreError> for CoreError {
    fn from(error: airkeys_core::CoreError) -> Self {
        use airkeys_core::CoreError as E;
        let message = error.to_string();
        match error {
            E::InvalidHex(_) => CoreError::InvalidHex(message),
            E::InvalidBase64Url(_) => CoreError::InvalidBase64Url(message),
            E::InvalidItem(_) => CoreError::InvalidItem(message),
            E::InvalidVault(_) => CoreError::InvalidVault(message),
            E::I18nMissingKey(_) => CoreError::I18nMissingKey(message),
            E::I18nUnsupportedOption(_) => CoreError::I18nUnsupportedOption(message),
            E::I18nCatalogUnavailable(_) => CoreError::I18nCatalogUnavailable(message),
            E::I18nCatalogParse(_) => CoreError::I18nCatalogParse(message),
            E::Internal(_) => CoreError::Internal(message),
        }
    }
}

// ---------------------------------------------------------------------------
// Records / enums (mirrors of airkeys_core's public value types)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum Strength {
    Weak,
    Fair,
    Strong,
    Excellent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum StrengthHint {
    TooShort,
    NoUppercase,
    NoDigits,
    NoSymbols,
    RepeatedCharacters,
    SequentialRun,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct StrengthReport {
    pub strength: Strength,
    pub bits: f64,
    pub hints: Vec<StrengthHint>,
}

impl From<airkeys_core::strength::StrengthReport> for StrengthReport {
    fn from(report: airkeys_core::strength::StrengthReport) -> Self {
        use airkeys_core::strength::{Strength as S, StrengthHint as H};
        StrengthReport {
            strength: match report.strength {
                S::Weak => Strength::Weak,
                S::Fair => Strength::Fair,
                S::Strong => Strength::Strong,
                S::Excellent => Strength::Excellent,
            },
            bits: report.bits,
            hints: report
                .hints
                .into_iter()
                .map(|hint| match hint {
                    H::TooShort => StrengthHint::TooShort,
                    H::NoUppercase => StrengthHint::NoUppercase,
                    H::NoDigits => StrengthHint::NoDigits,
                    H::NoSymbols => StrengthHint::NoSymbols,
                    H::RepeatedCharacters => StrengthHint::RepeatedCharacters,
                    H::SequentialRun => StrengthHint::SequentialRun,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct LanguageState {
    pub requested: String,
    pub resolved_language: Option<String>,
    pub chain: Vec<String>,
}

// ---------------------------------------------------------------------------
// Exported functions — one line of conversion each
// ---------------------------------------------------------------------------

#[uniffi::export]
pub fn to_hex(bytes: Vec<u8>, prefix: bool) -> String {
    airkeys_core::primitives::to_hex(&bytes, prefix)
}

#[uniffi::export]
pub fn from_hex(text: String) -> Result<Vec<u8>, CoreError> {
    airkeys_core::primitives::from_hex(&text).map_err(Into::into)
}

#[uniffi::export]
pub fn to_base64url(bytes: Vec<u8>) -> String {
    airkeys_core::primitives::to_base64url(&bytes)
}

#[uniffi::export]
pub fn from_base64url(text: String) -> Result<Vec<u8>, CoreError> {
    airkeys_core::primitives::from_base64url(&text).map_err(Into::into)
}

#[uniffi::export]
pub fn sha256(bytes: Vec<u8>) -> Vec<u8> {
    airkeys_core::primitives::sha256(&bytes).to_vec()
}

#[uniffi::export]
pub fn item_fingerprint(kind: String, title: String, account: String) -> String {
    airkeys_core::primitives::item_fingerprint(&kind, &title, &account)
}

#[uniffi::export]
pub fn secret_strength(secret: String) -> StrengthReport {
    airkeys_core::strength::secret_strength(&secret).into()
}

#[uniffi::export]
pub fn resolve_language(requested: String) -> LanguageState {
    let state = airkeys_core::i18n::resolve_language(&requested);
    LanguageState {
        requested: state.requested,
        resolved_language: state.resolved_language,
        chain: state.chain,
    }
}

#[uniffi::export]
pub fn supported_languages() -> Vec<String> {
    airkeys_core::i18n::SUPPORTED_LANGUAGES
        .iter()
        .map(|tag| (*tag).to_owned())
        .collect()
}

// ---------------------------------------------------------------------------
// The i18n engine, as an object
// ---------------------------------------------------------------------------

/// A translation engine the foreign side owns and mutates.
///
/// `Mutex` rather than `RefCell`: uniffi objects are `Send + Sync`, and Kotlin
/// or Swift may well resolve a string from a background dispatch queue while
/// the main thread is switching language.
#[derive(uniffi::Object)]
pub struct I18n {
    inner: std::sync::Mutex<airkeys_core::i18n::I18n>,
}

#[uniffi::export]
impl I18n {
    /// Build from the compiled-in fallback catalog.
    #[uniffi::constructor]
    pub fn embedded() -> Result<Self, CoreError> {
        Ok(Self {
            inner: std::sync::Mutex::new(airkeys_core::i18n::I18n::embedded()?),
        })
    }

    /// Switch language; the caller then loads the matching catalog JSON.
    pub fn change_language(&self, requested: String) -> Result<LanguageState, CoreError> {
        let mut engine = self
            .inner
            .lock()
            .map_err(|_| CoreError::Internal("i18n lock poisoned".into()))?;
        let state = engine.change_language(&requested);
        Ok(LanguageState {
            requested: state.requested,
            resolved_language: state.resolved_language,
            chain: state.chain,
        })
    }

    /// Install a catalog fetched or bundled by the platform.
    pub fn load_catalog_json(&self, language: String, json: String) -> Result<(), CoreError> {
        let catalog = airkeys_core::i18n::Catalog::from_json(&language, json.as_bytes())?;
        let mut engine = self
            .inner
            .lock()
            .map_err(|_| CoreError::Internal("i18n lock poisoned".into()))?;
        engine.load_catalog(catalog);
        Ok(())
    }

    /// Resolve a key. `count` selects the plural form when present.
    ///
    /// `vars` is a flat map because uniffi has no untagged-union type; numbers
    /// arrive already formatted by the caller, which keeps number formatting a
    /// platform concern rather than a second implementation in here.
    pub fn t(
        &self,
        key: String,
        vars: std::collections::HashMap<String, String>,
        count: Option<f64>,
    ) -> Result<String, CoreError> {
        let engine = self
            .inner
            .lock()
            .map_err(|_| CoreError::Internal("i18n lock poisoned".into()))?;
        let owned: Vec<(&str, airkeys_core::i18n::Var<'_>)> = vars
            .iter()
            .map(|(key, value)| (key.as_str(), airkeys_core::i18n::Var::Str(value.as_str())))
            .collect();
        engine
            .t(
                &key,
                &airkeys_core::i18n::Options {
                    vars: &owned,
                    count,
                    lng: None,
                },
            )
            .map_err(Into::into)
    }

    pub fn language(&self) -> Result<String, CoreError> {
        let engine = self
            .inner
            .lock()
            .map_err(|_| CoreError::Internal("i18n lock poisoned".into()))?;
        Ok(engine.language().to_owned())
    }
}
