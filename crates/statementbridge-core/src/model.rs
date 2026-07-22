//! Canonical statement model, mirroring `schemas/statementbridge-v1.schema.json`.
//!
//! Monetary values are `rust_decimal::Decimal` in memory and decimal strings in
//! JSON (`rust_decimal::serde::str*`). Dates are `chrono::NaiveDate`
//! (ISO `YYYY-MM-DD` in JSON).

use crate::issues::Issue;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Version of the canonical output schema (not the crate version).
pub const SCHEMA_VERSION: &str = "1.0";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Statement {
    pub schema_version: String,
    pub source: Source,
    pub account: Account,
    pub period: Option<Period>,
    pub transactions: Vec<Transaction>,
    pub summary: Summary,
    pub validation: Validation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub file_name: String,
    /// `csv` or `xlsx`.
    pub format: String,
    pub adapter_id: String,
    /// ISO-8601 timestamp supplied by the caller; `None` keeps output
    /// deterministic (the engine never reads the clock).
    pub imported_at: Option<String>,
    /// SHA-256 of the input bytes (lowercase hex).
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Account {
    pub institution: Option<String>,
    pub account_name: Option<String>,
    /// Masked identifier such as `****4821`; full numbers are never emitted.
    pub account_masked: Option<String>,
    /// ISO 4217 code when known; never invented.
    pub currency: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Period {
    pub from: NaiveDate,
    pub to: NaiveDate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Deterministic id: `sb1-` + first 16 hex chars of
    /// SHA-256(fingerprint ":" original_row).
    pub id: String,
    pub date: NaiveDate,
    pub posted_at: Option<NaiveDate>,
    pub description: String,
    pub reference: Option<String>,
    /// Debit magnitude (non-negative) when the movement is a debit.
    #[serde(with = "rust_decimal::serde::str_option", default)]
    pub debit: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::str_option", default)]
    pub credit: Option<Decimal>,
    /// Canonical signed amount: debits negative, credits positive.
    #[serde(with = "rust_decimal::serde::str")]
    pub amount: Decimal,
    #[serde(with = "rust_decimal::serde::str_option", default)]
    pub balance: Option<Decimal>,
    pub currency: Option<String>,
    /// 1-based row number in the source file (provenance).
    pub original_row: u32,
    /// SHA-256 fingerprint over normalized (date, amount, description,
    /// reference, balance, account) — see `fingerprint` module.
    pub fingerprint: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub duplicate_of_row: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub duplicate_kind: Option<DuplicateKind>,
    pub warnings: Vec<Issue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DuplicateKind {
    /// Identical normalized fields, including balance/reference when present.
    Exact,
    /// Same date, amount and description, and nothing (reference, balance)
    /// proves the rows are distinct. Flagged for human review, never removed
    /// automatically.
    Possible,
    /// Looks similar but evidence (distinct reference or distinct balance)
    /// says it is a legitimately repeated transaction. Informational only.
    Repeated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    pub transaction_count: u32,
    /// Sum of debit magnitudes (non-negative).
    #[serde(with = "rust_decimal::serde::str")]
    pub total_debits: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub total_credits: Decimal,
    #[serde(with = "rust_decimal::serde::str_option", default)]
    pub opening_balance: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::str_option", default)]
    pub closing_balance: Option<Decimal>,
    pub exact_duplicates: u32,
    pub possible_duplicates: u32,
    pub duplicates_removed: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationStatus {
    Valid,
    ValidWithWarnings,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BalanceStatus {
    Consistent,
    Inconsistent,
    /// No balance column, or fewer than two consecutive balances to check.
    NotAvailable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceCheck {
    pub status: BalanceStatus,
    /// Number of consecutive (previous balance, amount, balance) pairs checked.
    pub checked_pairs: u32,
    pub mismatches: u32,
    pub first_mismatch_row: Option<u32>,
    /// Expected balance at the first mismatching row.
    #[serde(with = "rust_decimal::serde::str_option", default)]
    pub expected: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::str_option", default)]
    pub actual: Option<Decimal>,
}

impl Default for BalanceCheck {
    fn default() -> Self {
        Self {
            status: BalanceStatus::NotAvailable,
            checked_pairs: 0,
            mismatches: 0,
            first_mismatch_row: None,
            expected: None,
            actual: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Validation {
    pub status: ValidationStatus,
    pub errors: Vec<Issue>,
    pub warnings: Vec<Issue>,
    pub balance_check: BalanceCheck,
}
