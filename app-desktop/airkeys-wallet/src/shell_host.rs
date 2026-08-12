//! The half of the architecture that **does**.
//!
//! `airkeys-core` decides and asks; this module performs. It is the desktop
//! equivalent of the web app's effect loop and of the executor each native
//! shell will implement — and it is deliberately the only place in this crate
//! that touches storage, timers, the clipboard or the biometric API.
//!
//! Two pieces:
//!
//! - [`CoreHost`] — generic over any machine, holds the `Core`, hands out
//!   correlated [`Pending`] operations and takes answers back. Identical in
//!   shape to the wasm bridge and the test driver, because they are three
//!   consumers of the same [`SplitEffect`] contract.
//! - [`DemoShell`] — a fixture-backed executor. It performs the operations
//!   against in-memory data so the whole app runs, and every state is
//!   reachable, before a real keystore exists. Swapping it for the real thing
//!   is a change to this file and to nothing else.
//!
//! ```text
//!   GPUI entity                CoreHost              DemoShell
//!       │ dispatch(event) ────────►│
//!       │◄──────── Vec<Pending> ───│
//!       │ perform(op) ─────────────────────────────────►│
//!       │◄──────────────────── Performed ───────────────│
//!       │ resolve(id, result) ────►│
//!       │◄──────── Vec<Pending> ───│   …until the queue drains
//! ```

use std::collections::HashMap;
use std::time::Duration;

use crux_core::{App, Core, Request};

use airkeys_core::app::shell::{CopyField, Effect, ShellOperation, ShellResult};
use airkeys_core::types::{Item, ItemKind, VaultMeta};

/// One operation the core is waiting on, and the id that will answer it.
pub struct Pending {
    pub id: u64,
    pub operation: ShellOperation,
}

/// Drives one Crux machine in-process.
///
/// The correlation rules are the same three the wasm bridge documents, and for
/// the same reasons: ids are monotonic, an unknown id means the answer outlived
/// the question, and a resolve error means the owning command was aborted. All
/// three are *normal*, none is a bug, and none should surface to the user.
pub struct CoreHost<A>
where
    A: App<Effect = Effect> + Default,
    A::Model: Default,
{
    core: Core<A>,
    pending: HashMap<u64, Request<ShellOperation>>,
    next_id: u64,
}

