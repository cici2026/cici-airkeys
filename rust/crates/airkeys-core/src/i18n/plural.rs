//! Plural categories for the shipped locales.
//!
//! A CLDR *subset*, not a reimplementation: only the languages in
//! [`SUPPORTED_LANGUAGES`](super::SUPPORTED_LANGUAGES) are here, and adding a
//! locale to the corpus means adding its rule here in the same change. The
//! alternative — a general CLDR engine — is a large dependency to decide
//! between two suffixes for the three languages this app actually ships.
//!
//! Suffixes match i18next v21+ (`key_one`, `key_other`), which is what the
//! corpus is authored against.

/// CLDR plural categories. Only the ones the shipped locales use are ever
/// returned; the rest exist so adding a Slavic locale is a rule change and not
/// a type change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Zero,
    One,
    Two,
    Few,
    Many,
    Other,
}

impl Category {
    pub fn suffix(self) -> &'static str {
        match self {
            Category::Zero => "_zero",
            Category::One => "_one",
            Category::Two => "_two",
            Category::Few => "_few",
            Category::Many => "_many",
            Category::Other => "_other",
        }
    }
}

/// The category `count` falls into for `language`.
///
/// The language may be a full tag (`zh-CN`); only the primary subtag matters
/// for plural rules.
pub fn plural_category(language: &str, count: f64) -> Category {
    let primary = language.split('-').next().unwrap_or(language);
    match primary {
        // Chinese and Japanese have no grammatical plural: one form, always.
        // Writing `_other` in the corpus and nothing else is correct, not lazy.
        "zh" | "ja" => Category::Other,
        // en: one for exactly 1, other otherwise. `1.0` is one; `1.5` is not.
        _ => {
            if count == 1.0 {
                Category::One
            } else {
                Category::Other
            }
        }
    }
}

/// The key suffix to try before the bare key.
pub fn plural_suffix(language: &str, count: f64) -> &'static str {
    plural_category(language, count).suffix()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_singles_out_exactly_one() {
        assert_eq!(plural_category("en", 1.0), Category::One);
        assert_eq!(plural_category("en", 0.0), Category::Other);
        assert_eq!(plural_category("en", 2.0), Category::Other);
        assert_eq!(plural_category("en", 1.5), Category::Other);
    }

    #[test]
    fn cjk_has_one_form() {
        for count in [0.0, 1.0, 7.0] {
            assert_eq!(plural_category("zh", count), Category::Other);
            assert_eq!(plural_category("ja", count), Category::Other);
        }
    }

    #[test]
    fn a_region_subtag_does_not_change_the_rule() {
        assert_eq!(plural_category("zh-CN", 1.0), plural_category("zh", 1.0));
        assert_eq!(plural_category("en-GB", 1.0), Category::One);
    }

    #[test]
    fn suffixes_match_the_corpus_convention() {
        assert_eq!(plural_suffix("en", 1.0), "_one");
        assert_eq!(plural_suffix("en", 9.0), "_other");
        assert_eq!(plural_suffix("zh", 1.0), "_other");
    }
}
