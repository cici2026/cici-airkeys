//! Recovery phrases (BIP-39) and hardened key derivation (BIP-32). Only hardened steps exist
//! here — there is no public derivation and no extended public key to leak (PROTOCOL §3).

use crate::{MAX_INDEX, Network, Result, bail, bitcoin, evm, hash};
use hmac::{Hmac, KeyInit, Mac};
use k256::elliptic_curve::ff::PrimeField;
use sha2::Sha512;
use std::sync::LazyLock;
use zeroize::{Zeroize, Zeroizing};

const HARDENED: u32 = 0x8000_0000;

static WORDS: LazyLock<Vec<&'static str>> =
    LazyLock::new(|| include_str!("keys/english.txt").lines().collect());

/// The BIP-39 English word list, sorted, 2048 words.
pub fn wordlist() -> &'static [&'static str] {
    &WORDS
}

pub fn word_index(word: &str) -> Option<usize> {
    WORDS.binary_search(&word).ok()
}

/// Words that start with `prefix`, in list order (an empty prefix gives none).
pub fn completions(prefix: &str, limit: usize) -> Vec<&'static str> {
    if prefix.is_empty() {
        return Vec::new();
    }
    let start = WORDS.partition_point(|w| *w < prefix);
    WORDS[start..].iter().take_while(|w| w.starts_with(prefix)).take(limit).copied().collect()
}

/// A valid recovery phrase: words from the list, a supported length, a correct checksum.
#[derive(Clone)]
pub struct Mnemonic(Zeroizing<String>);

impl Mnemonic {
    pub const WORD_COUNTS: [usize; 5] = [12, 15, 18, 21, 24];

    /// 16, 20, 24, 28 or 32 bytes of entropy → 12 … 24 words.
    pub fn from_entropy(entropy: &[u8]) -> Result<Mnemonic> {
        if !(16..=32).contains(&entropy.len()) || !entropy.len().is_multiple_of(4) {
            bail!("Entropy must be 16 to 32 bytes, in steps of 4.");
        }
        let checksum = hash::sha256(entropy)[0];
        let mut bits = Zeroizing::new(Vec::with_capacity(entropy.len() * 8 + 8));
        for byte in entropy.iter().chain(std::iter::once(&checksum)) {
            bits.extend((0..8).rev().map(|i| (byte >> i) & 1));
        }
        let count = entropy.len() * 3 / 4;
        let mut phrase = Zeroizing::new(String::new());
        for (n, chunk) in bits.chunks(11).take(count).enumerate() {
            let index = chunk.iter().fold(0usize, |acc, &b| acc << 1 | b as usize);
            if n > 0 {
                phrase.push(' ');
            }
            phrase.push_str(WORDS[index]);
        }
        Ok(Mnemonic(phrase))
    }

    /// Words separated by any white space, any letter case.
    pub fn parse(phrase: &str) -> Result<Mnemonic> {
        let lower = Zeroizing::new(phrase.to_lowercase());
        let words: Vec<&str> = lower.split_whitespace().collect();
        let mut indices = Zeroizing::new(Vec::with_capacity(words.len()));
        for (n, w) in words.iter().enumerate() {
            match word_index(w) {
                Some(i) => indices.push(i),
                None => bail!("Word {} is not in the BIP-39 English word list.", n + 1),
            }
        }
        if !Self::WORD_COUNTS.contains(&words.len()) {
            bail!("A recovery phrase has 12, 15, 18, 21 or 24 words, not {}.", words.len());
        }
        let mut bits = Zeroizing::new(Vec::with_capacity(words.len() * 11));
        for i in indices.iter() {
            bits.extend((0..11).rev().map(|b| ((i >> b) & 1) as u8));
        }
        let ent_bits = words.len() * 32 / 3;
        let mut entropy = Zeroizing::new(vec![0u8; ent_bits / 8]);
        for (n, bit) in bits[..ent_bits].iter().enumerate() {
            entropy[n / 8] |= bit << (7 - n % 8);
        }
        let cs_bits = ent_bits / 32;
        let expected = hash::sha256(&entropy)[0] >> (8 - cs_bits);
        let got = bits[ent_bits..].iter().fold(0u8, |acc, &b| acc << 1 | b);
        if expected != got {
            bail!(
                "These words are in the list, but the phrase is not valid. Check each word and their order."
            );
        }
        Ok(Mnemonic(Zeroizing::new(words.join(" "))))
    }

