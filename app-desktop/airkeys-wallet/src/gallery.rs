//! Dev-only state gallery: every screen state, selectable from a list.
//!
//! **This file is the payoff for making screens pure functions.** A state like
//! "the authenticator is locked out, and the vault has one favourited item that
//! is currently on the clipboard" is nearly impossible to reach by hand and
//! trivial to *write down* — so here it is written down, and the same render
//! function that ships draws it.
//!
//! Reached only when `AIRKEYS_GALLERY=1` — the same env-pin family as
//! `AIRKEYS_THEME`, `AIRKEYS_LANG` and `AIRKEYS_DEMO`. Release users never see
//! it. Presses are logged, never wired: the gallery has no state machine, on
//! purpose.

use std::rc::Rc;

use gpui::{
    Context, Div, FocusHandle, FontWeight, InteractiveElement as _, IntoElement, KeyDownEvent,
    ParentElement, Render, SharedString, Stateful, StatefulInteractiveElement as _, Styled, Window,
    div, px,
};

use airkeys_core::app::StatusKey;
use airkeys_core::app::item_list::{EmptyReason, ItemListView};
use airkeys_core::app::unlock::UnlockView;
use airkeys_core::strength::{StrengthReport, secret_strength};
use airkeys_core::types::{Item, ItemKind};

use crate::loc::Loc;
use crate::screens::{Note, ScreenCtx, Sink, unlock as unlock_screen, vault as vault_screen};
use crate::strings::Strings;
use crate::theme::{self, GALLERY_SIDEBAR_W, GAP_SM, RADIUS_ROW, SIDEBAR_PAD, Theme, ThemeMode};
use crate::ui::StatusTone;

/// The gallery gate.
pub fn gallery_enabled() -> bool {
    std::env::var("AIRKEYS_GALLERY").as_deref() == Ok("1")
}

/// One authored screen state.
///
/// The variants differ a lot in size and that is fine here: exactly one exists
/// at a time, in a dev-only tool, and boxing the larger one would buy an
/// indirection in exchange for nothing measurable.
#[allow(clippy::large_enum_variant)]
enum State {
    Unlock {
        view: UnlockView,
        note: Option<Note>,
    },
    Vault {
        view: ItemListView,
        selected: Option<Item>,
        /// The strength verdict for the selected item, authored rather than
        /// computed — a Weak secret next to an Excellent one is exactly the
        /// comparison the meter has to survive, and no fixture vault would
        /// naturally contain both.
        strength: Option<StrengthReport>,
        note: Option<Note>,
    },
}

struct Fixture {
    group: &'static str,
    name: &'static str,
    state: State,
}

pub struct Gallery {
    mode: ThemeMode,
    loc: Loc,
    strings: Strings,
    fixtures: Vec<Fixture>,
    selected: usize,
    focus_handle: FocusHandle,
}

impl Gallery {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let loc = Loc::from_env();
        eprintln!("[airkeys] gallery: locale `{}`", loc.language());
        let strings = Strings::resolve(&loc);
        let focus_handle = cx.focus_handle();
        focus_handle.focus(window, cx);

        Self {
            mode: ThemeMode::detect(window),
            loc,
            strings,
            fixtures: fixtures(),
            selected: 0,
            focus_handle,
        }
    }

    fn select(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.fixtures.len() {
            self.selected = index;
            cx.notify();
        }
    }

    fn step(&mut self, delta: isize, cx: &mut Context<Self>) {
        let count = self.fixtures.len() as isize;
        if count == 0 {
            return;
        }
        let next = (self.selected as isize + delta).rem_euclid(count) as usize;
        self.select(next, cx);
    }

    fn sidebar(&self, theme: &Theme, cx: &mut Context<Self>) -> Stateful<Div> {
        let mut column = div()
            .w(px(GALLERY_SIDEBAR_W))
            .h_full()
            .flex_none()
            .bg(theme.bg_base)
            .border_r_1()
            .border_color(theme.panel_edge)
            .p(px(SIDEBAR_PAD))
            .pt(px(theme::SIDEBAR_TOP))
            .flex()
            .flex_col()
            .gap(px(2.))
            .id("gallery-list")
            .overflow_y_scroll();

        let mut last_group = "";
        for (index, fixture) in self.fixtures.iter().enumerate() {
            if fixture.group != last_group {
                last_group = fixture.group;
                column = column.child(
                    div()
                        .px(px(GAP_SM))
                        .pt(px(GAP_SM))
                        .pb(px(4.))
                        .text_size(theme::text_label())
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.fg_subtle)
                        .child(SharedString::from(fixture.group)),
                );
            }

            let selected = index == self.selected;
            let hover = theme.bg_sunken;
            column = column.child(
                div()
                    .id(SharedString::from(format!("fixture-{index}")))
                    .px(px(GAP_SM))
                    .py(px(6.))
                    .rounded(px(RADIUS_ROW))
                    .cursor_pointer()
                    .text_size(theme::text_row_sub())
                    .text_color(if selected {
                        theme.fg_base
                    } else {
                        theme.fg_muted
                    })
                    .bg(if selected {
                        theme.accent_soft
                    } else {
                        theme.bg_base
                    })
                    .hover(move |s| s.bg(hover))
                    .child(SharedString::from(fixture.name))
                    .on_click(cx.listener(move |this, _, _, cx| this.select(index, cx))),
            );
        }
        column
    }
}

