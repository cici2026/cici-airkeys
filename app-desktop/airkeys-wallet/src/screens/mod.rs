//! Screens: **pure functions from a view model to an element.**
//!
//! ```text
//! (ViewModel, Theme, Strings) ─► Div          + a sink for what the user did
//! ```
//!
//! No screen owns state, reads a clock, or calls into a machine. Everything it
//! draws comes from the view model the core produced; everything the user does
//! goes out through the [`Sink`] as an *intent*, and `root.rs` decides which
//! core event that becomes.
//!
//! Two things fall out of that shape, and both are the reason for it:
//!
//! - **`gallery.rs` can render every state** by handing these functions a
//!   hand-written view model. No machine, no fixtures wired to a store, no
//!   "how do I get the app into the locked-out state" — the state *is* the
//!   argument.
//! - **A screen cannot invent behaviour.** It has no way to save anything, so a
//!   rule cannot quietly appear in the UI layer where the other three clients
//!   will never see it.

pub mod unlock;
pub mod vault;

use std::rc::Rc;

use gpui::{App, Window};

use crate::loc::Loc;
use crate::strings::Strings;
use crate::theme::Theme;

/// How a screen reports what the user did. The host decides what it means.
pub type Sink<E> = Rc<dyn Fn(E, &mut Window, &mut App)>;

/// Everything a screen is allowed to read besides its own view model.
pub struct ScreenCtx<'a> {
    pub theme: &'a Theme,
    pub strings: &'a Strings,
    /// For the handful of strings that need interpolation or a plural at the
    /// point of use — a count, an elapsed second, a search query.
    pub loc: &'a Loc,
}

/// A banner the host wants shown.
///
/// Prompts are `ShellOperation`s, not view-model fields: the core *asks* for one
/// and waits for the answer. So the host owns the pending prompt and hands it to
/// the screen, rather than the screen finding it in the view model.
pub struct Note {
    pub tone: crate::ui::StatusTone,
    pub title: gpui::SharedString,
    pub body: Option<gpui::SharedString>,
}
