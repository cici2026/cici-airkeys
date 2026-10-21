//! The Airkeys phone app as one Crux state machine.
//!
//! Everything here **decides**; nothing here **does**. An [`Event`] updates the [`Model`] and
//! returns commands: render, a [`Request`] for the platform (camera, screen), or a [`CodeOp`]
//! for the Rust runtime (Lumen Code, QR codes). No clock, no I/O, no threads — so every flow
//! is tested in microseconds and behaves the same on iOS and Android.
//!
//! Airkeys keeps nothing. The person brings their recovery phrase each session — typed or
//! from a SeedQR — and it lives in memory until they close the wallet or the app is closed.
//! Nothing creates a phrase here, and nothing is written anywhere.
//!
//! What the platform does follows from the screen alone: `Model::reconcile` compares what
//! the current phase needs (camera, screen kept on, screenshots blocked, a code playing, a
//! receiver listening) with what was last asked for, and asks for the difference. No path
//! can leave the camera on or a phrase unprotected.

use crate::ops::{Action, CodeOp, Haptic, RadioState, Request};
use crate::view::*;
use airkeys_wallet::keys::{check_passphrase, completions, word_index};
use airkeys_wallet::protocol::{Account, Message};
use airkeys_wallet::{Mnemonic, Network, Review, Wallet, review};
use crux_core::macros::effect;
use crux_core::render::{RenderOperation, render};
use crux_core::{App, Command};
use std::ops::Range;
use zeroize::{Zeroize, Zeroizing};

#[effect]
pub enum Effect {
    Render(RenderOperation),
    Platform(Request),
    Code(CodeOp),
}

pub enum Event {
    Action(Action),
    /// A complete Lumen Code transfer (from the runtime).
    Received(Vec<u8>),
    /// Reception progress, in fountain symbols (from the runtime).
    Progress {
        received: u32,
        needed: u32,
    },
    /// The payload of a QR code (from the runtime).
    QrScanned(Vec<u8>),
}

