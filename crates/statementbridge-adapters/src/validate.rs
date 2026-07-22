//! Structural validation of adapter documents. Returns human-readable
//! problems; empty = valid. The JSON Schema in `schemas/` is the formal
//! contract; these checks add semantic rules a schema cannot express.

use crate::spec::AdapterSpec;
use std::fmt::Write as _;

pub fn validate_spec(spec: &AdapterSpec) -> Vec<String> {
    let mut problems = Vec::new();

    let id_ok = !spec.id.is_empty()
        && spec.id.len() <= 64
        && spec
            .id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !spec.id.starts_with('-');
    if !id_ok {
        problems.push(format!(
            "id '{}' must be 1-64 chars of [a-z0-9-], not starting with '-'",
            spec.id
        ));
    }

    let vparts: Vec<&str> = spec.version.split('.').collect();
    if vparts.len() != 3 || vparts.iter().any(|p| p.parse::<u32>().is_err()) {
        problems.push(format!(
            "version '{}' must be MAJOR.MINOR.PATCH",
            spec.version
        ));
    }

    if spec.display_name.trim().is_empty() {
        problems.push("display_name must not be empty".to_string());
    }
    if spec.description.trim().is_empty() {
        problems.push("description must not be empty".to_string());
    }

    if spec.supported_formats.is_empty() {
        problems.push("supported_formats must list at least one of: csv, xlsx".to_string());
    }
    for f in &spec.supported_formats {
        if f != "csv" && f != "xlsx" {
            problems.push(format!("unsupported format '{f}' (allowed: csv, xlsx)"));
        }
    }

    match spec.amount_mode.as_str() {
        "auto" | "signed" | "debit_credit" => {}
        other => problems.push(format!(
            "amount_mode '{other}' must be one of: auto, debit_credit, signed"
        )),
    }
    match spec.transaction_order.as_str() {
        "auto" | "ascending" | "descending" => {}
        other => problems.push(format!(
            "transaction_order '{other}' must be one of: auto, ascending, descending"
        )),
    }

    for s in [&spec.decimal_separator, &spec.thousands_separator]
        .into_iter()
        .flatten()
    {
        if s.chars().count() != 1 || !".,' ".contains(s.as_str()) {
            problems.push(format!(
                "separator '{s}' must be one of: . , ' (or a space)"
            ));
        }
    }
    if let (Some(d), Some(t)) = (&spec.decimal_separator, &spec.thousands_separator) {
        if d == t {
            problems.push("decimal_separator and thousands_separator must differ".to_string());
        }
    }

    if let Some(c) = &spec.currency {
        if c.len() != 3 || !c.bytes().all(|b| b.is_ascii_uppercase()) {
            problems.push(format!(
                "currency '{c}' must be an ISO 4217 alphabetic code (e.g. USD)"
            ));
        }
    }

    for fmt in &spec.date_formats {
        if !chrono_format_is_renderable(fmt) {
            problems.push(format!("date format '{fmt}' is not a valid chrono pattern"));
        }
    }

    for pat in &spec.footer_patterns {
        if let Err(e) = regex::RegexBuilder::new(pat)
            .case_insensitive(true)
            .size_limit(1 << 20)
            .build()
        {
            problems.push(format!("footer pattern '{pat}' is not a valid regex: {e}"));
        }
    }

    for field in spec.header_aliases.keys() {
        if statementbridge_core::config::Field::parse(field).is_none() {
            problems.push(format!(
                "header_aliases key '{field}' is not a canonical field (date, posted_at, description, reference, debit, credit, amount, balance, currency, account)"
            ));
        }
    }

    if spec.rows_to_skip > 100 {
        problems.push("rows_to_skip must be <= 100".to_string());
    }
    if spec.detection_priority > 100 {
        problems.push("detection_priority must be <= 100".to_string());
    }
    let mr = &spec.account_masking_rules;
    if mr.keep_last < 2 || mr.keep_last > 6 || mr.min_digits < 4 || mr.min_digits > 34 {
        problems.push("account_masking_rules: min_digits must be 4-34, keep_last 2-6".to_string());
    }

    if spec.amount_mode == "debit_credit" && spec.signed_amount_column.is_some() {
        problems.push("debit_credit mode must not set signed_amount_column".to_string());
    }
    if spec.amount_mode == "signed" && (spec.debit_column.is_some() || spec.credit_column.is_some())
    {
        problems.push("signed mode must not set debit_column/credit_column".to_string());
    }

    problems
}

/// Check a chrono format is renderable without panicking: format a known date
/// into a throwaway writer and watch for `fmt::Error`.
fn chrono_format_is_renderable(fmt: &str) -> bool {
    use chrono::NaiveDate;
    let Some(d) = NaiveDate::from_ymd_opt(2026, 1, 31) else {
        return false;
    };
    let dt = d.and_hms_opt(1, 2, 3).unwrap_or_default();
    let mut sink = String::new();
    write!(sink, "{}", dt.format(fmt)).is_ok()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn accepts_valid_and_rejects_broken() {
        let mut a = AdapterSpec {
            id: "my-bank".into(),
            display_name: "My Bank".into(),
            description: "Test".into(),
            ..AdapterSpec::default()
        };
        assert!(validate_spec(&a).is_empty(), "{:?}", validate_spec(&a));

        a.id = "Bad_ID!".into();
        a.amount_mode = "weird".into();
        a.currency = Some("usd".into());
        a.date_formats = vec!["%Q".into()];
        a.footer_patterns = vec!["(".into()];
        let problems = validate_spec(&a);
        assert!(problems.len() >= 5, "{problems:?}");
    }
}
