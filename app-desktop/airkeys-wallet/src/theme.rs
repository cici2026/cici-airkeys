//! Design tokens — **the only module allowed to name a colour or a layout
//! magic number.**
//!
//! That rule is the whole point. Once a component is allowed to write
//! `rgb(0x5b5bd6)` or `px(17.)`, the design system exists in the documentation
//! and nowhere else: a redesign becomes a grep, dark mode becomes a second grep
//! that misses three places, and the accessibility floor becomes unverifiable
//! because nothing knows what the pairs are. Components read semantic tokens;
//! nobody reads hex.

use gpui::{Hsla, Pixels, Window, px, rgb};

/// Which palette is active. Follows the OS appearance unless `AIRKEYS_THEME`
/// pins it — the hook the screenshot matrix drives.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThemeMode {
    Light,
    Dark,
}

impl ThemeMode {
    /// `AIRKEYS_THEME` override, else the window's current system appearance.
    pub fn detect(window: &Window) -> Self {
        match std::env::var("AIRKEYS_THEME").as_deref() {
            Ok("light") => Self::Light,
            Ok("dark") => Self::Dark,
            _ => match window.appearance() {
                gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark => Self::Dark,
                gpui::WindowAppearance::Light | gpui::WindowAppearance::VibrantLight => Self::Light,
            },
        }
    }

    /// Whether `AIRKEYS_THEME` pins the mode — appearance changes are then
    /// ignored, so a screenshot run cannot be spoiled by the OS flipping to
    /// dark at sunset.
    pub fn is_pinned() -> bool {
        matches!(
            std::env::var("AIRKEYS_THEME").as_deref(),
            Ok("light") | Ok("dark")
        )
    }
}

/// Semantic colour tokens.
pub struct Theme {
    // surfaces, from furthest back to nearest front
    pub bg_base: Hsla,
    pub bg_raised: Hsla,
    pub bg_sunken: Hsla,
    /// The "well" surface: search field, secret field, code block, selected row.
    /// Its own token rather than `bg_sunken` because on the dark ladder a well
    /// must step *darker* than the raised panel, and `bg_sunken` steps lighter.
    pub bg_well: Hsla,

    // foreground ladder
    pub fg_base: Hsla,
    pub fg_muted: Hsla,
    pub fg_subtle: Hsla,
    pub fg_inverse: Hsla,

    // brand accent + interaction states
    pub accent: Hsla,
    pub accent_hover: Hsla,
    pub accent_active: Hsla,
    /// Tinted accent wash behind a selected row.
    pub accent_soft: Hsla,

    // structure
    pub border_card: Hsla,
    pub outline_strong: Hsla,
    pub divider: Hsla,
    /// The 1 px edge between columns. The dark design draws none — the surface
    /// step is the separation — so dark sets it to `bg_raised`: painted but
    /// invisible, which keeps one code path instead of two.
    pub panel_edge: Hsla,

    // status
    pub success: Hsla,
    pub warning: Hsla,
    pub error: Hsla,
    pub info: Hsla,

    // secret-strength bands. One per `airkeys_core::strength::Strength`
    // variant, and `strength_color` below is exhaustive over that enum — so
    // adding a band to the core fails this crate's build rather than silently
    // rendering the new band in the old colour.
    pub strength_weak: Hsla,
    pub strength_fair: Hsla,
    pub strength_strong: Hsla,
    pub strength_excellent: Hsla,
}

fn c(hex: u32) -> Hsla {
    rgb(hex).into()
}