    /// The phrase in a scanned QR code, in any of the forms wallets print:
    ///
    /// - SeedQR: each word as its four-digit index in the word list (numeric QR);
    /// - CompactSeedQR: the entropy itself, 16 to 32 bytes (binary QR);
    /// - the words as text.
    pub fn from_qr(payload: &[u8]) -> Result<Mnemonic> {
        let digits = !payload.is_empty() && payload.iter().all(u8::is_ascii_digit);
        if digits && payload.len().is_multiple_of(4) && Self::WORD_COUNTS.contains(&(payload.len() / 4)) {
            let mut words = Zeroizing::new(String::new());
            for (n, d) in payload.chunks(4).enumerate() {
                let i: usize = std::str::from_utf8(d).unwrap().parse().unwrap();
                let Some(w) = WORDS.get(i) else {
                    bail!("This SeedQR names word number {i}, but the list has 2048 words.")
                };
                if n > 0 {
                    words.push(' ');
                }
                words.push_str(w);
            }
            return Mnemonic::parse(&words);
        }
        if let Ok(text) = std::str::from_utf8(payload)
            && text.trim().contains(' ')
        {
            return Mnemonic::parse(text);
        }
        if (16..=32).contains(&payload.len()) && payload.len().is_multiple_of(4) && !digits {
            return Mnemonic::from_entropy(payload);
        }
        bail!("This QR code does not hold a recovery phrase (SeedQR, CompactSeedQR or the words as text).")
    }

    pub fn phrase(&self) -> &str {
        &self.0
    }

    pub fn words(&self) -> Vec<&str> {
        self.0.split(' ').collect()
    }

    /// The 64-byte seed (PBKDF2-HMAC-SHA512, 2048 rounds, salt "mnemonic" + passphrase).
    ///
    /// The passphrase is limited to printable ASCII. BIP-39 normalises it to NFKD; with ASCII
    /// that is the identity, so every other wallet derives the same seed from it, and no
    /// keyboard can slip in a look-alike character that cannot be typed again.
    pub fn seed(&self, passphrase: &str) -> Result<Seed> {
        check_passphrase(passphrase)?;
        let salt = Zeroizing::new(format!("mnemonic{passphrase}"));
        let mut seed = Zeroizing::new([0u8; 64]);
        pbkdf2::pbkdf2_hmac::<Sha512>(self.0.as_bytes(), salt.as_bytes(), 2048, &mut seed[..]);
        Ok(Seed(seed))
    }
}

/// Never prints the words.
impl std::fmt::Debug for Mnemonic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Mnemonic({} words)", self.words().len())
    }
}

pub fn check_passphrase(passphrase: &str) -> Result<()> {
    if passphrase.len() > 256 {
        bail!("The passphrase is longer than 256 characters.");
    }
    if !passphrase.bytes().all(|b| (0x20..0x7f).contains(&b)) {
        bail!("Use only letters, digits, spaces and symbols from an English keyboard in the passphrase.");
    }
    Ok(())
}

pub struct Seed(Zeroizing<[u8; 64]>);

impl Seed {
    pub fn from_bytes(bytes: [u8; 64]) -> Seed {
        Seed(Zeroizing::new(bytes))
    }

    pub fn as_bytes(&self) -> &[u8; 64] {
        &self.0
    }
}

/// A private key and its chain code.
struct Node {
    key: [u8; 32],
    chain: [u8; 32],
}