/// Sizes, never contents: a QR code may hold a recovery phrase.
impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Event::Action(a) => f.debug_tuple("Action").field(a).finish(),
            Event::Received(b) => write!(f, "Received({} bytes)", b.len()),
            Event::Progress { received, needed } => write!(f, "Progress({received}/{needed})"),
            Event::QrScanned(b) => write!(f, "QrScanned({} bytes)", b.len()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Purpose {
    /// A recovery phrase QR code (SeedQR, CompactSeedQR, text).
    Phrase,
    /// A sign request from the web app (Lumen Code).
    Request,
}

enum Phase {
    Welcome,
    Import {
        text: Zeroizing<String>,
    },
    Passphrase {
        phrase: Mnemonic,
        typed: Zeroizing<String>,
        passphrase: Zeroizing<String>,
        problem: Option<String>,
    },
    Home,
    Account {
        network: Network,
    },
    Connect {
        selected: [bool; 2],
        /// Every account number (`0..ACCOUNTS`), or only the one on the home screen.
        all: bool,
    },
    Code {
        title: String,
        caption: String,
        message: Message,
        json: String,
        back: Box<Phase>,
    },
    Scan {
        purpose: Purpose,
        progress: Option<(u32, u32)>,
        problem: Option<String>,
        typed: Option<Zeroizing<String>>,
    },
    Review {
        review: Box<Review>,
    },
    Refused {
        title: String,
        message: String,
    },
}

/// What the platform has been asked to do, as the screen requires it.
#[derive(Clone, Default, PartialEq)]
struct Platform {
    camera: bool,
    awake: bool,
    private: bool,
    listening: Option<Purpose>,
    /// The JSON of the code on screen.
    showing: Option<String>,
}

pub struct Model {
    rev: u64,
    /// Wi-Fi, Bluetooth, mobile data, as the shell last reported them.
    radios: [RadioState; 4],
    toast: Option<String>,
    phase: Phase,
    /// The open wallet; `None` whenever none is open.
    wallet: Option<Wallet>,
    /// The account number on the home screen.
    index: u32,
    platform: Platform,
}

impl Default for Model {
    fn default() -> Self {
        Model {
            rev: 0,
            radios: [RadioState::Unknown; 4],
            toast: None,
            phase: Phase::Welcome,
            wallet: None,
            index: 0,
            platform: Platform::default(),
        }
    }
}

#[derive(Default)]
pub struct Airkeys;

type Cmd = Command<Effect, Event>;

fn notify(r: Request) -> Cmd {
    Command::notify_shell(r).build()
}

fn code(op: CodeOp) -> Cmd {
    Command::notify_shell(op).build()
}

fn haptic(kind: Haptic) -> Cmd {
    notify(Request::Haptic { kind })
}

const NETWORK_ORDER: [Network; 2] = [Network::Bitcoin, Network::Evm];

/// The rows of the offline check: what to do while one is on, and what "still active" means.
const RADIOS: [(&str, &str, &str); 4] = [
    (
        "Wi-Fi",
        "Switch Wi-Fi off.",
        "Switched off, but the phone still scans for Wi-Fi networks to find its location. Turn off Wi-Fi scanning (in Location settings), or turn Location off.",
    ),
    (
        "Bluetooth",
        "Switch Bluetooth off.",
        "Switched off, but still working, usually for Bluetooth scanning (in Location settings). Turn that off, or turn Location off.",
    ),
    (
        "Mobile data",
        "Turn on Airplane mode.",
        "Airplane mode is on, but the phone still has a cellular signal.",
    ),
    (
        "Network connection",
        "Disconnect every network: Wi-Fi, a cable, tethering, a VPN.",
        "The phone still has a network: tethering, USB, Ethernet or VPN. Disconnect it.",
    ),
];

/// The account numbers the phone offers to show and connect: #0 to #99, one turn of the
/// home screen's wheel. Signing is not limited: a request names its own account.
pub const ACCOUNTS: u32 = 100;

fn network_name(n: Network) -> &'static str {
    match n {
        Network::Bitcoin => "Bitcoin",
        Network::Evm => "Ethereum & EVM",
    }
}

fn network_assets(n: Network) -> &'static str {
    match n {
        Network::Bitcoin => "BTC",
        Network::Evm => "ETH, ERC-20 tokens",
    }
}

impl Model {
    /// Airkeys works only on a phone with every radio off.
    fn offline(&self) -> bool {
        self.radios.iter().all(|r| *r == RadioState::Off)
    }

    /// Forgets the wallet and anything typed: the next session starts from nothing.
    fn forget(&mut self) {
        self.wallet = None;
        self.index = 0;
        self.phase = Phase::Welcome;
    }

    /// What the current phase needs from the platform: nothing while the phone is online —
    /// except keeping screenshots blocked, which follows the phase alone, so no frame with a
    /// phrase on it is ever unprotected.
    fn wanted(&self) -> Platform {
        let private = matches!(
            self.phase,
            Phase::Import { .. } | Phase::Passphrase { .. } | Phase::Scan { purpose: Purpose::Phrase, .. }
        );
        if !self.offline() {
            return Platform { private, ..Platform::default() };
        }
        let (camera, listening) = match &self.phase {
            Phase::Scan { purpose, problem: None, .. } => (true, Some(*purpose)),
            _ => (false, None),
        };
        Platform {
            camera,
            awake: matches!(self.phase, Phase::Code { .. } | Phase::Scan { .. }),
            private,
            listening,
            showing: match &self.phase {
                Phase::Code { json, .. } => Some(json.clone()),
                _ => None,
            },
        }
    }

