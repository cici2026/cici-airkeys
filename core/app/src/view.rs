//! What the shells draw. Every string a person reads is decided here, so iOS and Android
//! say the same thing; the shells choose only how it looks.

/// The whole screen. `rev` grows with every change, so a shell can drop a stale view that
/// arrives late from another thread.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct ViewModel {
    pub rev: u64,
    pub screen: Screen,
    /// A one-time message (an error, a confirmation), shown over the screen.
    pub toast: Option<String>,
    /// "Airkeys 0.1.0 · protocol v1"
    pub version: String,
}

#[derive(Clone, Debug, PartialEq, uniffi::Enum)]
pub enum Screen {
    /// A radio is on (or cannot be checked): nothing works until the phone is offline.
    Offline {
        radios: Vec<RadioRow>,
        wallet_open: bool,
    },
    /// No wallet open. Nothing is stored, so this is where every session starts.
    Welcome,
    /// Typing the recovery phrase.
    Import {
        phrase: PhraseInput,
    },
    /// The optional passphrase, then opening the wallet.
    Passphrase {
        passphrase: String,
        problem: Option<String>,
    },
    Home {
        index: u32,
        accounts: Vec<AccountRow>,
    },
    Account {
        account: AccountRow,
    },
    /// Choosing what to show the web app: which networks, and account `index` or all of them.
    Connect {
        index: u32,
        choices: Vec<NetworkChoice>,
        scopes: Vec<ScopeChoice>,
    },
    /// An animated code on screen (Lumen Code). `json` is exactly what it carries.
    Code {
        title: String,
        caption: String,
        json: String,
    },
    /// The camera, reading a request (Lumen Code) or a recovery phrase (QR code).
    Scan {
        title: String,
        hint: String,
        progress: Option<ScanProgress>,
        problem: Option<String>,
    },
    Review {
        review: ReviewView,
    },
    /// A request the phone will not sign, and why.
    Refused {
        title: String,
        message: String,
    },
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct RadioRow {
    /// "Wi-Fi", "Bluetooth", "Mobile data", "Network connection"
    pub name: String,
    pub state: crate::ops::RadioState,
    /// What to do, when the row is not off.
    pub detail: Option<String>,
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct PhraseInput {
    pub text: String,
    /// The words so far, numbered as on paper.
    pub words: Vec<WordEntry>,
    /// Completions for the word being typed.
    pub suggestions: Vec<String>,
    /// What is wrong so far, in a sentence.
    pub problem: Option<String>,
    pub valid: bool,
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct WordEntry {
    pub word: String,
    pub state: WordState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum WordState {
    /// In the word list.
    Known,
    /// Being typed: the start of at least one word.
    Typing,
    /// Not in the word list.
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum NetworkKind {
    Bitcoin,
    Evm,
}

impl From<airkeys_wallet::Network> for NetworkKind {
    fn from(n: airkeys_wallet::Network) -> Self {
        use airkeys_wallet::Network as N;
        match n {
            N::Bitcoin => NetworkKind::Bitcoin,
            N::Evm => NetworkKind::Evm,
        }
    }
}

impl From<NetworkKind> for airkeys_wallet::Network {
    fn from(n: NetworkKind) -> Self {
        use airkeys_wallet::Network as N;
        match n {
            NetworkKind::Bitcoin => N::Bitcoin,
            NetworkKind::Evm => N::Evm,
        }
    }
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct AccountRow {
    pub network: NetworkKind,
    /// "Bitcoin", "Ethereum & EVM", …
    pub name: String,
    /// The coins it holds, e.g. "BTC" or "ETH, ERC-20 tokens".
    pub assets: String,
    pub index: u32,
    pub address: String,
    pub path: String,
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct NetworkChoice {
    pub network: NetworkKind,
    pub name: String,
    pub selected: bool,
}

/// One account, or every account number: `Action::ConnectScope { all }` picks it.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct ScopeChoice {
    pub all: bool,
    pub title: String,
    pub detail: String,
    pub selected: bool,
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct ScanProgress {
    pub received: u32,
    pub needed: u32,
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct ReviewView {
    pub network: NetworkKind,
    pub chain: String,
    /// A test network: the screen says so, its coins have no value.
    pub testnet: bool,
    pub from_index: u32,
    pub from_address: String,
    pub transfers: Vec<TransferView>,
    pub fee: String,
    pub details: Vec<Detail>,
    pub warnings: Vec<String>,
    pub notices: Vec<String>,
    /// "Sign 3 transactions"
    pub sign_label: String,
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct TransferView {
    pub to: String,
    pub own_index: Option<u32>,
    pub amount: String,
    pub asset: String,
    pub token: Option<String>,
    pub verified: bool,
    pub fee: Option<String>,
    pub note: Option<String>,
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct Detail {
    pub label: String,
    pub value: String,
}
