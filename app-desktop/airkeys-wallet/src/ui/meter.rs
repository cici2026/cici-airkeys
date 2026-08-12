//! The secret-strength meter.
//!
//! The *verdict* comes from `airkeys_core::strength` — shared with iOS, Android
//! and web, so a password rated Strong here is rated Strong everywhere. This
//! module owns only how that verdict looks.
//!
//! Colour is never the sole carrier: the band is also stated in words, because
//! roughly one man in twelve cannot reliably tell the red bar from the green
//! one.

use gpui::{Div, FontWeight, ParentElement, SharedString, Styled, div, px};

use airkeys_core::strength::{Strength, StrengthReport};

use crate::theme::{self, GAP_SM, METER_H, Theme};

/// How much of the bar each band fills. Quarters, so the bar and the label
/// always agree — a continuous fill driven by raw bits would put a Fair secret
/// at 59 % and a Strong one at 61 %, which reads as "the same" while the words
/// say otherwise.
fn fill_fraction(strength: Strength) -> f32 {
    match strength {
        Strength::Weak => 0.25,
        Strength::Fair => 0.5,
        Strength::Strong => 0.75,
        Strength::Excellent => 1.0,
    }
}

/// `label` is the band's name and `hints` the advice lines, both already
/// translated — this component never resolves a key.
pub fn strength_meter(
    theme: &Theme,
    report: &StrengthReport,
    label: SharedString,
    hints: Vec<SharedString>,
) -> Div {
    let color = theme.strength_color(report.strength);

    let bar = div()
        .w_full()
        .h(px(METER_H))
        .rounded(px(METER_H / 2.))
        .bg(theme.bg_well)
        .child(
            div()
                .h_full()
                .w(gpui::relative(fill_fraction(report.strength)))
                .rounded(px(METER_H / 2.))
                .bg(color),
        );

    let mut root = div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(GAP_SM))
        .child(bar)
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_size(theme::text_row_sub())
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(color)
                        .child(label),
                )
                .child(
                    div()
                        .text_size(theme::text_label())
                        .text_color(theme.fg_subtle)
                        // Bits are for the curious, not the decision — small,
                        // muted, and never the only thing said.
                        .child(format!("{:.0} bits", report.bits)),
                ),
        );

    for hint in hints {
        root = root.child(
            div()
                .text_size(theme::text_label())
                .text_color(theme.fg_muted)
                .child(hint),
        );
    }
    root
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bar and the words must not be able to disagree.
    #[test]
    fn fill_increases_with_the_band() {
        let mut previous = 0.0;
        for strength in [
            Strength::Weak,
            Strength::Fair,
            Strength::Strong,
            Strength::Excellent,
        ] {
            let fill = fill_fraction(strength);
            assert!(fill > previous, "{strength:?} did not increase the fill");
            previous = fill;
        }
        assert_eq!(fill_fraction(Strength::Excellent), 1.0);
    }
}