    /// Asks the platform for the difference between what the phase needs and what it has.
    fn reconcile(&mut self) -> Cmd {
        let want = self.wanted();
        let had = std::mem::replace(&mut self.platform, want.clone());
        let mut cmds = Vec::new();
        if want.listening != had.listening {
            cmds.push(code(match want.listening {
                Some(Purpose::Request) => CodeOp::Listen,
                Some(Purpose::Phrase) => CodeOp::ListenQr,
                None => CodeOp::Stop,
            }));
        }
        if want.camera != had.camera {
            cmds.push(notify(Request::Camera { on: want.camera }));
        }
        if want.awake != had.awake {
            cmds.push(notify(Request::KeepScreenOn { on: want.awake }));
        }
        if want.private != had.private {
            cmds.push(notify(Request::Private { on: want.private }));
        }
        if want.showing != had.showing {
            cmds.push(match (&want.showing, &self.phase) {
                (Some(json), Phase::Code { message, .. }) => {
                    code(CodeOp::Show { name: message.file_name(), data: json.clone().into_bytes() })
                }
                _ => code(CodeOp::Hide),
            });
        }
        Command::all(cmds)
    }
}

impl Airkeys {
    fn show_code(&self, model: &mut Model, title: String, caption: String, message: Message, back: Phase) {
        let json = message.to_json();
        model.phase = Phase::Code { title, caption, message, json, back: Box::new(back) };
    }

    /// Shows the accounts message for `networks` and account numbers `indexes`; Done returns
    /// to `back`.
    fn share(&self, model: &mut Model, networks: &[Network], indexes: Range<u32>, back: Phase) {
        let Some(wallet) = &model.wallet else { return };
        let mut accounts = Vec::new();
        for index in indexes.clone() {
            for &n in networks {
                match wallet.address(n, index) {
                    Ok(address) => accounts.push(Account { network: n, index, address }),
                    Err(e) => {
                        model.toast = Some(e.0);
                        return;
                    }
                }
            }
        }
        let names: Vec<_> = networks.iter().map(|n| network_name(*n)).collect();
        let which = match indexes.len() {
            1 => format!("Account #{}", indexes.start),
            _ => format!("Accounts #{} to #{}", indexes.start, indexes.end - 1),
        };
        let caption =
            format!("{which} · {}. Addresses only: no public key leaves this phone.", names.join(", "));
        self.show_code(model, "Show this to the web app".into(), caption, Message::Accounts(accounts), back);
    }

    /// A Lumen Code transfer arrived while scanning for a request.
    fn received(&self, model: &mut Model, bytes: &[u8]) -> Cmd {
        let refuse = |model: &mut Model, title: &str, message: String| {
            model.phase = Phase::Refused { title: title.into(), message };
            haptic(Haptic::Error)
        };
        match Message::parse(bytes) {
            Err(e) => refuse(model, "Not an Airkeys request", e.0),
            Ok(Message::Sign(request)) => match &model.wallet {
                None => Command::done(),
                Some(wallet) => match review(&request, wallet) {
                    Ok(r) => {
                        let warn = !r.warnings.is_empty();
                        model.phase = Phase::Review { review: Box::new(r) };
                        haptic(if warn { Haptic::Warning } else { Haptic::Success })
                    }
                    Err(e) => refuse(model, "Airkeys will not sign this", e.0),
                },
            },
            Ok(m) => refuse(
                model,
                "This is not a request",
                format!(
                    "This code holds an “{}” message from a phone. Scan the code on the Send screen of the web app.",
                    m.type_name()
                ),
            ),
        }
    }

    /// A QR code arrived while scanning for a recovery phrase.
    fn qr(&self, model: &mut Model, payload: &[u8]) -> Cmd {
        let Phase::Scan { problem, .. } = &mut model.phase else { return Command::done() };
        match Mnemonic::from_qr(payload) {
            Ok(phrase) => {
                model.phase = Phase::Import { text: Zeroizing::new(format!("{} ", phrase.phrase())) };
                haptic(Haptic::Success)
            }
            Err(e) => {
                // the camera stays off until the person tries again
                *problem = Some(e.0);
                haptic(Haptic::Error)
            }
        }
    }

