//! The unlocked vault: nav sidebar, item list, detail panel.

use gpui::{
    Div, FontWeight, InteractiveElement as _, ParentElement, SharedString,
    StatefulInteractiveElement as _, Styled, div, px,
};

use airkeys_core::app::item_list::{EmptyReason, ItemListView};
use airkeys_core::strength::StrengthReport;
use airkeys_core::types::Item;

use crate::strings::{item_kind_label, strength_hint_label, strength_label};
use crate::theme::{
    self, DETAIL_W, GAP_LG, GAP_MD, GAP_SM, HAIRLINE, NAV_ROW_H, PAD_TOP, PAD_X, RADIUS_ROW,
    SIDEBAR_PAD, SIDEBAR_TOP, SIDEBAR_W,
};
use crate::ui::{
    ButtonVariant, ListRow, button, kind_badge, list_row, search_field, status_note, strength_meter,
};

use super::{Note, ScreenCtx, Sink};

/// What the user did. The host translates these into core events.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    Select(String),
    ToggleFavorite(String),
    CopyAccount(String),
    CopySecret(String),
    ToggleFavoritesOnly,
    Lock,
    DismissNote,
}

/// Which nav row is highlighted. Presentation state, owned by the host — the
/// item-list machine has no opinion about navigation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Nav {
    Items,
    Favorites,
    Generator,
    Settings,
}

pub struct VaultChrome<'a> {
    pub nav: Nav,
    pub selected: Option<&'a Item>,
    /// The selected item's strength verdict. A verdict, not a secret — see
    /// `DemoShell::strength_of`.
    pub strength: Option<&'a StrengthReport>,
    pub search_focused: bool,
    pub note: Option<&'a Note>,
    pub vault_label: SharedString,
}

pub fn render(
    view: &ItemListView,
    chrome: &VaultChrome<'_>,
    ctx: &ScreenCtx<'_>,
    sink: Sink<Intent>,
) -> Div {
    div()
        .size_full()
        .flex()
        .text_color(ctx.theme.fg_base)
        .child(sidebar(view, chrome, ctx, sink.clone()))
        .child(list_column(view, chrome, ctx, sink.clone()))
        .child(detail_column(view, chrome, ctx, sink))
}

// ---------------------------------------------------------------------------
// Column 1 — nav
// ---------------------------------------------------------------------------

fn sidebar(
    view: &ItemListView,
    chrome: &VaultChrome<'_>,
    ctx: &ScreenCtx<'_>,
    sink: Sink<Intent>,
) -> Div {
    let theme = ctx.theme;
    let strings = ctx.strings;

    let row = |id: &'static str, label: SharedString, nav: Nav| {
        let selected = chrome.nav == nav;
        let (hover, active) = (theme.bg_sunken, theme.divider);
        div()
            .id(id)
            .h(px(NAV_ROW_H))
            .w_full()
            .px(px(GAP_SM))
            .rounded(px(RADIUS_ROW))
            .flex()
            .items_center()
            .cursor_pointer()
            .text_size(theme::text_row_title())
            .font_weight(if selected {
                FontWeight::SEMIBOLD
            } else {
                FontWeight::NORMAL
            })
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
            .active(move |s| s.bg(active))
            .child(label)
    };

    let favorites_sink = sink.clone();
    let lock_sink = sink;

    div()
        .w(px(SIDEBAR_W))
        .h_full()
        .flex_none()
        .bg(theme.bg_base)
        .border_r_1()
        .border_color(theme.panel_edge)
        // Clears the macOS traffic lights, which sit over the content because
        // the titlebar is transparent.
        .pt(px(SIDEBAR_TOP))
        .px(px(SIDEBAR_PAD))
        .pb(px(SIDEBAR_PAD))
        .flex()
        .flex_col()
        .gap(px(4.))
        .child(
            div()
                .px(px(GAP_SM))
                .pb(px(GAP_MD))
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(theme::text_section())
                        .font_weight(FontWeight::BOLD)
                        .truncate()
                        .child(chrome.vault_label.clone()),
                )
                .child(
                    div()
                        .text_size(theme::text_label())
                        .text_color(theme.fg_subtle)
                        .child(ctx.loc.t_count("vault.itemCount", f64::from(view.total))),
                ),
        )
        .child(
            row("nav-items", ctx.strings.nav_items.clone(), Nav::Items).on_click({
                let sink = favorites_sink.clone();
                move |_, window, cx| {
                    // "All items" clears the favorites filter when it is on.
                    sink(Intent::ToggleFavoritesOnly, window, cx)
                }
            }),
        )
        .child(
            row(
                "nav-favorites",
                strings.nav_favorites.clone(),
                Nav::Favorites,
            )
            .on_click({
                let sink = favorites_sink;
                move |_, window, cx| sink(Intent::ToggleFavoritesOnly, window, cx)
            }),
        )
        .child(row(
            "nav-generator",
            strings.nav_generator.clone(),
            Nav::Generator,
        ))
        .child(row(
            "nav-settings",
            strings.nav_settings.clone(),
            Nav::Settings,
        ))
        .child(div().flex_1())
        .child(button(
            "lock-now",
            ButtonVariant::Quiet,
            strings.lock_now.clone(),
            theme,
            move |_, window, cx| lock_sink(Intent::Lock, window, cx),
        ))
}

