//! One language's strings, flattened to dotted paths.
//!
//! Two constructors, one representation. `from_json` is what web calls after
//! fetching `/i18n/<lng>.json`; `embedded` is what desktop and native call
//! against the generated tables. Both produce the same `BTreeMap`, so no lookup
//! path is exercised only on one platform.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::error::CoreError;
use crate::i18n_catalogs;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Catalog {
    language: String,
    entries: BTreeMap<String, String>,
}

impl Catalog {
    /// Parse a nested JSON object into dotted paths: `{"a":{"b":"c"}}` →
    /// `a.b = c`.
    ///
    /// Non-string leaves are an error, not a coercion. A number in the corpus
    /// means someone wrote `"count": 5` where a template belonged, and silently
    /// stringifying it ships that mistake to every locale that copied it.
    pub fn from_json(language: &str, bytes: &[u8]) -> Result<Self, CoreError> {
        let root: Value = serde_json::from_slice(bytes)
            .map_err(|error| CoreError::I18nCatalogParse(error.to_string()))?;
        let mut entries = BTreeMap::new();
        flatten(&root, &mut String::new(), &mut entries)?;
        Ok(Self {
            language: language.to_owned(),
            entries,
        })
    }

    /// The catalog compiled into this binary for `language`.
    ///
    /// Errors when the matching `i18n-<lng>` feature is off — including for the
    /// fallback language, which is how a build that forgot `i18n-en` finds out
    /// at startup instead of when a user opens a screen.
    pub fn embedded(language: &str) -> Result<Self, CoreError> {
        let table = i18n_catalogs::embedded(language).ok_or_else(|| {
            CoreError::I18nCatalogUnavailable(format!(
                "`{language}` is not compiled in (enable the `i18n-{language}` feature)"
            ))
        })?;
        Ok(Self {
            language: language.to_owned(),
            entries: table
                .iter()
                .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
                .collect(),
        })
    }

    pub fn language(&self) -> &str {
        &self.language
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries.get(key).map(String::as_str)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Every path in this catalog, sorted. The drift test compares these across
    /// locales — a key present in `en` and missing in `zh` is a silent English
    /// fallback on a Chinese screen.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }
}

fn flatten(
    value: &Value,
    path: &mut String,
    out: &mut BTreeMap<String, String>,
) -> Result<(), CoreError> {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let restore = path.len();
                if !path.is_empty() {
                    path.push('.');
                }
                path.push_str(key);
                flatten(child, path, out)?;
                path.truncate(restore);
            }
            Ok(())
        }
        Value::String(text) => {
            out.insert(path.clone(), text.clone());
            Ok(())
        }
        other => Err(CoreError::I18nCatalogParse(format!(
            "`{path}` is {}, expected a string",
            match other {
                Value::Null => "null",
                Value::Bool(_) => "a boolean",
                Value::Number(_) => "a number",
                Value::Array(_) => "an array",
                _ => "unsupported",
            }
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nesting_flattens_to_dotted_paths() {
        let catalog =
            Catalog::from_json("en", br#"{"a":{"b":{"c":"deep"}},"top":"flat"}"#).unwrap();
        assert_eq!(catalog.get("a.b.c"), Some("deep"));
        assert_eq!(catalog.get("top"), Some("flat"));
        assert_eq!(catalog.len(), 2);
    }

    #[test]
    fn a_non_string_leaf_is_rejected() {
        let error = Catalog::from_json("en", br#"{"count": 5}"#).unwrap_err();
        assert!(
            matches!(&error, CoreError::I18nCatalogParse(msg) if msg.contains("count")),
            "got {error:?}"
        );
    }

    #[test]
    fn malformed_json_is_rejected() {
        assert!(Catalog::from_json("en", b"{nope").is_err());
    }
}