impl Drop for Node {
    fn drop(&mut self) {
        self.key.zeroize();
        self.chain.zeroize();
    }
}

fn hmac512(key: &[u8], parts: &[&[u8]]) -> Zeroizing<[u8; 64]> {
    let mut mac = Hmac::<Sha512>::new_from_slice(key).expect("HMAC takes any key length");
    for p in parts {
        mac.update(p);
    }
    Zeroizing::new(mac.finalize().into_bytes().into())
}

fn split(i: &[u8; 64]) -> Node {
    Node { key: i[..32].try_into().unwrap(), chain: i[32..].try_into().unwrap() }
}

/// The BIP-32 master node of a seed.
fn master(seed: &Seed) -> Result<Node> {
    let node = split(&hmac512(b"Bitcoin seed", &[&seed.0[..]]));
    secp_scalar(&node.key)?;
    Ok(node)
}

/// The hardened child `index'`.
fn child(parent: &Node, index: u32) -> Result<Node> {
    if index > MAX_INDEX {
        bail!("Account numbers go up to {MAX_INDEX}.");
    }
    let i = hmac512(&parent.chain, &[&[0], &parent.key, &(index | HARDENED).to_be_bytes()]);
    let mut node = split(&i);
    // k = IL + k_parent (mod n); IL ≥ n or k = 0 makes the index unusable (probability below
    // 2⁻¹²⁷ — reported, never silently skipped)
    let k = secp_scalar(&node.key)? + secp_scalar(&parent.key)?;
    if bool::from(k.is_zero()) {
        bail!("This account cannot be derived (BIP-32 invalid child). Use another account number.");
    }
    node.key = k.to_bytes().into();
    Ok(node)
}

fn derive(seed: &Seed, path: &[u32]) -> Result<Node> {
    let mut node = master(seed)?;
    for &i in path {
        node = child(&node, i)?;
    }
    Ok(node)
}

fn secp_scalar(bytes: &[u8; 32]) -> Result<k256::Scalar> {
    let s: Option<k256::Scalar> = k256::Scalar::from_repr((*bytes).into()).into();
    match s {
        Some(s) if !bool::from(s.is_zero()) => Ok(s),
        _ => bail!("This account cannot be derived (BIP-32 invalid key). Use another account number."),
    }
}

/// The node just above the account index, per network (PROTOCOL §3).
fn parent_path(network: Network) -> &'static [u32] {
    match network {
        Network::Bitcoin => &[84, 0, 0, 0],
        Network::Evm => &[44, 60, 0, 0],
    }
}

/// An unlocked wallet: the nodes above the account index of each network. The seed itself
/// is not kept.
pub struct Wallet {
    parents: [Node; 2],
}

impl Wallet {
    pub fn new(seed: &Seed) -> Result<Wallet> {
        let [a, b] = Network::ALL;
        Ok(Wallet { parents: [derive(seed, parent_path(a))?, derive(seed, parent_path(b))?] })
    }

    /// The private key of account `index` on `network`.
    pub(crate) fn key(&self, network: Network, index: u32) -> Result<k256::ecdsa::SigningKey> {
        let parent = &self.parents[Network::ALL.iter().position(|&n| n == network).unwrap()];
        let node = child(parent, index)?;
        Ok(k256::ecdsa::SigningKey::from_bytes(&node.key.into()).expect("checked by BIP-32 derivation"))
    }

