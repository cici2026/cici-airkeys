//! Machine A's rules, one test per rule.
//!
//! Every one of these runs in microseconds with no device and no clock. That is
//! the return on keeping the decisions in the core: "does a cancelled biometric
//! prompt leave an error on screen?" is answerable in CI, on every platform, on
//! every commit.

#![cfg(feature = "crux")]

mod support;

use airkeys_core::app::shell::{AuthPurpose, ShellOperation, ShellResult};
use airkeys_core::app::unlock::{Event, Unlock};
use airkeys_core::app::{FailureKind, PromptKind, StatusKey};
use support::{vault, Driver, NOW};

type Unlocker = Driver<Unlock>;

/// Walk a driver to the point where the OS sheet is up.
fn awaiting_biometric() -> Unlocker {
    let mut driver = Unlocker::new();
    driver.dispatch(Event::Start);
    driver.resolve(ShellResult::VaultMetaLoaded {
        vault: Some(vault(12)),
    });
    driver.dispatch(Event::Unlock);
    driver.resolve(ShellResult::UnlockSupport { supported: true });
    driver
}

#[test]
fn mounting_reads_the_vault_without_prompting_for_anything() {
    let mut driver = Unlocker::new();
    let ops = driver.dispatch(Event::Start);
    assert_eq!(ops, vec![ShellOperation::LoadVaultMeta]);

    driver.resolve(ShellResult::VaultMetaLoaded {
        vault: Some(vault(12)),
    });
    let view = driver.view();
    assert_eq!(view.vault_label.as_deref(), Some("Personal"));
    assert_eq!(view.item_count, 12);
    assert!(!view.vault_missing);
    assert!(!view.busy);
}

/// Rule 1. Prompting for a fingerprint and *then* saying "there is no vault"
/// teaches users to approve prompts that do nothing.
#[test]
fn unlocking_with_no_vault_never_raises_the_os_sheet() {
    let mut driver = Unlocker::new();
    driver.dispatch(Event::Start);
    driver.resolve(ShellResult::VaultMetaLoaded { vault: None });
    assert!(driver.view().vault_missing);

    let ops = driver.dispatch(Event::Unlock);
    assert_eq!(
        ops,
        vec![ShellOperation::Prompt {
            kind: PromptKind::VaultMissing,
            confirmable: false,
        }],
        "no CheckUnlockSupport, no AuthenticateDevice"
    );
}

/// A failed *read* is not an empty vault: saying so would invite the user to
/// create a second vault over the one that exists.
#[test]
fn a_failed_read_is_not_reported_as_an_empty_vault() {
    let mut driver = Unlocker::new();
    driver.dispatch(Event::Start);
    driver.resolve(ShellResult::StorageFailed {
        message: "disk is on fire".into(),
    });

    assert!(!driver.view().vault_missing, "must not read as 'no vault'");
    assert_eq!(
        driver.outstanding(),
        vec![ShellOperation::Prompt {
            kind: PromptKind::StorageFailed {
                detail: "disk is on fire".into()
            },
            confirmable: false,
        }]
    );
}

#[test]
fn the_happy_path_ends_in_handover() {
    let mut driver = awaiting_biometric();
    assert!(driver
        .outstanding()
        .contains(&ShellOperation::AuthenticateDevice {
            purpose: AuthPurpose::Unlock,
        }));
    assert_eq!(driver.view().status, Some(StatusKey::AwaitingBiometric));

    let ops = driver.resolve_matching(
        |op| matches!(op, ShellOperation::AuthenticateDevice { .. }),
        ShellResult::DeviceAuthenticated {
            now_iso: NOW.into(),
        },
    );
    assert_eq!(
        ops,
        vec![ShellOperation::OpenVault {
            vault_id: "vault-1".into()
        }]
    );

    // `resolve_matching`, not `resolve`: the elapsed ticker's `Wait` is still
    // outstanding and is older than `OpenVault`. Answering the oldest here
    // would feed a vault result to the ticker — which the machine correctly
    // ignores, and the test would then be asserting nothing.
    let ops = driver.resolve_matching(
        |op| matches!(op, ShellOperation::OpenVault { .. }),
        ShellResult::VaultOpened {
            now_iso: NOW.into(),
        },
    );
    assert_eq!(
        ops,
        vec![ShellOperation::CompleteUnlock { vault: vault(12) }]
    );
}

