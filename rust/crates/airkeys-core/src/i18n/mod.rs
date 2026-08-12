//! The translation engine, shared by every surface.
//!
//! # Why the core owns this
//!
//! Because the alternative is four of them. A string table per platform drifts
//! within a release: web gets the new copy, iOS keeps the old, and the bug is
//! invisible to every test that asserts behavior rather than words. Here there
//! is one corpus, one lookup, one plural rule set, and a test that fails when a
//! locale falls back to English by accident.
//!
//! # The shape
//!
//! ```text
//! i18n/locales/<lng>.json   hand-edited corpus — the ONLY hand-edited files
//!         │  scripts/gen-i18n.mjs
//!         ├─► src/i18n_catalogs/<lng>.rs   compiled-in, behind a per-locale feature
//!         └─► ../app-web/…/public/i18n/<lng>.json   fetched at runtime by web
//! ```
//!
//! Edit a string, run `node rust/scripts/gen-i18n.mjs`, and commit the
//! regeneration **with** the corpus change. A corpus edit without it leaves the
//! app rendering the old string while the Rust suite renders the new one, and
//! both would be green.

mod catalog;
mod interpolate;
mod plural;
mod resolve;

pub use catalog::Catalog;
pub use plural::{plural_category, plural_suffix, Category};
pub use resolve::{resolve_language, LanguageState, SUPPORTED_LANGUAGES};

use crate::error::CoreError;

/// The language every miss falls back to. Pinned, not configurable: a fallback
/// chain the user can change is a fallback chain that can be empty.
pub const FALLBACK_LANGUAGE: &str = "en";

/// An interpolation value. Numbers format through Rust's `Display`, which
/// matches what the shells send over JSON.
#[derive(Clone, Copy, Debug)]
pub enum Var<'a> {
    Str(&'a str),
    Num(f64),
}

/// Everything `t` accepts besides the key.
#[derive(Clone, Copy, Debug, Default)]
pub struct Options<'a> {
    /// `{{name}}` substitutions.
    pub vars: &'a [(&'a str, Var<'a>)],
    /// Selects the plural form and is itself interpolatable as `{{count}}`.
    pub count: Option<f64>,
    /// Resolve in this language instead of the active one. Used by the parity
    /// tests; production changes the language instead.
    pub lng: Option<&'a str>,
}

/// The engine. Holds the active catalog and the pinned fallback.
///
/// Not `Send`-hostile, not interior-mutable, no globals: a shell that wants two
/// languages resident at once builds two engines. This is deliberate — a
/// process-global `t()` is how a background task ends up rendering the previous
/// language into a notification.
pub struct I18n {
    active: Option<Catalog>,
    fallback: Option<Catalog>,
    language: String,
}

impl I18n {
    /// Build from a catalog, which becomes both the active language and — if it
    /// is the fallback language — the fallback.
    pub fn new(catalog: Catalog) -> Result<Self, CoreError> {
        let language = catalog.language().to_owned();
        let fallback = if language == FALLBACK_LANGUAGE {
            Some(catalog.clone())
        } else {
            Catalog::embedded(FALLBACK_LANGUAGE).ok()
        };
        Ok(Self {
            active: Some(catalog),
            fallback,
            language,
        })
    }

    /// Build from the compiled-in fallback catalog. Requires the `i18n-en`
    /// feature; a build without it gets a clear error rather than a silent
    /// key-echoing engine.
    pub fn embedded() -> Result<Self, CoreError> {
        Self::new(Catalog::embedded(FALLBACK_LANGUAGE)?)
    }

    /// Switch language. Reports what actually resolved — the caller loads the
    /// matching catalog (web fetches it, desktop takes it from the binary).
    ///
    /// Resolution happens even when no catalog is available for the result,
    /// which is what lets web ask "which file should I fetch?" before it has
    /// anything to fetch.
    pub fn change_language(&mut self, requested: &str) -> LanguageState {
        let state = resolve_language(requested);
        if let Some(resolved) = state.resolved_language.as_deref() {
            self.language = resolved.to_owned();
            // Drop a stale catalog immediately: rendering the previous
            // language while the new one loads is worse than rendering the
            // fallback, because it looks correct.
            if self
                .active
                .as_ref()
                .is_some_and(|catalog| catalog.language() != resolved)
            {
                self.active = None;
            }
        }
        state
    }