    /// The address of account `index` on `network`, in the network's text form.
    pub fn address(&self, network: Network, index: u32) -> Result<String> {
        let key = self.key(network, index)?;
        Ok(match network {
            Network::Bitcoin => bitcoin::address(&key),
            Network::Evm => evm::checksum(&evm::address(&key)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::{hex, unhex};

    /// The official BIP-39 vectors (trezor/python-mnemonic), passphrase "TREZOR".
    const BIP39: &[(&str, &str, &str)] = &[
        (
            "00000000000000000000000000000000",
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
            "c55257c360c07c72029aebc1b53c05ed0362ada38ead3e3e9efa3708e53495531f09a6987599d18264c1e1c92f2cf141630c7a3c4ab7c81b2f001698e7463b04",
        ),
        (
            "7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f",
            "legal winner thank year wave sausage worth useful legal winner thank yellow",
            "2e8905819b8723fe2c1d161860e5ee1830318dbf49a83bd451cfb8440c28bd6fa457fe1296106559a3c80937a1c1069be3a3a5bd381ee6260e8d9739fce1f607",
        ),
        (
            "80808080808080808080808080808080",
            "letter advice cage absurd amount doctor acoustic avoid letter advice cage above",
            "d71de856f81a8acc65e6fc851a38d4d7ec216fd0796d0a6827a3ad6ed5511a30fa280f12eb2e47ed2ac03b5c462a0358d18d69fe4f985ec81778c1b370b652a8",
        ),
        (
            "ffffffffffffffffffffffffffffffff",
            "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong",
            "ac27495480225222079d7be181583751e86f571027b0497b5b5d11218e0a8a13332572917f0f8e5a589620c6f15b11c61dee327651a14c34e18231052e48c069",
        ),
        (
            "000000000000000000000000000000000000000000000000",
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon agent",
            "035895f2f481b1b0f01fcf8c289c794660b289981a78f8106447707fdd9666ca06da5a9a565181599b79f53b844d8a71dd9f439c52a3d7b3e8a79c906ac845fa",
        ),
        (
            "0000000000000000000000000000000000000000000000000000000000000000",
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art",
            "bda85446c68413707090a52022edd26a1c9462295029f2e60cd7c4f2bbd3097170af7a4d73245cafa9c3cca8d561a7c3de6f5d4a10be8ed2a5e608d68f92fcc8",
        ),
        (
            "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo vote",
            "dd48c104698c30cfe2b6142103248622fb7bb0ff692eebb00089b32d22484e1613912f0a5b694407be899ffd31ed3992c456cdf60f5d4564b8ba3f05a69890ad",
        ),
        (
            "9e885d952ad362caeb4efe34a8e91bd2",
            "ozone drill grab fiber curtain grace pudding thank cruise elder eight picnic",
            "274ddc525802f7c828d8ef7ddbcdc5304e87ac3535913611fbbfa986d0c9e5476c91689f9c8a54fd55bd38606aa6a8595ad213d4c9c9f9aca3fb217069a41028",
        ),
        (
            "f585c11aec520db57dd353c69554b21a89b20fb0650966fa0a9d6f74fd989d8f",
            "void come effort suffer camp survey warrior heavy shoot primary clutch crush open amazing screen patrol group space point ten exist slush involve unfold",
            "01f5bced59dec48e362f2c45b5de68b9fd6c92c6634f44d6d40aab69056506f0e35524a518034ddc1192e1dacd32c1ed3eaa3c3b131c88ed8e7e54c49a5d0998",
        ),
    ];

    #[test]
    fn bip39_vectors() {
        for (entropy, phrase, seed) in BIP39 {
            let m = Mnemonic::from_entropy(&unhex(entropy).unwrap()).unwrap();
            assert_eq!(m.phrase(), *phrase);
            let parsed = Mnemonic::parse(&phrase.to_uppercase().replace(' ', "  \n")).unwrap();
            assert_eq!(parsed.phrase(), *phrase);
            assert_eq!(hex(parsed.seed("TREZOR").unwrap().as_bytes()), *seed);
        }
    }

    #[test]
    fn rejects_bad_phrases() {
        let e = |p: &str| Mnemonic::parse(p).err().unwrap().0;
        assert!(e("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon").contains("not valid"));
        assert!(
            e("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abou")
                .contains("Word 12")
        );
        assert!(e("abandon abandon abandon").contains("not 3"));
        assert!(Mnemonic::parse("").is_err());
        assert!(check_passphrase("correct horse ~!@#").is_ok());
        assert!(check_passphrase("pässword").is_err());
        assert!(check_passphrase("smart’quote").is_err());
    }

    /// SeedQR forms, as SeedSigner prints them.
    #[test]
    fn qr_forms() {
        let about =
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let seedqr = format!("{}0003", "0000".repeat(11));
        assert_eq!(Mnemonic::from_qr(seedqr.as_bytes()).unwrap().phrase(), about);
        assert_eq!(Mnemonic::from_qr(&[0u8; 16]).unwrap().phrase(), about);
        assert_eq!(Mnemonic::from_qr(about.to_uppercase().as_bytes()).unwrap().phrase(), about);
        let zoo = Mnemonic::from_qr(&[0xff; 32]).unwrap();
        assert_eq!(zoo.words().len(), 24);
        // the index of every word round-trips through the numeric form
        let digits: String = zoo.words().iter().map(|w| format!("{:04}", word_index(w).unwrap())).collect();
        assert_eq!(Mnemonic::from_qr(digits.as_bytes()).unwrap().phrase(), zoo.phrase());
        assert!(
            Mnemonic::from_qr(format!("{}2048", "0000".repeat(11)).as_bytes())
                .unwrap_err()
                .0
                .contains("2048")
        );
        assert!(
            Mnemonic::from_qr(format!("{}0000", "0000".repeat(11)).as_bytes())
                .unwrap_err()
                .0
                .contains("not valid")
        );
        assert!(Mnemonic::from_qr(b"https://example.com").unwrap_err().0.contains("does not hold"));
        assert!(Mnemonic::from_qr(&[1u8; 15]).is_err());
    }

    #[test]
    fn word_completion() {
        assert_eq!(completions("aba", 5), vec!["abandon"]);
        assert_eq!(completions("zo", 5), vec!["zone", "zoo"]);
        assert!(completions("", 5).is_empty());
        assert!(completions("qz", 5).is_empty());
        assert_eq!(wordlist().len(), 2048);
        assert!(wordlist().windows(2).all(|w| w[0] < w[1]));
    }

    /// BIP-32 test vectors 1 and 3, the hardened steps (m/0H).
    #[test]
    fn bip32_hardened_vectors() {
        let seed = |h: &str| {
            let mut b = [0u8; 64];
            let v = unhex(h).unwrap();
            // a shorter seed is fine for HMAC, but Seed is 64 bytes: use the node directly
            b[..v.len()].copy_from_slice(&v);
            (b, v.len())
        };
        for (s, chain, key) in [
            (
                "000102030405060708090a0b0c0d0e0f",
                "47fdacbd0f1097043b78c63c20c34ef4ed9a111d980047ad16282c7ae6236141",
                "edb2e14f9ee77d26dd93b4ecede8d16ed408ce149b6cd80b0715a2d911a0afea",
            ),
            (
                "4b381541583be4423346c643850da4b320e46a87ae3d2a4e6da11eba819cd4acba45d239319ac14f863b8d5ab5a0d0c64d2e8a1e7d1457df2e5a3c51c73235be",
                "e5fea12a97b927fc9dc3d2cb0d1ea1cf50aa5a1fdc1f933e8906bb38df3377bd",
                "491f7a2eebc7b57028e0d3faa0acda02e75c33b03c48fb288c41e2ea44e1daef",
            ),
        ] {
            let (bytes, len) = seed(s);
            let master = split(&hmac512(b"Bitcoin seed", &[&bytes[..len]]));
            let child = child(&master, 0).unwrap();
            assert_eq!(hex(&child.chain), chain);
            assert_eq!(hex(&child.key), key);
        }
    }

    #[test]
    fn index_range() {
        let m = Mnemonic::parse(BIP39[0].1).unwrap();
        let w = Wallet::new(&m.seed("").unwrap()).unwrap();
        assert!(w.address(Network::Evm, MAX_INDEX).is_ok());
        assert!(w.address(Network::Evm, MAX_INDEX + 1).is_err());
    }
}
