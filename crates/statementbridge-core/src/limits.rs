//! Resource limits protecting against oversized or hostile inputs.
//!
//! Every limit is explicit and overridable by the caller; hitting one either
//! fails fast with an actionable error (file size, columns) or degrades
//! loudly with a warning (row truncation, field truncation).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Maximum input size in bytes. Default 100 MiB.
    pub max_file_bytes: u64,
    /// Maximum data rows read. Reading stops here with a `rows_truncated`
    /// warning. Default 1,000,000.
    pub max_rows: usize,
    /// Maximum columns per row; more is treated as a hostile/unsupported file.
    pub max_columns: usize,
    /// Maximum characters kept per field; longer fields are truncated with a
    /// `field_truncated` warning. Default 2,000.
    pub max_field_chars: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_file_bytes: 100 * 1024 * 1024,
            max_rows: 1_000_000,
            max_columns: 256,
            max_field_chars: 2_000,
        }
    }
}
