//! Machine B — the unlocked vault's item list: load, search, favorite, copy.
//!
//! Where [`unlock`](super::unlock) is a linear ceremony, this one is the shape
//! most product screens actually have: several independent operations in flight
//! at once, over a list the user is editing while they resolve. Three rules
//! carry it:
//!
//! 1. **The visible list is derived, never stored.** `view()` filters; nothing
//!    caches the filtered result. A cached list is how a favorited item stays
//!    visible in a filter it no longer matches.
//! 2. **A favorite toggle is optimistic, and reverts on failure.** The tap
//!    lands instantly; if the write fails the star goes back and the error is
//!    surfaced. Silently keeping the optimistic state is how a user believes
//!    something is saved that is not.
//! 3. **A newer copy supersedes the older clipboard clear.** Copy A schedules a
//!    wipe 30 s out; copy B at 20 s must not have its value wiped at 30. Every
//!    copy carries a generation, and a clear whose generation is stale does
//!    nothing. This is the whole reason those `generation` fields exist.

use crux_core::{render::render, App, Command};
use serde::{Deserialize, Serialize};

use super::shell::{CopyField, CopyRequest, Effect, ShellOperation, ShellResult};
use crate::types::Item;

#[cfg(feature = "bindings")]
use ts_rs::TS;

/// How long a copied secret is allowed to sit on the clipboard.
pub const CLIPBOARD_CLEAR_MS: u32 = 30_000;

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[cfg_attr(feature = "bindings", derive(TS), ts(rename = "ItemListEvent"))]
pub enum Event {
    /// Screen mounted for an open vault.
    Start {
        vault_id: String,
    },
    QueryChanged {
        query: String,
    },
    FavoritesOnlyToggled,
    FavoriteToggled {
        item_id: String,
    },
    CopyRequested {
        item_id: String,
        field: CopyField,
    },

    /// Internal: the item load resolved.
    #[serde(skip)]
    ItemsCompleted {
        result: ShellResult,
    },
    /// Internal: a favorite write resolved. Carries what to revert to — the
    /// answer must know which optimistic edit it belongs to, because a second
    /// toggle may already have landed.
    #[serde(skip)]
    SaveCompleted {
        item_id: String,
        previous: bool,
        result: ShellResult,
    },
    /// Internal: a copy resolved.
    #[serde(skip)]
    CopyCompleted {
        generation: u64,
        result: ShellResult,
    },
    /// Internal: a scheduled clipboard wipe came due.
    #[serde(skip)]
    ClearCompleted {
        generation: u64,
        result: ShellResult,
    },
}

// ---------------------------------------------------------------------------
// Model
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct Model {
    vault_id: String,
    items: Vec<Item>,
    query: String,
    favorites_only: bool,
    loading: bool,
    loaded: bool,
    error: Option<String>,
    /// Which item's field is on the clipboard right now, and the generation
    /// that put it there.
    copied: Option<(String, u64)>,
    copy_generation: u64,
}

// ---------------------------------------------------------------------------
// ViewModel
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "bindings", derive(TS))]
pub enum EmptyReason {
    /// The vault genuinely has nothing in it.
    NoItems,
    /// It has items; none match the current query or filter. Different copy,
    /// and different next action — clearing the search versus adding an item.
    NoMatches,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(TS))]
pub struct ItemListView {
    pub loading: bool,
    pub query: String,
    pub favorites_only: bool,
    /// Everything in the vault, before filtering — the "12 items" counter.
    pub total: u32,
    /// What the list should render, already filtered and ordered.
    pub visible: Vec<Item>,
    /// The item whose field is currently on the clipboard, for the transient
    /// "Copied" affordance.
    pub copied_item_id: Option<String>,
    pub empty_reason: Option<EmptyReason>,
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct ItemList;

impl App for ItemList {
    type Event = Event;
    type Model = Model;
    type ViewModel = ItemListView;
    type Effect = Effect;

