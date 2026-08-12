//! Feature card — numeral, title, body on the raised surface.
//!
//! Grid-agnostic: the screen owns columns and gaps, the card owns its interior.
//! A card that knew its own width could not be reused in a different layout,
//! which is the usual reason design systems grow a second card component.

use gpui::{Div, FontWeight, ParentElement, SharedString, Styled, div, px};

use crate::theme::{self, GAP_SM, RADIUS_CARD, Theme};

pub fn feature_card(
    theme: &Theme,
    numeral: SharedString,
    title: SharedString,
    body: SharedString,
) -> Div {
    div()
        // Equal share of the row. `min_w(0)` — not a fixed width — so a window
        // below the design minimum squeezes the cards instead of pushing a
        // fixed-width neighbour out of the window. Flex children must opt in to
        // shrinking; the default minimum is the content width.
        .flex_1()
        .min_w(px(0.))
        .p(px(theme::GAP_MD))
        .rounded(px(RADIUS_CARD))
        .bg(theme.bg_raised)
        .border_1()
        .border_color(theme.border_card)
        .flex()
        .flex_col()
        .child(
            div()
                .text_size(theme::text_label())
                .font_weight(FontWeight::MEDIUM)
                .text_color(theme.fg_subtle)
                .child(numeral),
        )
        .child(
            div()
                .mt(px(GAP_SM))
                .text_size(theme::text_section())
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.fg_base)
                .child(title),
        )
        .child(
            div()
                .mt(px(6.))
                .text_size(theme::text_body())
                .line_height(theme::line_height_body())
                .text_color(theme.fg_muted)
                .child(body),
        )
}