    /// Install a catalog. Becomes active when it matches the current language.
    pub fn load_catalog(&mut self, catalog: Catalog) {
        if catalog.language() == FALLBACK_LANGUAGE {
            self.fallback = Some(catalog.clone());
        }
        if catalog.language() == self.language {
            self.active = Some(catalog);
        }
    }

    pub fn language(&self) -> &str {
        &self.language
    }

    /// Resolve `key`.
    ///
    /// Order: active catalog (plural form, then bare key) → fallback catalog
    /// (same) → `Err(I18nMissingKey)`. The shell decides what a miss looks like
    /// on screen; i18next echoes the key, and the visual checks treat an echoed
    /// key as a failure signal rather than something to hide.
    pub fn t(&self, key: &str, options: &Options<'_>) -> Result<String, CoreError> {
        if key.is_empty() {
            return Err(CoreError::I18nMissingKey(String::new()));
        }
        let language = options.lng.unwrap_or(&self.language);

        let raw = self
            .lookup(self.active.as_ref(), key, language, options.count)
            .or_else(|| {
                self.lookup(
                    self.fallback.as_ref(),
                    key,
                    FALLBACK_LANGUAGE,
                    options.count,
                )
            })
            .ok_or_else(|| CoreError::I18nMissingKey(key.to_owned()))?;

        Ok(interpolate::interpolate(raw, options))
    }

    fn lookup<'c>(
        &self,
        catalog: Option<&'c Catalog>,
        key: &str,
        language: &str,
        count: Option<f64>,
    ) -> Option<&'c str> {
        let catalog = catalog?;
        if let Some(count) = count {
            let suffix = plural::plural_suffix(language, count);
            if let Some(value) = catalog.get(&format!("{key}{suffix}")) {
                return Some(value);
            }
        }
        catalog.get(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> I18n {
        let catalog = Catalog::from_json(
            "en",
            br#"{
                "vault": {
                    "unlock": "Unlock",
                    "greeting": "Hello, {{name}}",
                    "itemCount_one": "{{count}} item",
                    "itemCount_other": "{{count}} items"
                }
            }"#,
        )
        .expect("fixture parses");
        I18n::new(catalog).expect("engine builds")
    }

    #[test]
    fn resolves_a_flat_path() {
        assert_eq!(
            engine().t("vault.unlock", &Options::default()).unwrap(),
            "Unlock"
        );
    }

    #[test]
    fn interpolates_named_vars() {
        let options = Options {
            vars: &[("name", Var::Str("Ann"))],
            ..Options::default()
        };
        assert_eq!(
            engine().t("vault.greeting", &options).unwrap(),
            "Hello, Ann"
        );
    }

    #[test]
    fn selects_the_plural_form_and_interpolates_count() {
        let one = Options {
            count: Some(1.0),
            ..Options::default()
        };
        let many = Options {
            count: Some(4.0),
            ..Options::default()
        };
        let engine = engine();
        assert_eq!(engine.t("vault.itemCount", &one).unwrap(), "1 item");
        assert_eq!(engine.t("vault.itemCount", &many).unwrap(), "4 items");
    }

    #[test]
    fn a_miss_is_an_error_not_an_empty_string() {
        let error = engine().t("vault.nope", &Options::default()).unwrap_err();
        assert!(matches!(error, CoreError::I18nMissingKey(key) if key == "vault.nope"));
    }

    #[test]
    fn switching_language_drops_the_stale_catalog() {
        let mut engine = engine();
        let state = engine.change_language("zh-CN");
        assert_eq!(state.resolved_language.as_deref(), Some("zh"));
        assert_eq!(engine.language(), "zh");
        // No zh catalog loaded yet — the fallback answers, rather than the
        // previous language pretending to be the new one.
        assert_eq!(
            engine.t("vault.unlock", &Options::default()).unwrap(),
            "Unlock"
        );
    }
}
