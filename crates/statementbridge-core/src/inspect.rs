//! Lightweight inspection: run detection only and report what the engine
//! sees, with samples redacted (long digit runs masked) so previews are safe
//! to display and log.

use crate::config::NormalizeOptions;
use crate::issues::Issue;
use crate::mask;
use crate::normalize::detect_all;
use crate::raw::RawTable;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct InspectColumn {
    /// 1-based column index.
    pub index: u32,
    pub header: String,
    /// Canonical field name, when detected.
    pub field: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InspectReport {
    pub file_name: String,
    pub format: String,
    pub encoding: Option<String>,
    pub delimiter: Option<String>,
    pub sheet_name: Option<String>,
    pub sheet_names: Vec<String>,
    pub total_rows: u32,
    /// 1-based header row, when found.
    pub header_row: Option<u32>,
    pub data_rows: u32,
    pub columns: Vec<InspectColumn>,
    pub amount_mode: String,
    pub date_format: Option<String>,
    pub decimal_separator: String,
    pub thousands_separator: Option<String>,
    pub currency_hint: Option<String>,
    /// First data rows, with long digit runs masked and cells capped at 60
    /// chars — safe to print.
    pub sample_rows: Vec<Vec<String>>,
    pub errors: Vec<Issue>,
    pub warnings: Vec<Issue>,
}

pub fn inspect(raw: &RawTable, opts: &NormalizeOptions) -> InspectReport {
    let det = detect_all(raw, opts);
    let mut samples = Vec::new();
    let mut taken = 0;
    for r in det.header.data_start..raw.rows.len() {
        if taken >= 10 {
            break;
        }
        if raw.row_is_empty(r) {
            continue;
        }
        taken += 1;
        samples.push(
            raw.rows[r]
                .iter()
                .map(|c| {
                    let masked = mask::redact_digit_runs(c.trim(), 8);
                    let mut s: String = masked.chars().take(60).collect();
                    if masked.chars().count() > 60 {
                        s.push('…');
                    }
                    s
                })
                .collect(),
        );
    }

    let mode = match det.amount_mode {
        crate::config::AmountMode::DebitCredit => "debit_credit",
        crate::config::AmountMode::Signed => "signed",
        crate::config::AmountMode::Auto => "auto",
    };

    let mut warnings = raw.meta.warnings.clone();
    warnings.extend(det.warnings.clone());

    InspectReport {
        file_name: raw.meta.file_name.clone(),
        format: raw.meta.format.clone(),
        encoding: raw.meta.encoding.clone(),
        delimiter: raw.meta.delimiter.map(|c| {
            if c == '\t' {
                "\\t".to_string()
            } else {
                c.to_string()
            }
        }),
        sheet_name: raw.meta.sheet_name.clone(),
        sheet_names: raw.meta.sheet_names.clone(),
        total_rows: raw.rows.len() as u32,
        header_row: det.header.header_row.map(|r| (r + 1) as u32),
        data_rows: raw.rows.len().saturating_sub(det.header.data_start) as u32,
        columns: det
            .header
            .columns
            .iter()
            .map(|c| InspectColumn {
                index: (c.index + 1) as u32,
                header: c.header.clone(),
                field: c.field.map(|f| f.name().to_string()),
            })
            .collect(),
        amount_mode: mode.to_string(),
        date_format: det.date_formats.first().cloned(),
        decimal_separator: det.number_format.decimal.to_string(),
        thousands_separator: det.number_format.thousands.map(|c| c.to_string()),
        currency_hint: opts
            .currency
            .clone()
            .or_else(|| det.currency_from_symbol.map(String::from)),
        sample_rows: samples,
        errors: det.errors,
        warnings,
    }
}