impl Render for Gallery {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::of(self.mode);
        let ctx = ScreenCtx {
            theme: &theme,
            strings: &self.strings,
            loc: &self.loc,
        };

        // The sinks log rather than act. A gallery that could change state would
        // stop being a gallery of *authored* states.
        let unlock_sink: Sink<unlock_screen::Intent> = Rc::new(|intent, _, _| {
            eprintln!("[airkeys] gallery: unlock intent {intent:?}");
        });
        let vault_sink: Sink<vault_screen::Intent> = Rc::new(|intent, _, _| {
            eprintln!("[airkeys] gallery: vault intent {intent:?}");
        });

        let stage = match &self.fixtures[self.selected].state {
            State::Unlock { view, note } => {
                unlock_screen::render(view, note.as_ref(), &ctx, unlock_sink)
            }
            State::Vault {
                view,
                selected,
                strength,
                note,
            } => {
                let chrome = vault_screen::VaultChrome {
                    nav: vault_screen::Nav::Items,
                    selected: selected.as_ref(),
                    strength: strength.as_ref(),
                    search_focused: false,
                    note: note.as_ref(),
                    vault_label: SharedString::from("Personal"),
                };
                vault_screen::render(view, &chrome, &ctx, vault_sink)
            }
        };

        div()
            .size_full()
            .flex()
            .bg(theme.bg_base)
            .text_color(theme.fg_base)
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                match event.keystroke.key.as_str() {
                    "down" | "j" => this.step(1, cx),
                    "up" | "k" => this.step(-1, cx),
                    _ => {}
                }
            }))
            .child(self.sidebar(&theme, cx))
            .child(div().flex_1().min_w(px(0.)).h_full().child(stage))
    }
}

// ---------------------------------------------------------------------------
// The authored states
// ---------------------------------------------------------------------------

fn item(id: &str, kind: ItemKind, title: &str, account: &str, favorite: bool) -> Item {
    Item {
        id: id.into(),
        kind,
        title: title.into(),
        account: account.into(),
        site: "example.com".into(),
        favorite,
        updated_at_iso: "2026-08-11T21:14:00.000Z".into(),
    }
}

fn items() -> Vec<Item> {
    vec![
        item("itm-1", ItemKind::Login, "GitHub", "ann@example.com", true),
        item(
            "itm-2",
            ItemKind::Login,
            "Cloudflare",
            "ops@example.com",
            false,
        ),
        item(
            "itm-3",
            ItemKind::Key,
            "Deploy key — prod",
            "deploy@infra",
            true,
        ),
        item(
            "itm-6",
            ItemKind::Login,
            "A service with a genuinely long display name",
            "a-very-long-account-identifier@example.com",
            false,
        ),
    ]
}

fn unlock_view(busy: bool, status: Option<StatusKey>, waited: u32) -> UnlockView {
    UnlockView {
        busy,
        status,
        waited_secs: waited,
        vault_label: Some("Personal".into()),
        vault_missing: false,
        item_count: 12,
    }
}

fn list_view(visible: Vec<Item>) -> ItemListView {
    ItemListView {
        loading: false,
        query: String::new(),
        favorites_only: false,
        total: 4,
        visible,
        copied_item_id: None,
        empty_reason: None,
        error: None,
    }
}

