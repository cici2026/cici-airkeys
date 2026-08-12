//! The corpus invariants, asserted against the *compiled-in* catalogs.
//!
//! `gen-i18n.mjs` checks the same things against the JSON before generating.
//! Both exist on purpose: the generator guards the corpus, and this suite
//! guards the generated output — so a hand-edited `i18n_catalogs/*.rs` (the
//! thing the header of every generated file tells you not to do) fails here
//! rather than shipping.
//!
//! Requires every locale resident: `cargo test -p airkeys-core --features i18n-all`.

#![cfg(feature = "i18n-all")]

use airkeys_core::i18n::{plural_suffix, Catalog, I18n, Options, Var, SUPPORTED_LANGUAGES};

/// Strip a trailing plural suffix: `a.b_one` → `a.b`.
fn base_key(key: &str) -> &str {
    for suffix in ["_zero", "_one", "_two", "_few", "_many", "_other"] {
        if let Some(base) = key.strip_suffix(suffix) {
            return base;
        }
    }
    key
}

fn catalog(language: &str) -> Catalog {
    Catalog::embedded(language).unwrap_or_else(|error| panic!("{language}: {error}"))
}

#[test]
fn every_shipped_language_is_compiled_in_under_i18n_all() {
    for language in SUPPORTED_LANGUAGES {
        let catalog = catalog(language);
        assert!(!catalog.is_empty(), "{language} catalog is empty");
    }
}

/// A key present in `en` and missing elsewhere is a silent English string on a
/// non-English screen — the failure mode that is invisible to every test that
/// asserts behavior rather than words.
#[test]
fn no_locale_is_missing_a_key_the_fallback_has() {
    let en = catalog("en");
    let en_bases: std::collections::BTreeSet<&str> = en.keys().map(base_key).collect();

    for language in SUPPORTED_LANGUAGES {
        if *language == "en" {
            continue;
        }
        let other = catalog(language);
        let bases: std::collections::BTreeSet<&str> = other.keys().map(base_key).collect();
        let missing: Vec<&&str> = en_bases.difference(&bases).collect();
        assert!(missing.is_empty(), "{language} is missing {missing:?}");
        let extra: Vec<&&str> = bases.difference(&en_bases).collect();
        assert!(extra.is_empty(), "{language} has orphan keys {extra:?}");
    }
}

/// Each locale supplies exactly the plural forms its own rules select. A `_one`
/// entry in a language with no singular is a dead string that shadows nothing;
/// a missing one is a key echo on screen.
#[test]
fn plural_forms_match_each_language_s_own_rules() {
    for language in SUPPORTED_LANGUAGES {
        let catalog = catalog(language);
        let plural_bases: std::collections::BTreeSet<&str> = catalog
            .keys()
            .filter(|key| base_key(key) != *key)
            .map(base_key)
            .collect();

        for base in plural_bases {
            for count in [0.0_f64, 1.0, 2.0, 11.0] {
                let key = format!("{base}{}", plural_suffix(language, count));
                assert!(
                    catalog.get(&key).is_some(),
                    "{language}: `{key}` is selected for count {count} but is not in the catalog"
                );
            }
        }
    }
}

/// The translated string must carry the same `{{vars}}` as the fallback, or the
/// sentence renders with a hole in it in exactly one language.
#[test]
fn interpolation_variables_match_the_fallback() {
    fn vars(text: &str) -> Vec<String> {
        let mut found = Vec::new();
        let mut rest = text;
        while let Some(start) = rest.find("{{") {
            let after = &rest[start + 2..];
            let Some(end) = after.find("}}") else { break };
            found.push(after[..end].trim().to_owned());
            rest = &after[end + 2..];
        }
        found.sort();
        found
    }

    let en = catalog("en");
    for language in SUPPORTED_LANGUAGES {
        if *language == "en" {
            continue;
        }
        let other = catalog(language);
        for key in other.keys() {
            let (Some(translated), Some(fallback)) = (
                other.get(key),
                en.get(key).or_else(|| {
                    // The plural form may differ; compare against any en form of
                    // the same base key.
                    en.keys()
                        .find(|en_key| base_key(en_key) == base_key(key))
                        .and_then(|en_key| en.get(en_key))
                }),
            ) else {
                continue;
            };
            assert_eq!(
                vars(translated),
                vars(fallback),
                "{language}: `{key}` interpolation variables diverge from en"
            );
        }
    }
}

