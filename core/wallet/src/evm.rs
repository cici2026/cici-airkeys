//! EVM chains: addresses with EIP-55 checksums, and transfer transactions — native coin or
//! ERC-20 `transfer` — as EIP-1559 (type 2) or legacy EIP-155 transactions.

use crate::amount::Amount;
use crate::hash::{hex, keccak256, unhex};
use crate::{MAX_ADDRESS_LEN, Result, bail, shown};
use k256::ecdsa::SigningKey;

/// A 20-byte EVM address.
pub type Address = [u8; 20];

/// Keccak-256 of the uncompressed public key without its 0x04 prefix, last 20 bytes.
pub fn address(key: &SigningKey) -> Address {
    let point = key.verifying_key().to_sec1_point(false);
    keccak256(&point.as_bytes()[1..])[12..].try_into().unwrap()
}

/// EIP-55: `0x` and hex, a letter upper case when its nibble of the address hash is ≥ 8.
pub fn checksum(addr: &Address) -> String {
    let lower = hex(addr);
    let h = keccak256(lower.as_bytes());
    let mut s = String::from("0x");
    for (i, c) in lower.chars().enumerate() {
        let nibble = (h[i / 2] >> (if i % 2 == 0 { 4 } else { 0 })) & 15;
        s.push(if nibble >= 8 { c.to_ascii_uppercase() } else { c });
    }
    s
}

/// `0x` + 40 hex digits. All lower case or all upper case carries no checksum; mixed case
/// must be the correct EIP-55 checksum.
pub fn parse_address(s: &str) -> Result<Address> {
    if s.len() > MAX_ADDRESS_LEN {
        bail!(
            "{} is not an EVM address (0x and 40 hex digits).",
            "{} 不是 EVM 地址（0x 加 40 位十六进制数字）。",
            shown(s)
        );
    }
    let Some(body) = s.strip_prefix("0x").filter(|b| b.len() == 40) else {
        bail!(
            "{} is not an EVM address (0x and 40 hex digits).",
            "{} 不是 EVM 地址（0x 加 40 位十六进制数字）。",
            shown(s)
        );
    };
    let Some(bytes) = unhex(&body.to_ascii_lowercase()) else {
        bail!(
            "{} is not an EVM address (0x and 40 hex digits).",
            "{} 不是 EVM 地址（0x 加 40 位十六进制数字）。",
            shown(s)
        );
    };
    let addr: Address = bytes.try_into().unwrap();
    let mixed = body.chars().any(|c| c.is_ascii_lowercase()) && body.chars().any(|c| c.is_ascii_uppercase());
    if mixed && checksum(&addr) != s {
        bail!(
            "{} has a wrong checksum: a character is mistyped.",
            "{} 的校验和不对：有字符输错了。",
            shown(s)
        );
    }
    Ok(addr)
}

// ---------------------------------------------------------------- RLP

fn rlp_length(out: &mut Vec<u8>, len: usize, short: u8) {
    if len < 56 {
        out.push(short + len as u8);
    } else {
        let be = (len as u64).to_be_bytes();
        let first = be.iter().position(|&b| b != 0).unwrap();
        out.push(short + 55 + (8 - first) as u8);
        out.extend_from_slice(&be[first..]);
    }
}

fn rlp_bytes(out: &mut Vec<u8>, b: &[u8]) {
    if b.len() == 1 && b[0] < 0x80 {
        out.push(b[0]);
    } else {
        rlp_length(out, b.len(), 0x80);
        out.extend_from_slice(b);
    }
}

/// An integer: big-endian, no leading zeros (zero is the empty string).
fn rlp_uint(out: &mut Vec<u8>, be: &[u8]) {
    let first = be.iter().position(|&b| b != 0).unwrap_or(be.len());
    rlp_bytes(out, &be[first..]);
}

fn rlp_list(items: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(items.len() + 9);
    rlp_length(&mut out, items.len(), 0xc0);
    out.extend_from_slice(items);
    out
}

// ---------------------------------------------------------------- transactions

/// How a transaction pays for gas.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fee {
    /// EIP-1559: both in wei per gas, priority ≤ max.
    Eip1559 { max_fee: u128, max_priority: u128 },
    /// Legacy, EIP-155 replay protection.
    Legacy { gas_price: u128 },
}

/// An EVM transaction before signing: a native transfer (`data` empty) or a token transfer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tx {
    pub chain_id: u64,
    pub nonce: u64,
    pub gas: u64,
    pub fee: Fee,
    /// The call target: the recipient, or the token contract.
    pub to: Address,
    pub value: Amount,
    pub data: Vec<u8>,
}

/// `transfer(address,uint256)` calldata.
pub fn erc20_transfer(to: &Address, amount: &Amount) -> Vec<u8> {
    let mut d = vec![0xa9, 0x05, 0x9c, 0xbb];
    d.extend_from_slice(&[0; 12]);
    d.extend_from_slice(to);
    d.extend_from_slice(&amount.to_be_bytes());
    d
}

impl Tx {
    /// The most the transaction can cost in fees, in wei (`None` on overflow).
    pub fn max_fee(&self) -> Option<u128> {
        let price = match self.fee {
            Fee::Eip1559 { max_fee, .. } => max_fee,
            Fee::Legacy { gas_price } => gas_price,
        };
        price.checked_mul(self.gas as u128)
    }

