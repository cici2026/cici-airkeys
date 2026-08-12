//! Encoding and hashing kernels — the pure functions every surface calls.
//!
//! **Never hand-roll a primitive.** Hashing comes from `sha2`, base64 from
//! `base64`. What lives here is the *strictness*: junk input errors instead of
//! being silently coerced, because a credential manager that quietly accepts a
//! malformed blob loses the user's secrets rather than telling them so.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use sha2::{Digest, Sha256};

use crate::error::CoreError;

/// Lowercase hex, optionally `0x`-prefixed.
pub fn to_hex(bytes: &[u8], prefix: bool) -> String {
    let mut out = String::with_capacity(bytes.len() * 2 + if prefix { 2 } else { 0 });
    if prefix {
        out.push_str("0x");
    }
    for byte in bytes {
        out.push(char::from_digit((byte >> 4) as u32, 16).unwrap_or('0'));
        out.push(char::from_digit((byte & 0x0f) as u32, 16).unwrap_or('0'));
    }
    out
}

/// Strict hex decode: an odd length, a stray character or a non-`0x` prefix is
/// an error, never a truncated read.
pub fn from_hex(text: &str) -> Result<Vec<u8>, CoreError> {
    let body = text.strip_prefix("0x").unwrap_or(text);
    if body.len() % 2 != 0 {
        return Err(CoreError::InvalidHex(format!(
            "odd length ({})",
            body.len()
        )));
    }
    let mut out = Vec::with_capacity(body.len() / 2);
    let bytes = body.as_bytes();
    for pair in bytes.chunks_exact(2) {
        let hi = (pair[0] as char)
            .to_digit(16)
            .ok_or_else(|| CoreError::InvalidHex(format!("bad digit `{}`", pair[0] as char)))?;
        let lo = (pair[1] as char)
            .to_digit(16)
            .ok_or_else(|| CoreError::InvalidHex(format!("bad digit `{}`", pair[1] as char)))?;
        out.push(((hi << 4) | lo) as u8);
    }
    Ok(out)
}

/// base64url without padding — the shape WebAuthn and JWT use.
pub fn to_base64url(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Strict base64url decode. Padding is rejected rather than tolerated, so a
/// value that round-trips here round-trips everywhere.
pub fn from_base64url(text: &str) -> Result<Vec<u8>, CoreError> {
    if text.contains('=') {
        return Err(CoreError::InvalidBase64Url("padding is not allowed".into()));
    }
    URL_SAFE_NO_PAD
        .decode(text)
        .map_err(|error| CoreError::InvalidBase64Url(error.to_string()))
}

/// SHA-256 of `bytes`.
pub fn sha256(bytes: &[u8]) -> [u8; 32] {
    let digest = Sha256::digest(bytes);
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

/// The stable identifier for a vault item: `sha256(kind ‖ 0x00 ‖ title ‖ 0x00 ‖
/// account)`, hex, first 16 bytes.
///
/// Derived rather than random because every surface must arrive at the same id
/// for the same logical item — a random id minted on one device would duplicate
/// the entry on the next sync instead of merging with it. The `0x00` separators
/// are what stop `("a", "bc")` and `("ab", "c")` from colliding.
pub fn item_fingerprint(kind: &str, title: &str, account: &str) -> String {
    let mut buf = Vec::with_capacity(kind.len() + title.len() + account.len() + 2);
    buf.extend_from_slice(kind.as_bytes());
    buf.push(0);
    buf.extend_from_slice(title.as_bytes());
    buf.push(0);
    buf.extend_from_slice(account.as_bytes());
    to_hex(&sha256(&buf)[..16], false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trips_and_rejects_junk() {
        assert_eq!(to_hex(&[0xde, 0xad, 0xbe, 0xef], false), "deadbeef");
        assert_eq!(to_hex(&[0x01], true), "0x01");
        assert_eq!(
            from_hex("0xdeadbeef").unwrap(),
            vec![0xde, 0xad, 0xbe, 0xef]
        );
        assert!(from_hex("abc").is_err(), "odd length must error");
        assert!(from_hex("zz").is_err(), "bad digit must error");
    }

    #[test]
    fn base64url_rejects_padding() {
        assert_eq!(to_base64url(b"hi"), "aGk");
        assert_eq!(from_base64url("aGk").unwrap(), b"hi");
        assert!(from_base64url("aGk=").is_err(), "padding must error");
    }

    #[test]
    fn item_fingerprint_separates_its_fields() {
        // Without the 0x00 separators these two would hash identically.
        assert_ne!(
            item_fingerprint("login", "a", "bc"),
            item_fingerprint("login", "ab", "c")
        );
        // Same inputs, same id, on every surface.
        assert_eq!(
            item_fingerprint("login", "GitHub", "ann@example.com"),
            item_fingerprint("login", "GitHub", "ann@example.com")
        );
    }
}
