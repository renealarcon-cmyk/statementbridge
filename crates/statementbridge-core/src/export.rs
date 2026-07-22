//! Export: canonical JSON and normalized CSV.
//!
//! CSV export defends against formula injection (CWE-1236): text cells that
//! start with `=`, `+`, `-`, `@`, tab or carriage return are prefixed with a
//! single quote so spreadsheet apps treat them as text. Numeric/date columns
//! are engine-generated and never carry payloads.

use crate::model::Statement;

/// Pretty canonical JSON with trailing newline. Field order is fixed by the
/// struct definitions, so output is byte-stable.
pub fn to_canonical_json(st: &Statement) -> Result<String, crate::EngineError> {
    let mut s = serde_json::to_string_pretty(st)
        .map_err(|e| crate::EngineError::Io(format!("JSON encoding failed: {e}")))?;
    s.push('\n');
    Ok(s)
}

/// Escape a text cell against CSV formula injection.
pub fn csv_guard(s: &str) -> String {
    match s.chars().next() {
        Some('=') | Some('+') | Some('-') | Some('@') | Some('\t') | Some('\r') => {
            format!("'{s}")
        }
        _ => s.to_string(),
    }
}

const CSV_HEADERS: [&str; 12] = [
    "id",
    "date",
    "posted_at",
    "description",
    "reference",
    "debit",
    "credit",
    "amount",
    "balance",
    "currency",
    "original_row",
    "duplicate_kind",
];

/// Normalized CSV (UTF-8, `\n`, comma, RFC 4180 quoting via the `csv` crate).
pub fn to_normalized_csv(st: &Statement) -> Result<String, crate::EngineError> {
    let mut w = csv::WriterBuilder::new()
        .terminator(csv::Terminator::Any(b'\n'))
        .from_writer(Vec::new());
    w.write_record(CSV_HEADERS)
        .map_err(|e| crate::EngineError::Io(format!("CSV write failed: {e}")))?;
    for t in &st.transactions {
        let dup = t
            .duplicate_kind
            .map(|k| {
                serde_json::to_value(k)
                    .ok()
                    .and_then(|v| v.as_str().map(String::from))
                    .unwrap_or_default()
            })
            .unwrap_or_default();
        let rec = [
            t.id.clone(),
            t.date.format("%Y-%m-%d").to_string(),
            t.posted_at
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_default(),
            csv_guard(&t.description),
            csv_guard(t.reference.as_deref().unwrap_or("")),
            t.debit.map(|d| d.to_string()).unwrap_or_default(),
            t.credit.map(|d| d.to_string()).unwrap_or_default(),
            t.amount.to_string(),
            t.balance.map(|d| d.to_string()).unwrap_or_default(),
            t.currency.clone().unwrap_or_default(),
            t.original_row.to_string(),
            dup,
        ];
        w.write_record(&rec)
            .map_err(|e| crate::EngineError::Io(format!("CSV write failed: {e}")))?;
    }
    let bytes = w
        .into_inner()
        .map_err(|e| crate::EngineError::Io(format!("CSV flush failed: {e}")))?;
    String::from_utf8(bytes).map_err(|e| crate::EngineError::Io(format!("CSV not UTF-8: {e}")))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn guards_formula_prefixes() {
        assert_eq!(csv_guard("=SUM(A1:A9)"), "'=SUM(A1:A9)");
        assert_eq!(csv_guard("+1234"), "'+1234");
        assert_eq!(csv_guard("-cmd"), "'-cmd");
        assert_eq!(csv_guard("@import"), "'@import");
        assert_eq!(csv_guard("normal text"), "normal text");
    }
}
