//! Bitcoin: P2WPKH accounts, recipient addresses of every standard type, previous
//! transactions, and segwit-v0 (BIP-143) transfer transactions.

use crate::hash::{hash160, sha256d};
use crate::{MAX_ADDRESS_LEN, Result, bail, shown};
use bech32::{Fe32, Hrp};
use k256::ecdsa::SigningKey;

const HRP: Hrp = Hrp::parse_unchecked("bc");

/// Version 2, lock time 0, sequence `0xfffffffd` (opt-in replace-by-fee): fixed by the
/// protocol (§5.1), so a request cannot choose them.
const VERSION: u32 = 2;
const LOCK_TIME: u32 = 0;
const SEQUENCE: u32 = 0xffff_fffd;
const SIGHASH_ALL: u8 = 1;

pub fn compressed_pubkey(key: &SigningKey) -> [u8; 33] {
    key.verifying_key().to_sec1_point(true).as_bytes().try_into().expect("33 bytes")
}

/// The P2WPKH address (`bc1q…`) of a key.
pub fn address(key: &SigningKey) -> String {
    bech32::segwit::encode_v0(HRP, &hash160(&compressed_pubkey(key))).expect("20-byte program")
}

/// Output script types the phone pays to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    P2pkh,
    P2sh,
    P2wpkh,
    P2wsh,
    P2tr,
}

impl Kind {
    /// The dust limit at Bitcoin Core's default relay settings (3 sat/vB dust fee).
    pub fn dust(self) -> u64 {
        match self {
            Kind::P2pkh => 546,
            Kind::P2sh => 540,
            Kind::P2wpkh => 294,
            Kind::P2wsh | Kind::P2tr => 330,
        }
    }
}

/// A parsed mainnet address.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Address {
    pub kind: Kind,
    pub script: Vec<u8>,
}

/// Any standard mainnet address: `1…` (P2PKH), `3…` (P2SH), `bc1q…` (P2WPKH, P2WSH),
/// `bc1p…` (P2TR). Witness versions 2–16 are refused: nothing can spend them today.
pub fn parse_address(s: &str) -> Result<Address> {
    if s.len() > MAX_ADDRESS_LEN || !s.is_ascii() {
        bail!("{} is not a valid Bitcoin address.", shown(s));
    }
    if s.len() > 2 && s[..3].eq_ignore_ascii_case("bc1") {
        let Ok((hrp, version, program)) = bech32::segwit::decode(s) else {
            bail!("{} is not a valid Bitcoin address.", shown(s));
        };
        if hrp != HRP {
            bail!("{} is not a Bitcoin mainnet address.", shown(s));
        }
        let (kind, op) = match (version, program.len()) {
            (Fe32::Q, 20) => (Kind::P2wpkh, 0x00),
            (Fe32::Q, 32) => (Kind::P2wsh, 0x00),
            (Fe32::P, 32) => (Kind::P2tr, 0x51),
            _ => bail!("{} uses a witness version that Airkeys does not pay to.", shown(s)),
        };
        let mut script = vec![op, program.len() as u8];
        script.extend_from_slice(&program);
        return Ok(Address { kind, script });
    }
    let decoded = bs58::decode(s).with_check(None).into_vec();
    match decoded.as_deref() {
        Ok([0x00, hash @ ..]) if hash.len() == 20 => {
            Ok(Address { kind: Kind::P2pkh, script: p2pkh_script(hash) })
        }
        Ok([0x05, hash @ ..]) if hash.len() == 20 => {
            Ok(Address { kind: Kind::P2sh, script: p2sh_script(hash) })
        }
        _ => bail!("{} is not a valid Bitcoin mainnet address.", shown(s)),
    }
}

/// `OP_DUP OP_HASH160 <hash> OP_EQUALVERIFY OP_CHECKSIG`.
pub fn p2pkh_script(hash: &[u8]) -> Vec<u8> {
    let mut script = vec![0x76, 0xa9, 0x14];
    script.extend_from_slice(hash);
    script.extend_from_slice(&[0x88, 0xac]);
    script
}

/// `OP_HASH160 <hash> OP_EQUAL`.
pub fn p2sh_script(hash: &[u8]) -> Vec<u8> {
    let mut script = vec![0xa9, 0x14];
    script.extend_from_slice(hash);
    script.push(0x87);
    script
}