fn fixtures() -> Vec<Fixture> {
    let note = |tone, title: &str, body: &str| Note {
        tone,
        title: SharedString::from(title.to_owned()),
        body: Some(SharedString::from(body.to_owned())),
    };

    vec![
        // -- unlock ----------------------------------------------------------
        Fixture {
            group: "Unlock",
            name: "idle — vault known",
            state: State::Unlock {
                view: unlock_view(false, None, 0),
                note: None,
            },
        },
        Fixture {
            group: "Unlock",
            name: "checking the device",
            state: State::Unlock {
                view: unlock_view(true, Some(StatusKey::CheckingDevice), 0),
                note: None,
            },
        },
        Fixture {
            group: "Unlock",
            name: "awaiting biometric",
            state: State::Unlock {
                view: unlock_view(true, Some(StatusKey::AwaitingBiometric), 1),
                note: None,
            },
        },
        Fixture {
            // The state the elapsed counter exists for, and the one nobody can
            // reach on purpose without waiting for it.
            group: "Unlock",
            name: "awaiting biometric — 9s",
            state: State::Unlock {
                view: unlock_view(true, Some(StatusKey::AwaitingBiometric), 9),
                note: None,
            },
        },
        Fixture {
            group: "Unlock",
            name: "opening the vault",
            state: State::Unlock {
                view: unlock_view(true, Some(StatusKey::OpeningVault), 0),
                note: None,
            },
        },
        Fixture {
            group: "Unlock",
            name: "no vault on this device",
            state: State::Unlock {
                view: UnlockView {
                    busy: false,
                    status: None,
                    waited_secs: 0,
                    vault_label: None,
                    vault_missing: true,
                    item_count: 0,
                },
                note: None,
            },
        },
        Fixture {
            group: "Unlock",
            name: "authenticator not enrolled",
            state: State::Unlock {
                view: unlock_view(false, None, 0),
                note: Some(note(
                    StatusTone::Warning,
                    "This device cannot unlock a vault",
                    "No biometric or device passcode is enrolled.",
                )),
            },
        },
        Fixture {
            group: "Unlock",
            name: "locked out",
            state: State::Unlock {
                view: unlock_view(false, None, 0),
                note: Some(note(
                    StatusTone::Error,
                    "Could not unlock",
                    "Too many attempts. Try again later.",
                )),
            },
        },
        Fixture {
            group: "Unlock",
            name: "storage read failed",
            state: State::Unlock {
                view: UnlockView {
                    busy: false,
                    status: None,
                    waited_secs: 0,
                    vault_label: None,
                    // NOT `vault_missing` — a failed read is a different state
                    // from an absent vault, and this fixture is where that
                    // difference is visible side by side.
                    vault_missing: false,
                    item_count: 0,
                },
                note: Some(note(
                    StatusTone::Error,
                    "Could not unlock",
                    "The vault file could not be read.",
                )),
            },
        },
        // -- vault -----------------------------------------------------------
        Fixture {
            group: "Vault",
            name: "list — nothing selected",
            state: State::Vault {
                view: list_view(items()),
                selected: None,
                strength: None,
                note: None,
            },
        },
        Fixture {
            group: "Vault",
            name: "list — item selected",
            state: State::Vault {
                view: list_view(items()),
                selected: items().into_iter().next(),
                strength: Some(secret_strength("hunter2")),
                note: None,
            },
        },
        Fixture {
            group: "Vault",
            name: "copied — transient affordance",
            state: State::Vault {
                view: ItemListView {
                    copied_item_id: Some("itm-1".into()),
                    ..list_view(items())
                },
                selected: items().into_iter().next(),
                strength: Some(secret_strength("hunter2")),
                note: None,
            },
        },
        Fixture {
            group: "Vault",
            name: "search — no matches",
            state: State::Vault {
                view: ItemListView {
                    query: "nothing".into(),
                    visible: vec![],
                    empty_reason: Some(EmptyReason::NoMatches),
                    ..list_view(vec![])
                },
                selected: None,
                strength: None,
                note: None,
            },
        },
        Fixture {
            group: "Vault",
            name: "empty vault",
            state: State::Vault {
                view: ItemListView {
                    total: 0,
                    visible: vec![],
                    empty_reason: Some(EmptyReason::NoItems),
                    ..list_view(vec![])
                },
                selected: None,
                strength: None,
                note: None,
            },
        },
        Fixture {
            group: "Vault",
            name: "read failed — NOT empty",
            state: State::Vault {
                view: ItemListView {
                    total: 0,
                    visible: vec![],
                    // Deliberately `None`: a failed read must not render as an
                    // empty vault, and this fixture is the proof.
                    empty_reason: None,
                    error: Some("the vault file could not be read".into()),
                    ..list_view(vec![])
                },
                selected: None,
                strength: None,
                note: None,
            },
        },
        Fixture {
            group: "Vault",
            name: "favorite write failed",
            state: State::Vault {
                view: ItemListView {
                    error: Some("the vault is read-only".into()),
                    ..list_view(items())
                },
                selected: items().into_iter().next(),
                strength: Some(secret_strength("hunter2")),
                note: None,
            },
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The gallery is only useful if it stays complete. This is the cheap
    /// version of that: every group has states, and no two fixtures share a
    /// name (which would make one of them unreachable in the sidebar).
    #[test]
    fn fixtures_are_named_uniquely_and_cover_both_screens() {
        let fixtures = fixtures();
        assert!(fixtures.iter().any(|f| f.group == "Unlock"));
        assert!(fixtures.iter().any(|f| f.group == "Vault"));

        let mut names: Vec<&str> = fixtures.iter().map(|f| f.name).collect();
        names.sort_unstable();
        let count = names.len();
        names.dedup();
        assert_eq!(count, names.len(), "two fixtures share a name");
    }

    /// The two states that are easiest to conflate, and most costly to get
    /// wrong, must both be present and must differ.
    #[test]
    fn a_failed_read_and_an_empty_vault_are_both_authored() {
        let fixtures = fixtures();
        let empty = fixtures
            .iter()
            .find(|f| f.name == "empty vault")
            .expect("empty vault fixture");
        let failed = fixtures
            .iter()
            .find(|f| f.name == "read failed — NOT empty")
            .expect("read failed fixture");

        let (State::Vault { view: empty, .. }, State::Vault { view: failed, .. }) =
            (&empty.state, &failed.state)
        else {
            panic!("both fixtures must be vault states");
        };
        assert_eq!(empty.empty_reason, Some(EmptyReason::NoItems));
        assert_eq!(failed.empty_reason, None);
        assert!(failed.error.is_some());
    }
}
