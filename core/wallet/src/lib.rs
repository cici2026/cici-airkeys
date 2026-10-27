//! What an Airkeys wallet computes: recovery phrases, hardened key derivation, addresses, and
//! transfer transactions for Bitcoin and EVM chains — plus the wire protocol that carries
//! them between the offline phone and the online web app
//! ([`docs/PROTOCOL.md`](https://github.com/mondaylabsltd/airkeys/blob/main/docs/PROTOCOL.md)).
//!
//! Pure functions only: no I/O, no clock, no randomness. Cryptographic primitives come from
//! RustCrypto; this crate composes them and checks every composition against published test
//! vectors.

#![forbid(unsafe_code)]

pub mod amount;
pub mod bitcoin;
pub mod evm;
mod hash;
pub mod keys;
pub mod protocol;
pub mod registry;
pub mod review;

pub use keys::{Mnemonic, Wallet};
pub use protocol::Message;
pub use review::{Review, review};

use serde::{Deserialize, Serialize};
use std::fmt;

/// The two network families. Both addresses are hashes of a public key, which stays unknown
/// until the address spends. Every EVM chain shares one address per account.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    Bitcoin,
    Evm,
}

impl Network {
    /// Every network, in the order the phone lists them.
    pub const ALL: [Network; 2] = [Network::Bitcoin, Network::Evm];

    /// The derivation path of account `index`, every step hardened (PROTOCOL §3).
    pub fn path(self, index: u32) -> String {
        match self {
            Network::Bitcoin => format!("m/{PURPOSE}'/84'/0'/0'/0'/{index}'"),
            Network::Evm => format!("m/{PURPOSE}'/44'/60'/0'/0'/{index}'"),
        }
    }
}

impl fmt::Display for Network {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Network::Bitcoin => "Bitcoin",
            Network::Evm => "EVM",
        })
    }
}

/// Why something was refused. The text is shown to the person as it is, so it is a whole
/// sentence in plain English.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error(pub String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

/// A result whose error is a sentence for the person ([`Error`]).
pub type Result<T> = std::result::Result<T, Error>;

/// `Err(Error(format!(…)))`.
macro_rules! bail {
    ($($t:tt)*) => { return Err($crate::Error(format!($($t)*))) };
}
pub(crate) use bail;

/// The first step of every Airkeys path: "AIRK" as ASCII decimal codes (65 73 82 75), in the
/// way BIP-85 spells "SEED" as `83696968'`. No standard wallet derives under it, so no extended
/// public key a wallet ever exported for this phrase is an ancestor of an Airkeys account
/// (PROTOCOL §3, DESIGN "A path of its own").
pub const PURPOSE: u32 = 65_738_275;

/// Accounts are numbered 0 to 2³¹ − 1 (a hardened index).
pub const MAX_INDEX: u32 = 0x7fff_ffff;

/// The longest address text any network has (a bech32 address is at most 90 characters).
/// Longer input is refused before it is parsed: some decoders slow down with length.
pub(crate) const MAX_ADDRESS_LEN: usize = 90;

/// Untrusted text as it may appear in a sentence shown to the person: at most 48 characters
/// (the middle elided), control and direction-changing characters replaced, so no request can
/// fill the screen or make its text read in another order.
pub(crate) fn shown(s: &str) -> String {
    let clean: Vec<char> = s
        .chars()
        .map(|c| match c {
            c if c.is_control() => '?',
            '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}' => '?',
            c => c,
        })
        .collect();
    if clean.len() <= 48 {
        return clean.into_iter().collect();
    }
    let head: String = clean[..30].iter().collect();
    let tail: String = clean[clean.len() - 10..].iter().collect();
    format!("{head}…{tail}")
}
