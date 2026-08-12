//! The search field.
//!
//! A drawn field, not a text input: gpui has no built-in text editor, and Zed's
//! lives in a crate this app does not depend on. Typing is handled by the
//! screen's key handler, which owns the string; this component renders it, plus
//! a caret when focused and a placeholder when empty.
//!
//! Writing it this way is not a compromise — an input that owns its own buffer
//! would put a second copy of the query outside the state machine, and the two
//! would disagree the first time the machine cleared it.

use gpui::{Div, FontWeight, ParentElement, SharedString, Styled, div, px};

use crate::theme::{self, GAP_SM, INPUT_H, RADIUS_FIELD, Theme};

pub fn search_field(theme: &Theme, value: &str, placeholder: SharedString, focused: bool) -> Div {
    let showing_placeholder = value.is_empty();

    let text = div()
        .flex_1()
        .min_w(px(0.))
        .truncate()
        .text_size(theme::text_body())
        .text_color(if showing_placeholder {
            theme.fg_subtle
        } else {
            theme.fg_base
        })
        .child(if showing_placeholder {
            placeholder
        } else {
            SharedString::from(value.to_owned())
        });

    let mut row = div()
        .w_full()
        .h(px(INPUT_H))
        .px(px(GAP_SM))
        .rounded(px(RADIUS_FIELD))
        .bg(theme.bg_well)
        .border_1()
        .border_color(if focused {
            theme.accent
        } else {
            theme.border_card
        })
        .flex()
        .items_center()
        .gap(px(GAP_SM))
        .child(
            div()
                .flex_none()
                .text_size(theme::text_row_sub())
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.fg_subtle)
                .child("⌕"),
        )
        .child(text);

    if focused {
        row = row.child(div().flex_none().w(px(1.5)).h(px(16.)).bg(theme.accent));
    }
    row
}
