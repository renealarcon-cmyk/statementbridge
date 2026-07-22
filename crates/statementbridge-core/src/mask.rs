//! Masking and redaction of account-like digit sequences.
//!
//! Full account numbers never reach the canonical output: account columns are
//! masked to `****NNNN`, and `--redact` additionally rewrites long digit runs
//! inside descriptions/references.

/// Mask an account identifier: keeps the last `keep_last` digits when the
/// value contains at least `min_digits` digits (separators allowed).
/// Returns `None` when the value does not look like an account number.
pub fn mask_account(raw: &str, min_digits: usize, keep_last: usize) -> Option<String> {
    let digits: Vec<char> = raw.chars().filter(char::is_ascii_digit).collect();
    if digits.len() < min_digits {
        // Short tokens (e.g. "12") are not account numbers; but an already
        // masked value like ****4821 passes through unchanged.
        let t = raw.trim();
        if t.starts_with("****") && !t.is_empty() {
            return Some(t.to_string());
        }
        return None;
    }
    let tail: String = digits[digits.len().saturating_sub(keep_last)..]
        .iter()
        .collect();
    Some(format!("****{tail}"))
}

/// Replace every contiguous digit run of `min_len`+ digits with `****` + its
/// last 4 digits. Used by `--redact` on free-text fields.
pub fn redact_digit_runs(s: &str, min_len: usize) -> String {
    let mut out = String::with_capacity(s.len());
    let mut run = String::new();
    for c in s.chars().chain(std::iter::once('\u{0}')) {
        if c.is_ascii_digit() {
            run.push(c);
        } else {
            if !run.is_empty() {
                if run.len() >= min_len {
                    out.push_str("****");
                    out.push_str(&run[run.len() - 4.min(run.len())..]);
                } else {
                    out.push_str(&run);
                }
                run.clear();
            }
            if c != '\u{0}' {
                out.push(c);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn masks_accounts() {
        assert_eq!(
            mask_account("ES91 2100 0418 4502 0005 1332", 6, 4).as_deref(),
            Some("****1332")
        );
        assert_eq!(mask_account("00124821", 6, 4).as_deref(), Some("****4821"));
        assert_eq!(mask_account("****4821", 6, 4).as_deref(), Some("****4821"));
        assert_eq!(mask_account("ref 12", 6, 4), None);
    }

    #[test]
    fn redacts_long_runs_only() {
        assert_eq!(
            redact_digit_runs("card 4111111111111111 ok", 8),
            "card ****1111 ok"
        );
        assert_eq!(redact_digit_runs("invoice 12345", 8), "invoice 12345");
        assert_eq!(redact_digit_runs("acct 12345678", 8), "acct ****5678");
    }
}
