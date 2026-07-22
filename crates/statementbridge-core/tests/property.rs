//! Property-based tests for the money and date parsers.

use proptest::prelude::*;
use rust_decimal::Decimal;
use statementbridge_core::dates::parse_date;
use statementbridge_core::numbers::{parse_decimal, NumberFormat};

fn fmt(dec: char, thou: Option<char>) -> NumberFormat {
    NumberFormat {
        decimal: dec,
        thousands: thou,
    }
}

/// Render a decimal with a given decimal separator and optional 3-digit
/// grouping — the inverse of what the parser must undo.
fn render(d: Decimal, dec: char, thou: Option<char>) -> String {
    let s = d.abs().to_string();
    let (int_part, frac_part) = match s.split_once('.') {
        Some((i, f)) => (i.to_string(), Some(f.to_string())),
        None => (s, None),
    };
    let grouped = match thou {
        Some(t) => {
            let digits: Vec<char> = int_part.chars().collect();
            let mut out = String::new();
            for (i, c) in digits.iter().enumerate() {
                if i > 0 && (digits.len() - i) % 3 == 0 {
                    out.push(t);
                }
                out.push(*c);
            }
            out
        }
        None => int_part,
    };
    let mut out = String::new();
    if d.is_sign_negative() {
        out.push('-');
    }
    out.push_str(&grouped);
    if let Some(f) = frac_part {
        out.push(dec);
        out.push_str(&f);
    }
    out
}

proptest! {
    #[test]
    fn decimal_roundtrip_plain(mantissa in -1_000_000_000_000i64..1_000_000_000_000i64, scale in 0u32..=4) {
        let d = Decimal::new(mantissa, scale);
        let rendered = render(d, '.', None);
        let parsed = parse_decimal(&rendered, &fmt('.', None)).expect("roundtrip parse");
        prop_assert_eq!(parsed, d);
    }

    #[test]
    fn decimal_roundtrip_grouped_en(mantissa in -1_000_000_000_000i64..1_000_000_000_000i64, scale in 0u32..=4) {
        let d = Decimal::new(mantissa, scale);
        let rendered = render(d, '.', Some(','));
        let parsed = parse_decimal(&rendered, &fmt('.', Some(','))).expect("roundtrip parse");
        prop_assert_eq!(parsed, d);
    }

    #[test]
    fn decimal_roundtrip_grouped_es(mantissa in -1_000_000_000_000i64..1_000_000_000_000i64, scale in 0u32..=4) {
        let d = Decimal::new(mantissa, scale);
        let rendered = render(d, ',', Some('.'));
        let parsed = parse_decimal(&rendered, &fmt(',', Some('.'))).expect("roundtrip parse");
        prop_assert_eq!(parsed, d);
    }

    #[test]
    fn parser_never_panics(s in "[0-9.,'() +\\-a-zA-Z€$£\u{00a0}]{0,30}") {
        // Outcome may be Ok or Err — it must simply never panic.
        let _ = parse_decimal(&s, &fmt('.', Some(',')));
        let _ = parse_decimal(&s, &fmt(',', Some('.')));
    }

    #[test]
    fn date_roundtrip_dmy(y in 1990i32..2100, m in 1u32..=12, d in 1u32..=28) {
        let date = chrono::NaiveDate::from_ymd_opt(y, m, d).expect("valid date");
        let rendered = date.format("%d/%m/%Y").to_string();
        let parsed = parse_date(&rendered, &["%d/%m/%Y".to_string()]).expect("parse");
        prop_assert_eq!(parsed, date);
    }

    #[test]
    fn date_roundtrip_mdy(y in 1990i32..2100, m in 1u32..=12, d in 1u32..=28) {
        let date = chrono::NaiveDate::from_ymd_opt(y, m, d).expect("valid date");
        let rendered = date.format("%m/%d/%Y").to_string();
        let parsed = parse_date(&rendered, &["%m/%d/%Y".to_string()]).expect("parse");
        prop_assert_eq!(parsed, date);
    }

    #[test]
    fn date_parser_never_panics(s in ".{0,24}") {
        let fmts = ["%Y-%m-%d".to_string(), "%d/%m/%Y".to_string()];
        let _ = parse_date(&s, &fmts);
    }
}
