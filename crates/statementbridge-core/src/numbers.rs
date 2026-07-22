//! Exact decimal parsing for monetary values.
//!
//! Rules (documented in docs/canonical-schema.md):
//! * Money is never a float. Parsing goes straight from text to `Decimal`.
//! * The engine does not round parsed values; original scale is preserved
//!   (`"250.00"` stays `250.00`, not `250`).
//! * Negatives: leading `-`, trailing `-`, or accounting parentheses `(1.50)`.
//! * Currency symbols/codes at the edges are ignored for parsing (currency
//!   detection is separate); letters *inside* the number are an error.
//! * Ambiguous formats are resolved deterministically and produce an
//!   `ambiguous_number_format` warning at file level.

use rust_decimal::Decimal;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NumberFormat {
    pub decimal: char,
    pub thousands: Option<char>,
}

impl Default for NumberFormat {
    fn default() -> Self {
        Self {
            decimal: '.',
            thousands: Some(','),
        }
    }
}

const CURRENCY_SYMBOLS: [char; 10] = ['$', '€', '£', '¥', '₡', '₱', '₩', '₹', '¢', '¤'];

/// Currency inferred from an unambiguous symbol, if any.
pub fn currency_from_symbol(raw: &str) -> Option<&'static str> {
    if raw.contains('€') {
        Some("EUR")
    } else if raw.contains('£') {
        Some("GBP")
    } else if raw.contains('₡') {
        Some("CRC")
    } else if raw.contains('₹') {
        Some("INR")
    } else if raw.contains('₩') {
        Some("KRW")
    } else {
        None // `$` and `¥` are ambiguous across countries: never guessed.
    }
}

/// Parse one monetary cell with a known format. Returns an error string
/// (safe to show; includes the raw token, which is a number, not a name).
pub fn parse_decimal(raw: &str, fmt: &NumberFormat) -> Result<Decimal, String> {
    let s = raw.trim();
    if s.is_empty() {
        return Err("empty value".to_string());
    }
    let s: String = s
        .chars()
        .map(|c| match c {
            '\u{00a0}' | '\u{202f}' | '\u{2009}' => ' ',
            '\u{2212}' => '-', // Unicode minus
            _ => c,
        })
        .collect();

    let mut negative = false;
    let mut t = s.trim().to_string();
    if t.starts_with('(') && t.ends_with(')') && t.len() >= 2 {
        negative = true;
        t = t[1..t.len() - 1].trim().to_string();
    }
    if let Some(rest) = t.strip_prefix('-') {
        negative = true;
        t = rest.trim_start().to_string();
    } else if let Some(rest) = t.strip_suffix('-') {
        negative = true;
        t = rest.trim_end().to_string();
    } else if let Some(rest) = t.strip_prefix('+') {
        t = rest.trim_start().to_string();
    }

    // Split off leading/trailing currency tokens (symbols or letter codes).
    let first_digit = t.find(|c: char| c.is_ascii_digit());
    let last_digit = t.rfind(|c: char| c.is_ascii_digit());
    let (Some(a), Some(b)) = (first_digit, last_digit) else {
        return Err(format!("no digits in '{}'", safe_token(&t)));
    };
    let prefix = &t[..a];
    let suffix = &t[b + t[b..].chars().next().map(char::len_utf8).unwrap_or(1)..];
    for (edge, is_prefix) in [(prefix, true), (suffix, false)] {
        if !currency_edge_ok(edge, is_prefix) {
            return Err(format!(
                "invalid characters around number in '{}'",
                safe_token(&t)
            ));
        }
    }
    let body = &t[a..b + t[b..].chars().next().map(char::len_utf8).unwrap_or(1)];

    // Body may contain digits, separators and spaces only.
    let mut cleaned = String::with_capacity(body.len());
    for c in body.chars() {
        match c {
            '0'..='9' | '.' | ',' => cleaned.push(c),
            '\'' | ' ' => {} // apostrophe/space groupings are always thousands
            _ => return Err(format!("invalid character '{c}' in '{}'", safe_token(body))),
        }
    }

    let dec = fmt.decimal;
    let decimal_count = cleaned.matches(dec).count();
    if decimal_count > 1 {
        return Err(format!(
            "multiple '{dec}' separators in '{}'",
            safe_token(body)
        ));
    }
    let dec_pos = cleaned.rfind(dec);
    let mut out = String::with_capacity(cleaned.len());
    for (i, c) in cleaned.char_indices() {
        if c == dec {
            out.push('.');
        } else if c == '.' || c == ',' {
            // The other separator: valid only as thousands, i.e. before the
            // decimal separator (or anywhere when no decimal present).
            if let Some(dp) = dec_pos {
                if i > dp {
                    return Err(format!(
                        "unexpected '{c}' after decimal separator in '{}'",
                        safe_token(body)
                    ));
                }
            }
            // dropped (thousands)
        } else {
            out.push(c);
        }
    }
    if out.is_empty() || out == "." {
        return Err(format!("no digits in '{}'", safe_token(body)));
    }

    let mut d = Decimal::from_str(&out).map_err(|e| format!("not a number: {e}"))?;
    if d.scale() > 12 {
        return Err("more than 12 decimal places".to_string());
    }
    if negative {
        d.set_sign_negative(true);
    }
    Ok(d)
}

