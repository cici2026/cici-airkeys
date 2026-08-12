//! The vocabulary every machine uses to ask the outside world for something,
//! and the vocabulary the outside world answers in.
//!
//! Shared on purpose: several of these operations are used by more than one
//! machine, so each shell implements **one** executor and the generated
//! TypeScript has one union instead of several overlapping ones.
//!
//! Nothing here performs the work. `AuthenticateDevice` is a *sentence*, not a
//! ceremony; `CopyField` is a *request*, not a write to the clipboard. The
//! shell owns the keychain, the biometric prompt, the storage paths, the
//! clipboard and the clock — none of which appear in this file, which is why no
//! core test can depend on a device, a filesystem or the time of day.

use crux_core::capability::Operation;
use crux_core::macros::effect;
use crux_core::render::RenderOperation;
use serde::{Deserialize, Serialize};

use super::PromptKind;
use crate::types::{Item, VaultMeta};

#[cfg(feature = "bindings")]
use ts_rs::TS;

/// Why the device is being asked to authenticate. The shell picks the prompt
/// copy and the biometric policy from this; the core only says which moment it
/// is, so the two never disagree about what the user is being asked to approve.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "bindings", derive(TS))]
pub enum AuthPurpose {
    /// Opening a locked vault.
    Unlock,
    /// Revealing or copying a secret from an already-open vault. Separate from
    /// `Unlock` because the OS prompt says something different, and because a
    /// policy that re-authenticates for secrets but not for browsing needs the
    /// two moments to be distinguishable.
    RevealSecret,
}

/// Which field of an item the shell should put on the clipboard.
///
/// **This is the whole reason the core never holds plaintext.** The request
/// names an item and a field; the shell reads the decrypted value out of its
/// own keystore and copies it. A `String` here instead would put every password
/// the user copies into the model, the view model, the serialized effect log
/// and any crash dump taken afterwards.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(TS))]
pub struct CopyRequest {
    pub item_id: String,
    pub field: CopyField,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "bindings", derive(TS))]
pub enum CopyField {
    Account,
    Secret,
    Site,
}

/// Everything below this line must be performed by a platform shell.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[cfg_attr(feature = "bindings", derive(TS))]
pub enum ShellOperation {
    /// Can this device authenticate a human at all?
    CheckUnlockSupport,
    /// Face ID / Touch ID / Windows Hello / polkit — whatever this platform has.
    AuthenticateDevice {
        purpose: AuthPurpose,
    },
    /// Read what is known about the local vault without opening it.
    LoadVaultMeta,
    /// Decrypt the vault into the shell's own memory. The key never crosses
    /// this boundary; only the fact that it worked does.
    OpenVault {
        vault_id: String,
    },
    LoadItems {
        vault_id: String,
    },
    SaveItem {
        item: Item,
    },
    /// Copy a field to the clipboard. See [`CopyRequest`].
    CopyField {
        request: CopyRequest,
    },
    /// Wipe the clipboard, if it still holds what we put there.
    ClearClipboard,
    /// Wait, without the core owning a clock. Used for the elapsed-seconds
    /// ticker and for the clipboard auto-clear.
    ///
    /// `u32`, not `u64`: the wire is JSON and `JSON.parse` yields a `number`, so
    /// a 64-bit type would generate a TypeScript `bigint` the shell never
    /// actually sends.
    Wait {
        ms: u32,
    },
    /// Ask (or tell) the user something. `confirmable` selects a two-button
    /// dialog whose answer is a business decision.
    Prompt {
        kind: PromptKind,
        confirmable: bool,
    },
    /// Hand the open vault to the app and leave the unlock screen.
    CompleteUnlock {
        vault: VaultMeta,
    },
}

impl Operation for ShellOperation {
    type Output = ShellResult;
}

/// What the shell observed.
///
/// Failures arrive as *variants*, never as exceptions: the executor converts
/// every rejected promise, every thrown platform error, into the failure result
/// that belongs to the operation. That is what lets the core own the
/// classification instead of matching on error strings.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[cfg_attr(feature = "bindings", derive(TS))]
pub enum ShellResult {
    UnlockSupport {
        supported: bool,
    },
    /// `now_iso` is the wall clock the shell observed while doing the work —
    /// reported alongside the observation so the core stays a pure function of
    /// its inputs. No clock effect, no clock in tests.
    DeviceAuthenticated {
        now_iso: String,
    },
    AuthFailed {
        kind: super::FailureKind,
        /// The platform's own words, forwarded verbatim for the "something went
        /// wrong" case. Absent for classified failures, whose copy comes from
        /// the classification.
        message: Option<String>,
    },
    /// `None` means there is no vault on this device — recoverable (create or
    /// restore one), and deliberately distinct from `StorageFailed`, which is
    /// not.
    VaultMetaLoaded {
        vault: Option<VaultMeta>,
    },
    VaultOpened {
        now_iso: String,
    },
    ItemsLoaded {
        items: Vec<Item>,
    },
    ItemSaved,
    StorageFailed {
        message: String,
    },
    /// The clipboard now holds the requested field.
    Copied,
    ClipboardCleared,
    Waited,
    PromptAnswered {
        accepted: bool,
    },
    UnlockCompleted,
}

/// Render, or ask the shell for something. Shared by every machine.
#[effect]
pub enum Effect {
    Render(RenderOperation),
    Shell(ShellOperation),
}

impl super::SplitEffect for Effect {
    type Op = ShellOperation;
    fn into_shell(self) -> Option<crux_core::Request<ShellOperation>> {
        match self {
            Effect::Render(_) => None,
            Effect::Shell(request) => Some(request),
        }
    }
}
