//! XLSX reading via calamine.
//!
//! Notes and guarantees:
//! * calamine reads cell *values*; macros, external links and embedded
//!   objects are never executed or followed (they are simply not read).
//! * Formula cells contribute their cached value when the workbook stores
//!   one; otherwise the cell reads as empty and normalization reports the
//!   missing value.
//! * XLSX stores numbers as IEEE-754 doubles by format design. Values are
//!   converted with shortest round-trip formatting (`1234.56` stays
//!   `"1234.56"`) and then parsed as exact decimals downstream. This is the
//!   only place floats can appear, it is inherent to XLSX, and it is
//!   documented in docs/canonical-schema.md.

use crate::ReadOptions;
use calamine::{Data, Reader, Xlsx};
use statementbridge_core::issues::{codes, Issue};
use statementbridge_core::raw::{RawMeta, RawTable};
use statementbridge_core::EngineError;
use std::io::Cursor;

fn cell_to_string(d: &Data, error_cells: &mut u32) -> String {
    match d {
        Data::Empty => String::new(),
        Data::String(s) => s.clone(),
        Data::Float(f) => f.to_string(),
        Data::Int(i) => i.to_string(),
        Data::Bool(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
        Data::DateTime(dt) => match dt.as_datetime() {
            Some(ndt) => {
                if ndt.time() == chrono::NaiveTime::MIN {
                    ndt.date().format("%Y-%m-%d").to_string()
                } else {
                    ndt.format("%Y-%m-%d %H:%M:%S").to_string()
                }
            }
            None => dt.as_f64().to_string(),
        },
        Data::DateTimeIso(s) => s.clone(),
        Data::DurationIso(s) => s.clone(),
        Data::Error(_) => {
            *error_cells += 1;
            String::new()
        }
    }
}

pub fn read(bytes: &[u8], opts: &ReadOptions) -> Result<RawTable, EngineError> {
    let mut wb: Xlsx<_> =
        Xlsx::new(Cursor::new(bytes.to_vec())).map_err(|e| EngineError::Xlsx(e.to_string()))?;
    let sheet_names: Vec<String> = wb.sheet_names().to_vec();
    if sheet_names.is_empty() {
        return Err(EngineError::Xlsx("workbook has no sheets".to_string()));
    }

    let mut warnings: Vec<Issue> = Vec::new();

    // Choose sheet: explicit name, or the sheet with the most non-empty rows.
    let chosen: String = match &opts.sheet {
        Some(name) => {
            if !sheet_names.iter().any(|s| s == name) {
                return Err(EngineError::InvalidConfig(format!(
                    "sheet '{}' not found; available sheets: {}",
                    name,
                    sheet_names.join(", ")
                )));
            }
            name.clone()
        }
        None => {
            let mut best: (String, usize) = (sheet_names[0].clone(), 0);
            let mut non_empty_sheets = 0;
            for name in &sheet_names {
                let rows = match wb.worksheet_range(name) {
                    Ok(range) => range
                        .rows()
                        .filter(|r| r.iter().any(|c| !matches!(c, Data::Empty)))
                        .count(),
                    Err(_) => 0,
                };
                if rows > 0 {
                    non_empty_sheets += 1;
                }
                if rows > best.1 {
                    best = (name.clone(), rows);
                }
            }
            if non_empty_sheets > 1 {
                warnings.push(Issue::new(
                    codes::MULTIPLE_SHEETS,
                    format!(
                        "workbook has {} non-empty sheets; using '{}' (pass an explicit sheet to override)",
                        non_empty_sheets, best.0
                    ),
                ));
            }
            best.0
        }
    };

    let range = wb
        .worksheet_range(&chosen)
        .map_err(|e| EngineError::Xlsx(e.to_string()))?;

    // Align provenance with real sheet rows: pad up to the range start.
    let start_row = range.start().map(|(r, _)| r as usize).unwrap_or(0);
    let mut rows: Vec<Vec<String>> = Vec::with_capacity(range.height() + start_row);
    rows.resize(start_row, Vec::new());

    let mut truncated = false;
    let mut error_cells = 0u32;
    let hard_cell_cap = 32 * 1024;
    for row in range.rows() {
        if row.len() > opts.limits.max_columns {
            return Err(EngineError::TooManyColumns {
                found: row.len(),
                limit: opts.limits.max_columns,
            });
        }
        if rows.len() >= opts.limits.max_rows {
            truncated = true;
            break;
        }
        rows.push(
            row.iter()
                .map(|c| {
                    let s = cell_to_string(c, &mut error_cells);
                    if s.chars().count() > hard_cell_cap {
                        s.chars().take(hard_cell_cap).collect()
                    } else {
                        s
                    }
                })
                .collect(),
        );
    }
    if truncated {
        warnings.push(Issue::new(
            codes::ROWS_TRUNCATED,
            format!(
                "reading stopped at the row limit ({})",
                opts.limits.max_rows
            ),
        ));
    }
    if error_cells > 0 {
        warnings.push(Issue::new(
            codes::CELL_ERROR,
            format!("{error_cells} cell(s) contain spreadsheet errors (#N/A, #DIV/0!…) and were read as empty"),
        ));
    }

    Ok(RawTable {
        rows,
        meta: RawMeta {
            sheet_name: Some(chosen),
            sheet_names,
            truncated,
            warnings,
            ..RawMeta::default()
        },
    })
}