    /// The fields shared by the signing payload and the signed transaction.
    fn fields(&self, out: &mut Vec<u8>) {
        match self.fee {
            Fee::Eip1559 { max_fee, max_priority } => {
                rlp_uint(out, &self.chain_id.to_be_bytes());
                rlp_uint(out, &self.nonce.to_be_bytes());
                rlp_uint(out, &max_priority.to_be_bytes());
                rlp_uint(out, &max_fee.to_be_bytes());
            }
            Fee::Legacy { gas_price } => {
                rlp_uint(out, &self.nonce.to_be_bytes());
                rlp_uint(out, &gas_price.to_be_bytes());
            }
        }
        rlp_uint(out, &self.gas.to_be_bytes());
        rlp_bytes(out, &self.to);
        rlp_uint(out, self.value.to_minimal_be());
        rlp_bytes(out, &self.data);
        if let Fee::Eip1559 { .. } = self.fee {
            out.push(0xc0); // empty access list
        }
    }

    /// Signs; returns the raw transaction and its hash.
    pub fn sign(&self, key: &SigningKey) -> (Vec<u8>, [u8; 32]) {
        let mut body = Vec::new();
        self.fields(&mut body);
        let typed = matches!(self.fee, Fee::Eip1559 { .. });
        let payload = if typed {
            [&[0x02][..], &rlp_list(&body)].concat()
        } else {
            // EIP-155: chain id, 0, 0
            let mut b = body.clone();
            rlp_uint(&mut b, &self.chain_id.to_be_bytes());
            b.extend_from_slice(&[0x80, 0x80]);
            rlp_list(&b)
        };
        let (sig, recid) = key.sign_prehash_recoverable(&keccak256(&payload));
        let (r, s) = sig.split_bytes();
        let v: u128 = if typed {
            recid.is_y_odd() as u128
        } else {
            self.chain_id as u128 * 2 + 35 + recid.is_y_odd() as u128
        };
        let mut signed = body;
        rlp_uint(&mut signed, &v.to_be_bytes());
        rlp_uint(&mut signed, &r);
        rlp_uint(&mut signed, &s);
        let raw = if typed { [&[0x02][..], &rlp_list(&signed)].concat() } else { rlp_list(&signed) };
        let hash = keccak256(&raw);
        (raw, hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eip55_checksums() {
        // the examples of EIP-55
        for s in [
            "0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed",
            "0xfB6916095ca1df60bB79Ce92cE3Ea74c37c5d359",
            "0xdbF03B407c01E7cD3CBea99509d93f8DDDC8C6FB",
            "0xD1220A0cf47c7B9Be7A2E6BA89F429762e7b9aDb",
        ] {
            let a = parse_address(s).unwrap();
            assert_eq!(checksum(&a), s);
            assert!(parse_address(&s.to_lowercase()).is_ok());
            assert!(parse_address(&s.to_uppercase().replace("0X", "0x")).is_ok());
        }
        assert!(parse_address("0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAeD").is_err());
        assert!(parse_address("5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed").is_err());
        assert!(parse_address("0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeA").is_err());
        assert!(parse_address("0xzaaeb6053f3e94c9b9a09f33669435e7ef1beaed").is_err());
    }

    #[test]
    fn known_key_address() {
        let key = SigningKey::from_slice(
            &unhex("4c0883a69102937d6231471b5dbb6204fe5129617082792ae468d01a3f362318").unwrap(),
        )
        .unwrap();
        assert_eq!(checksum(&address(&key)), "0x2c7536E3605D9C16a7a3D7b1898e529396a65c23");
    }

    /// EIP-155's worked example: nonce 9, 20 gwei, 21000 gas, 1 ether to 0x3535…, chain 1.
    #[test]
    fn eip155_example() {
        let key = SigningKey::from_slice(&[0x46; 32]).unwrap();
        let tx = Tx {
            chain_id: 1,
            nonce: 9,
            gas: 21000,
            fee: Fee::Legacy { gas_price: 20_000_000_000 },
            to: [0x35; 20],
            value: Amount::parse("1000000000000000000").unwrap(),
            data: vec![],
        };
        let (raw, _) = tx.sign(&key);
        assert_eq!(
            hex(&raw),
            "f86c098504a817c800825208943535353535353535353535353535353535353535880de0b6b3a76400008025a028ef61340bd939bc2195fe537567866003e1a15d3c71ff63e1590620aa636276a067cbe9d8997f761aecb703304b3800ccf555c9f3dc64214b297fb1966a3b6d83"
        );
    }

    #[test]
    fn rlp_long_forms() {
        let mut out = Vec::new();
        rlp_bytes(&mut out, &[0xaa; 56]);
        assert_eq!(&out[..2], &[0xb8, 56]);
        assert_eq!(rlp_list(&[0; 60])[..2], [0xf8, 60]);
        let mut z = Vec::new();
        rlp_uint(&mut z, &0u64.to_be_bytes());
        assert_eq!(z, vec![0x80]);
    }
}
