//! Reusable visuals.
//!
//! Everything here takes `&Theme` plus **already-resolved strings** and returns
//! an element. No i18n, no page state, no window management, no core types
//! beyond plain values.
//!
//! That signature is the contract: a component cannot look up a translation,
//! cannot read the machine's model, and cannot decide what happens on click —
//! it takes a handler. The payoff is `gallery.rs`, which renders every visual
//! state of the app without running a single state machine, because there is
//! nothing in these functions that needs one.

mod badge;
mod button;
mod card;
mod list_row;
mod meter;
mod search_field;
mod status_note;

pub use badge::kind_badge;
pub use button::{ButtonVariant, button, button_opts};
pub use card::feature_card;
pub use list_row::{ListRow, list_row};
pub use meter::strength_meter;
pub use search_field::search_field;
pub use status_note::{StatusTone, status_note};
