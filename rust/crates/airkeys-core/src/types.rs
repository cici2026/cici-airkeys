//! The value types that cross every boundary.
//!
//! These serialise 1:1 to the stored records and to the FFI mirrors. A field
//! added here is a wire change on four surfaces at once, so the rule is the
//! same as for `CoreError`: add it, mirror it in `airkeys-core-uniffi`,
//! regenerate the TypeScript, commit the regeneration in the same change.

use serde::{Deserialize, Serialize};

#[cfg(feature = "bindings")]
use ts_rs::TS;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "bindings", derive(TS))]
pub enum ItemKind {
    Login,
    Note,
    /// An SSH or API key: the thing the product is named after.
    Key,
    Card,
}

/// One entry in the vault, as the UI sees it.
///
/// **The secret is not in this struct.** A list view, a search index and a
/// sync payload all carry `Item`; a decrypted secret belongs to exactly one
/// screen for as long as it is on it. Keeping them in separate types is what
/// makes "did this code path just log a password?" answerable by reading the
/// signature.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(TS))]
pub struct Item {
    pub id: String,
    pub kind: ItemKind,
    pub title: String,
    /// Username, email, key comment — whatever identifies the account.
    pub account: String,
    /// Host or service this belongs to, for matching and grouping.
    pub site: String,
    pub favorite: bool,
    /// Wall clock observed by the shell when the item was last touched. The
    /// core never reads a clock; it is handed one.
    pub updated_at_iso: String,
}

/// What is known about a vault without unlocking it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(TS))]
pub struct VaultMeta {
    pub id: String,
    pub label: String,
    pub item_count: u32,
    pub created_at_iso: String,
}
