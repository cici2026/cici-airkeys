//! The state machines, exported to the web app.
//!
//! Deliberately thin: JSON in, JSON out, no business logic. Adding a machine is
//! one `bridge_class!` line — if a change to this file needs more than that,
//! the logic being added belongs in `airkeys-core`, where the other three
//! surfaces can see it too.

use crate::bridge::bridge_class;

bridge_class!(
    /// Opening a locked vault: support → authenticate → open → enter.
    UnlockCore,
    airkeys_core::app::unlock::Unlock
);

bridge_class!(
    /// The unlocked vault's item list: load → search → favorite → copy.
    ItemListCore,
    airkeys_core::app::item_list::ItemList
);
