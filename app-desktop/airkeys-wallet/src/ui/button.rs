//! The CTA variants, on theme tokens only.

use gpui::{
    App, ClickEvent, Div, ElementId, FontWeight, InteractiveElement, ParentElement, SharedString,
    Stateful, StatefulInteractiveElement, Styled, Window, div, px,
};

use crate::theme::{self, BTN_H_PRIMARY, BTN_H_SECONDARY, OPACITY_DISABLED, Theme};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ButtonVariant {
    /// Accent-filled, inverse label. Reserved for the screen's one primary
    /// action — two primaries on a screen means neither is.
    Primary,
    /// Outlined on the raised surface.
    Secondary,
    /// Filled with the well surface, base label. The quiet row action.
    Quiet,
}

/// Returns `Stateful<Div>` rather than `impl IntoElement`: under edition 2024 an
/// RPIT would capture the `&Theme` borrow, which forbids a second call in the
/// same render.
pub fn button(
    id: impl Into<ElementId>,
    variant: ButtonVariant,
    label: SharedString,
    theme: &Theme,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    button_opts(id, variant, label, true, theme, on_click)
}

/// `button` plus the disabled treatment: the same fill at reduced emphasis —
/// never a grey swap, so the control still reads as the thing it will become —
/// with the pointer affordance and the click handler withheld.
pub fn button_opts(
    id: impl Into<ElementId>,
    variant: ButtonVariant,
    label: SharedString,
    enabled: bool,
    theme: &Theme,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let height = match variant {
        ButtonVariant::Primary => BTN_H_PRIMARY,
        ButtonVariant::Secondary | ButtonVariant::Quiet => BTN_H_SECONDARY,
    };

    // The height is a MINIMUM. A long-locale label wraps inside a
    // width-constrained centred block and grows the row, instead of escaping
    // the capsule — German and Japanese both overflow the English width here.
    let label_block = div().w_full().min_w(px(0.)).text_center().child(label);

    let base = div()
        .id(id)
        .min_h(px(height))
        .w_full()
        .flex_none()
        .rounded(px(height / 2.))
        .flex()
        .items_center()
        .justify_center()
        .px(px(theme::BTN_PAD_X))
        .py(px(theme::BTN_PAD_Y))
        .text_size(theme::text_button())
        .font_weight(FontWeight::SEMIBOLD);

    if !enabled {
        let styled = match variant {
            ButtonVariant::Primary => base.bg(theme.accent).text_color(theme.fg_inverse),
            ButtonVariant::Secondary => base
                .bg(theme.bg_raised)
                .text_color(theme.fg_base)
                .border_1()
                .border_color(theme.outline_strong),
            ButtonVariant::Quiet => base.bg(theme.bg_well).text_color(theme.fg_base),
        };
        return styled.opacity(OPACITY_DISABLED).child(label_block);
    }

    let styled = match variant {
        ButtonVariant::Primary => {
            let (hover, active) = (theme.accent_hover, theme.accent_active);
            base.bg(theme.accent)
                .text_color(theme.fg_inverse)
                .hover(move |s| s.bg(hover))
                .active(move |s| s.bg(active))
        }
        ButtonVariant::Secondary => {
            let (hover, active) = (theme.bg_sunken, theme.divider);
            base.bg(theme.bg_raised)
                .text_color(theme.fg_base)
                .border_1()
                .border_color(theme.outline_strong)
                .hover(move |s| s.bg(hover))
                .active(move |s| s.bg(active))
        }
        ButtonVariant::Quiet => {
            let (hover, active) = (theme.divider, theme.bg_sunken);
            base.bg(theme.bg_well)
                .text_color(theme.fg_base)
                .hover(move |s| s.bg(hover))
                .active(move |s| s.bg(active))
        }
    };

    styled
        .cursor_pointer()
        .child(label_block)
        .on_click(on_click)
}
