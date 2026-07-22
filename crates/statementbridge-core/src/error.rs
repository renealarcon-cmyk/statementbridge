//! Fatal engine errors (pre-normalization). Row-level problems are *not*
//! errors here — they land in `validation.errors` of the output statement so
//! users can see everything at once.
//!
//! Messages are actionable and never contain financial field values.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("unsupported file format: {0}")]
    UnsupportedFormat(String),

    #[error("file is empty")]
    EmptyFile,

    #[error("file too large: {size} bytes (limit {limit}). Raise the limit explicitly if this is intentional")]
    FileTooLarge { size: u64, limit: u64 },

    #[error(
        "too many columns: {found} (limit {limit}); this does not look like a statement export"
    )]
    TooManyColumns { found: usize, limit: usize },

    #[error("could not decode text: {0}")]
    Encoding(String),

    #[error("CSV parse error: {0}")]
    Csv(String),

    #[error("XLSX parse error: {0}")]
    Xlsx(String),

    #[error(
        "adapter not found: '{0}'. Run `statementbridge adapters list` to see available adapters"
    )]
    AdapterNotFound(String),

    #[error("invalid adapter '{id}': {reason}")]
    InvalidAdapter { id: String, reason: String },

    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("I/O error: {0}")]
    Io(String),
}

impl EngineError {
    /// Stable machine-readable code for JSON output.
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnsupportedFormat(_) => "unsupported_format",
            Self::EmptyFile => "empty_file",
            Self::FileTooLarge { .. } => "file_too_large",
            Self::TooManyColumns { .. } => "too_many_columns",
            Self::Encoding(_) => "encoding_error",
            Self::Csv(_) => "csv_error",
            Self::Xlsx(_) => "xlsx_error",
            Self::AdapterNotFound(_) => "adapter_not_found",
            Self::InvalidAdapter { .. } => "invalid_adapter",
            Self::InvalidConfig(_) => "invalid_config",
            Self::Io(_) => "io_error",
        }
    }
}
