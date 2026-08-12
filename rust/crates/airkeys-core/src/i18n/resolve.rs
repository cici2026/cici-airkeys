//! Which of the shipped languages a requested tag maps to.
//!
//! Resolution is **feature-independent**: it answers for every shipped locale
//! whether or not that locale's catalog is compiled into this binary. That is
//! what lets the web shell ask "which file should I fetch?" before it has
//! anything fetched, and it is why this table is not derived from the cargo
//! features.

use super::FALLBACK_LANGUAGE;

/// Every language the corpus ships. Adding one means: a
/// `i18n/locales/<lng>.json` file, a cargo feature, a plural rule, and this
/// entry — all in the same change, or `gen-i18n.mjs` fails.
pub const SUPPORTED_LANGUAGES: &[&str] = &["en", "zh", "ja"];

/// What `change_language` resolved to, and what it walked through getting
/// there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LanguageState {
    /// Exactly as the caller asked, normalized.
    pub requested: String,
    /// The shipped language it maps to, or `None` when nothing matched and the
    /// fallback stands.
    pub resolved_language: Option<String>,
    /// The chain that was tried, in order — `["zh-CN", "zh", "en"]`. Logged
    /// when a locale unexpectedly renders English.
    pub chain: Vec<String>,
}

/// Resolve a BCP-47-ish tag against [`SUPPORTED_LANGUAGES`].
///
/// The ladder is i18next's: exact tag, then the primary subtag, then the
/// fallback. `zh_CN.UTF-8` is *not* handled here — POSIX normalization belongs
/// to whoever reads the environment, because only they know they are reading an
/// environment.
pub fn resolve_language(requested: &str) -> LanguageState {
    let normalized = normalize(requested);
    let mut chain = Vec::new();

    if !normalized.is_empty() {
        chain.push(normalized.clone());
        if let Some(matched) = match_supported(&normalized) {
            return LanguageState {
                requested: normalized,
                resolved_language: Some(matched.to_owned()),
                chain,
            };
        }
        if let Some(primary) = normalized.split('-').next().filter(|p| *p != normalized) {
            chain.push(primary.to_owned());
            if let Some(matched) = match_supported(primary) {
                return LanguageState {
                    requested: normalized,
                    resolved_language: Some(matched.to_owned()),
                    chain,
                };
            }
        }
    }

    chain.push(FALLBACK_LANGUAGE.to_owned());
    LanguageState {
        requested: normalized,
        resolved_language: Some(FALLBACK_LANGUAGE.to_owned()),
        chain,
    }
}

/// Case-insensitive match against the shipped list, returning the canonical
/// spelling. `ZH-cn` and `zh-CN` must not resolve differently.
fn match_supported(tag: &str) -> Option<&'static str> {
    SUPPORTED_LANGUAGES
        .iter()
        .find(|supported| supported.eq_ignore_ascii_case(tag))
        .copied()
}

/// `_` → `-`, trimmed, lowercase primary subtag. Region case is left alone
/// because nothing here compares it case-sensitively.
fn normalize(tag: &str) -> String {
    let tag = tag.trim();
    if tag.is_empty() {
        return String::new();
    }
    let mut parts = tag.replace('_', "-");
    if let Some(first_dash) = parts.find('-') {
        let primary = parts[..first_dash].to_ascii_lowercase();
        parts = format!("{primary}{}", &parts[first_dash..]);
    } else {
        parts = parts.to_ascii_lowercase();
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_exact_tag_resolves_to_itself() {
        let state = resolve_language("zh");
        assert_eq!(state.resolved_language.as_deref(), Some("zh"));
        assert_eq!(state.chain, vec!["zh"]);
    }

    #[test]
    fn a_region_tag_falls_back_to_its_primary_subtag() {
        let state = resolve_language("zh-CN");
        assert_eq!(state.resolved_language.as_deref(), Some("zh"));
        assert_eq!(state.chain, vec!["zh-CN", "zh"]);
    }

    #[test]
    fn an_unknown_tag_lands_on_the_fallback() {
        let state = resolve_language("kl-GL");
        assert_eq!(state.resolved_language.as_deref(), Some("en"));
        assert_eq!(state.chain, vec!["kl-GL", "kl", "en"]);
    }

    #[test]
    fn empty_and_posix_c_land_on_the_fallback() {
        assert_eq!(
            resolve_language("").resolved_language.as_deref(),
            Some("en")
        );
        assert_eq!(
            resolve_language("C").resolved_language.as_deref(),
            Some("en")
        );
    }

    #[test]
    fn underscores_and_case_do_not_change_the_result() {
        assert_eq!(
            resolve_language("ZH_cn").resolved_language.as_deref(),
            Some("zh")
        );
    }
}
