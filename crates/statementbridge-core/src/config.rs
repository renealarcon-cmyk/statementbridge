//! Normalization options — the fully-resolved configuration the engine runs
//! with. Adapters (see `statementbridge-adapters`) and CLI/web flags both
//! reduce to this struct.

use crate::limits::Limits;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Canonical fields a source column can map to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Field {
    Date,
    PostedAt,
    Description,
    Reference,
    Debit,
    Credit,
    Amount,
    Balance,
    Currency,
    Account,
}

impl Field {
    pub const ALL: [Field; 10] = [
        Field::Date,
        Field::PostedAt,
        Field::Description,
        Field::Reference,
        Field::Debit,
        Field::Credit,
        Field::Amount,
        Field::Balance,
        Field::Currency,
        Field::Account,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Field::Date => "date",
            Field::PostedAt => "posted_at",
            Field::Description => "description",
            Field::Reference => "reference",
            Field::Debit => "debit",
            Field::Credit => "credit",
            Field::Amount => "amount",
            Field::Balance => "balance",
            Field::Currency => "currency",
            Field::Account => "account",
        }
    }

    pub fn parse(s: &str) -> Option<Field> {
        Field::ALL.into_iter().find(|f| f.name() == s)
    }
}

/// Reference to a source column: by header name (matched case/accent
/// insensitively) or `Index` (0-based).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ColumnRef {
    Index(usize),
    Name(String),
}

impl ColumnRef {
    /// Parse user syntax: `#3` = 1-based index 3, anything else = header name.
    pub fn parse(s: &str) -> ColumnRef {
        if let Some(rest) = s.strip_prefix('#') {
            if let Ok(n) = rest.parse::<usize>() {
                if n >= 1 {
                    return ColumnRef::Index(n - 1);
                }
            }
        }
        ColumnRef::Name(s.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderHint {
    Auto,
    Ascending,
    Descending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AmountMode {
    Auto,
    DebitCredit,
    Signed,
}

/// Locale hint used only to break genuine ambiguities (e.g. `1,234` with no
/// other evidence). Never overrides evidence found in the data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Locale {
    En,
    Es,
}

#[derive(Debug, Clone)]
pub struct NormalizeOptions {
    pub adapter_id: String,
    /// ISO 4217 override; never invented by the engine.
    pub currency: Option<String>,
    pub locale: Option<Locale>,
    /// chrono formats tried in order; empty = infer from data.
    pub date_formats: Vec<String>,
    pub decimal_separator: Option<char>,
    pub thousands_separator: Option<char>,
    pub order: OrderHint,
    pub amount_mode: AmountMode,
    /// Rows to skip before looking for the header.
    pub rows_to_skip: usize,
    /// Extra case-insensitive footer regexes (defaults always apply).
    pub footer_patterns: Vec<String>,
    /// Extra header aliases per field, merged over the built-in table
    /// (adapter aliases win on conflict).
    pub aliases: BTreeMap<Field, Vec<String>>,
    /// Explicit column mapping; overrides detection.
    pub mapping: BTreeMap<Field, ColumnRef>,
    pub strict: bool,
    /// Redact long digit runs in descriptions/references on export.
    pub redact: bool,
    pub remove_exact_duplicates: bool,
    /// Account masking: minimum digits for a value to count as an account
    /// number, and how many trailing digits to keep (`****NNNN`).
    pub mask_min_digits: usize,
    pub mask_keep_last: usize,
    /// Caller-provided ISO-8601 timestamp (None keeps output deterministic).
    pub imported_at: Option<String>,
    pub limits: Limits,
}

impl Default for NormalizeOptions {
    fn default() -> Self {
        Self {
            adapter_id: "generic-csv".to_string(),
            currency: None,
            locale: None,
            date_formats: Vec::new(),
            decimal_separator: None,
            thousands_separator: None,
            order: OrderHint::Auto,
            amount_mode: AmountMode::Auto,
            rows_to_skip: 0,
            footer_patterns: Vec::new(),
            aliases: BTreeMap::new(),
            mapping: BTreeMap::new(),
            strict: false,
            redact: false,
            remove_exact_duplicates: false,
            mask_min_digits: 6,
            mask_keep_last: 4,
            imported_at: None,
            limits: Limits::default(),
        }
    }
}
