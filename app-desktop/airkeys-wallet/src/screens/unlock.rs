//! The unlock screen: brand and value on the left, the one action on the right.

use gpui::{Div, FontWeight, ParentElement, SharedString, Stateful, Styled, div, px};

use airkeys_core::app::StatusKey;
use airkeys_core::app::unlock::UnlockView;

use crate::theme::{self, DETAIL_W, GAP_MD, GAP_SM, PAD_X};
use crate::ui::{ButtonVariant, button, feature_card, status_note};

use super::{Note, ScreenCtx, Sink};

/// What the user did on this screen. Not core events — the host translates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Intent {
    Unlock,
    CreateVault,
    DismissNote,
}

pub fn render(
    view: &UnlockView,
    note: Option<&Note>,
    ctx: &ScreenCtx<'_>,
    sink: Sink<Intent>,
) -> Div {
    div()
        .size_full()
        .flex()
        .text_color(ctx.theme.fg_base)
        .child(left_column(ctx))
        .child(action_panel(view, note, ctx, sink))
}

// ---------------------------------------------------------------------------
// Left column
// ---------------------------------------------------------------------------

fn left_column(ctx: &ScreenCtx<'_>) -> Div {
    let theme = ctx.theme;

    let brand = div()
        .flex()
        .items_center()
        .gap(px(GAP_MD))
        .child(mark(ctx))
        .child(
            div()
                .text_size(theme::text_display())
                .font_weight(FontWeight::BOLD)
                .text_color(theme.fg_base)
                // A proper name, rendered verbatim: never translated, never in
                // the corpus.
                .child("Airkeys"),
        );

    let mut cards = div().flex().gap(px(GAP_MD));
    for (index, (title, body)) in ctx.strings.features.iter().enumerate() {
        cards = cards.child(feature_card(
            theme,
            SharedString::from(format!("{:02}", index + 1)),
            title.clone(),
            body.clone(),
        ));
    }

    div()
        .flex_1()
        // Without this the column's automatic minimum is its content width, and
        // a window below the design size pushes the fixed-width action panel
        // clean out of the window instead of compressing the cards.
        .min_w(px(0.))
        .h_full()
        .pt(px(96.))
        .pl(px(72.))
        .pr(px(PAD_X))
        .flex()
        .flex_col()
        .child(brand)
        .child(
            div()
                .mt(px(40.))
                .text_size(theme::text_title())
                .text_color(theme.fg_muted)
                .child(ctx.strings.tagline.clone()),
        )
        .child(div().mt(px(40.)).child(cards))
}

/// The brand mark, drawn rather than loaded: a key's bow and bit in the accent.
/// Three divs beat an asset that has to be rasterized, themed and shipped at
/// four densities.
fn mark(ctx: &ScreenCtx<'_>) -> Div {
    let theme = ctx.theme;
    div()
        .size(px(52.))
        .flex_none()
        .rounded(px(16.))
        .bg(theme.accent)
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .size(px(18.))
                .rounded_full()
                .border_2()
                .border_color(theme.fg_inverse),
        )
}

// ---------------------------------------------------------------------------
// Right column
// ---------------------------------------------------------------------------

fn action_panel(
    view: &UnlockView,
    note: Option<&Note>,
    ctx: &ScreenCtx<'_>,
    sink: Sink<Intent>,
) -> Div {
    let theme = ctx.theme;

    let mut panel = div()
        .w(px(DETAIL_W))
        .h_full()
        .flex_none()
        .bg(theme.bg_raised)
        .border_l_1()
        .border_color(theme.panel_edge)
        .px(px(48.))
        .flex()
        .flex_col()
        .justify_center()
        .gap(px(GAP_MD));

    if let Some(note) = note {
        let sink = sink.clone();
        panel = panel
            .child(status_note(
                theme,
                note.tone,
                note.title.clone(),
                note.body.clone(),
            ))
            .child(button(
                "dismiss-note",
                ButtonVariant::Quiet,
                ctx.strings.close.clone(),
                theme,
                move |_, window, cx| sink(Intent::DismissNote, window, cx),
            ));
    }

    // Vault label and item count, once known — the reassurance that this is the
    // right machine before the user commits a fingerprint to it.
    if let Some(label) = &view.vault_label {
        panel = panel.child(
            div()
                .flex()
                .flex_col()
                .gap(px(4.))
                .child(
                    div()
                        .text_size(theme::text_label())
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(theme.fg_subtle)
                        .child(ctx.strings.unlock_title.clone()),
                )
                .child(
                    div()
                        .text_size(theme::text_title())
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.fg_base)
                        .child(SharedString::from(label.clone())),
                )
                .child(
                    div()
                        .text_size(theme::text_row_sub())
                        .text_color(theme.fg_muted)
                        .child(
                            ctx.loc
                                .t_count("vault.itemCount", f64::from(view.item_count)),
                        ),
                ),
        );
    }

    // `vault_missing` is only ever true after a load that found nothing — the
    // screen must not flash "no vault here" before it has looked.
    if view.vault_missing {
        panel = panel.child(status_note(
            theme,
            crate::ui::StatusTone::Info,
            ctx.strings.missing_title.clone(),
            Some(ctx.strings.missing_body.clone()),
        ));
    }

    if let Some(status) = view.status {
        panel = panel.child(status_line(view, status, ctx));
    }

    let unlock_sink = sink.clone();
    panel = panel.child(button_for_state(view, ctx, unlock_sink));

    if view.vault_missing {
        let create_sink = sink;
        panel = panel.child(button(
            "create-vault",
            ButtonVariant::Secondary,
            ctx.strings.create_btn.clone(),
            theme,
            move |_, window, cx| create_sink(Intent::CreateVault, window, cx),
        ));
    }

    panel
}

fn button_for_state(view: &UnlockView, ctx: &ScreenCtx<'_>, sink: Sink<Intent>) -> Stateful<Div> {
    crate::ui::button_opts(
        "unlock",
        ButtonVariant::Primary,
        ctx.strings.unlock_btn.clone(),
        // Disabled while a ceremony is in flight: the machine ignores a second
        // press anyway, but a live-looking button that does nothing is how a
        // user concludes the app is broken and presses it four more times.
        !view.busy,
        ctx.theme,
        move |_, window, cx| sink(Intent::Unlock, window, cx),
    )
}

fn status_line(view: &UnlockView, status: StatusKey, ctx: &ScreenCtx<'_>) -> Div {
    let theme = ctx.theme;
    let text = match status {
        StatusKey::CheckingDevice => ctx.strings.status_checking.clone(),
        StatusKey::AwaitingBiometric => ctx.strings.status_awaiting.clone(),
        StatusKey::OpeningVault => ctx.strings.status_opening.clone(),
    };

    let mut row = div().w_full().flex().flex_col().gap(px(GAP_SM)).child(
        div()
            .text_size(theme::text_body())
            .text_color(theme.fg_muted)
            .child(text),
    );

    // The elapsed counter appears only once waiting is long enough to notice.
    // Showing "waited 0s" the instant the sheet opens turns a normal pause into
    // something the user is invited to worry about.
    if view.waited_secs >= 3 {
        row = row.child(
            div()
                .text_size(theme::text_label())
                .text_color(theme.fg_subtle)
                .child(ctx.loc.t_vars(
                    "unlock.waitedSeconds",
                    &[("seconds", &view.waited_secs.to_string())],
                )),
        );
    }
    row
}
