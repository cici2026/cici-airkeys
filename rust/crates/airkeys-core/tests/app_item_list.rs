//! Machine B's rules: derived filtering, optimistic favorites, and the
//! clipboard generation guard.

#![cfg(feature = "crux")]

mod support;

use airkeys_core::app::item_list::EmptyReason;
use airkeys_core::app::item_list::{Event, ItemList};
use airkeys_core::app::shell::{CopyField, CopyRequest, ShellOperation, ShellResult};
use support::{item, Driver};

type Lister = Driver<ItemList>;

fn loaded() -> Lister {
    let mut driver = Lister::new();
    driver.dispatch(Event::Start {
        vault_id: "vault-1".into(),
    });
    driver.resolve(ShellResult::ItemsLoaded {
        items: vec![
            item("a", "GitHub", "ann@example.com", "github.com"),
            item("b", "Bank", "8891", "bank.example"),
            item("c", "SSH key", "deploy", "infra.example"),
        ],
    });
    driver
}

#[test]
fn mounting_loads_the_items_for_the_open_vault() {
    let mut driver = Lister::new();
    let ops = driver.dispatch(Event::Start {
        vault_id: "vault-1".into(),
    });
    assert_eq!(
        ops,
        vec![ShellOperation::LoadItems {
            vault_id: "vault-1".into()
        }]
    );
    assert!(driver.view().loading);

    driver.resolve(ShellResult::ItemsLoaded { items: vec![] });
    assert!(!driver.view().loading);
}

/// Rule 1: the visible list is derived every frame, never cached.
#[test]
fn search_filters_the_visible_list_without_touching_the_total() {
    let mut driver = loaded();
    driver.dispatch(Event::QueryChanged {
        query: "git".into(),
    });

    let view = driver.view();
    assert_eq!(view.total, 3, "the counter shows the vault, not the filter");
    assert_eq!(view.visible.len(), 1);
    assert_eq!(view.visible[0].title, "GitHub");
}

/// Two empties that need different copy and different next actions.
#[test]
fn an_empty_vault_and_an_empty_search_are_different_states() {
    let mut empty = Lister::new();
    empty.dispatch(Event::Start {
        vault_id: "vault-1".into(),
    });
    empty.resolve(ShellResult::ItemsLoaded { items: vec![] });
    assert_eq!(empty.view().empty_reason, Some(EmptyReason::NoItems));

    let mut driver = loaded();
    driver.dispatch(Event::QueryChanged {
        query: "nothing matches this".into(),
    });
    assert_eq!(driver.view().empty_reason, Some(EmptyReason::NoMatches));
}

/// A read that failed must not render as "this vault is empty" — the user would
/// reasonably conclude their data is gone.
#[test]
fn a_failed_load_is_not_an_empty_vault() {
    let mut driver = Lister::new();
    driver.dispatch(Event::Start {
        vault_id: "vault-1".into(),
    });
    driver.resolve(ShellResult::StorageFailed {
        message: "vault file is locked".into(),
    });

    let view = driver.view();
    assert_eq!(view.empty_reason, None);
    assert_eq!(view.error.as_deref(), Some("vault file is locked"));
}

/// Rule 2, the happy half: the star lands before the write does.
#[test]
fn favoriting_is_optimistic() {
    let mut driver = loaded();
    let ops = driver.dispatch(Event::FavoriteToggled {
        item_id: "a".into(),
    });

    assert!(
        driver
            .view()
            .visible
            .iter()
            .any(|i| i.id == "a" && i.favorite),
        "the star must land before the write resolves"
    );
    assert!(matches!(
        ops.as_slice(),
        [ShellOperation::SaveItem { item }] if item.id == "a" && item.favorite
    ));
}

/// Rule 2, the half that matters: a failed write puts the star back and says so.
#[test]
fn a_failed_favorite_write_reverts_and_surfaces_the_error() {
    let mut driver = loaded();
    driver.dispatch(Event::FavoriteToggled {
        item_id: "a".into(),
    });
    driver.resolve(ShellResult::StorageFailed {
        message: "read-only filesystem".into(),
    });

    let view = driver.view();
    assert!(
        view.visible.iter().any(|i| i.id == "a" && !i.favorite),
        "the optimistic star must revert"
    );
    assert_eq!(view.error.as_deref(), Some("read-only filesystem"));
}

#[test]
fn copying_asks_for_a_field_by_reference_never_by_value() {
    let mut driver = loaded();
    let ops = driver.dispatch(Event::CopyRequested {
        item_id: "a".into(),
        field: CopyField::Secret,
    });

    assert_eq!(
        ops,
        vec![ShellOperation::CopyField {
            request: CopyRequest {
                item_id: "a".into(),
                field: CopyField::Secret,
            },
        }],
        "the operation names the field; the plaintext never enters the core"
    );
    assert_eq!(driver.view().copied_item_id.as_deref(), Some("a"));
}

/// Rule 3, the whole point of the generation counter: copy A schedules a wipe
/// 30 s out; copy B lands at 20 s; A's wipe must not clear B's value.
#[test]
fn a_newer_copy_supersedes_the_older_clipboard_wipe() {
    let mut driver = loaded();

    driver.dispatch(Event::CopyRequested {
        item_id: "a".into(),
        field: CopyField::Secret,
    });
    driver.resolve(ShellResult::Copied); // schedules wipe for generation 1

    driver.dispatch(Event::CopyRequested {
        item_id: "b".into(),
        field: CopyField::Secret,
    });
    driver.resolve_matching(
        |op| matches!(op, ShellOperation::CopyField { .. }),
        ShellResult::Copied,
    ); // schedules wipe for generation 2

    // Generation 1's timer comes due. It must do nothing at all.
    let ops = driver.resolve_matching(
        |op| matches!(op, ShellOperation::Wait { .. }),
        ShellResult::Waited,
    );
    assert!(
        !ops.contains(&ShellOperation::ClearClipboard),
        "a stale wipe must not clear the newer copy, got {ops:?}"
    );
    assert_eq!(driver.view().copied_item_id.as_deref(), Some("b"));

    // Generation 2's timer, however, does clear it.
    let ops = driver.resolve_matching(
        |op| matches!(op, ShellOperation::Wait { .. }),
        ShellResult::Waited,
    );
    assert!(ops.contains(&ShellOperation::ClearClipboard));
    assert_eq!(driver.view().copied_item_id, None);
}

#[test]
fn copying_an_unknown_item_asks_for_nothing() {
    let mut driver = loaded();
    let ops = driver.dispatch(Event::CopyRequested {
        item_id: "does-not-exist".into(),
        field: CopyField::Secret,
    });
    assert!(ops.is_empty(), "got {ops:?}");
}

#[test]
fn the_favorites_filter_composes_with_search() {
    let mut driver = loaded();
    driver.dispatch(Event::FavoriteToggled {
        item_id: "a".into(),
    });
    driver.dispatch(Event::FavoritesOnlyToggled);
    assert_eq!(driver.view().visible.len(), 1);

    driver.dispatch(Event::QueryChanged {
        query: "bank".into(),
    });
    assert!(
        driver.view().visible.is_empty(),
        "Bank is not favorited, so the two filters intersect to nothing"
    );
}