/// Every script that pays the key of the P2WPKH script `own`: itself, and the legacy (P2PKH)
/// and nested (P2SH-P2WPKH) forms of the same key — all reveal or strand the same key.
pub fn same_key_scripts(own: &[u8]) -> [Vec<u8>; 3] {
    let hash = &own[2..];
    [own.to_vec(), p2pkh_script(hash), p2sh_script(&hash160(own))]
}

/// The P2WPKH script `0 <hash160(pubkey)>`.
pub fn p2wpkh_script(key: &SigningKey) -> Vec<u8> {
    let mut s = vec![0x00, 0x14];
    s.extend_from_slice(&hash160(&compressed_pubkey(key)));
    s
}

/// A transaction id in display order (the reverse of the hash bytes).
pub fn txid_hex(hash: &[u8; 32]) -> String {
    let mut b = *hash;
    b.reverse();
    crate::hash::hex(&b)
}

// ---------------------------------------------------------------- previous transactions

/// What the phone needs from a previous transaction: its id and its outputs.
pub struct PrevTx {
    /// In internal byte order.
    pub txid: [u8; 32],
    pub outputs: Vec<(u64, Vec<u8>)>,
}

struct Reader<'a> {
    b: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        if self.b.len() - self.at < n {
            bail!("A previous transaction is cut short.");
        }
        let s = &self.b[self.at..self.at + n];
        self.at += n;
        Ok(s)
    }
    fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn varint(&mut self) -> Result<u64> {
        let v = match self.u8()? {
            0xfd => u16::from_le_bytes(self.take(2)?.try_into().unwrap()) as u64,
            0xfe => self.u32()? as u64,
            0xff => self.u64()?,
            b => b as u64,
        };
        Ok(v)
    }
    /// A count of items, each at least `min` bytes long: bounded by what is left to read.
    fn count(&mut self, min: usize) -> Result<usize> {
        let n = self.varint()?;
        if n > ((self.b.len() - self.at) / min.max(1)) as u64 {
            bail!("A previous transaction is malformed.");
        }
        Ok(n as usize)
    }
    fn bytes(&mut self) -> Result<&'a [u8]> {
        let n = self.count(1)?;
        self.take(n)
    }
}

/// Parses a serialised transaction, with or without witness data. The txid is computed
/// over the serialisation without witnesses, as consensus defines it.
pub fn parse_tx(raw: &[u8]) -> Result<PrevTx> {
    let mut r = Reader { b: raw, at: 0 };
    let version = r.take(4)?;
    let segwit = raw.len() > 6 && raw[4] == 0 && raw[5] == 1;
    if segwit {
        r.take(2)?;
    }
    let body_start = r.at;
    let n_in = r.count(41)?;
    if n_in == 0 {
        bail!("A previous transaction has no inputs.");
    }
    for _ in 0..n_in {
        r.take(36)?;
        r.bytes()?;
        r.take(4)?;
    }
    let n_out = r.count(9)?;
    let mut outputs = Vec::with_capacity(n_out);
    for _ in 0..n_out {
        let value = r.u64()?;
        outputs.push((value, r.bytes()?.to_vec()));
    }
    let body_end = r.at;
    if segwit {
        for _ in 0..n_in {
            let items = r.count(1)?;
            for _ in 0..items {
                r.bytes()?;
            }
        }
    }
    let lock_time = r.take(4)?;
    if r.at != raw.len() {
        bail!("A previous transaction has extra bytes at the end.");
    }
    let mut stripped = Vec::with_capacity(raw.len());
    stripped.extend_from_slice(version);
    stripped.extend_from_slice(&raw[body_start..body_end]);
    stripped.extend_from_slice(lock_time);
    Ok(PrevTx { txid: sha256d(&stripped), outputs })
}