    fn update(&self, event: Event, model: &mut Model) -> Command<Effect, Event> {
        match event {
            Event::Start { vault_id } => {
                model.vault_id = vault_id.clone();
                model.loading = true;
                model.error = None;
                Command::all([
                    Command::request_from_shell(ShellOperation::LoadItems { vault_id })
                        .then_send(|result| Event::ItemsCompleted { result }),
                    render(),
                ])
            }

            Event::QueryChanged { query } => {
                model.query = query;
                render()
            }

            Event::FavoritesOnlyToggled => {
                model.favorites_only = !model.favorites_only;
                render()
            }

            Event::FavoriteToggled { item_id } => toggle_favorite(model, &item_id),

            Event::CopyRequested { item_id, field } => copy(model, item_id, field),

            Event::ItemsCompleted { result } => {
                model.loading = false;
                match result {
                    ShellResult::ItemsLoaded { items } => {
                        model.items = items;
                        model.loaded = true;
                        model.error = None;
                    }
                    ShellResult::StorageFailed { message } => {
                        // `loaded` stays false: an empty list because the read
                        // failed must not render as "this vault is empty".
                        model.error = Some(message);
                    }
                    _ => {}
                }
                render()
            }

            Event::SaveCompleted {
                item_id,
                previous,
                result,
            } => {
                match result {
                    ShellResult::ItemSaved => Command::done(),
                    ShellResult::StorageFailed { message } => {
                        // Rule 2: put the star back exactly where it was.
                        if let Some(item) = model.items.iter_mut().find(|item| item.id == item_id) {
                            item.favorite = previous;
                        }
                        model.error = Some(message);
                        render()
                    }
                    _ => Command::done(),
                }
            }

            Event::CopyCompleted { generation, result } => {
                if generation != model.copy_generation {
                    return Command::done(); // a newer copy already superseded this one
                }
                match result {
                    ShellResult::Copied => {
                        // Schedule the wipe. Rule 3: it is tagged with the
                        // generation that scheduled it.
                        Command::all([
                            Command::request_from_shell(ShellOperation::Wait {
                                ms: CLIPBOARD_CLEAR_MS,
                            })
                            .then_send(move |result| Event::ClearCompleted { generation, result }),
                            render(),
                        ])
                    }
                    ShellResult::StorageFailed { message } => {
                        model.copied = None;
                        model.error = Some(message);
                        render()
                    }
                    _ => Command::done(),
                }
            }

            Event::ClearCompleted { generation, result } => {
                // Rule 3, the half that matters: a wipe scheduled by an older
                // copy must not touch what a newer copy put there.
                if generation != model.copy_generation {
                    return Command::done();
                }
                match result {
                    ShellResult::Waited => {
                        model.copied = None;
                        Command::all([
                            Command::request_from_shell(ShellOperation::ClearClipboard).then_send(
                                move |result| Event::ClearCompleted { generation, result },
                            ),
                            render(),
                        ])
                    }
                    ShellResult::ClipboardCleared => render(),
                    _ => Command::done(),
                }
            }
        }
    }

    fn view(&self, model: &Model) -> ItemListView {
        // Rule 1: derived here, every frame, from the model — never cached.
        let visible: Vec<Item> = model
            .items
            .iter()
            .filter(|item| !model.favorites_only || item.favorite)
            .filter(|item| matches_query(item, &model.query))
            .cloned()
            .collect();

        let empty_reason = if !visible.is_empty() || model.loading || !model.loaded {
            None
        } else if model.items.is_empty() {
            Some(EmptyReason::NoItems)
        } else {
            Some(EmptyReason::NoMatches)
        };

        ItemListView {
            loading: model.loading,
            query: model.query.clone(),
            favorites_only: model.favorites_only,
            total: model.items.len() as u32,
            visible,
            copied_item_id: model.copied.as_ref().map(|(id, _)| id.clone()),
            empty_reason,
            error: model.error.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// Transitions
// ---------------------------------------------------------------------------

fn toggle_favorite(model: &mut Model, item_id: &str) -> Command<Effect, Event> {
    let Some(item) = model.items.iter_mut().find(|item| item.id == item_id) else {
        return Command::done();
    };
    let previous = item.favorite;
    item.favorite = !previous;
    let item = item.clone();
    let id = item.id.clone();
    model.error = None;

    Command::all([
        Command::request_from_shell(ShellOperation::SaveItem { item }).then_send(move |result| {
            Event::SaveCompleted {
                item_id: id.clone(),
                previous,
                result,
            }
        }),
        render(),
    ])
}

fn copy(model: &mut Model, item_id: String, field: CopyField) -> Command<Effect, Event> {
    if !model.items.iter().any(|item| item.id == item_id) {
        return Command::done();
    }
    model.copy_generation += 1;
    let generation = model.copy_generation;
    model.copied = Some((item_id.clone(), generation));
    model.error = None;

    Command::all([
        Command::request_from_shell(ShellOperation::CopyField {
            request: CopyRequest { item_id, field },
        })
        .then_send(move |result| Event::CopyCompleted { generation, result }),
        render(),
    ])
}

/// Case-insensitive substring match over the fields a user would search by.
///
/// Deliberately not fuzzy. A vault search that returns *nearly* the right login
/// is worse than one that returns nothing: the user copies the wrong password
/// into the right site and only finds out when the account locks.
fn matches_query(item: &Item, query: &str) -> bool {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return true;
    }
    [&item.title, &item.account, &item.site]
        .into_iter()
        .any(|field| field.to_lowercase().contains(&needle))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ItemKind;

    fn item(title: &str, account: &str, site: &str) -> Item {
        Item {
            id: "id".into(),
            kind: ItemKind::Login,
            title: title.into(),
            account: account.into(),
            site: site.into(),
            favorite: false,
            updated_at_iso: "2026-08-12T00:00:00Z".into(),
        }
    }

    #[test]
    fn an_empty_query_matches_everything() {
        assert!(matches_query(&item("GitHub", "ann", "github.com"), ""));
        assert!(matches_query(&item("GitHub", "ann", "github.com"), "   "));
    }

    #[test]
    fn matching_is_case_insensitive_across_the_searchable_fields() {
        let entry = item("GitHub", "ann@example.com", "github.com");
        assert!(matches_query(&entry, "github"));
        assert!(matches_query(&entry, "ANN@"));
        assert!(matches_query(&entry, ".com"));
        assert!(!matches_query(&entry, "gitlab"));
    }
}