// ---------------------------------------------------------------------------
// Column 2 — the list
// ---------------------------------------------------------------------------

fn list_column(
    view: &ItemListView,
    chrome: &VaultChrome<'_>,
    ctx: &ScreenCtx<'_>,
    sink: Sink<Intent>,
) -> Div {
    let theme = ctx.theme;

    let mut column = div()
        .flex_1()
        // Same lesson as the unlock screen's left column: opt into shrinking,
        // or the fixed-width detail panel gets pushed out of the window.
        .min_w(px(0.))
        .h_full()
        .bg(theme.bg_base)
        .pt(px(PAD_TOP))
        .px(px(PAD_X))
        .flex()
        .flex_col()
        .gap(px(GAP_MD))
        .child(search_field(
            theme,
            &view.query,
            ctx.strings.search_placeholder.clone(),
            chrome.search_focused,
        ));

    if let Some(note) = chrome.note {
        let sink = sink.clone();
        column = column.child(
            div()
                .flex()
                .flex_col()
                .gap(px(GAP_SM))
                .child(status_note(
                    theme,
                    note.tone,
                    note.title.clone(),
                    note.body.clone(),
                ))
                .child(button(
                    "vault-dismiss-note",
                    ButtonVariant::Quiet,
                    ctx.strings.close.clone(),
                    theme,
                    move |_, window, cx| sink(Intent::DismissNote, window, cx),
                )),
        );
    }

    // An error is not an empty state: the list may well have items the read
    // could not reach, and telling the user their vault is empty in that
    // situation is the worst possible wrong answer.
    if let Some(error) = &view.error {
        column = column.child(status_note(
            theme,
            crate::ui::StatusTone::Error,
            SharedString::from(error.clone()),
            None,
        ));
    }

    match view.empty_reason {
        Some(EmptyReason::NoItems) => {
            column = column.child(empty_state(
                ctx,
                ctx.strings.empty_title.clone(),
                ctx.strings.empty_body.clone(),
            ));
        }
        Some(EmptyReason::NoMatches) => {
            column = column.child(empty_state(
                ctx,
                ctx.strings.no_match_title.clone(),
                ctx.loc
                    .t_vars("vault.noMatchBody", &[("query", &view.query)]),
            ));
        }
        None => {
            let mut rows = div()
                .id("item-list")
                .flex_1()
                .min_h(px(0.))
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .gap(px(2.));
            for item in &view.visible {
                let id = item.id.clone();
                let sink = sink.clone();
                rows = rows.child(list_row(
                    SharedString::from(format!("row-{id}")),
                    theme,
                    ListRow {
                        glyph: glyph_for(item),
                        title: SharedString::from(item.title.clone()),
                        subtitle: SharedString::from(item.account.clone()),
                        favorite: item.favorite,
                        selected: chrome.selected.map(|s| s.id == item.id).unwrap_or(false),
                        copied: view
                            .copied_item_id
                            .as_deref()
                            .filter(|copied| *copied == item.id)
                            .map(|_| ctx.strings.copied.clone()),
                    },
                    move |_, window, cx| sink(Intent::Select(id.clone()), window, cx),
                ));
            }
            column = column.child(rows);
        }
    }

    column
}

/// A one-character stand-in for a per-kind icon. Deliberately not an emoji: the
/// glyph a platform substitutes for an emoji varies wildly, and a list where
/// every row's icon looks different on Windows is worse than initials.
fn glyph_for(item: &Item) -> SharedString {
    use airkeys_core::types::ItemKind;
    SharedString::from(match item.kind {
        ItemKind::Login => "L",
        ItemKind::Note => "N",
        ItemKind::Key => "K",
        ItemKind::Card => "C",
    })
}