impl<A> CoreHost<A>
where
    A: App<Effect = Effect> + Default,
    A::Model: Default,
{
    pub fn new() -> Self {
        Self {
            core: Core::new(),
            pending: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn view(&self) -> A::ViewModel {
        self.core.view()
    }

    /// Send an event; returns what the core now needs performed.
    pub fn dispatch(&mut self, event: A::Event) -> Vec<Pending> {
        let effects = self.core.process_event(event);
        self.collect(effects)
    }

    /// Answer an operation by id.
    pub fn resolve(&mut self, id: u64, result: ShellResult) -> Vec<Pending> {
        let Some(mut request) = self.pending.remove(&id) else {
            // The answer outlived the question. Expected — a timer that fired
            // after the screen moved on — and a no-op.
            return Vec::new();
        };
        match self.core.resolve(&mut request, result) {
            Ok(effects) => self.collect(effects),
            // The owning command was aborted before the answer arrived.
            Err(_) => Vec::new(),
        }
    }

    fn collect(&mut self, effects: Vec<Effect>) -> Vec<Pending> {
        let mut out = Vec::new();
        for effect in effects {
            // `Effect::Render` falls out here as `None`. The caller re-renders
            // unconditionally after a pump, so the render effect needs no
            // separate handling — but it must still be *split off*, or a render
            // would be queued as an operation nobody can perform.
            if let Some(request) = effect.into_shell() {
                self.next_id += 1;
                let id = self.next_id;
                out.push(Pending {
                    id,
                    operation: request.operation.clone(),
                });
                self.pending.insert(id, request);
            }
        }
        out
    }
}

impl<A> Default for CoreHost<A>
where
    A: App<Effect = Effect> + Default,
    A::Model: Default,
{
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// The executor
// ---------------------------------------------------------------------------

/// What performing an operation produced.
pub enum Performed {
    /// Answer immediately, in the same pump.
    Now(ShellResult),
    /// Answer after a delay. The caller schedules a timer; the core is unaware
    /// that time passed, which is what keeps it testable.
    After(Duration, ShellResult),
    /// Not this module's business — the screen itself handles it (a prompt to
    /// show, a hand-off to perform) and answers when it has.
    Screen,
}

/// Which fixture state the app starts in. `AIRKEYS_DEMO` — the same env-pin
/// family as `AIRKEYS_THEME` and `AIRKEYS_LANG`.
///
/// This exists because the interesting states are the ones that are hard to
/// reach on purpose: an authenticator that is not enrolled, a keystore that is
/// locked out, a vault that is not there. Being able to *start* in one of them
/// is the difference between designing those screens and hoping.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Scenario {
    #[default]
    Ok,
    NoVault,
    AuthCancelled,
    AuthUnsupported,
    LockedOut,
    StorageError,
}

impl Scenario {
    pub fn from_env() -> Self {
        match std::env::var("AIRKEYS_DEMO").as_deref() {
            Ok("no-vault") => Self::NoVault,
            Ok("cancel") => Self::AuthCancelled,
            Ok("unsupported") => Self::AuthUnsupported,
            Ok("locked-out") => Self::LockedOut,
            Ok("storage-error") => Self::StorageError,
            _ => Self::Ok,
        }
    }
}

/// A fixture-backed stand-in for the platform: storage, biometrics, clipboard.
///
/// **The secrets live here and only here.** `Item` carries no secret field, and
/// the core addresses them by `(item_id, field)` — so the plaintext a user
/// copies exists in this struct, on the clipboard, and nowhere else in the
/// process. That is not an accident of the fixture; it is the property the real
/// keystore implementation has to preserve.
pub struct DemoShell {
    scenario: Scenario,
    vault: Option<VaultMeta>,
    items: Vec<Item>,
    secrets: HashMap<String, String>,
    /// What this app last put on the clipboard, so a wipe only wipes its own
    /// value and never something the user copied from elsewhere.
    clipboard: Option<String>,
}

impl DemoShell {
    pub fn from_env() -> Self {
        let scenario = Scenario::from_env();
        let items = fixture_items();
        let secrets = items
            .iter()
            .enumerate()
            .map(|(index, item)| (item.id.clone(), format!("demo-secret-{index}")))
            .collect();
        Self {
            scenario,
            vault: match scenario {
                Scenario::NoVault => None,
                _ => Some(VaultMeta {
                    id: "vault-local".into(),
                    label: "Personal".into(),
                    item_count: items.len() as u32,
                    created_at_iso: "2026-01-04T10:00:00.000Z".into(),
                }),
            },
            items,
            secrets,
            clipboard: None,
        }
    }

    pub fn scenario(&self) -> Scenario {
        self.scenario
    }

    /// How strong an item's secret is.
    ///
    /// The plaintext is read here and never leaves: what comes back is a
    /// *verdict* — a band, a bit count and some advice — which is safe to put
    /// in a view model, a log line or a screenshot. The distinction between
    /// "the secret" and "a summary of the secret" is the one that decides
    /// whether a password ends up in a crash dump.
    pub fn strength_of(&self, item_id: &str) -> Option<airkeys_core::strength::StrengthReport> {
        self.secrets
            .get(item_id)
            .map(|secret| airkeys_core::strength::secret_strength(secret))
    }

    /// Perform one operation.
    pub fn perform(&mut self, operation: &ShellOperation) -> Performed {
        use airkeys_core::app::FailureKind;

        match operation {
            ShellOperation::CheckUnlockSupport => Performed::Now(ShellResult::UnlockSupport {
                supported: self.scenario != Scenario::AuthUnsupported,
            }),

            ShellOperation::AuthenticateDevice { .. } => {
                // A real OS sheet takes as long as the human does. The delay is
                // what makes the elapsed counter and the busy state visible at
                // all — without it every ceremony would complete inside one
                // frame and those states would be unreachable by hand.
                let delay = Duration::from_millis(1200);
                match self.scenario {
                    Scenario::AuthCancelled => Performed::After(
                        delay,
                        ShellResult::AuthFailed {
                            kind: FailureKind::Cancelled,
                            message: None,
                        },
                    ),
                    Scenario::LockedOut => Performed::After(
                        delay,
                        ShellResult::AuthFailed {
                            kind: FailureKind::LockedOut,
                            message: Some("too many attempts".into()),
                        },
                    ),
                    _ => Performed::After(
                        delay,
                        ShellResult::DeviceAuthenticated { now_iso: now_iso() },
                    ),
                }
            }

            ShellOperation::LoadVaultMeta => {
                if self.scenario == Scenario::StorageError {
                    return Performed::Now(ShellResult::StorageFailed {
                        message: "the vault file could not be read".into(),
                    });
                }
                Performed::Now(ShellResult::VaultMetaLoaded {
                    vault: self.vault.clone(),
                })
            }

            ShellOperation::OpenVault { .. } => {
                Performed::Now(ShellResult::VaultOpened { now_iso: now_iso() })
            }

            ShellOperation::LoadItems { .. } => {
                if self.scenario == Scenario::StorageError {
                    return Performed::Now(ShellResult::StorageFailed {
                        message: "the vault file could not be read".into(),
                    });
                }
                Performed::Now(ShellResult::ItemsLoaded {
                    items: self.items.clone(),
                })
            }

            ShellOperation::SaveItem { item } => {
                if self.scenario == Scenario::StorageError {
                    return Performed::Now(ShellResult::StorageFailed {
                        message: "the vault is read-only".into(),
                    });
                }
                if let Some(slot) = self.items.iter_mut().find(|slot| slot.id == item.id) {
                    *slot = item.clone();
                }
                Performed::Now(ShellResult::ItemSaved)
            }

            ShellOperation::CopyField { request } => {
                let Some(item) = self.items.iter().find(|item| item.id == request.item_id) else {
                    return Performed::Now(ShellResult::StorageFailed {
                        message: "that item is no longer in the vault".into(),
                    });
                };
                // The one place plaintext is resolved from an id.
                let value = match request.field {
                    CopyField::Account => item.account.clone(),
                    CopyField::Site => item.site.clone(),
                    CopyField::Secret => self
                        .secrets
                        .get(&request.item_id)
                        .cloned()
                        .unwrap_or_default(),
                };
                self.clipboard = Some(value);
                Performed::Now(ShellResult::Copied)
            }

            ShellOperation::ClearClipboard => {
                // Only ever wipe our own value: the user may well have copied
                // something else in the meantime, and stealing their clipboard
                // is a worse bug than leaving a secret on it a moment longer.
                self.clipboard = None;
                Performed::Now(ShellResult::ClipboardCleared)
            }

            ShellOperation::Wait { ms } => {
                Performed::After(Duration::from_millis(u64::from(*ms)), ShellResult::Waited)
            }

            // The screen owns what the user sees and where the app goes next.
            ShellOperation::Prompt { .. } | ShellOperation::CompleteUnlock { .. } => {
                Performed::Screen
            }
        }
    }
}

/// The wall clock, read **here** and handed to the core as data.
///
/// This function is the entire reason the core has no clock: every timestamp it
/// stores arrived as a field on a result. Formatting is deliberately crude —
/// the desktop fixture does not need a date library, and a real shell would use
/// the platform's own ISO-8601 formatter.
fn now_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default();
    format!("@{secs}")
}

/// The demo vault's contents. Content, not lorem ipsum: a list of plausible
/// entries is what makes truncation, long accounts and mixed scripts show up in
/// review instead of in production.
fn fixture_items() -> Vec<Item> {
    let make =
        |id: &str, kind: ItemKind, title: &str, account: &str, site: &str, favorite: bool| Item {
            id: id.into(),
            kind,
            title: title.into(),
            account: account.into(),
            site: site.into(),
            favorite,
            updated_at_iso: "2026-08-11T21:14:00.000Z".into(),
        };
    vec![
        make(
            "itm-1",
            ItemKind::Login,
            "GitHub",
            "ann@example.com",
            "github.com",
            true,
        ),
        make(
            "itm-2",
            ItemKind::Login,
            "Cloudflare",
            "ops@example.com",
            "dash.cloudflare.com",
            false,
        ),
        make(
            "itm-3",
            ItemKind::Key,
            "Deploy key — prod",
            "deploy@infra",
            "infra.example",
            true,
        ),
        make(
            "itm-4",
            ItemKind::Note,
            "Recovery codes",
            "—",
            "accounts.example",
            false,
        ),
        make(
            "itm-5",
            ItemKind::Card,
            "工商银行储蓄卡",
            "6222 •••• 8891",
            "icbc.com.cn",
            false,
        ),
        make(
            "itm-6",
            ItemKind::Login,
            "A service with a genuinely long display name",
            "a-very-long-account-identifier@example.com",
            "long.subdomain.example.com",
            false,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use airkeys_core::app::shell::CopyRequest;

    #[test]
    fn the_demo_vault_matches_its_own_item_count() {
        let shell = DemoShell::from_env();
        let vault = shell
            .vault
            .clone()
            .expect("the default scenario has a vault");
        assert_eq!(vault.item_count as usize, shell.items.len());
    }

    #[test]
    fn copying_resolves_plaintext_from_an_id_and_nothing_else_holds_it() {
        let mut shell = DemoShell::from_env();
        let id = shell.items[0].id.clone();
        let result = shell.perform(&ShellOperation::CopyField {
            request: CopyRequest {
                item_id: id.clone(),
                field: CopyField::Secret,
            },
        });
        assert!(matches!(result, Performed::Now(ShellResult::Copied)));
        assert_eq!(
            shell.clipboard.as_deref(),
            shell.secrets.get(&id).map(String::as_str)
        );

        // And the item the core sees carries no secret at all — this is the
        // property the real keystore must preserve.
        let serialized = serde_json_string(&shell.items[0]);
        assert!(
            !serialized.contains("demo-secret"),
            "an Item must never carry plaintext: {serialized}"
        );
    }

    #[test]
    fn clearing_only_wipes_our_own_value() {
        let mut shell = DemoShell::from_env();
        shell.clipboard = Some("something".into());
        shell.perform(&ShellOperation::ClearClipboard);
        assert_eq!(shell.clipboard, None);
    }

    #[test]
    fn a_wait_is_reported_as_a_delay_never_performed_inline() {
        let mut shell = DemoShell::from_env();
        match shell.perform(&ShellOperation::Wait { ms: 250 }) {
            Performed::After(delay, ShellResult::Waited) => {
                assert_eq!(delay, Duration::from_millis(250));
            }
            _ => panic!("a Wait must schedule, not block"),
        }
    }

    /// Serialize without pulling serde_json into this crate's dependency list —
    /// `Item` derives Debug, and the assertion only needs the field values.
    fn serde_json_string(item: &Item) -> String {
        format!("{item:?}")
    }
}
