//! wasm-bindgen shell over airkeys-core for the web app.
//!
//! Two kinds of export live here and nothing else:
//!
//! - **kernels** — pure functions, called directly (`secret_strength`, hex,
//!   base64url, the i18n engine);
//! - **machines** — Crux cores behind the generic [`bridge`], driven by the web
//!   effect loop.
//!
//! No logic. If a function here does more than convert a type, it belongs in
//! `airkeys-core` where iOS, Android and desktop can also reach it.

mod bridge;
mod machines;

use wasm_bindgen::prelude::*;

pub use machines::{ItemListCore, UnlockCore};

fn js_error(error: airkeys_core::CoreError) -> JsValue {
    JsValue::from_str(&error.to_string())
}

// ---------------------------------------------------------------------------
// Kernels
// ---------------------------------------------------------------------------

#[wasm_bindgen]
pub fn to_hex(bytes: &[u8], prefix: bool) -> String {
    airkeys_core::primitives::to_hex(bytes, prefix)
}

#[wasm_bindgen]
pub fn from_hex(text: &str) -> Result<Vec<u8>, JsValue> {
    airkeys_core::primitives::from_hex(text).map_err(js_error)
}

#[wasm_bindgen]
pub fn to_base64url(bytes: &[u8]) -> String {
    airkeys_core::primitives::to_base64url(bytes)
}

#[wasm_bindgen]
pub fn from_base64url(text: &str) -> Result<Vec<u8>, JsValue> {
    airkeys_core::primitives::from_base64url(text).map_err(js_error)
}

#[wasm_bindgen]
pub fn sha256(bytes: &[u8]) -> Vec<u8> {
    airkeys_core::primitives::sha256(bytes).to_vec()
}

#[wasm_bindgen]
pub fn item_fingerprint(kind: &str, title: &str, account: &str) -> String {
    airkeys_core::primitives::item_fingerprint(kind, title, account)
}

/// `StrengthReport` as a plain JS object.
#[wasm_bindgen]
pub fn secret_strength(secret: &str) -> Result<JsValue, JsValue> {
    let report = airkeys_core::strength::secret_strength(secret);
    serde_wasm_bindgen::to_value(&report).map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Which shipped language a tag resolves to, and the chain it walked. Web calls
/// this **before** fetching anything — it is what decides which catalog file to
/// request.
#[wasm_bindgen]
pub fn resolve_language(requested: &str) -> Result<JsValue, JsValue> {
    let state = airkeys_core::i18n::resolve_language(requested);
    #[derive(serde::Serialize)]
    struct Wire {
        requested: String,
        resolved_language: Option<String>,
        chain: Vec<String>,
    }
    serde_wasm_bindgen::to_value(&Wire {
        requested: state.requested,
        resolved_language: state.resolved_language,
        chain: state.chain,
    })
    .map_err(|error| JsValue::from_str(&error.to_string()))
}

#[wasm_bindgen]
pub fn supported_languages() -> Vec<String> {
    airkeys_core::i18n::SUPPORTED_LANGUAGES
        .iter()
        .map(|tag| (*tag).to_owned())
        .collect()
}

// ---------------------------------------------------------------------------
// i18n engine
// ---------------------------------------------------------------------------

/// The translation engine, owned by the JS side.
///
/// Constructed empty rather than from an embedded catalog: this module ships
/// **zero** locales (see the crate's Cargo.toml), so the first thing the web app
/// does is fetch `/i18n/en.json` and call `load_catalog_json`. That is one
/// network round trip the native shells do not need — and it is what keeps the
/// wasm module from carrying every language for every user.
#[wasm_bindgen]
pub struct I18n {
    inner: airkeys_core::i18n::I18n,
}

#[wasm_bindgen]
impl I18n {
    /// An engine with no strings yet. `t()` errors until a catalog is loaded,
    /// which the shell renders as the key — never as an empty string, because
    /// an empty string looks like a design decision.
    #[wasm_bindgen(constructor)]
    pub fn new(language: &str, catalog_json: &str) -> Result<I18n, JsValue> {
        let catalog = airkeys_core::i18n::Catalog::from_json(language, catalog_json.as_bytes())
            .map_err(js_error)?;
        Ok(I18n {
            inner: airkeys_core::i18n::I18n::new(catalog).map_err(js_error)?,
        })
    }

    pub fn change_language(&mut self, requested: &str) -> Result<JsValue, JsValue> {
        let state = self.inner.change_language(requested);
        #[derive(serde::Serialize)]
        struct Wire {
            requested: String,
            resolved_language: Option<String>,
            chain: Vec<String>,
        }
        serde_wasm_bindgen::to_value(&Wire {
            requested: state.requested,
            resolved_language: state.resolved_language,
            chain: state.chain,
        })
        .map_err(|error| JsValue::from_str(&error.to_string()))
    }

    pub fn load_catalog_json(&mut self, language: &str, json: &str) -> Result<(), JsValue> {
        let catalog =
            airkeys_core::i18n::Catalog::from_json(language, json.as_bytes()).map_err(js_error)?;
        self.inner.load_catalog(catalog);
        Ok(())
    }

    /// Resolve a key.
    ///
    /// `options_json` is the i18next object literal verbatim: `count` selects
    /// the plural form, every other key is an interpolation variable. Taking it
    /// as JSON rather than a typed struct is what lets a TypeScript caller pass
    /// the object it already has.
    pub fn t(&self, key: &str, options_json: &str) -> Result<String, JsValue> {
        let options: serde_json::Value = if options_json.is_empty() {
            serde_json::Value::Object(Default::default())
        } else {
            serde_json::from_str(options_json)
                .map_err(|error| JsValue::from_str(&format!("invalid t() options: {error}")))?
        };

        let count = options.get("count").and_then(serde_json::Value::as_f64);
        // Values are stringified here so the core never has to decide how a
        // number renders — that is a locale decision the platform already made.
        let owned: Vec<(String, String)> = options
            .as_object()
            .map(|map| {
                map.iter()
                    .filter(|(key, _)| key.as_str() != "count")
                    .map(|(key, value)| {
                        let text = match value {
                            serde_json::Value::String(text) => text.clone(),
                            other => other.to_string(),
                        };
                        (key.clone(), text)
                    })
                    .collect()
            })
            .unwrap_or_default();
        let vars: Vec<(&str, airkeys_core::i18n::Var<'_>)> = owned
            .iter()
            .map(|(key, value)| (key.as_str(), airkeys_core::i18n::Var::Str(value.as_str())))
            .collect();

        self.inner
            .t(
                key,
                &airkeys_core::i18n::Options {
                    vars: &vars,
                    count,
                    lng: None,
                },
            )
            .map_err(js_error)
    }

    #[wasm_bindgen(getter)]
    pub fn language(&self) -> String {
        self.inner.language().to_owned()
    }
}
