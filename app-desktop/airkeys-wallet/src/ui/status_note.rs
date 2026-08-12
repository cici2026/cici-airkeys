//! The banner that carries a prompt or an error.
//!
//! Tone is a parameter rather than a colour, so a screen says *what kind of
//! thing happened* and the theme decides how that looks — which is what keeps
//! the error red identical in nine places instead of nearly identical.

use gpui::{Div, FontWeight, ParentElement, SharedString, Styled, div, px};

use crate::theme::{self, GAP_MD, GAP_SM, RADIUS_FIELD, Theme};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StatusTone {
    Info,
    Warning,
    Error,
}

pub fn status_note(
    theme: &Theme,
    tone: StatusTone,
    title: SharedString,
    body: Option<SharedString>,
) -> Div {
    let color = match tone {
        StatusTone::Info => theme.info,
        StatusTone::Warning => theme.warning,
        StatusTone::Error => theme.error,
    };

    let mut root = div()
        .w_full()
        .p(px(GAP_MD))
        .rounded(px(RADIUS_FIELD))
        .bg(theme.bg_well)
        // The tone is carried by a rule *and* by the words, never by colour
        // alone — a red block with no message is not an error report.
        .border_l_2()
        .border_color(color)
        .flex()
        .flex_col()
        .gap(px(GAP_SM))
        .child(
            div()
                .text_size(theme::text_row_title())
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(color)
                .child(title),
        );

    if let Some(body) = body {
        root = root.child(
            div()
                .text_size(theme::text_body())
                .line_height(theme::line_height_body())
                .text_color(theme.fg_muted)
                .child(body),
        );
    }
    root
}