    fn back(&self, model: &mut Model) {
        let phase = std::mem::replace(&mut model.phase, Phase::Welcome);
        model.phase = match phase {
            Phase::Import { .. } => Phase::Welcome,
            Phase::Passphrase { typed, .. } => Phase::Import { text: typed },
            Phase::Code { back, .. } => *back,
            Phase::Scan { purpose: Purpose::Phrase, typed, .. } => match typed {
                Some(text) => Phase::Import { text },
                None => Phase::Welcome,
            },
            Phase::Scan { purpose: Purpose::Request, .. }
            | Phase::Account { .. }
            | Phase::Connect { .. }
            | Phase::Review { .. }
            | Phase::Refused { .. } => Phase::Home,
            other => other,
        };
    }

    fn action(&self, action: Action, model: &mut Model) -> Cmd {
        let open = model.wallet.is_some();
        // while the phone is online, only the radios and closing the wallet are heard
        if !model.offline() && !matches!(action, Action::Radios { .. } | Action::Close | Action::DismissToast)
        {
            return Command::done();
        }
        match action {
            Action::Close => model.forget(),
            Action::Radios { wifi, bluetooth, cellular, network } => {
                model.radios = [wifi, bluetooth, cellular, network]
            }
            Action::Back => self.back(model),
            Action::DismissToast => model.toast = None,

            // ---------------------------------------------------------- the phrase
            Action::Begin if matches!(model.phase, Phase::Welcome) => {
                model.phase = Phase::Import { text: Zeroizing::new(String::new()) };
            }
            Action::PhraseInput { text } => {
                if let Phase::Import { text: t } = &mut model.phase {
                    *t = Zeroizing::new(autocomplete(t, &text));
                }
            }
            Action::Suggestion { word } => {
                if let Phase::Import { text } = &mut model.phase {
                    *text = Zeroizing::new(complete_last(text, &word));
                }
            }
            Action::PhraseDone => {
                if let Phase::Import { text } = &mut model.phase {
                    *text = Zeroizing::new(expand(text, true));
                    match Mnemonic::parse(text) {
                        Ok(phrase) => {
                            let typed = text.clone();
                            let passphrase = Zeroizing::new(String::new());
                            model.phase = Phase::Passphrase { phrase, typed, passphrase, problem: None };
                        }
                        Err(e) => model.toast = Some(e.0),
                    }
                }
            }
            Action::PassphraseInput { passphrase: p } => {
                if let Phase::Passphrase { passphrase, problem, .. } = &mut model.phase {
                    *passphrase = Zeroizing::new(p.chars().take(256).collect());
                    *problem = None;
                }
            }
            Action::Open => {
                let Phase::Passphrase { phrase, passphrase, problem, .. } = &mut model.phase else {
                    return Command::done();
                };
                match phrase.seed(passphrase).and_then(|seed| Wallet::new(&seed)) {
                    Ok(wallet) => {
                        model.wallet = Some(wallet);
                        model.index = 0;
                        model.phase = Phase::Home;
                        return haptic(Haptic::Success);
                    }
                    Err(e) => *problem = Some(e.0),
                }
            }

            // ---------------------------------------------------------- accounts
            Action::Account { index } if open && index < ACCOUNTS => model.index = index,
            Action::OpenAccount { network } if open => {
                model.phase = Phase::Account { network: network.into() }
            }
            Action::ShareAccount { network } if open => {
                let index = model.index;
                self.share(
                    model,
                    &[network.into()],
                    index..index + 1,
                    Phase::Account { network: network.into() },
                )
            }
            Action::Connect if open => model.phase = Phase::Connect { selected: [true; 2], all: true },
            Action::ConnectScope { all: choice } => {
                if let Phase::Connect { all, .. } = &mut model.phase {
                    *all = choice;
                }
            }
            Action::ToggleNetwork { network } => {
                if let Phase::Connect { selected, .. } = &mut model.phase {
                    let n: Network = network.into();
                    let i = NETWORK_ORDER.iter().position(|x| *x == n).unwrap();
                    selected[i] = !selected[i];
                }
            }
            Action::ShowAccounts if open => {
                if let Phase::Connect { selected, all } = model.phase {
                    let networks: Vec<Network> =
                        NETWORK_ORDER.iter().zip(selected).filter(|(_, s)| *s).map(|(n, _)| *n).collect();
                    let indexes = if all { 0..ACCOUNTS } else { model.index..model.index + 1 };
                    if networks.is_empty() {
                        model.toast = Some("Choose at least one network.".into());
                    } else {
                        self.share(model, &networks, indexes, Phase::Home);
                    }
                }
            }

            // ---------------------------------------------------------- scanning, signing
            Action::Scan => {
                let scan = |purpose, typed| Phase::Scan { purpose, progress: None, problem: None, typed };
                let next = match &model.phase {
                    // "try again" after a failure
                    Phase::Scan { purpose, typed, problem: Some(_), .. } => {
                        Some(scan(*purpose, typed.clone()))
                    }
                    _ if open => Some(scan(Purpose::Request, None)),
                    Phase::Welcome => Some(scan(Purpose::Phrase, None)),
                    Phase::Import { text } => Some(scan(Purpose::Phrase, Some(text.clone()))),
                    _ => None,
                };
                if let Some(next) = next {
                    model.phase = next;
                }
            }
            Action::CameraFailed { message } => {
                if let Phase::Scan { problem, .. } = &mut model.phase {
                    *problem = Some(message);
                }
            }
            Action::Reject => {
                if matches!(model.phase, Phase::Review { .. }) {
                    model.phase = Phase::Home;
                }
            }
            Action::Sign => {
                let (Phase::Review { review }, Some(wallet)) = (&model.phase, &model.wallet) else {
                    return Command::done();
                };
                return match review.sign(wallet) {
                    Ok(signed) => {
                        let n = signed.txs.len();
                        let what = if n == 1 {
                            "the transaction".to_string()
                        } else {
                            format!("all {n} transactions")
                        };
                        let caption = format!(
                            "Scan this with the web app to broadcast {what}. Nothing is sent until the web app broadcasts it."
                        );
                        self.show_code(model, "Signed".into(), caption, Message::Signed(signed), Phase::Home);
                        haptic(Haptic::Success)
                    }
                    Err(e) => {
                        model.phase = Phase::Refused { title: "Signing failed".into(), message: e.0 };
                        haptic(Haptic::Error)
                    }
                };
            }
            Action::Done => {
                if matches!(model.phase, Phase::Code { .. } | Phase::Refused { .. }) {
                    self.back(model);
                }
            }
            _ => {}
        }
        Command::done()
    }
}

