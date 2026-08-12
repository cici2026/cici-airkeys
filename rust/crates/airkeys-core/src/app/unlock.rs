//! Machine A — opening a locked vault with the device's own authenticator.
//!
//! ```text
//! Idle ─start─► load vault meta ──none──► "no vault here" (never a failure)
//!   │                             └─some─► Idle, showing the vault label
//!   │
//!   └─unlock─► support probe ─► authenticate ─► open ─► enter
//!                    │no             │cancel
//!                    ▼               ▼
//!              "set one up"        Idle, silently
//! ```
//!
//! Four rules, each of which cost someone a support ticket somewhere:
//!
//! 1. **Read the vault before raising the OS sheet.** Prompting for a
//!    fingerprint and *then* saying "there is no vault on this device" trains
//!    users to approve prompts that do nothing.
//! 2. **A cancel is not an error.** Dismissing the sheet returns to idle with
//!    no alert, no red text and no retry counter.
//! 3. **One ceremony at a time**, and a result belonging to a superseded
//!    attempt is dropped rather than applied to the current one.
//! 4. **The elapsed counter is not a clock.** The core owns no time; it asks
//!    the shell to `Wait { ms: 1000 }` and counts the answers. That is the only
//!    reason the "waited 7s" copy is testable without sleeping for seven
//!    seconds.

use crux_core::{command::AbortHandle, render::render, App, Command};
use serde::{Deserialize, Serialize};

use super::shell::{AuthPurpose, Effect, ShellOperation, ShellResult};
use super::{FailureKind, PromptKind, StatusKey};
use crate::types::VaultMeta;

#[cfg(feature = "bindings")]
use ts_rs::TS;

/// Gap between elapsed-counter ticks while the OS sheet is up.
const TICK_MS: u32 = 1000;

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[cfg_attr(feature = "bindings", derive(TS), ts(rename = "UnlockEvent"))]
pub enum Event {
    /// Screen mounted — reads the vault metadata, nothing else.
    Start,
    /// The user pressed "Unlock".
    Unlock,
    /// Internal: a ceremony effect resolved, tagged with the attempt that asked
    /// for it.
    #[serde(skip)]
    ShellCompleted { attempt: u64, result: ShellResult },
    /// Internal: an elapsed-counter tick resolved. A separate channel on
    /// purpose — the ticker runs alongside the ceremony and must not be
    /// mistaken for its answer.
    #[serde(skip)]
    TickCompleted { attempt: u64 },
}

// ---------------------------------------------------------------------------
// Model
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Stage {
    #[default]
    Idle,
    LoadingVault,
    CheckingSupport,
    Authenticating,
    Opening,
    Completing,
}

#[derive(Default)]
pub struct Model {
    stage: Stage,
    /// What is known about the local vault. `None` before the first load *and*
    /// when there is genuinely no vault — `loaded` tells the two apart, because
    /// the screen must not flash "no vault here" before it has looked.
    vault: Option<VaultMeta>,
    loaded: bool,
    /// Seconds the OS sheet has been up, counted from `Wait` answers.
    waited_secs: u32,
    attempt: u64,
    abort: Option<AbortHandle>,
}

// ---------------------------------------------------------------------------
// ViewModel
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(TS))]
pub struct UnlockView {
    /// A ceremony is in flight — the button shows its spinner and ignores
    /// further presses.
    pub busy: bool,
    /// Semantic status line, or `None` when idle. The shell owns the words.
    pub status: Option<StatusKey>,
    /// Whole seconds the OS sheet has been up. `0` unless authenticating.
    pub waited_secs: u32,
    /// The vault's display label, once known.
    pub vault_label: Option<String>,
    /// True only after a load that found nothing — never before the first load.
    pub vault_missing: bool,
    pub item_count: u32,
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct Unlock;

impl App for Unlock {
    type Event = Event;
    type Model = Model;
    type ViewModel = UnlockView;
    type Effect = Effect;

    fn update(&self, event: Event, model: &mut Model) -> Command<Effect, Event> {
        match event {
            Event::Start => {
                model.stage = Stage::LoadingVault;
                request(model, ShellOperation::LoadVaultMeta)
            }
            Event::Unlock => unlock(model),
            Event::ShellCompleted { attempt, result } => {
                if attempt != model.attempt {
                    return Command::done(); // belongs to a superseded attempt
                }
                accept(model, result)
            }
            Event::TickCompleted { attempt } => {
                // The ticker outlives its ceremony by at most one tick; when it
                // does, it stops rather than counting against the next one.
                if attempt != model.attempt || model.stage != Stage::Authenticating {
                    return Command::done();
                }
                model.waited_secs += 1;
                Command::all([tick(attempt), render()])
            }
        }
    }