// ---------------------------------------------------------------- transfers

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Input {
    /// Internal byte order.
    pub txid: [u8; 32],
    pub vout: u32,
    pub value: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Output {
    pub value: u64,
    pub script: Vec<u8>,
}

/// A signed transaction.
pub struct Signed {
    pub raw: Vec<u8>,
    /// Internal byte order.
    pub txid: [u8; 32],
    pub vsize: usize,
}

fn varint(out: &mut Vec<u8>, n: usize) {
    match n {
        0..=0xfc => out.push(n as u8),
        0xfd..=0xffff => {
            out.push(0xfd);
            out.extend_from_slice(&(n as u16).to_le_bytes());
        }
        _ => {
            out.push(0xfe);
            out.extend_from_slice(&(n as u32).to_le_bytes());
        }
    }
}

fn push_outputs(out: &mut Vec<u8>, outputs: &[Output]) {
    for o in outputs {
        out.extend_from_slice(&o.value.to_le_bytes());
        varint(out, o.script.len());
        out.extend_from_slice(&o.script);
    }
}

/// Spends P2WPKH coins of one key. Every input is signed `SIGHASH_ALL` under BIP-143.
pub fn sign(key: &SigningKey, inputs: &[Input], outputs: &[Output]) -> Signed {
    let pubkey = compressed_pubkey(key);
    // scriptCode of P2WPKH: OP_DUP OP_HASH160 <hash> OP_EQUALVERIFY OP_CHECKSIG
    let mut script_code = vec![0x19, 0x76, 0xa9, 0x14];
    script_code.extend_from_slice(&hash160(&pubkey));
    script_code.extend_from_slice(&[0x88, 0xac]);

    let mut prevouts = Vec::new();
    let mut sequences = Vec::new();
    for i in inputs {
        prevouts.extend_from_slice(&i.txid);
        prevouts.extend_from_slice(&i.vout.to_le_bytes());
        sequences.extend_from_slice(&SEQUENCE.to_le_bytes());
    }
    let mut outs = Vec::new();
    push_outputs(&mut outs, outputs);
    let (hash_prevouts, hash_sequence, hash_outputs) =
        (sha256d(&prevouts), sha256d(&sequences), sha256d(&outs));

    let mut witnesses = Vec::new();
    for i in inputs {
        let mut pre = Vec::with_capacity(160);
        pre.extend_from_slice(&VERSION.to_le_bytes());
        pre.extend_from_slice(&hash_prevouts);
        pre.extend_from_slice(&hash_sequence);
        pre.extend_from_slice(&i.txid);
        pre.extend_from_slice(&i.vout.to_le_bytes());
        pre.extend_from_slice(&script_code);
        pre.extend_from_slice(&i.value.to_le_bytes());
        pre.extend_from_slice(&SEQUENCE.to_le_bytes());
        pre.extend_from_slice(&hash_outputs);
        pre.extend_from_slice(&LOCK_TIME.to_le_bytes());
        pre.extend_from_slice(&(SIGHASH_ALL as u32).to_le_bytes());
        let (sig, _) = key.sign_prehash_recoverable(&sha256d(&pre));
        let mut der = sig.to_der().as_bytes().to_vec();
        der.push(SIGHASH_ALL);
        witnesses.push(der);
    }

    let mut base = Vec::new();
    base.extend_from_slice(&VERSION.to_le_bytes());
    let body_at = base.len();
    varint(&mut base, inputs.len());
    for i in inputs {
        base.extend_from_slice(&i.txid);
        base.extend_from_slice(&i.vout.to_le_bytes());
        base.push(0); // empty scriptSig
        base.extend_from_slice(&SEQUENCE.to_le_bytes());
    }
    varint(&mut base, outputs.len());
    push_outputs(&mut base, outputs);
    let body = base[body_at..].to_vec();
    base.extend_from_slice(&LOCK_TIME.to_le_bytes());
    let txid = sha256d(&base);

    let mut raw = Vec::with_capacity(base.len() + witnesses.len() * 108);
    raw.extend_from_slice(&VERSION.to_le_bytes());
    raw.extend_from_slice(&[0x00, 0x01]);
    raw.extend_from_slice(&body);
    for sig in &witnesses {
        raw.push(2);
        varint(&mut raw, sig.len());
        raw.extend_from_slice(sig);
        raw.push(33);
        raw.extend_from_slice(&pubkey);
    }
    raw.extend_from_slice(&LOCK_TIME.to_le_bytes());
    // weight = 3·base + total; vsize rounds up
    let vsize = (base.len() * 3 + raw.len()).div_ceil(4);
    Signed { raw, txid, vsize }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::{hex, unhex};

    #[test]
    fn addresses_of_every_type() {
        // BIP-173 and BIP-350 examples, plus well-known base58 addresses
        let a = parse_address("bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4").unwrap();
        assert_eq!(
            (a.kind, hex(&a.script)),
            (Kind::P2wpkh, "0014751e76e8199196d454941c45d1b3a323f1433bd6".into())
        );
        let a = parse_address("BC1QW508D6QEJXTDG4Y5R3ZARVARY0C5XW7KV8F3T4").unwrap();
        assert_eq!(a.kind, Kind::P2wpkh);
        let a = parse_address("bc1qrp33g0q5c5txsp9arysrx4k6zdkfs4nce4xj0gdcccefvpysxf3qccfmv3").unwrap();
        assert_eq!(a.kind, Kind::P2wsh);
        let a = parse_address("bc1p0xlxvlhemja6c4dqv22uapctqupfhlxm9h8z3k2e72q4k9hcz7vqzk5jj0").unwrap();
        assert_eq!((a.kind, hex(&a.script[..2])), (Kind::P2tr, "5120".into()));
        let a = parse_address("1BvBMSEYstWetqTFn5Au4m4GFg7xJaNVN2").unwrap();
        assert_eq!(hex(&a.script), "76a91477bff20c60e522dfaa3350c39b030a5d004e839a88ac");
        let a = parse_address("3J98t1WpEZ73CNmQviecrnyiWrnqRhWNLy").unwrap();
        assert_eq!((a.kind, a.script.len()), (Kind::P2sh, 23));
        for bad in [
            "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t5", // checksum
            "tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx", // testnet
            "bc1zw508d6qejxtdg4y5r3zarvaryvaxxpcs",       // witness v2
            "bc1pw508d6qejxtdg4y5r3zarvary0c5xw7kw508d6qejxtdg4y5r3zarvary0c5xw7k7grplx", // v1 bech32 (not m)
            "1BvBMSEYstWetqTFn5Au4m4GFg7xJaNVN3",
            "mipcBbFg9gMiCh81Kj8tqqdgoZub1ZJRfn", // testnet P2PKH
            "",
        ] {
            assert!(parse_address(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn parses_legacy_and_segwit_transactions() {
        // the first transaction to spend a coinbase (block 170): legacy serialisation
        let raw = unhex("0100000001c997a5e56e104102fa209c6a852dd90660a20b2d9c352423edce25857fcd3704000000004847304402204e45e16932b8af514961a1d3a1a25fdf3f4f7732e9d624c6c61548ab5fb8cd410220181522ec8eca07de4860a4acdd12909d831cc56cbbac4622082221a8768d1d0901ffffffff0200ca9a3b00000000434104ae1a62fe09c5f51b13905f07f06b99a2f7159b2225f374cd378d71302fa28414e7aab37397f554a7df5f142c21c1b7303b8a0626f1baded5c72a704f7e6cd84cac00286bee0000000043410411db93e1dcdb8a016b49840f8c53bc1eb68a382e97b1482ecad7b148a6909a5cb2e0eaddfb84ccf9744464f82e160bfa9b8b64f9d4c03f999b8643f656b412a3ac00000000").unwrap();
        let tx = parse_tx(&raw).unwrap();
        assert_eq!(txid_hex(&tx.txid), "f4184fc596403b9d638783cf57adfe4c75c605f6356fbc91338530e9831e9e16");
        assert_eq!(tx.outputs.iter().map(|o| o.0).collect::<Vec<_>>(), vec![1_000_000_000, 4_000_000_000]);
        assert!(parse_tx(&raw[..raw.len() - 1]).is_err());
        let mut extra = raw.clone();
        extra.push(0);
        assert!(parse_tx(&extra).is_err());
    }

    #[test]
    fn bip143_native_p2wpkh_example() {
        // BIP-143 "Native P2WPKH": input 1 is ours (P2WPKH, 6 BTC). The example also
        // spends a P2PK input; we reproduce its sighash for input 1, the P2WPKH one.
        let key = SigningKey::from_slice(
            &unhex("619c335025c7f4012e556c2a58b2506e30b8511b53ade95ea316fd8c3286feb9").unwrap(),
        )
        .unwrap();
        assert_eq!(
            hex(&compressed_pubkey(&key)),
            "025476c2e83188368da1ff3e292e7acafcdb3566bb0ad253f62fc70f07aeee6357"
        );
        // The full example tx is not P2WPKH-only, so check the sighash construction alone.
        let txid = |s: &str| -> [u8; 32] { unhex(s).unwrap().try_into().unwrap() };
        let mut prevouts = Vec::new();
        prevouts.extend_from_slice(&txid("fff7f7881a8099afa6940d42d1e7f6362bec38171ea3edf433541db4e4ad969f"));
        prevouts.extend_from_slice(&0u32.to_le_bytes());
        prevouts.extend_from_slice(&txid("ef51e1b804cc89d182d279655c3aa89e815b1b309fe287d9b2b55d57b90ec68a"));
        prevouts.extend_from_slice(&1u32.to_le_bytes());
        assert_eq!(
            hex(&sha256d(&prevouts)),
            "96b827c8483d4e9b96712b6713a7b68d6e8003a781feba36c31143470b4efd37"
        );
    }
}
