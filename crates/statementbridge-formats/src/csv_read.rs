//! CSV reading with delimiter detection.

use crate::encoding;
use crate::ReadOptions;
use statementbridge_core::issues::{codes, Issue};
use statementbridge_core::raw::{RawMeta, RawTable};
use statementbridge_core::EngineError;

const CANDIDATES: [char; 4] = [',', ';', '\t', '|'];
const DETECT_LINES: usize = 40;

/// Score a candidate delimiter over the first lines: we want a consistent
/// field count > 1. Returns (consistent_lines, mode_field_count).
fn score(sample: &str, delim: char) -> (usize, usize) {
    let mut counts: Vec<usize> = Vec::new();
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(delim as u8)
        .has_headers(false)
        .flexible(true)
        .from_reader(sample.as_bytes());
    for rec in rdr.records().take(DETECT_LINES).flatten() {
        counts.push(rec.len());
    }
    if counts.is_empty() {
        return (0, 1);
    }
    // Mode of field counts.
    let mut freq: std::collections::BTreeMap<usize, usize> = std::collections::BTreeMap::new();
    for c in &counts {
        *freq.entry(*c).or_insert(0) += 1;
    }
    let (&mode, &mode_n) = freq
        .iter()
        .max_by_key(|(count, n)| (**n, **count))
        .unwrap_or((&1, &0));
    if mode < 2 {
        return (0, mode);
    }
    (mode_n, mode)
}

pub fn detect_delimiter(text: &str) -> (char, bool) {
    // Use a prefix that ends on a line boundary to avoid a torn last line.
    let sample: String = text
        .lines()
        .take(DETECT_LINES)
        .collect::<Vec<_>>()
        .join("\n");
    let mut best = (',', 0usize, 0usize); // (delim, consistent, width)
    for d in CANDIDATES {
        let (consistent, width) = score(&sample, d);
        // Prefer more consistent lines; break ties by wider rows, then by
        // candidate order (deterministic).
        if consistent > best.1 || (consistent == best.1 && width > best.2) {
            best = (d, consistent, width);
        }
    }
    let confident = best.1 >= 2 && best.2 >= 2;
    (best.0, confident)
}

pub fn read(bytes: &[u8], opts: &ReadOptions) -> Result<RawTable, EngineError> {
    let decoded = encoding::decode(bytes)?;
    let mut warnings: Vec<Issue> = Vec::new();
    if let Some(w) = decoded.warning {
        warnings.push(w);
    }

    let (delimiter, confident) = match opts.delimiter {
        Some(d) => (d, true),
        None => detect_delimiter(&decoded.text),
    };
    if !confident {
        warnings.push(Issue::new(
            codes::DELIMITER_LOW_CONFIDENCE,
            format!(
                "delimiter detection is low-confidence (assumed '{}'); pass an explicit delimiter if columns look wrong",
                if delimiter == '\t' { "\\t".to_string() } else { delimiter.to_string() }
            ),
        ));
    }

    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(delimiter as u8)
        .has_headers(false)
        .flexible(true)
        .from_reader(decoded.text.as_bytes());

    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut truncated = false;
    let hard_cell_cap = 32 * 1024; // chars; memory guard before core's soft cap
                                   // Recover *real* file line numbers (the csv crate skips blank lines and
                                   // its Position does not count them): map byte offsets to lines, and skip
                                   // past blank lines the parser attributed to the next record's start.
    let mut line_starts: Vec<usize> = vec![0];
    for (i, b) in decoded.text.bytes().enumerate() {
        if b == b'\n' {
            line_starts.push(i + 1);
        }
    }
    let line_blank: Vec<bool> = decoded.text.lines().map(|l| l.trim().is_empty()).collect();
    for rec in rdr.records() {
        let rec = rec.map_err(|e| EngineError::Csv(e.to_string()))?;
        let mut line = rec
            .position()
            .map(|p| line_starts.partition_point(|s| *s <= p.byte() as usize))
            .unwrap_or(rows.len() + 1);
        while line_blank.get(line - 1).copied().unwrap_or(false) {
            line += 1;
        }
        // Pad so `rows[i]` is always file line `i + 1`.
        while rows.len() + 1 < line && rows.len() < opts.limits.max_rows {
            rows.push(Vec::new());
        }
        if rec.len() > opts.limits.max_columns {
            return Err(EngineError::TooManyColumns {
                found: rec.len(),
                limit: opts.limits.max_columns,
            });
        }
        if rows.len() >= opts.limits.max_rows {
            truncated = true;
            break;
        }
        rows.push(
            rec.iter()
                .map(|c| {
                    if c.chars().count() > hard_cell_cap {
                        c.chars().take(hard_cell_cap).collect()
                    } else {
                        c.to_string()
                    }
                })
                .collect(),
        );
    }
    if truncated {
        warnings.push(Issue::new(
            codes::ROWS_TRUNCATED,
            format!(
                "reading stopped at the row limit ({}); increase the limit to process the whole file",
                opts.limits.max_rows
            ),
        ));
    }

    Ok(RawTable {
        rows,
        meta: RawMeta {
            encoding: Some(decoded.encoding.to_string()),
            delimiter: Some(delimiter),
            truncated,
            warnings,
            ..RawMeta::default()
        },
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn detects_common_delimiters() {
        assert_eq!(detect_delimiter("a,b,c\n1,2,3\n4,5,6\n").0, ',');
        assert_eq!(detect_delimiter("a;b;c\n1;2;3\n").0, ';');
        assert_eq!(detect_delimiter("a\tb\tc\n1\t2\t3\n").0, '\t');
        assert_eq!(detect_delimiter("a|b|c\n1|2|3\n").0, '|');
    }

    #[test]
    fn semicolon_with_decimal_commas() {
        // Commas appear inside numbers; semicolon must still win.
        let text = "Fecha;Concepto;Importe\n01/02/2026;Pago;1.234,56\n02/02/2026;Abono;99,10\n";
        assert_eq!(detect_delimiter(text).0, ';');
    }

    #[test]
    fn blank_lines_keep_real_line_numbers() {
        let text = b"a,b,c\nx,y,z\n\nFecha,Concepto,Importe\n01/02/2026,Pago,-10.00\n";
        let t = read(text, &ReadOptions::default()).unwrap();
        assert_eq!(t.rows.len(), 5);
        assert!(t.rows[2].is_empty()); // the blank line holds its slot
        assert_eq!(t.rows[3][0], "Fecha");
        assert_eq!(t.rows[4][0], "01/02/2026");
    }

    #[test]
    fn quoted_fields_do_not_break_detection() {
        let text = "date,desc,amount\n2026-01-01,\"a, b, c\",5.00\n2026-01-02,x,6.00\n";
        assert_eq!(detect_delimiter(text).0, ',');
    }
}