    fn view(&self, model: &Model) -> UnlockView {
        UnlockView {
            busy: model.stage != Stage::Idle,
            status: match model.stage {
                Stage::CheckingSupport | Stage::LoadingVault => Some(StatusKey::CheckingDevice),
                Stage::Authenticating => Some(StatusKey::AwaitingBiometric),
                Stage::Opening | Stage::Completing => Some(StatusKey::OpeningVault),
                Stage::Idle => None,
            },
            waited_secs: if model.stage == Stage::Authenticating {
                model.waited_secs
            } else {
                0
            },
            vault_label: model.vault.as_ref().map(|vault| vault.label.clone()),
            vault_missing: model.loaded && model.vault.is_none(),
            item_count: model.vault.as_ref().map_or(0, |vault| vault.item_count),
        }
    }
}

// ---------------------------------------------------------------------------
// Transitions
// ---------------------------------------------------------------------------

fn unlock(model: &mut Model) -> Command<Effect, Event> {
    if model.stage != Stage::Idle {
        return Command::done(); // one ceremony at a time
    }
    // Rule 1: never raise the OS sheet for a vault that is not there.
    if model.loaded && model.vault.is_none() {
        return idle_with_prompt(model, PromptKind::VaultMissing);
    }
    model.attempt += 1;
    model.waited_secs = 0;
    model.stage = Stage::CheckingSupport;
    request(model, ShellOperation::CheckUnlockSupport)
}

fn accept(model: &mut Model, result: ShellResult) -> Command<Effect, Event> {
    match (model.stage, result) {
        // -- vault metadata ---------------------------------------------------
        (Stage::LoadingVault, ShellResult::VaultMetaLoaded { vault }) => {
            model.vault = vault;
            model.loaded = true;
            model.stage = Stage::Idle;
            render()
        }
        (Stage::LoadingVault, ShellResult::StorageFailed { message }) => {
            // A read failure is NOT "no vault": saying so would invite the user
            // to create a second vault over the top of the one that exists.
            model.loaded = false;
            idle_with_prompt(model, PromptKind::StorageFailed { detail: message })
        }

        // -- support probe ----------------------------------------------------
        (Stage::CheckingSupport, ShellResult::UnlockSupport { supported }) => {
            if !supported {
                return idle_with_prompt(model, PromptKind::Unsupported);
            }
            model.stage = Stage::Authenticating;
            model.waited_secs = 0;
            let attempt = model.attempt;
            Command::all([
                request(
                    model,
                    ShellOperation::AuthenticateDevice {
                        purpose: AuthPurpose::Unlock,
                    },
                ),
                tick(attempt),
            ])
        }

        // -- authentication ---------------------------------------------------
        (Stage::Authenticating, ShellResult::DeviceAuthenticated { .. }) => {
            let Some(vault) = model.vault.clone() else {
                // The metadata vanished between the read and the ceremony.
                // Nothing is opened on a guess.
                return idle_with_prompt(model, PromptKind::VaultMissing);
            };
            model.stage = Stage::Opening;
            request(model, ShellOperation::OpenVault { vault_id: vault.id })
        }

        // -- opening and handover ----------------------------------------------
        (Stage::Opening, ShellResult::VaultOpened { .. }) => {
            let Some(vault) = model.vault.clone() else {
                return Command::done();
            };
            model.stage = Stage::Completing;
            request(model, ShellOperation::CompleteUnlock { vault })
        }
        (Stage::Opening, ShellResult::StorageFailed { message }) => {
            idle_with_prompt(model, PromptKind::StorageFailed { detail: message })
        }
        (Stage::Completing, ShellResult::UnlockCompleted) => Command::done(),

        // -- ceremony failures, from any stage ---------------------------------
        (_, ShellResult::AuthFailed { kind, message }) => match kind {
            // Rule 2: the user closed the sheet. Not an error, not an alert.
            FailureKind::Cancelled => {
                model.stage = Stage::Idle;
                model.waited_secs = 0;
                render()
            }
            FailureKind::NotSupported => idle_with_prompt(model, PromptKind::Unsupported),
            FailureKind::LockedOut => idle_with_prompt(model, PromptKind::LockedOut),
            FailureKind::Other => idle_with_prompt(
                model,
                PromptKind::UnlockFailed {
                    detail: message.unwrap_or_default(),
                },
            ),
        },

        // A result that does not belong to the current stage is dropped, not
        // asserted on: the shell is allowed to answer late.
        _ => Command::done(),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn idle_with_prompt(model: &mut Model, prompt: PromptKind) -> Command<Effect, Event> {
    model.stage = Stage::Idle;
    model.waited_secs = 0;
    let attempt = model.attempt;
    Command::all([
        Command::request_from_shell(ShellOperation::Prompt {
            kind: prompt,
            confirmable: false,
        })
        .then_send(move |result| Event::ShellCompleted { attempt, result }),
        render(),
    ])
}

fn tick(attempt: u64) -> Command<Effect, Event> {
    Command::request_from_shell(ShellOperation::Wait { ms: TICK_MS })
        .then_send(move |_| Event::TickCompleted { attempt })
}

fn request(model: &mut Model, operation: ShellOperation) -> Command<Effect, Event> {
    let attempt = model.attempt;
    let command = Command::request_from_shell(operation)
        .then_send(move |result| Event::ShellCompleted { attempt, result });
    model.abort = Some(command.abort_handle());
    Command::all([command, render()])
}
