//! Amounts of money: integers in base units (satoshi, wei), written as decimal
//! strings on the wire (PROTOCOL §2.3) because JSON numbers lose precision above 2⁵³.
//!
//! An [`Amount`] is exact up to 2²⁵⁶ − 1 (an ERC-20 balance can use the whole range).
//! Arithmetic is only ever needed on fees and native coins, which fit in `u128`.

use crate::{Error, Result, bail, shown};
use serde::{Deserialize, Serialize};

/// An amount in the smallest unit (satoshi, wei, a token's base unit), as the protocol writes
/// it: a decimal string, so that no JSON number loses precision (PROTOCOL §2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Amount {
    digits: String,
    /// Big-endian, 256 bits.
    be: [u8; 32],
}

impl Amount {
    /// Decimal digits, no sign, no leading zeros, below 2²⁵⁶.
    pub fn parse(s: &str) -> Result<Amount> {
        // 2²⁵⁶ has 78 digits: anything longer is refused before it is read
        let ok = (1..=78).contains(&s.len())
            && s.bytes().all(|b| b.is_ascii_digit())
            && (s == "0" || !s.starts_with('0'));
        if !ok {
            bail!(
                "“{}” is not an amount: amounts are whole numbers of base units, written in digits.",
                shown(s)
            );
        }
        // acc = acc·10 + d over four 64-bit limbs, most significant first
        let mut limbs = [0u64; 4];
        for d in s.bytes().map(|b| (b - b'0') as u128) {
            let mut carry = d;
            for limb in limbs.iter_mut().rev() {
                let v = *limb as u128 * 10 + carry;
                *limb = v as u64;
                carry = v >> 64;
            }
            if carry != 0 {
                bail!("The amount {} is too large.", shown(s));
            }
        }
        let mut be = [0u8; 32];
        for (i, limb) in limbs.iter().enumerate() {
            be[i * 8..i * 8 + 8].copy_from_slice(&limb.to_be_bytes());
        }
        Ok(Amount { digits: s.to_string(), be })
    }

    /// The decimal digits, exactly as written.
    pub fn as_str(&self) -> &str {
        &self.digits
    }

    /// Whether the amount is 0.
    pub fn is_zero(&self) -> bool {
        self.digits == "0"
    }

    /// The 256-bit big-endian encoding (ABI words).
    pub fn to_be_bytes(&self) -> [u8; 32] {
        self.be
    }

    /// The big-endian encoding without leading zero bytes (RLP integers).
    pub fn to_minimal_be(&self) -> &[u8] {
        let first = self.be.iter().position(|&b| b != 0).unwrap_or(32);
        &self.be[first..]
    }

    /// The value, if it fits 128 bits.
    pub fn to_u128(&self) -> Option<u128> {
        if self.be[..16].iter().any(|&b| b != 0) {
            return None;
        }
        Some(u128::from_be_bytes(self.be[16..].try_into().unwrap()))
    }

    /// The value, if it fits 64 bits (every bitcoin amount does).
    pub fn to_u64(&self) -> Option<u64> {
        self.to_u128().and_then(|v| u64::try_from(v).ok())
    }

    /// The value in whole units: `decimals` 6 turns 1234500 into “1.2345”. Exact (no
    /// rounding), trailing zeros dropped, thousands separated by commas.
    pub fn format(&self, decimals: u8) -> String {
        format_digits(&self.digits, decimals)
    }
}

impl From<u128> for Amount {
    fn from(v: u128) -> Amount {
        Amount::parse(&v.to_string()).expect("a u128 is an amount")
    }
}

impl From<u64> for Amount {
    fn from(v: u64) -> Amount {
        Amount::from(v as u128)
    }
}

impl TryFrom<String> for Amount {
    type Error = Error;
    fn try_from(s: String) -> Result<Amount> {
        Amount::parse(&s)
    }
}

impl From<Amount> for String {
    fn from(a: Amount) -> String {
        a.digits
    }
}

/// [`Amount::format`] for a value that is already a `u128`.
pub fn format_u128(v: u128, decimals: u8) -> String {
    format_digits(&v.to_string(), decimals)
}

fn format_digits(digits: &str, decimals: u8) -> String {
    let d = decimals as usize;
    let padded = if digits.len() <= d {
        format!("{}{digits}", "0".repeat(d + 1 - digits.len()))
    } else {
        digits.to_string()
    };
    let (int, frac) = padded.split_at(padded.len() - d);
    let mut out = String::new();
    for (i, c) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    let frac = frac.trim_end_matches('0');
    if !frac.is_empty() {
        out.push('.');
        out.push_str(frac);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_only_canonical_decimals() {
        for bad in ["", "-1", "01", "1.5", "1e9", " 1", "0x10", "１"] {
            assert!(Amount::parse(bad).is_err(), "{bad:?}");
        }
        assert_eq!(Amount::parse("0").unwrap().to_u64(), Some(0));
        assert_eq!(Amount::parse("18446744073709551615").unwrap().to_u64(), Some(u64::MAX));
        assert_eq!(Amount::parse("18446744073709551616").unwrap().to_u64(), None);
    }

    #[test]
    fn covers_256_bits_exactly() {
        let max = "115792089237316195423570985008687907853269984665640564039457584007913129639935";
        assert_eq!(Amount::parse(max).unwrap().to_be_bytes(), [0xff; 32]);
        let over = "115792089237316195423570985008687907853269984665640564039457584007913129639936";
        assert!(Amount::parse(over).is_err());
        let a = Amount::parse("1000000000000000000").unwrap();
        assert_eq!(a.to_minimal_be(), &[0x0d, 0xe0, 0xb6, 0xb3, 0xa7, 0x64, 0x00, 0x00]);
        assert_eq!(Amount::parse("0").unwrap().to_minimal_be(), &[] as &[u8]);
    }

    #[test]
    fn formats_exactly() {
        let f = |s: &str, d| Amount::parse(s).unwrap().format(d);
        assert_eq!(f("1500000000000000000", 18), "1.5");
        assert_eq!(f("1", 18), "0.000000000000000001");
        assert_eq!(f("0", 8), "0");
        assert_eq!(f("123456789", 0), "123,456,789");
        assert_eq!(f("2100000000000000", 8), "21,000,000");
        assert_eq!(f("1234567", 3), "1,234.567");
        assert_eq!(f("100", 2), "1");
    }
}
