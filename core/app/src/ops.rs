//! What the core asks of the world, and what the world tells the core.
//!
//! - [`Request`]: performed by the platform shell (Swift, Kotlin) — the camera and the screen.
//!   Every request is a notification: the core never waits for an answer.
//! - [`CodeOp`]: performed by the Rust runtime — Lumen Code send and receive. No platform
//!   shell sees these.
//! - [`Action`]: what a person did, or what happened to the app.
//!
//! There is no storage and no randomness here, by design: Airkeys never creates a recovery
//! phrase and never writes one down. The person brings their phrase each session; it lives
//! in the app's memory until they close the wallet or the app is closed.

use crux_core::capability::Operation;

/// A request to the platform shell.
#[derive(Clone, Debug, PartialEq, uniffi::Enum)]
pub enum Request {
    /// Start or stop the back camera; frames go to `AirkeysCore::scan`.
    Camera {
        on: bool,
    },
    /// Keep the screen on (and bright, for a code) while a code is shown or scanned.
    KeepScreenOn {
        on: bool,
    },
    /// The screen shows a recovery phrase as it is typed: block screenshots and recording.
    Private {
        on: bool,
    },
    Haptic {
        kind: Haptic,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum Haptic {
    Success,
    Warning,
    Error,
}

impl Operation for Request {
    type Output = ();
}

/// Camera-code work for the Rust runtime: Lumen Code out and in, QR codes in.
#[derive(Clone, Debug, PartialEq)]
pub enum CodeOp {
    /// Play `data` as an animated code, as file `name`.
    Show {
        name: String,
        data: Vec<u8>,
    },
    Hide,
    /// Read a Lumen Code transfer from camera frames (a sign request).
    Listen,
    /// Read a QR code from camera frames (a recovery phrase).
    ListenQr,
    Stop,
}

impl Operation for CodeOp {
    type Output = ();
}

/// Whether a radio is off — its switch, and what the phone can observe of it. Only `Off`
/// counts as off: the phone must show that it is offline.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, uniffi::Enum)]
pub enum RadioState {
    /// Switched off, and no sign of it working.
    Off,
    /// Switched on.
    On,
    /// Switched off, yet observed working: a cellular signal, a recent Wi-Fi scan, a Bluetooth
    /// scan that runs.
    Active,
    #[default]
    Unknown,
}

/// What a person did, or what happened to the app.
#[derive(Clone, PartialEq, uniffi::Enum)]
pub enum Action {
    /// The phone's radios and its network connections, at start and whenever one changes.
    /// Airkeys works only while all four are off; `network` is on while the phone has any
    /// network at all (Wi-Fi, cellular, Bluetooth or USB tethering, Ethernet, VPN).
    Radios {
        wifi: RadioState,
        bluetooth: RadioState,
        cellular: RadioState,
        network: RadioState,
    },
    Back,

    /// "Enter recovery phrase".
    Begin,
    PhraseInput {
        text: String,
    },
    /// Replace the word being typed with a suggestion.
    Suggestion {
        word: String,
    },
    PhraseDone,
    PassphraseInput {
        passphrase: String,
    },
    Open,
    /// Forget the wallet now.
    Close,

    /// Show another account number, below [`account_count`](crate::runtime::account_count).
    Account {
        index: u32,
    },
    OpenAccount {
        network: crate::view::NetworkKind,
    },
    /// Show one account as a code.
    ShareAccount {
        network: crate::view::NetworkKind,
    },
    Connect,
    /// On the Connect screen: every account number, or only the one on the home screen.
    ConnectScope {
        all: bool,
    },
    ToggleNetwork {
        network: crate::view::NetworkKind,
    },
    ShowAccounts,

    Scan,
    /// The camera could not start; the message says why.
    CameraFailed {
        message: String,
    },
    Sign,
    Reject,
    /// Leave a code screen or a refusal.
    Done,
    DismissToast,
}

/// Names only: some actions carry a recovery phrase, a passphrase or a word of one.
impl std::fmt::Debug for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Action::Radios { .. } => "Radios",
            Action::Back => "Back",
            Action::Begin => "Begin",
            Action::PhraseInput { .. } => "PhraseInput",
            Action::Suggestion { .. } => "Suggestion",
            Action::PhraseDone => "PhraseDone",
            Action::PassphraseInput { .. } => "PassphraseInput",
            Action::Open => "Open",
            Action::Close => "Close",
            Action::Account { .. } => "Account",
            Action::OpenAccount { .. } => "OpenAccount",
            Action::ShareAccount { .. } => "ShareAccount",
            Action::Connect => "Connect",
            Action::ConnectScope { .. } => "ConnectScope",
            Action::ToggleNetwork { .. } => "ToggleNetwork",
            Action::ShowAccounts => "ShowAccounts",
            Action::Scan => "Scan",
            Action::CameraFailed { .. } => "CameraFailed",
            Action::Sign => "Sign",
            Action::Reject => "Reject",
            Action::Done => "Done",
            Action::DismissToast => "DismissToast",
        })
    }
}
