//! Deterministic hashing: file content hash, transaction fingerprints and ids.
//!
//! The fingerprint combines the *normalized* values of date, amount,
//! description, reference, balance and masked account, joined with the ASCII
//! unit separator. Identical logical rows ⇒ identical fingerprint, on every
//! platform, on every run.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use sha2::{Digest, Sha256};

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex(&h.finalize())
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// Canonical decimal formatting for hashing: trailing zeros stripped so
/// `1.50` and `1.500` hash identically.
fn canon_dec(d: &Decimal) -> String {
    d.normalize().to_string()
}

#[allow(clippy::too_many_arguments)]
pub fn transaction_fingerprint(
    date: NaiveDate,
    amount: &Decimal,
    description_key: &str,
    reference: Option<&str>,
    balance: Option<&Decimal>,
    account_masked: Option<&str>,
) -> String {
    const US: char = '\u{1f}';
    let mut buf = String::new();
    buf.push_str(&date.format("%Y-%m-%d").to_string());
    buf.push(US);
    buf.push_str(&canon_dec(amount));
    buf.push(US);
    buf.push_str(description_key);
    buf.push(US);
    buf.push_str(
        &reference
            .map(|r| r.trim().to_lowercase())
            .unwrap_or_default(),
    );
    buf.push(US);
    buf.push_str(&balance.map(canon_dec).unwrap_or_default());
    buf.push(US);
    buf.push_str(account_masked.unwrap_or_default());
    sha256_hex(buf.as_bytes())
}

/// Loose key for possible-duplicate grouping: date + amount + description only.
pub fn loose_key(date: NaiveDate, amount: &Decimal, description_key: &str) -> String {
    format!(
        "{}\u{1f}{}\u{1f}{}",
        date.format("%Y-%m-%d"),
        canon_dec(amount),
        description_key
    )
}

/// Deterministic transaction id: `sb1-` + first 16 hex chars of
/// SHA-256(`fingerprint ":" original_row`). Stable for the same file bytes and
/// options; distinct even for identical duplicate rows.
pub fn transaction_id(fingerprint: &str, original_row: u32) -> String {
    let h = sha256_hex(format!("{fingerprint}:{original_row}").as_bytes());
    format!("sb1-{}", &h[..16])
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use std::str::FromStr;

    #[test]
    fn fingerprint_is_stable_and_scale_insensitive() {
        let d = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        let a1 = Decimal::from_str("-250.00").unwrap();
        let a2 = Decimal::from_str("-250.0").unwrap();
        let f1 = transaction_fingerprint(d, &a1, "payment supplier", Some("TRX-1"), None, None);
        let f2 = transaction_fingerprint(d, &a2, "payment supplier", Some("trx-1"), None, None);
        assert_eq!(f1, f2);
        let f3 = transaction_fingerprint(d, &a1, "payment supplier", Some("TRX-2"), None, None);
        assert_ne!(f1, f3);
    }

    #[test]
    fn id_shape() {
        let id = transaction_id("abc", 14);
        assert!(id.starts_with("sb1-"));
        assert_eq!(id.len(), 20);
    }
}