fn empty_state(ctx: &ScreenCtx<'_>, title: SharedString, body: SharedString) -> Div {
    div()
        .flex_1()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(GAP_SM))
        .child(
            div()
                .text_size(theme::text_section())
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(ctx.theme.fg_base)
                .child(title),
        )
        .child(
            div()
                .text_size(theme::text_body())
                .text_color(ctx.theme.fg_muted)
                .child(body),
        )
}

// ---------------------------------------------------------------------------
// Column 3 — detail
// ---------------------------------------------------------------------------

fn detail_column(
    view: &ItemListView,
    chrome: &VaultChrome<'_>,
    ctx: &ScreenCtx<'_>,
    sink: Sink<Intent>,
) -> Div {
    let theme = ctx.theme;

    let panel = div()
        .w(px(DETAIL_W))
        .h_full()
        .flex_none()
        .bg(theme.bg_raised)
        .border_l_1()
        .border_color(theme.panel_edge)
        .pt(px(PAD_TOP))
        .px(px(PAD_X))
        .flex()
        .flex_col()
        .gap(px(GAP_MD));

    let Some(item) = chrome.selected else {
        return panel.child(
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_size(theme::text_body())
                .text_color(ctx.theme.fg_subtle)
                .child(ctx.strings.search.clone()),
        );
    };

    let copied_this = view.copied_item_id.as_deref() == Some(item.id.as_str());
    let account_sink = sink.clone();
    let secret_sink = sink.clone();
    let favorite_sink = sink;
    let (account_id, secret_id, favorite_id) = (item.id.clone(), item.id.clone(), item.id.clone());

    panel
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(GAP_SM))
                .child(kind_badge(theme, item_kind_label(ctx.loc, item.kind)))
                .child(div().flex_1()),
        )
        .child(
            div()
                .text_size(theme::text_title())
                .font_weight(FontWeight::BOLD)
                .child(SharedString::from(item.title.clone())),
        )
        .child(div().h(px(HAIRLINE)).w_full().bg(theme.divider))
        .child(field(
            ctx,
            ctx.strings.label_account.clone(),
            &item.account,
            false,
        ))
        .child(field(
            ctx,
            ctx.strings.label_site.clone(),
            &item.site,
            false,
        ))
        // The secret is NEVER in the view model — this renders a fixed mask, and
        // the only way to see the real value is the copy button, which asks the
        // shell for it by id.
        .child(field(
            ctx,
            ctx.strings.label_secret.clone(),
            "••••••••••••",
            true,
        ))
        .children(chrome.strength.map(|report| {
            strength_meter(
                theme,
                report,
                strength_label(ctx.loc, report.strength),
                report
                    .hints
                    .iter()
                    .map(|hint| strength_hint_label(ctx.loc, *hint))
                    .collect(),
            )
        }))
        .child(field(
            ctx,
            ctx.strings.label_updated.clone(),
            &item.updated_at_iso,
            false,
        ))
        .child(div().flex_1())
        .child(
            div()
                .pb(px(GAP_LG))
                .flex()
                .flex_col()
                .gap(px(GAP_SM))
                .child(button(
                    "copy-secret",
                    ButtonVariant::Primary,
                    if copied_this {
                        ctx.strings.copied.clone()
                    } else {
                        ctx.strings.copy_secret.clone()
                    },
                    theme,
                    move |_, window, cx| {
                        secret_sink(Intent::CopySecret(secret_id.clone()), window, cx)
                    },
                ))
                .child(button(
                    "copy-account",
                    ButtonVariant::Secondary,
                    ctx.strings.copy_account.clone(),
                    theme,
                    move |_, window, cx| {
                        account_sink(Intent::CopyAccount(account_id.clone()), window, cx)
                    },
                ))
                .child(button(
                    "toggle-favorite",
                    ButtonVariant::Quiet,
                    SharedString::from(if item.favorite { "★" } else { "☆" }),
                    theme,
                    move |_, window, cx| {
                        favorite_sink(Intent::ToggleFavorite(favorite_id.clone()), window, cx)
                    },
                )),
        )
}

fn field(ctx: &ScreenCtx<'_>, label: SharedString, value: &str, mono: bool) -> Div {
    let theme = ctx.theme;
    let mut text = div()
        .truncate()
        .text_size(theme::text_body())
        .text_color(theme.fg_base)
        .child(SharedString::from(value.to_owned()));
    if mono {
        text = text
            .font_family(theme::font_mono())
            .text_size(theme::text_mono());
    }

    div()
        .flex()
        .flex_col()
        .gap(px(2.))
        .child(
            div()
                .text_size(theme::text_label())
                .font_weight(FontWeight::MEDIUM)
                .text_color(theme.fg_subtle)
                .child(label),
        )
        .child(text)
}