impl Theme {
    pub fn of(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Light => Self::light(),
            ThemeMode::Dark => Self::dark(),
        }
    }

    pub fn light() -> Self {
        Self {
            bg_base: c(0xf7f7fa),
            bg_raised: c(0xffffff),
            bg_sunken: c(0xeeeef4),
            bg_well: c(0xf1f1f7),
            fg_base: c(0x16161d),
            fg_muted: c(0x5f5f70),
            fg_subtle: c(0x7c7c8f),
            fg_inverse: c(0xffffff),
            accent: c(0x4f46c9),
            accent_hover: c(0x443bb4),
            accent_active: c(0x39319c),
            accent_soft: c(0xeceafb),
            border_card: c(0xe4e4ee),
            outline_strong: c(0x4a4a5c),
            divider: c(0xe4e4ee),
            panel_edge: c(0xe4e4ee),
            success: c(0x1f7a4d),
            warning: c(0x8a5a06),
            error: c(0xb3261e),
            info: c(0x2f5bd7),
            strength_weak: c(0xb3261e),
            strength_fair: c(0x8a5a06),
            strength_strong: c(0x1f7a4d),
            strength_excellent: c(0x0f6b7a),
        }
    }

    pub fn dark() -> Self {
        Self {
            bg_base: c(0x111114),
            bg_raised: c(0x1a1a1f),
            bg_sunken: c(0x232329),
            bg_well: c(0x0e0e11),
            fg_base: c(0xe9e9ef),
            fg_muted: c(0x9d9dae),
            fg_subtle: c(0x86869a),
            fg_inverse: c(0xffffff),
            accent: c(0x7b74f0),
            accent_hover: c(0x8c86f5),
            accent_active: c(0x6a62e0),
            accent_soft: c(0x1e1c34),
            border_card: c(0x1a1a1f), // no visible card border in the dark design
            outline_strong: c(0x4a4a5c),
            divider: c(0x2a2a32),
            panel_edge: c(0x1a1a1f),
            success: c(0x3fa877),
            warning: c(0xd0a24a),
            error: c(0xf07a72),
            info: c(0x7d9bf5),
            strength_weak: c(0xf07a72),
            strength_fair: c(0xd0a24a),
            strength_strong: c(0x3fa877),
            strength_excellent: c(0x4fc3d4),
        }
    }

    /// The band colour for a strength verdict.
    ///
    /// Exhaustive over the core's enum on purpose: a new band in
    /// `airkeys_core::strength::Strength` breaks this match, which is a compile
    /// error here rather than a band that renders in whatever the fallback arm
    /// happened to be.
    pub fn strength_color(&self, strength: airkeys_core::strength::Strength) -> Hsla {
        use airkeys_core::strength::Strength;
        match strength {
            Strength::Weak => self.strength_weak,
            Strength::Fair => self.strength_fair,
            Strength::Strong => self.strength_strong,
            Strength::Excellent => self.strength_excellent,
        }
    }
}

// ---------------------------------------------------------------------------
// Layout. 4 px grid.
// ---------------------------------------------------------------------------

/// Design window size; also the minimum — the three-column layout does not
/// reflow below it, so shrinking further would push a fixed column out of the
/// window rather than rearrange anything.
pub const WINDOW_W: f32 = 1280.;
pub const WINDOW_H: f32 = 800.;

/// Fixed sidebar width (column 1).
pub const SIDEBAR_W: f32 = 240.;
/// Fixed detail-panel width (column 3).
pub const DETAIL_W: f32 = 400.;
/// Inner padding of the sidebar.
pub const SIDEBAR_PAD: f32 = 16.;
/// Top padding of the sidebar header — clears the macOS traffic lights.
pub const SIDEBAR_TOP: f32 = 36.;

/// Content column padding.
pub const PAD_X: f32 = 24.;
pub const PAD_TOP: f32 = 28.;

/// Rhythm: within a group, between rows, between blocks.
pub const GAP_SM: f32 = 8.;
pub const GAP_MD: f32 = 16.;
pub const GAP_LG: f32 = 24.;

/// Row and control geometry.
pub const NAV_ROW_H: f32 = 40.;
pub const LIST_ROW_H: f32 = 56.;
pub const ROW_ICON: f32 = 36.;
pub const INPUT_H: f32 = 40.;
pub const BTN_H_PRIMARY: f32 = 44.;
pub const BTN_H_SECONDARY: f32 = 40.;
pub const BTN_PAD_X: f32 = 20.;
pub const BTN_PAD_Y: f32 = 8.;

/// Corner radii.
pub const RADIUS_CARD: f32 = 14.;
pub const RADIUS_FIELD: f32 = 10.;
pub const RADIUS_ROW: f32 = 10.;

/// Hairline rules.
pub const HAIRLINE: f32 = 1.;
/// Disabled control emphasis — a dimmed fill, never a grey swap, so the control
/// still reads as the thing it will become.
pub const OPACITY_DISABLED: f32 = 0.45;
/// The strength meter's bar.
pub const METER_H: f32 = 6.;
/// Dev-only state gallery: fixture list column width.
pub const GALLERY_SIDEBAR_W: f32 = 280.;

// ---------------------------------------------------------------------------
// Type scale.
// ---------------------------------------------------------------------------