/// End to end, through the real engine: a locale resolves its own strings, and
/// falls back to English only for keys it genuinely lacks.
#[test]
fn the_engine_resolves_each_locale_without_echoing_keys() {
    const SAMPLE: [&str; 5] = [
        "welcome.tagline",
        "unlock.title",
        "vault.navItems",
        "item.copySecret",
        "strength.weak",
    ];

    let mut en = I18n::embedded().expect("en compiled in");
    let en_values: Vec<String> = SAMPLE
        .iter()
        .map(|key| en.t(key, &Options::default()).expect("en resolves"))
        .collect();
    assert_eq!(
        en.change_language("en").resolved_language.as_deref(),
        Some("en")
    );

    for language in SUPPORTED_LANGUAGES {
        if *language == "en" {
            continue;
        }
        let mut engine = I18n::embedded().expect("en compiled in");
        engine.change_language(language);
        engine.load_catalog(catalog(language));

        for (key, en_value) in SAMPLE.iter().zip(&en_values) {
            let value = engine
                .t(key, &Options::default())
                .unwrap_or_else(|error| panic!("{language}: `{key}` {error}"));
            assert_ne!(value, *key, "{language}: `{key}` echoed the key");
            assert!(!value.is_empty(), "{language}: `{key}` resolved empty");
            assert_ne!(value, *en_value, "{language}: `{key}` fell back to English");
        }
    }
}

/// The Chinese copy is the source of record for the desktop mocks — pin a
/// couple of strings verbatim so a well-meaning rewording is a deliberate diff.
#[test]
fn zh_copy_matches_the_mocks_verbatim() {
    let mut engine = I18n::embedded().expect("en compiled in");
    engine.change_language("zh");
    engine.load_catalog(catalog("zh"));

    assert_eq!(
        engine.t("unlock.title", &Options::default()).unwrap(),
        "解锁"
    );
    assert_eq!(
        engine.t("vault.navItems", &Options::default()).unwrap(),
        "全部条目"
    );
}

/// CJK has one plural form; English has two. Both must interpolate `{{count}}`
/// without the caller restating it.
#[test]
fn plural_selection_works_through_the_engine() {
    let mut en = I18n::embedded().expect("en compiled in");
    let one = Options {
        count: Some(1.0),
        ..Options::default()
    };
    let many = Options {
        count: Some(12.0),
        ..Options::default()
    };
    assert_eq!(en.t("vault.itemCount", &one).unwrap(), "1 item");
    assert_eq!(en.t("vault.itemCount", &many).unwrap(), "12 items");

    en.change_language("zh");
    en.load_catalog(catalog("zh"));
    assert_eq!(en.t("vault.itemCount", &one).unwrap(), "1 个条目");
    assert_eq!(en.t("vault.itemCount", &many).unwrap(), "12 个条目");
}

/// Named variables survive translation.
#[test]
fn named_variables_interpolate_in_every_locale() {
    for language in SUPPORTED_LANGUAGES {
        let mut engine = I18n::embedded().expect("en compiled in");
        engine.change_language(language);
        engine.load_catalog(catalog(language));

        let options = Options {
            vars: &[("query", Var::Str("github"))],
            ..Options::default()
        };
        let value = engine.t("vault.noMatchBody", &options).expect("resolves");
        assert!(
            value.contains("github"),
            "{language}: `{value}` dropped the query variable"
        );
        assert!(
            !value.contains("{{"),
            "{language}: `{value}` left a placeholder unfilled"
        );
    }
}