/// A currency token at the edge of a number is acceptable when it is either
/// space-separated from the digits ("USD 12", "1.250,00 €"), a currency
/// symbol ("$100", "100€"), or an attached ISO-like code of ≤3 uppercase
/// letters ("EUR99"). Anything else glued to digits ("12abc") is an error —
/// silently ignoring it would corrupt amounts.
fn currency_edge_ok(edge: &str, is_prefix: bool) -> bool {
    if edge.is_empty() {
        return true;
    }
    if !edge
        .chars()
        .all(|c| c.is_alphabetic() || c == ' ' || c == '.' || CURRENCY_SYMBOLS.contains(&c))
    {
        return false;
    }
    let attached = if is_prefix {
        edge.rsplit(' ').next().unwrap_or("")
    } else {
        edge.split(' ').next().unwrap_or("")
    };
    if attached.is_empty() {
        return true;
    }
    let symbols_only = attached.chars().all(|c| CURRENCY_SYMBOLS.contains(&c));
    let iso_like =
        attached.chars().count() <= 3 && attached.chars().all(|c| c.is_ascii_uppercase());
    symbols_only || iso_like
}

/// Cap tokens quoted in error messages (they are numeric-ish, but keep them short).
fn safe_token(s: &str) -> String {
    let mut t: String = s.chars().take(24).collect();
    if s.chars().count() > 24 {
        t.push('…');
    }
    t
}

/// Infer decimal/thousands separators from sample values.
/// Deterministic: strong evidence (both separators present, or a separator
/// followed by 1–2 digits at the end, or repeated separators) casts votes;
/// with no strong evidence the result defaults to `.` decimal / `,` thousands
/// (or the locale convention) and `ambiguous = true`.
pub fn infer_number_format(
    samples: &[&str],
    locale: Option<crate::config::Locale>,
) -> (NumberFormat, bool) {
    let mut dot_votes = 0u32; // '.' is the decimal separator
    let mut comma_votes = 0u32;
    let mut saw_separator = false;

    for raw in samples {
        let s: String = raw
            .chars()
            .filter(|c| c.is_ascii_digit() || *c == '.' || *c == ',')
            .collect();
        let has_dot = s.contains('.');
        let has_comma = s.contains(',');
        if has_dot || has_comma {
            saw_separator = true;
        }
        if has_dot && has_comma {
            let last_dot = s.rfind('.').unwrap_or(0);
            let last_comma = s.rfind(',').unwrap_or(0);
            if last_dot > last_comma {
                dot_votes += 2;
            } else {
                comma_votes += 2;
            }
        } else if has_dot {
            match separator_evidence(&s, '.') {
                Evidence::Decimal => dot_votes += 2,
                Evidence::Thousands => comma_votes += 1,
                Evidence::Ambiguous => {}
            }
        } else if has_comma {
            match separator_evidence(&s, ',') {
                Evidence::Decimal => comma_votes += 2,
                Evidence::Thousands => dot_votes += 1,
                Evidence::Ambiguous => {}
            }
        }
    }

    if dot_votes > comma_votes {
        (
            NumberFormat {
                decimal: '.',
                thousands: Some(','),
            },
            false,
        )
    } else if comma_votes > dot_votes {
        (
            NumberFormat {
                decimal: ',',
                thousands: Some('.'),
            },
            false,
        )
    } else {
        // No evidence either way: locale convention, else '.' decimal.
        let fmt = match locale {
            Some(crate::config::Locale::Es) => NumberFormat {
                decimal: ',',
                thousands: Some('.'),
            },
            _ => NumberFormat::default(),
        };
        (fmt, saw_separator)
    }
}

enum Evidence {
    Decimal,
    Thousands,
    Ambiguous,
}

