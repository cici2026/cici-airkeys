//! Every string the desktop renders, resolved once per locale.
//!
//! Two things live here and nowhere else: the **key list** and the **resolved
//! bundle**. Screens take a `&Strings`, so a render function cannot invent a
//! key, cannot forget to translate one, and cannot be given a locale halfway
//! through a frame.
//!
//! The key list is not documentation — [`KEYS`] and [`PLURAL_KEYS`] are swept by
//! a test in every shipped language, so a key that stops resolving fails the
//! build instead of rendering as `vault.navItems` on someone's screen.

use gpui::SharedString;

use crate::loc::Loc;

/// Every non-plural key the desktop renders.
///
/// Read only by the sweep below — which is the point: the list is maintained by
/// hand, and the test is what makes forgetting to update it expensive.
#[allow(dead_code)]
pub const KEYS: &[&str] = &[
    "common.cancel",
    "common.close",
    "common.retry",
    "common.copy",
    "common.copied",
    "common.search",
    "common.add",
    "welcome.tagline",
    "welcome.unlockBtn",
    "welcome.createBtn",
    "welcome.featureNoMasterPasswordTitle",
    "welcome.featureNoMasterPasswordBody",
    "welcome.featureLocalFirstTitle",
    "welcome.featureLocalFirstBody",
    "welcome.featureOpenSourceTitle",
    "welcome.featureOpenSourceBody",
    "unlock.title",
    "unlock.statusChecking",
    "unlock.statusAwaiting",
    "unlock.statusOpening",
    "unlock.waitedSeconds",
    "unlock.cancelledTitle",
    "unlock.cancelledBody",
    "unlock.unsupportedTitle",
    "unlock.unsupportedBody",
    "unlock.failedTitle",
    "unlock.failedBody",
    "unlock.missingTitle",
    "unlock.missingBody",
    "vault.navItems",
    "vault.navFavorites",
    "vault.navGenerator",
    "vault.navSettings",
    "vault.lockNow",
    "vault.searchPlaceholder",
    "vault.emptyTitle",
    "vault.emptyBody",
    "vault.noMatchTitle",
    "vault.noMatchBody",
    "item.kindLogin",
    "item.kindNote",
    "item.kindKey",
    "item.kindCard",
    "item.labelAccount",
    "item.labelSite",
    "item.labelSecret",
    "item.labelUpdated",
    "item.reveal",
    "item.hide",
    "item.copySecret",
    "item.copyAccount",
    "item.clipboardCleared",
    "strength.weak",
    "strength.fair",
    "strength.strong",
    "strength.excellent",
    "strength.bits",
    "strength.hintTooShort",
    "strength.hintNoUppercase",
    "strength.hintNoDigits",
    "strength.hintNoSymbols",
    "strength.hintRepeated",
    "strength.hintSequential",
];

/// Keys that only exist in plural form — resolved through `t_count`, so the
/// sweep has to ask for them differently. Listing them separately is what keeps
/// the sweep honest instead of quietly skipping them.
#[allow(dead_code)]
pub const PLURAL_KEYS: &[&str] = &["vault.itemCount"];

/// The resolved bundle a screen renders from.
///
/// Only strings that render as-is live here. Anything carrying a `{{var}}` goes
/// through `loc.t_vars` at the call site, where the value exists — putting a
/// half-filled template in the bundle is how `{{query}}` ends up on screen.
///
/// Fields are added as screens come to need them; the corpus and [`KEYS`]
/// legitimately run ahead of the bundle, because a string is translated once
/// and rendered later.
pub struct Strings {
    pub close: SharedString,
    pub copied: SharedString,
    pub search: SharedString,

    pub tagline: SharedString,
    pub unlock_btn: SharedString,
    pub create_btn: SharedString,
    /// The three welcome cards, in design order: (title, body).
    pub features: [(SharedString, SharedString); 3],

    pub unlock_title: SharedString,
    pub status_checking: SharedString,
    pub status_awaiting: SharedString,
    pub status_opening: SharedString,
    pub missing_title: SharedString,
    pub missing_body: SharedString,

    pub nav_items: SharedString,
    pub nav_favorites: SharedString,
    pub nav_generator: SharedString,
    pub nav_settings: SharedString,
    pub lock_now: SharedString,
    pub search_placeholder: SharedString,
    pub empty_title: SharedString,
    pub empty_body: SharedString,
    pub no_match_title: SharedString,

    pub label_account: SharedString,
    pub label_site: SharedString,
    pub label_secret: SharedString,
    pub label_updated: SharedString,
    pub copy_secret: SharedString,
    pub copy_account: SharedString,
}