/// Typing forward expands every finished word — one followed by a space — that is the start
/// of exactly one word in the list (every BIP-39 word is unique by its first four letters):
/// "aban " becomes "abandon ". Nothing changes under the person's fingers mid-word, deleting
/// never expands, and a pasted phrase of prefixes expands in one go.
fn autocomplete(old: &str, new: &str) -> String {
    let text: String = new.chars().take(400).collect();
    // typing — at the end, or a word fixed in the middle — completes the finished words and
    // keeps one space between words; deleting changes nothing
    if text.chars().count() <= old.chars().count() {
        return text;
    }
    // where the keystroke landed: the word there is still being typed, unless the keystroke
    // was the space that finishes it
    let at = old
        .char_indices()
        .zip(text.char_indices())
        .find(|((_, a), (_, b))| a != b)
        .map_or(old.len(), |((i, _), _)| i);
    let typed_to = at + (text.len() - old.len());
    let finished = text[..typed_to].ends_with(char::is_whitespace);
    let done = expand_except(&text, false, (!finished).then_some(typed_to));
    let mut tidy = done.split_whitespace().collect::<Vec<_>>().join(" ");
    if done.ends_with(char::is_whitespace) && !tidy.is_empty() {
        tidy.push(' ');
    }
    tidy
}
/// Expands the finished words of `text` (and the last one too when `all`).
fn expand(text: &str, all: bool) -> String {
    expand_except(text, all, None)
}

