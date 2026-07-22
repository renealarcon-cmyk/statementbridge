//! Adapter auto-detection: score every candidate against the detected header
//! row and pick the best. Deterministic: ties break by priority, then id.

use crate::spec::AdapterSpec;
use statementbridge_core::header::{default_aliases, detect};
use statementbridge_core::text::matching_key;
use statementbridge_core::{NormalizeOptions, RawTable};

/// Score one adapter against header cells (already `matching_key`-normalized).
fn score(spec: &AdapterSpec, header_keys: &[String], format: &str) -> i64 {
    if !spec.supported_formats.iter().any(|f| f == format) {
        return i64::MIN;
    }
    let mut s: i64 = 0;
    // Expected headers: each one found is strong evidence. Expected headers
    // are matched against the built-in alias table too, so "debit" matches a
    // "Cargo" column.
    let aliases = default_aliases();
    for expected in &spec.expected_headers {
        let want = matching_key(expected);
        let direct = header_keys.iter().any(|h| h == &want || h.contains(&want));
        let via_alias = statementbridge_core::config::Field::parse(&want)
            .and_then(|f| aliases.get(&f))
            .map(|al| {
                header_keys
                    .iter()
                    .any(|h| al.iter().any(|a| h == &matching_key(a)))
            })
            .unwrap_or(false);
        if direct || via_alias {
            s += 25;
        } else {
            s -= 1000; // an expected header is missing: effectively disqualify
        }
    }
    // Adapter-specific aliases found in the header are evidence too.
    for al in spec.header_aliases.values().flatten() {
        let want = matching_key(al);
        if header_keys.iter().any(|h| h == &want) {
            s += 10;
        }
    }
    s + spec.detection_priority as i64
}

/// Pick the best adapter for a raw table. Always returns something when at
/// least one candidate supports the format (the generic fallback scores > MIN).
pub fn detect_adapter<'a>(
    raw: &RawTable,
    candidates: &'a [AdapterSpec],
) -> Option<&'a AdapterSpec> {
    let hd = detect(raw, &NormalizeOptions::default());
    let header_keys: Vec<String> = hd.columns.iter().map(|c| matching_key(&c.header)).collect();
    let format = raw.meta.format.as_str();

    candidates
        .iter()
        .map(|a| (score(a, &header_keys, format), a))
        .filter(|(s, _)| *s > i64::MIN)
        .max_by(|(sa, a), (sb, b)| {
            sa.cmp(sb)
                .then(a.detection_priority.cmp(&b.detection_priority))
                .then(b.id.cmp(&a.id)) // reversed: prefer lexicographically smaller id
        })
        .map(|(_, a)| a)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::builtin_adapters;
    use statementbridge_core::raw::{RawMeta, RawTable};

    fn table(header: Vec<&str>) -> RawTable {
        RawTable {
            rows: vec![
                header.into_iter().map(String::from).collect(),
                vec![
                    "2026-01-01".into(),
                    "x".into(),
                    "1.00".into(),
                    "2.00".into(),
                ],
            ],
            meta: RawMeta {
                format: "csv".into(),
                ..RawMeta::default()
            },
        }
    }

    #[test]
    fn picks_debit_credit_for_cargo_abono() {
        let adapters = builtin_adapters();
        let t = table(vec!["Fecha", "Concepto", "Cargo", "Abono"]);
        let a = detect_adapter(&t, &adapters).unwrap();
        assert_eq!(a.id, "generic-debit-credit");
    }

    #[test]
    fn picks_signed_for_amount() {
        let adapters = builtin_adapters();
        let t = table(vec!["Date", "Description", "Amount", "Balance"]);
        let a = detect_adapter(&t, &adapters).unwrap();
        assert_eq!(a.id, "generic-signed-amount");
    }

    #[test]
    fn falls_back_to_generic() {
        let adapters = builtin_adapters();
        let t = table(vec!["A", "B", "C", "D"]);
        let a = detect_adapter(&t, &adapters).unwrap();
        assert_eq!(a.id, "generic-csv");
    }
}
