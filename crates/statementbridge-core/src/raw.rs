//! Format-agnostic raw table handed from readers (CSV/XLSX) to the engine.

use crate::issues::Issue;

#[derive(Debug, Clone, Default)]
pub struct RawMeta {
    pub file_name: String,
    /// `csv` or `xlsx`.
    pub format: String,
    /// e.g. `utf-8`, `utf-8-bom`, `windows-1252`, `iso-8859-1`, `utf-16le`.
    pub encoding: Option<String>,
    pub delimiter: Option<char>,
    pub sheet_name: Option<String>,
    pub sheet_names: Vec<String>,
    /// SHA-256 (lowercase hex) of the original bytes.
    pub content_hash: String,
    pub byte_len: u64,
    /// True when reading stopped at the row limit.
    pub truncated: bool,
    /// Warnings produced while reading (encoding assumptions, sheet choice…).
    pub warnings: Vec<Issue>,
}

/// Rows exactly as read (no trimming, no reordering). `rows[i]` corresponds to
/// source row `i + 1` (1-based provenance).
#[derive(Debug, Clone, Default)]
pub struct RawTable {
    pub rows: Vec<Vec<String>>,
    pub meta: RawMeta,
}

impl RawTable {
    pub fn cell(&self, row: usize, col: usize) -> &str {
        self.rows
            .get(row)
            .and_then(|r| r.get(col))
            .map(String::as_str)
            .unwrap_or("")
    }

    pub fn row_is_empty(&self, row: usize) -> bool {
        self.rows
            .get(row)
            .map(|r| r.iter().all(|c| c.trim().is_empty()))
            .unwrap_or(true)
    }
}