/// [`expand`], leaving alone the word that ends at or contains byte `keep` (the one being typed).
fn expand_except(text: &str, all: bool, keep: Option<usize>) -> String {
    let mut out = String::with_capacity(text.len() + 16);
    let mut rest = text;
    while let Some(i) = rest.find(|c: char| !c.is_whitespace()) {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let start = text.len() - rest.len();
        let end = rest.find(char::is_whitespace);
        let token = &rest[..end.unwrap_or(rest.len())];
        let typing = keep.is_some_and(|k| (start..=start + token.len()).contains(&k));
        let lower = token.to_lowercase();
        match completions(&lower, 2)[..] {
            [only] if (end.is_some() || all) && !typing && word_index(&lower).is_none() => out.push_str(only),
            _ => out.push_str(token),
        }
        rest = &rest[token.len()..];
    }
    out.push_str(rest);
    out
}
/// The text with its last, unfinished word replaced by `word` and a space.
fn complete_last(text: &str, word: &str) -> String {
    let keep = text.trim_end_matches(|c: char| !c.is_whitespace()).len();
    format!("{}{word} ", &text[..keep])
}

/// What the import screen says about the phrase typed so far.
fn phrase_input(text: &str) -> PhraseInput {
    let lower = text.to_lowercase();
    let tokens: Vec<&str> = lower.split_whitespace().collect();
    let typing = !text.is_empty() && !text.ends_with(char::is_whitespace);
    let words: Vec<WordEntry> = tokens
        .iter()
        .enumerate()
        .map(|(i, w)| {
            let state = if word_index(w).is_some() {
                WordState::Known
            } else if typing && i == tokens.len() - 1 && !completions(w, 1).is_empty() {
                WordState::Typing
            } else {
                WordState::Unknown
            };
            WordEntry { word: w.to_string(), state }
        })
        .collect();
    let current = if typing { tokens.last().copied() } else { None };
    let suggestions: Vec<String> = current
        .map(|w| completions(w, 4))
        .unwrap_or_default()
        .into_iter()
        .filter(|s| Some(*s) != current)
        .map(String::from)
        .collect();
    let mut problem = words.iter().position(|w| w.state == WordState::Unknown).map(|i| {
        if typing && i == words.len() - 1 {
            format!("No word in the list starts with “{}”.", words[i].word)
        } else {
            format!("Word {} is not in the BIP-39 word list.", i + 1)
        }
    });
    let valid = Mnemonic::parse(text).is_ok();
    if problem.is_none() && !valid && !typing && Mnemonic::WORD_COUNTS.contains(&words.len()) {
        problem = Some(
            "These words are in the list, but the phrase is not valid. Check each word and their order."
                .into(),
        );
    }
    PhraseInput { text: text.to_string(), words, suggestions, problem, valid }
}

fn account_row(wallet: &Wallet, network: Network, index: u32) -> AccountRow {
    AccountRow {
        network: network.into(),
        name: network_name(network).into(),
        assets: network_assets(network).into(),
        index,
        address: wallet.address(network, index).unwrap_or_default(),
        path: network.path(index),
    }
}