/// Rule 2. Dismissing the sheet is not a failure: no alert, no error state.
#[test]
fn a_cancelled_prompt_returns_to_idle_silently() {
    let mut driver = awaiting_biometric();
    let ops = driver.resolve_matching(
        |op| matches!(op, ShellOperation::AuthenticateDevice { .. }),
        ShellResult::AuthFailed {
            kind: FailureKind::Cancelled,
            message: None,
        },
    );

    assert!(
        !ops.iter()
            .any(|op| matches!(op, ShellOperation::Prompt { .. })),
        "a cancel must not raise a prompt, got {ops:?}"
    );
    let view = driver.view();
    assert!(!view.busy);
    assert_eq!(view.status, None);
    assert_eq!(view.waited_secs, 0);
}

#[test]
fn each_failure_kind_maps_to_its_own_prompt() {
    let cases = [
        (FailureKind::NotSupported, PromptKind::Unsupported),
        (FailureKind::LockedOut, PromptKind::LockedOut),
        (
            FailureKind::Other,
            PromptKind::UnlockFailed {
                detail: "sensor unavailable".into(),
            },
        ),
    ];
    for (kind, expected) in cases {
        let mut driver = awaiting_biometric();
        let ops = driver.resolve_matching(
            |op| matches!(op, ShellOperation::AuthenticateDevice { .. }),
            ShellResult::AuthFailed {
                kind,
                message: Some("sensor unavailable".into()),
            },
        );
        assert!(
            ops.contains(&ShellOperation::Prompt {
                kind: expected.clone(),
                confirmable: false,
            }),
            "{kind:?} should prompt {expected:?}, got {ops:?}"
        );
        assert!(!driver.view().busy);
    }
}

/// Rule 3. A second press while a ceremony is in flight is ignored, and the
/// first ceremony's answer still applies.
#[test]
fn a_second_press_does_not_start_a_second_ceremony() {
    let mut driver = awaiting_biometric();
    let ops = driver.dispatch(Event::Unlock);
    assert!(ops.is_empty(), "expected no new operations, got {ops:?}");
}

/// Rule 4. The elapsed counter is `Wait` answers counted, not a clock read —
/// which is the only reason this test takes microseconds instead of 3 seconds.
#[test]
fn the_elapsed_counter_counts_wait_answers() {
    let mut driver = awaiting_biometric();
    assert_eq!(driver.view().waited_secs, 0);

    for expected in 1..=3 {
        driver.resolve_matching(
            |op| matches!(op, ShellOperation::Wait { .. }),
            ShellResult::Waited,
        );
        assert_eq!(driver.view().waited_secs, expected);
    }

    // Leaving the stage stops the ticker rather than counting into the next
    // ceremony.
    driver.resolve_matching(
        |op| matches!(op, ShellOperation::AuthenticateDevice { .. }),
        ShellResult::AuthFailed {
            kind: FailureKind::Cancelled,
            message: None,
        },
    );
    driver.resolve_matching(
        |op| matches!(op, ShellOperation::Wait { .. }),
        ShellResult::Waited,
    );
    assert_eq!(driver.view().waited_secs, 0);
}

/// The metadata vanished between the read and the ceremony (another device
/// deleted the vault mid-sync). Nothing is opened on a guess.
#[test]
fn authentication_without_vault_metadata_opens_nothing() {
    let mut driver = Unlocker::new();
    driver.dispatch(Event::Start);
    driver.resolve(ShellResult::VaultMetaLoaded {
        vault: Some(vault(3)),
    });
    driver.dispatch(Event::Unlock);
    driver.resolve(ShellResult::UnlockSupport { supported: true });

    // Simulate the vault disappearing: a fresh load returns nothing.
    driver.dispatch(Event::Start);
    driver.resolve_matching(
        |op| matches!(op, ShellOperation::LoadVaultMeta),
        ShellResult::VaultMetaLoaded { vault: None },
    );

    let ops = driver.resolve_matching(
        |op| matches!(op, ShellOperation::AuthenticateDevice { .. }),
        ShellResult::DeviceAuthenticated {
            now_iso: NOW.into(),
        },
    );
    assert!(
        !ops.iter()
            .any(|op| matches!(op, ShellOperation::OpenVault { .. })),
        "must not open a vault it cannot name, got {ops:?}"
    );
}