pub fn text_display() -> Pixels {
    px(34.)
}
pub fn text_title() -> Pixels {
    px(20.)
}
pub fn text_section() -> Pixels {
    px(16.)
}
pub fn text_row_title() -> Pixels {
    px(14.)
}
pub fn text_row_sub() -> Pixels {
    px(12.)
}
pub fn text_body() -> Pixels {
    px(13.)
}
pub fn text_button() -> Pixels {
    px(14.)
}
pub fn text_label() -> Pixels {
    px(11.)
}
pub fn text_mono() -> Pixels {
    px(13.)
}
/// Relaxed body line height (~1.55 at 13 px).
pub fn line_height_body() -> Pixels {
    px(20.)
}

/// Monospace family for secrets and fingerprints. Menlo ships on macOS; DejaVu
/// is the broadly-present face on the Linux images this app targets.
pub fn font_mono() -> &'static str {
    if cfg!(target_os = "macos") {
        "Menlo"
    } else {
        "DejaVu Sans Mono"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use airkeys_core::strength::Strength;

    /// WCAG 2.x relative luminance of a token.
    fn luminance(color: Hsla) -> f32 {
        let rgba = color.to_rgb();
        let lin = |c: f32| {
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * lin(rgba.r) + 0.7152 * lin(rgba.g) + 0.0722 * lin(rgba.b)
    }

    fn contrast(a: Hsla, b: Hsla) -> f32 {
        let (la, lb) = (luminance(a), luminance(b));
        let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
        (hi + 0.05) / (lo + 0.05)
    }

    /// The accessibility floor, as a test rather than a promise. Every
    /// text-on-surface pair the app actually paints, in both themes.
    ///
    /// This is what makes a palette edit safe: change a hex, and if it drops a
    /// pair below its floor the build says so — instead of a user with low
    /// vision discovering it.
    #[test]
    fn contrast_floor_holds_in_both_themes() {
        for (name, t) in [("light", Theme::light()), ("dark", Theme::dark())] {
            let pairs = [
                // body text, 4.5:1
                ("fg_base/bg_base", t.fg_base, t.bg_base, 4.5),
                ("fg_base/bg_raised", t.fg_base, t.bg_raised, 4.5),
                ("fg_base/bg_well", t.fg_base, t.bg_well, 4.5),
                ("fg_muted/bg_base", t.fg_muted, t.bg_base, 4.5),
                ("fg_muted/bg_raised", t.fg_muted, t.bg_raised, 4.5),
                // 11 px labels sit at the decorative-adjacent 3:1 floor
                ("fg_subtle/bg_raised", t.fg_subtle, t.bg_raised, 3.0),
                // the primary CTA label: semibold on accent, large-text floor
                ("fg_inverse/accent", t.fg_inverse, t.accent, 3.0),
                ("fg_inverse/accent_hover", t.fg_inverse, t.accent_hover, 3.0),
                (
                    "fg_inverse/accent_active",
                    t.fg_inverse,
                    t.accent_active,
                    3.0,
                ),
                // status text on the raised panel
                ("error/bg_raised", t.error, t.bg_raised, 3.0),
                ("success/bg_raised", t.success, t.bg_raised, 3.0),
                ("warning/bg_raised", t.warning, t.bg_raised, 3.0),
                ("info/bg_raised", t.info, t.bg_raised, 3.0),
            ];
            for (pair, fg, bg, floor) in pairs {
                let ratio = contrast(fg, bg);
                assert!(ratio >= floor, "{name} {pair}: {ratio:.2} < {floor}");
            }
        }
    }

    /// The strength meter is read at a glance and often by someone who will not
    /// stop to compare it to a legend — so each band must be legible on the
    /// surface it is painted on, in both themes.
    #[test]
    fn every_strength_band_is_legible_on_its_surface() {
        for (name, t) in [("light", Theme::light()), ("dark", Theme::dark())] {
            for strength in [
                Strength::Weak,
                Strength::Fair,
                Strength::Strong,
                Strength::Excellent,
            ] {
                let color = t.strength_color(strength);
                let ratio = contrast(color, t.bg_raised);
                assert!(ratio >= 3.0, "{name} {strength:?}: {ratio:.2} < 3.0");
            }
        }
    }

    /// The accent is the brand constant: the same hue family reads as "Airkeys"
    /// in both themes, even though the two are not the same value (dark needs a
    /// lighter accent to keep the inverse label above 3:1).
    #[test]
    fn the_two_accents_are_the_same_hue_family() {
        let (light, dark) = (Theme::light().accent, Theme::dark().accent);
        let delta = (light.h - dark.h).abs();
        assert!(
            delta < 0.03,
            "accents drifted apart in hue: {} vs {}",
            light.h,
            dark.h
        );
    }
}