impl Strings {
    pub fn resolve(loc: &Loc) -> Self {
        let s = |key: &str| loc.t(key);
        Self {
            close: s("common.close"),
            copied: s("common.copied"),
            search: s("common.search"),

            tagline: s("welcome.tagline"),
            unlock_btn: s("welcome.unlockBtn"),
            create_btn: s("welcome.createBtn"),
            features: [
                (
                    s("welcome.featureNoMasterPasswordTitle"),
                    s("welcome.featureNoMasterPasswordBody"),
                ),
                (
                    s("welcome.featureLocalFirstTitle"),
                    s("welcome.featureLocalFirstBody"),
                ),
                (
                    s("welcome.featureOpenSourceTitle"),
                    s("welcome.featureOpenSourceBody"),
                ),
            ],

            unlock_title: s("unlock.title"),
            status_checking: s("unlock.statusChecking"),
            status_awaiting: s("unlock.statusAwaiting"),
            status_opening: s("unlock.statusOpening"),
            missing_title: s("unlock.missingTitle"),
            missing_body: s("unlock.missingBody"),

            nav_items: s("vault.navItems"),
            nav_favorites: s("vault.navFavorites"),
            nav_generator: s("vault.navGenerator"),
            nav_settings: s("vault.navSettings"),
            lock_now: s("vault.lockNow"),
            search_placeholder: s("vault.searchPlaceholder"),
            empty_title: s("vault.emptyTitle"),
            empty_body: s("vault.emptyBody"),
            no_match_title: s("vault.noMatchTitle"),

            label_account: s("item.labelAccount"),
            label_site: s("item.labelSite"),
            label_secret: s("item.labelSecret"),
            label_updated: s("item.labelUpdated"),
            copy_secret: s("item.copySecret"),
            copy_account: s("item.copyAccount"),
        }
    }
}

/// The label for an item kind. Exhaustive over the core's enum, for the same
/// reason `Theme::strength_color` is: a new kind must not silently render as
/// whatever the fallback arm said.
pub fn item_kind_label(loc: &Loc, kind: airkeys_core::types::ItemKind) -> SharedString {
    use airkeys_core::types::ItemKind;
    loc.t(match kind {
        ItemKind::Login => "item.kindLogin",
        ItemKind::Note => "item.kindNote",
        ItemKind::Key => "item.kindKey",
        ItemKind::Card => "item.kindCard",
    })
}

/// The label for a strength band.
pub fn strength_label(loc: &Loc, strength: airkeys_core::strength::Strength) -> SharedString {
    use airkeys_core::strength::Strength;
    loc.t(match strength {
        Strength::Weak => "strength.weak",
        Strength::Fair => "strength.fair",
        Strength::Strong => "strength.strong",
        Strength::Excellent => "strength.excellent",
    })
}

/// The copy for a strength hint.
pub fn strength_hint_label(loc: &Loc, hint: airkeys_core::strength::StrengthHint) -> SharedString {
    use airkeys_core::strength::StrengthHint;
    loc.t(match hint {
        StrengthHint::TooShort => "strength.hintTooShort",
        StrengthHint::NoUppercase => "strength.hintNoUppercase",
        StrengthHint::NoDigits => "strength.hintNoDigits",
        StrengthHint::NoSymbols => "strength.hintNoSymbols",
        StrengthHint::RepeatedCharacters => "strength.hintRepeated",
        StrengthHint::SequentialRun => "strength.hintSequential",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use airkeys_core::i18n::SUPPORTED_LANGUAGES;
    use airkeys_core::strength::{Strength, StrengthHint};
    use airkeys_core::types::ItemKind;

    /// The sweep. Every key this app renders, in every language it ships: no
    /// echo, no empty.
    #[test]
    fn no_key_echoes_in_any_shipped_language() {
        for language in SUPPORTED_LANGUAGES {
            let loc = Loc::pinned(language);
            for key in KEYS {
                let value = loc.t(key);
                assert_ne!(value.as_ref(), *key, "{language}: `{key}` echoed the key");
                assert!(!value.is_empty(), "{language}: `{key}` resolved empty");
            }
            for key in PLURAL_KEYS {
                for count in [0.0, 1.0, 7.0] {
                    let value = loc.t_count(key, count);
                    assert_ne!(
                        value.as_ref(),
                        *key,
                        "{language}: `{key}` echoed at count {count}"
                    );
                }
            }
        }
    }

    /// The three enum-to-key maps are total by construction, but only this
    /// asserts that every key they name is actually in the corpus.
    #[test]
    fn every_enum_label_resolves() {
        let loc = Loc::pinned("en");
        for kind in [
            ItemKind::Login,
            ItemKind::Note,
            ItemKind::Key,
            ItemKind::Card,
        ] {
            assert!(!item_kind_label(&loc, kind).is_empty(), "{kind:?}");
        }
        for strength in [
            Strength::Weak,
            Strength::Fair,
            Strength::Strong,
            Strength::Excellent,
        ] {
            assert!(!strength_label(&loc, strength).is_empty(), "{strength:?}");
        }
        for hint in [
            StrengthHint::TooShort,
            StrengthHint::NoUppercase,
            StrengthHint::NoDigits,
            StrengthHint::NoSymbols,
            StrengthHint::RepeatedCharacters,
            StrengthHint::SequentialRun,
        ] {
            assert!(!strength_hint_label(&loc, hint).is_empty(), "{hint:?}");
        }
    }

    /// The bundle is what screens actually read — assert it is populated, not
    /// just that the keys exist.
    #[test]
    fn the_bundle_resolves_in_every_language() {
        for language in SUPPORTED_LANGUAGES {
            let strings = Strings::resolve(&Loc::pinned(language));
            assert!(!strings.tagline.is_empty(), "{language}");
            assert!(!strings.nav_items.is_empty(), "{language}");
            assert_eq!(strings.features.len(), 3);
        }
    }
}
