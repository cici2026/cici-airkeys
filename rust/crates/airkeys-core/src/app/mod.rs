//! Portable business state machines.
//!
//! # The boundary
//!
//! Everything in this module **decides**; nothing in it **does**. A machine
//! receives an [`Event`](unlock::Event), updates its `Model`, and returns
//! `Command`s that either re-render or declare a [`shell::ShellOperation`] —
//! "please authenticate this device", "please copy that field". The platform
//! shell performs the operation and answers with a [`shell::ShellResult`].
//!
//! There is no network, no storage, no clock and no randomness here. Wall-clock
//! instants arrive as *fields on the results the shell returns*, which is what
//! makes every rule in this module testable in milliseconds with no device, no
//! browser and no sleeping.
//!
//! ```text
//! Event ─► update(Model) ─► Model' + Command<Effect>
//!                                       │
//! ViewModel = view(Model')              └─► the shell performs I/O and answers
//! ```
//!
//! # The rule that matters most here
//!
//! **A decrypted secret never enters this module.** The core addresses secrets
//! by reference — [`shell::CopyRequest`] names an item and a field — and the
//! shell is what touches plaintext, for as long as the clipboard holds it and
//! no longer. That is why `Item` in [`crate::types`] has no secret field: if a
//! plaintext password can never be in the model, it can never be in a view
//! model, a serialized effect, a log line or a crash dump.
//!
//! # Layout
//!
//! - [`shell`] — the operation/result vocabulary every machine speaks
//! - [`unlock`] — machine A: support → authenticate → open → enter
//! - [`item_list`] — machine B: load → search → favorite → copy → auto-clear
//!
//! Compiled only with `--features crux`; the default build of this crate — the
//! one the iOS and Android bindings link — does not contain it.

use serde::{Deserialize, Serialize};

pub mod item_list;
pub mod shell;
pub mod unlock;

#[cfg(feature = "bindings")]
use ts_rs::TS;

/// Implemented by every per-domain effect enum so product-agnostic plumbing —
/// the wasm bridge, the test driver, the desktop host — can split shell
/// requests from renders without knowing the domain.
///
/// Three lines per machine; the bridge and the driver are written once, so
/// nothing about correlation, ids or abort handling can fork per machine.
pub trait SplitEffect {
    type Op: crux_core::capability::Operation;
    fn into_shell(self) -> Option<crux_core::Request<Self::Op>>;
}

/// How a device-authentication ceremony failed.
///
/// The **shell** reports the raw platform error; the classification is the
/// core's, so every machine branches on the same vocabulary instead of
/// pattern-matching error strings that differ per OS version.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "bindings", derive(TS))]
pub enum FailureKind {
    /// The user dismissed the OS sheet. Never an error state, never an alert.
    Cancelled,
    /// No biometric or passcode is enrolled on this device.
    NotSupported,
    /// Too many failed attempts; the OS is refusing for now.
    LockedOut,
    Other,
}

/// The transient status line under an in-flight ceremony.
///
/// Semantic, not prose — the shell owns the words, so translations stay out of
/// the wasm module and every platform can render the same state in its own
/// idiom.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "bindings", derive(TS))]
pub enum StatusKey {
    CheckingDevice,
    AwaitingBiometric,
    OpeningVault,
}

/// A question or notice for the user. One variant per real call site;
/// `confirmable` (on the operation) selects a two-button dialog whose answer is
/// a business decision rather than an acknowledgement.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[cfg_attr(feature = "bindings", derive(TS))]
pub enum PromptKind {
    /// No biometric/passcode enrolled — the user must fix this in system
    /// settings, so the copy points there.
    Unsupported,
    /// The OS is rate-limiting authentication.
    LockedOut,
    /// There is no vault on this device yet.
    VaultMissing,
    UnlockFailed {
        detail: String,
    },
    StorageFailed {
        detail: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The wire vocabulary is `snake_case` on every surface. A rename here is a
    /// breaking change for three shells at once, so it is pinned by a test
    /// rather than left to review.
    #[test]
    fn failure_kinds_serialize_snake_case() {
        let json = serde_json::to_string(&FailureKind::LockedOut).expect("serializes");
        assert_eq!(json, "\"locked_out\"");
    }

    #[test]
    fn prompts_are_tagged_unions() {
        let json = serde_json::to_string(&PromptKind::UnlockFailed {
            detail: "boom".into(),
        })
        .expect("serializes");
        assert_eq!(json, r#"{"type":"unlock_failed","detail":"boom"}"#);
    }
}
