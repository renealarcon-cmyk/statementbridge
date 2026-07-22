//! Structured, machine-readable errors and warnings.
//!
//! Codes are part of the public contract: adding a code is a minor change,
//! renaming or removing one is breaking. Messages are human-oriented and may
//! change between versions; automation must key on `code`, never on `message`.
//!
//! Privacy rule: messages may quote raw *date* and *amount* tokens (needed to
//! fix files) but must never quote descriptions, references, account numbers
//! or counterparty names.

use serde::{Deserialize, Serialize};

/// Stable issue codes.
pub mod codes {
    // File / structure level.
    pub const ENCODING_ASSUMED: &str = "encoding_assumed";
    pub const DELIMITER_LOW_CONFIDENCE: &str = "delimiter_low_confidence";
    pub const HEADER_ASSUMED: &str = "header_assumed";
    pub const PREAMBLE_ROWS_SKIPPED: &str = "preamble_rows_skipped";
    pub const FOOTER_ROW_SKIPPED: &str = "footer_row_skipped";
    pub const ROWS_TRUNCATED: &str = "rows_truncated";
    pub const MULTIPLE_SHEETS: &str = "multiple_sheets";
    pub const NO_DATE_COLUMN: &str = "no_date_column";
    pub const NO_AMOUNT_COLUMN: &str = "no_amount_column";
    pub const AMBIGUOUS_DATE_FORMAT: &str = "ambiguous_date_format";
    pub const AMBIGUOUS_NUMBER_FORMAT: &str = "ambiguous_number_format";
    pub const ORDER_ASSUMED: &str = "transaction_order_assumed";
    pub const CURRENCY_FROM_SYMBOL: &str = "currency_inferred_from_symbol";
    pub const MULTIPLE_CURRENCIES: &str = "multiple_currencies";
    pub const MULTIPLE_ACCOUNTS: &str = "multiple_accounts";
    pub const WARNINGS_CAPPED: &str = "warnings_capped";

    // Row level.
    pub const INVALID_DATE: &str = "invalid_date";
    pub const MISSING_DATE: &str = "missing_date";
    pub const INVALID_AMOUNT: &str = "invalid_amount";
    pub const MISSING_AMOUNT: &str = "missing_amount";
    pub const DEBIT_AND_CREDIT_SET: &str = "debit_and_credit_both_set";
    pub const NEGATIVE_DEBIT_NORMALIZED: &str = "negative_debit_normalized";
    pub const INVALID_BALANCE: &str = "invalid_balance";
    pub const INVALID_POSTED_AT: &str = "invalid_posted_at";
    pub const FIELD_TRUNCATED: &str = "field_truncated";
    pub const DESCRIPTION_MISSING: &str = "description_missing";
    pub const UNKNOWN_CURRENCY: &str = "unknown_currency";
    pub const CELL_ERROR: &str = "cell_error";

    // Duplicates.
    pub const EXACT_DUPLICATE: &str = "exact_duplicate";
    pub const POSSIBLE_DUPLICATE: &str = "possible_duplicate";
    pub const EXACT_DUPLICATE_REMOVED: &str = "exact_duplicate_removed";

    // Balance chain.
    pub const BALANCE_MISMATCH: &str = "balance_mismatch";
}

/// One error or warning, optionally anchored to a source row/column.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Issue {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub row: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub column: Option<String>,
}

impl Issue {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
            row: None,
            column: None,
        }
    }

    #[must_use]
    pub fn at_row(mut self, row: u32) -> Self {
        self.row = Some(row);
        self
    }

    #[must_use]
    pub fn at_column(mut self, column: impl Into<String>) -> Self {
        self.column = Some(column.into());
        self
    }
}

/// Collects warnings with a per-code cap so a million-row file cannot balloon
/// the report (memory safety + readable output). When the cap is hit, one
/// summary warning per code reports how many more occurrences were suppressed.
#[derive(Debug, Default)]
pub struct WarningCollector {
    warnings: Vec<Issue>,
    counts: std::collections::BTreeMap<String, u64>,
    cap: u64,
}

impl WarningCollector {
    pub fn new(cap_per_code: u64) -> Self {
        Self {
            warnings: Vec::new(),
            counts: std::collections::BTreeMap::new(),
            cap: cap_per_code.max(1),
        }
    }

    pub fn push(&mut self, issue: Issue) {
        let n = self.counts.entry(issue.code.clone()).or_insert(0);
        *n += 1;
        if *n <= self.cap {
            self.warnings.push(issue);
        }
    }

    pub fn count(&self, code: &str) -> u64 {
        self.counts.get(code).copied().unwrap_or(0)
    }

    /// Finish: returns warnings plus one `warnings_capped` summary per capped code.
    pub fn finish(mut self) -> Vec<Issue> {
        for (code, n) in &self.counts {
            if *n > self.cap {
                self.warnings.push(Issue::new(
                    codes::WARNINGS_CAPPED,
                    format!(
                        "{} more '{}' warnings were suppressed (total {})",
                        *n - self.cap,
                        code,
                        n
                    ),
                ));
            }
        }
        self.warnings
    }
}