fn separator_evidence(s: &str, sep: char) -> Evidence {
    let count = s.matches(sep).count();
    if count > 1 {
        return Evidence::Thousands;
    }
    let after = s.rsplit(sep).next().unwrap_or("");
    match after.len() {
        1 | 2 => Evidence::Decimal,
        3 => Evidence::Ambiguous, // "1.234" — could be either
        _ => Evidence::Decimal,   // "1.2345" groups of ≠3 are not thousands
    }
}

/// Format money for summaries with at least 2 decimal places (original scale
/// preserved when larger).
pub fn money(d: Decimal) -> Decimal {
    if d.scale() < 2 {
        let mut c = d;
        c.rescale(2);
        c
    } else {
        d
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn f(dec: char, thou: Option<char>) -> NumberFormat {
        NumberFormat {
            decimal: dec,
            thousands: thou,
        }
    }

    #[test]
    fn parses_plain_and_grouped() {
        assert_eq!(
            parse_decimal("1234.56", &f('.', None)).unwrap().to_string(),
            "1234.56"
        );
        assert_eq!(
            parse_decimal("1,234.56", &f('.', Some(',')))
                .unwrap()
                .to_string(),
            "1234.56"
        );
        assert_eq!(
            parse_decimal("1.234,56", &f(',', Some('.')))
                .unwrap()
                .to_string(),
            "1234.56"
        );
        assert_eq!(
            parse_decimal("1 234,56", &f(',', Some(' ')))
                .unwrap()
                .to_string(),
            "1234.56"
        );
        assert_eq!(
            parse_decimal("1'234.56", &f('.', Some('\'')))
                .unwrap()
                .to_string(),
            "1234.56"
        );
    }

    #[test]
    fn parses_signs_and_parens() {
        assert_eq!(
            parse_decimal("-25.10", &f('.', None)).unwrap().to_string(),
            "-25.10"
        );
        assert_eq!(
            parse_decimal("25.10-", &f('.', None)).unwrap().to_string(),
            "-25.10"
        );
        assert_eq!(
            parse_decimal("(2500.00)", &f('.', Some(',')))
                .unwrap()
                .to_string(),
            "-2500.00"
        );
        assert_eq!(parse_decimal("+7", &f('.', None)).unwrap().to_string(), "7");
    }

    #[test]
    fn strips_currency_edges() {
        assert_eq!(
            parse_decimal("$1,000.00", &f('.', Some(',')))
                .unwrap()
                .to_string(),
            "1000.00"
        );
        assert_eq!(
            parse_decimal("EUR 99,50", &f(',', Some('.')))
                .unwrap()
                .to_string(),
            "99.50"
        );
        assert_eq!(
            parse_decimal("1.250,00 €", &f(',', Some('.')))
                .unwrap()
                .to_string(),
            "1250.00"
        );
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_decimal("12a34", &f('.', None)).is_err());
        assert!(parse_decimal("12abc", &f('.', None)).is_err()); // lowercase glued to digits
        assert!(parse_decimal("EUR99", &f('.', None)).is_ok()); // attached ISO code is fine
        assert!(parse_decimal("100€", &f('.', None)).is_ok());
        assert!(parse_decimal("", &f('.', None)).is_err());
        assert!(parse_decimal("--", &f('.', None)).is_err());
        assert!(parse_decimal("1.2.3", &f('.', Some(','))).is_err());
        assert!(parse_decimal("1,2,3.45", &f('.', Some(','))).is_ok()); // grouped oddly but unambiguous
        assert!(parse_decimal("1.23,45", &f('.', Some(','))).is_err()); // ',' after decimal
    }

    #[test]
    fn preserves_scale() {
        assert_eq!(
            parse_decimal("250.00", &f('.', None)).unwrap().to_string(),
            "250.00"
        );
        assert_eq!(
            parse_decimal("0.005", &f('.', None)).unwrap().to_string(),
            "0.005"
        );
    }

    #[test]
    fn infers_formats() {
        let (fmt, amb) = infer_number_format(&["1,234.56", "-99.10"], None);
        assert_eq!(fmt.decimal, '.');
        assert!(!amb);
        let (fmt, amb) = infer_number_format(&["1.234,56", "99,10"], None);
        assert_eq!(fmt.decimal, ',');
        assert!(!amb);
        let (fmt, amb) = infer_number_format(&["1234", "99"], None);
        assert_eq!(fmt.decimal, '.');
        assert!(!amb); // integers only: nothing ambiguous
        let (_, amb) = infer_number_format(&["1.234"], None);
        assert!(amb); // single 3-digit group: genuinely ambiguous
    }
}
