//! Localization access — **the only module that touches `airkeys_core::i18n`.**
//!
//! Same rule as `theme`: one module owns the vocabulary, everything else
//! receives already-resolved values. A component that can call `t()` is a
//! component that can invent a key, and an invented key is a string that exists
//! in exactly one language.
//!
//! Desktop compiles all catalogs in (`i18n-all`), so selection is synchronous
//! and infallible: the engine always holds the pinned `en` fallback, plus the
//! resolved locale's catalog when that locale is not `en`.

use gpui::SharedString;

use airkeys_core::i18n::{Catalog, I18n, Options, Var};

pub struct Loc {
    engine: I18n,
}

impl Loc {
    /// Build the engine for the launch locale: `AIRKEYS_LANG` → `LC_ALL` →
    /// `LC_MESSAGES` → `LANG` → `en`, resolved through the core's ladder.
    pub fn from_env() -> Self {
        let requested = ["AIRKEYS_LANG", "LC_ALL", "LC_MESSAGES", "LANG"]
            .iter()
            .find_map(|key| std::env::var(key).ok().filter(|value| !value.is_empty()))
            .map(|raw| normalize_posix_tag(&raw))
            .unwrap_or_else(|| "en".to_owned());

        let mut engine = match I18n::embedded() {
            Ok(engine) => engine,
            // `i18n-en` is a compile-time feature of this binary, and the
            // dependency declaration makes its absence impossible. If that
            // invariant is ever broken, echo keys rather than refuse to start:
            // a visibly wrong screen beats no screen, and the echo is exactly
            // what the visual checks treat as a failure signal.
            Err(_) => return Self::key_echo(),
        };

        let state = engine.change_language(&requested);
        // `en` needs no second catalog — the engine already holds it as the
        // fallback, and loading it twice would just replace it with itself.
        if let Some(resolved) = state.resolved_language.as_deref()
            && resolved != "en"
            && let Ok(catalog) = Catalog::embedded(resolved)
        {
            engine.load_catalog(catalog);
        }
        Self { engine }
    }

    /// An engine with no strings: `t()` echoes keys. Never reached in a
    /// correctly built binary — see `from_env`.
    fn key_echo() -> Self {
        let empty = Catalog::from_json("en", b"{}").and_then(I18n::new);
        match empty {
            Ok(engine) => Self { engine },
            // `from_json` on `{}` and `new` on an empty catalog are both
            // infallible; this arm exists only because the types say so.
            Err(_) => unreachable!("an empty en catalog is infallible to build"),
        }
    }

    /// Resolve `key`. A miss echoes the key — i18next's contract — which the
    /// visual checks read as a failure rather than something to hide.
    pub fn t(&self, key: &str) -> SharedString {
        self.engine
            .t(key, &Options::default())
            .unwrap_or_else(|_| key.to_owned())
            .into()
    }

    /// `t` with named interpolation variables.
    pub fn t_vars(&self, key: &str, vars: &[(&str, &str)]) -> SharedString {
        let vars: Vec<(&str, Var<'_>)> = vars.iter().map(|(k, v)| (*k, Var::Str(v))).collect();
        self.engine
            .t(
                key,
                &Options {
                    vars: &vars,
                    ..Options::default()
                },
            )
            .unwrap_or_else(|_| key.to_owned())
            .into()
    }

    /// `t` with a plural count, which is also interpolatable as `{{count}}`.
    pub fn t_count(&self, key: &str, count: f64) -> SharedString {
        self.engine
            .t(
                key,
                &Options {
                    count: Some(count),
                    ..Options::default()
                },
            )
            .unwrap_or_else(|_| key.to_owned())
            .into()
    }

    /// The BCP-47 tag actually resolved. Logged at startup — when a screen
    /// renders unexpectedly in English, this line is the first question
    /// answered.
    pub fn language(&self) -> &str {
        self.engine.language()
    }

    /// Test-only: a `Loc` pinned to `language` regardless of the environment.
    #[cfg(test)]
    pub(crate) fn pinned(language: &str) -> Self {
        let mut engine = I18n::embedded().expect("en compiled in");
        let state = engine.change_language(language);
        if let Some(resolved) = state.resolved_language.as_deref()
            && resolved != "en"
        {
            engine.load_catalog(Catalog::embedded(resolved).expect("catalog compiled in"));
        }
        Self { engine }
    }
}

/// `zh_CN.UTF-8` → `zh-CN`. Strips the encoding suffix and maps `_` to `-`;
/// `resolve_language` takes it from there, including `C`/`POSIX` → `en` through
/// its unsupported-tag fallback.
///
/// This lives here rather than in the core on purpose: POSIX environment
/// spelling is a property of *reading an environment*, and the core does not
/// read environments.
fn normalize_posix_tag(raw: &str) -> String {
    let no_encoding = raw.split('.').next().unwrap_or(raw);
    no_encoding.replace('_', "-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn posix_tags_normalize() {
        assert_eq!(normalize_posix_tag("zh_CN.UTF-8"), "zh-CN");
        assert_eq!(normalize_posix_tag("ja"), "ja");
        assert_eq!(normalize_posix_tag("en_GB"), "en-GB");
    }

    #[test]
    fn a_pinned_locale_resolves_its_own_strings() {
        let zh = Loc::pinned("zh");
        assert_eq!(zh.language(), "zh");
        assert_eq!(zh.t("unlock.title").as_ref(), "解锁");
    }

    #[test]
    fn interpolation_and_plurals_reach_the_engine() {
        let en = Loc::pinned("en");
        assert_eq!(en.t_count("vault.itemCount", 1.0).as_ref(), "1 item");
        assert_eq!(en.t_count("vault.itemCount", 9.0).as_ref(), "9 items");
        assert!(
            en.t_vars("vault.noMatchBody", &[("query", "github")])
                .contains("github")
        );
    }

    /// A miss echoes the key rather than rendering empty — the contract the
    /// visual checks rely on.
    #[test]
    fn a_missing_key_echoes_itself() {
        let loc = Loc::pinned("en");
        assert_eq!(loc.t("no.such.key").as_ref(), "no.such.key");
    }
}
