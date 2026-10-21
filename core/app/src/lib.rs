//! The Airkeys phone app, shared by the iOS and Android shells.
//!
//! - [`app`] — the Crux state machine: every screen, every rule between screens, every string
//! - [`runtime`] — runs the core and its Lumen Code work, behind the uniffi interface
//! - [`view`], [`ops`] — what crosses to the shells: the view model, requests, actions
//!
//! The wallet itself is `airkeys-wallet`; the wire protocol is `docs/PROTOCOL.md`.

pub mod app;
pub mod ops;
pub mod runtime;
pub mod view;

pub use runtime::AirkeysCore;

uniffi::setup_scaffolding!();