fn review_view(r: &Review) -> ReviewView {
    let n = r.transfers.len();
    let sign_label = match (r.network, n) {
        (Network::Bitcoin, _) | (_, 1) => "Sign".to_string(),
        _ => format!("Sign {n} transactions"),
    };
    ReviewView {
        network: r.network.into(),
        chain: r.chain.clone(),
        testnet: r.testnet,
        from_index: r.from.index,
        from_address: r.from.address.clone(),
        transfers: r
            .transfers
            .iter()
            .map(|t| TransferView {
                to: t.to.clone(),
                own_index: t.own,
                amount: t.amount.clone(),
                asset: t.asset.clone(),
                token: t.token.clone(),
                verified: t.verified,
                fee: t.fee.clone(),
                note: t.note.clone(),
            })
            .collect(),
        fee: r.fee.clone(),
        details: r.details.iter().map(|(l, v)| Detail { label: l.clone(), value: v.clone() }).collect(),
        warnings: r.warnings.clone(),
        notices: r.notices.clone(),
        sign_label,
    }
}

impl App for Airkeys {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;
    type Effect = Effect;

    fn update(&self, event: Event, model: &mut Model) -> Cmd {
        let cmd = match event {
            Event::Action(a) => self.action(a, model),
            Event::Received(bytes) => match model.phase {
                Phase::Scan { purpose: Purpose::Request, .. } => self.received(model, &bytes),
                _ => Command::done(),
            },
            Event::QrScanned(mut payload) => {
                let cmd = match model.phase {
                    Phase::Scan { purpose: Purpose::Phrase, .. } => self.qr(model, &payload),
                    _ => Command::done(),
                };
                payload.zeroize(); // it may hold a phrase or its entropy
                cmd
            }
            Event::Progress { received, needed } => {
                if let Phase::Scan { progress, .. } = &mut model.phase {
                    *progress = Some((received, needed));
                }
                Command::done()
            }
        };
        model.rev += 1;
        let platform = model.reconcile();
        Command::all([platform, cmd, render()])
    }

