//! The small pill that names an item's kind.

use gpui::{Div, FontWeight, ParentElement, SharedString, Styled, div, px};

use crate::theme::{self, Theme};

pub fn kind_badge(theme: &Theme, label: SharedString) -> Div {
    div()
        .flex_none()
        .px(px(8.))
        .py(px(2.))
        .rounded(px(999.))
        .bg(theme.bg_well)
        .text_size(theme::text_label())
        .font_weight(FontWeight::MEDIUM)
        .text_color(theme.fg_muted)
        .child(label)
}
