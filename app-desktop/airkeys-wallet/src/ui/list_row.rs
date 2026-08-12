//! One vault item in the list.
//!
//! Takes a plain data struct rather than `airkeys_core::types::Item` on
//! purpose: the row renders a title, a subtitle and some flags, and saying so
//! in the signature keeps the component reusable for search results, favorites
//! and anything else that is a list of things — and keeps it honest that a row
//! has no business holding a secret.

use gpui::{
    App, ClickEvent, Div, ElementId, FontWeight, InteractiveElement, ParentElement, SharedString,
    Stateful, StatefulInteractiveElement, Styled, Window, div, px,
};

use crate::theme::{self, GAP_SM, LIST_ROW_H, RADIUS_ROW, ROW_ICON, Theme};

/// Everything a row draws.
pub struct ListRow {
    /// Short badge text — the item kind's initial or a glyph.
    pub glyph: SharedString,
    pub title: SharedString,
    pub subtitle: SharedString,
    pub favorite: bool,
    pub selected: bool,
    /// Shows the transient "Copied" affordance instead of the favorite star.
    pub copied: Option<SharedString>,
}

pub fn list_row(
    id: impl Into<ElementId>,
    theme: &Theme,
    row: ListRow,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let (hover_bg, active_bg) = (theme.bg_sunken, theme.divider);

    let glyph = div()
        .size(px(ROW_ICON))
        .flex_none()
        .rounded(px(RADIUS_ROW))
        .bg(theme.bg_well)
        .flex()
        .items_center()
        .justify_center()
        .text_size(theme::text_row_sub())
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(theme.fg_muted)
        .child(row.glyph);

    // `min_w(0)` plus `truncate` is the pair that makes a long title shorten
    // instead of shoving the trailing affordance off the row.
    let text = div()
        .flex_1()
        .min_w(px(0.))
        .flex()
        .flex_col()
        .child(
            div()
                .truncate()
                .text_size(theme::text_row_title())
                .font_weight(FontWeight::MEDIUM)
                .text_color(theme.fg_base)
                .child(row.title),
        )
        .child(
            div()
                .truncate()
                .text_size(theme::text_row_sub())
                .text_color(theme.fg_muted)
                .child(row.subtitle),
        );

    let trailing = match (row.copied, row.favorite) {
        (Some(copied), _) => div()
            .flex_none()
            .text_size(theme::text_label())
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(theme.success)
            .child(copied),
        (None, true) => div()
            .flex_none()
            .text_size(theme::text_row_title())
            .text_color(theme.accent)
            .child("★"),
        (None, false) => div().flex_none(),
    };

    div()
        .id(id)
        .h(px(LIST_ROW_H))
        .w_full()
        .px(px(GAP_SM))
        .rounded(px(RADIUS_ROW))
        .flex()
        .items_center()
        .gap(px(GAP_SM))
        .cursor_pointer()
        .when_selected(row.selected, theme)
        .hover(move |s| s.bg(hover_bg))
        .active(move |s| s.bg(active_bg))
        .child(glyph)
        .child(text)
        .child(trailing)
        .on_click(on_click)
}

/// Selection wash, as an extension trait so the builder chain above stays flat.
trait Selected: Sized {
    fn when_selected(self, selected: bool, theme: &Theme) -> Self;
}

impl Selected for Stateful<Div> {
    fn when_selected(self, selected: bool, theme: &Theme) -> Self {
        if selected {
            self.bg(theme.accent_soft)
        } else {
            self
        }
    }
}