    fn view(&self, model: &Model) -> ViewModel {
        let wallet = model.wallet.as_ref();
        let screen = match (&model.phase, wallet) {
            _ if !model.offline() => Screen::Offline {
                radios: RADIOS
                    .iter()
                    .zip(model.radios)
                    .map(|(&(name, on, active), state)| RadioRow {
                        name: name.to_string(),
                        state,
                        detail: match state {
                            RadioState::Off => None,
                            RadioState::On => Some(on.to_string()),
                            RadioState::Active => Some(active.to_string()),
                            RadioState::Unknown => {
                                Some("This phone does not say. Airkeys counts it as on.".into())
                            }
                        },
                    })
                    .collect(),
                wallet_open: wallet.is_some(),
            },
            (Phase::Welcome, _) => Screen::Welcome,
            (Phase::Import { text }, _) => Screen::Import { phrase: phrase_input(text) },
            (Phase::Passphrase { passphrase, problem, .. }, _) => Screen::Passphrase {
                passphrase: passphrase.to_string(),
                problem: problem.clone().or_else(|| check_passphrase(passphrase).err().map(|e| e.0)),
            },
            (Phase::Home, Some(w)) => Screen::Home {
                index: model.index,
                accounts: NETWORK_ORDER.iter().map(|&n| account_row(w, n, model.index)).collect(),
            },
            (Phase::Account { network }, Some(w)) => {
                Screen::Account { account: account_row(w, *network, model.index) }
            }
            (Phase::Connect { selected, all }, Some(_)) => Screen::Connect {
                index: model.index,
                scopes: vec![
                    ScopeChoice {
                        all: false,
                        title: format!("Account #{}", model.index),
                        detail: "The account on the home screen.".into(),
                        selected: !all,
                    },
                    ScopeChoice {
                        all: true,
                        title: format!("All {ACCOUNTS} accounts"),
                        detail: format!(
                            "#0 to #{}, so the web app shows every balance. It, and the services it asks for balances, see that these addresses belong together.",
                            ACCOUNTS - 1
                        ),
                        selected: *all,
                    },
                ],
                choices: NETWORK_ORDER
                    .iter()
                    .zip(selected)
                    .map(|(n, s)| NetworkChoice {
                        network: (*n).into(),
                        name: network_name(*n).into(),
                        selected: *s,
                    })
                    .collect(),
            },
            (Phase::Code { title, caption, json, .. }, _) => Screen::Code {
                title: title.clone(),
                caption: caption.clone(),
                json: json.clone(),
                message: crate::view::lay_out_json(json),
            },
            (Phase::Scan { purpose, progress, problem, .. }, _) => {
                let (title, hint) = match purpose {
                    Purpose::Request => {
                        ("Scan request", "Point the camera at the code on the web app's Send screen.")
                    }
                    Purpose::Phrase => (
                        "Scan recovery phrase",
                        "Point the camera at your SeedQR or CompactSeedQR. The words appear for you to check before anything opens.",
                    ),
                };
                Screen::Scan {
                    title: title.into(),
                    hint: hint.into(),
                    progress: progress.map(|(received, needed)| ScanProgress { received, needed }),
                    problem: problem.clone(),
                }
            }
            (Phase::Review { review }, Some(_)) => Screen::Review { review: review_view(review) },
            (Phase::Refused { title, message }, _) => {
                Screen::Refused { title: title.clone(), message: message.clone() }
            }
            // a wallet screen without a wallet cannot happen; show the start
            (_, None) => Screen::Welcome,
        };
        ViewModel {
            rev: model.rev,
            screen,
            toast: model.toast.clone(),
            version: format!("Airkeys {} · protocol v1", env!("CARGO_PKG_VERSION")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completing_the_last_word() {
        assert_eq!(complete_last("abandon aba", "abandon"), "abandon abandon ");
        assert_eq!(complete_last("zo", "zoo"), "zoo ");
        assert_eq!(complete_last("abandon ", "about"), "abandon about ");
    }

    #[test]
    fn finished_prefixes_expand_while_typing_forward() {
        assert_eq!(autocomplete("", "a"), "a");
        assert_eq!(autocomplete("ab", "aba"), "aba", "nothing changes mid-word");
        assert_eq!(autocomplete("aban", "aban "), "abandon ");
        assert_eq!(autocomplete("Aban", "Aban "), "abandon ");
        assert_eq!(autocomplete("abandon actu", "abandon actu "), "abandon actual ");
        // a word that begins others stays itself; an ambiguous prefix stays as typed
        assert_eq!(autocomplete("act", "act "), "act ");
        assert_eq!(autocomplete("ab", "ab "), "ab ");
        // deleting never expands
        assert_eq!(autocomplete("abandon abou ", "abandon abou"), "abandon abou");
        // a pasted phrase of prefixes expands in one go, except the word still open
        assert_eq!(autocomplete("", "aban abou zoo"), "abandon about zoo");
        assert_eq!(expand("aban abou", true), "abandon about");
    }

    #[test]
    fn phrase_hints() {
        let p = phrase_input("abandon abandon xyz abandon");
        assert_eq!(p.problem.as_deref(), Some("Word 3 is not in the BIP-39 word list."));
        assert_eq!(p.words[2].state, WordState::Unknown);
        let p = phrase_input("abandon ab");
        assert_eq!(p.suggestions, vec!["abandon", "ability", "able", "about"]);
        assert_eq!(p.words[1].state, WordState::Typing);
        assert!(p.problem.is_none());
        let p = phrase_input("abandon qq");
        assert!(p.problem.unwrap().contains("starts with “qq”"));
        let twelve = "abandon ".repeat(12);
        assert!(phrase_input(&twelve).problem.unwrap().contains("not valid"));
        let ok = "abandon ".repeat(11) + "about ";
        let p = phrase_input(&ok);
        assert!(p.valid && p.problem.is_none() && p.suggestions.is_empty() && p.words.len() == 12);
    }
}
