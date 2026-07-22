//! The declarative adapter document (`adapter.json`), and its conversion into
//! engine `NormalizeOptions`.

use serde::{Deserialize, Serialize};
use statementbridge_core::config::{AmountMode, ColumnRef, Field, NormalizeOptions, OrderHint};
use statementbridge_core::EngineError;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, default)]
pub struct MaskingRules {
    pub min_digits: u32,
    pub keep_last: u32,
}

impl Default for MaskingRules {
    fn default() -> Self {
        Self {
            min_digits: 6,
            keep_last: 4,
        }
    }
}

/// One declarative adapter. See `schemas/statementbridge-adapter-v1.schema.json`
/// and `docs/adapter-authoring.md`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, default)]
pub struct AdapterSpec {
    pub id: String,
    pub version: String,
    pub display_name: String,
    pub description: String,
    pub supported_formats: Vec<String>,
    pub locales: Vec<String>,
    pub expected_headers: Vec<String>,
    pub header_aliases: BTreeMap<String, Vec<String>>,
    pub rows_to_skip: u32,
    pub footer_patterns: Vec<String>,
    pub date_formats: Vec<String>,
    pub decimal_separator: Option<String>,
    pub thousands_separator: Option<String>,
    pub transaction_order: String,
    pub currency: Option<String>,
    pub amount_mode: String,
    pub date_column: Option<String>,
    pub debit_column: Option<String>,
    pub credit_column: Option<String>,
    pub signed_amount_column: Option<String>,
    pub balance_column: Option<String>,
    pub reference_column: Option<String>,
    pub description_column: Option<String>,
    pub currency_column: Option<String>,
    pub account_column: Option<String>,
    pub account_masking_rules: MaskingRules,
    pub detection_priority: u32,
}

impl Default for AdapterSpec {
    fn default() -> Self {
        Self {
            id: String::new(),
            version: "0.1.0".to_string(),
            display_name: String::new(),
            description: String::new(),
            supported_formats: vec!["csv".to_string(), "xlsx".to_string()],
            locales: Vec::new(),
            expected_headers: Vec::new(),
            header_aliases: BTreeMap::new(),
            rows_to_skip: 0,
            footer_patterns: Vec::new(),
            date_formats: Vec::new(),
            decimal_separator: None,
            thousands_separator: None,
            transaction_order: "auto".to_string(),
            currency: None,
            amount_mode: "auto".to_string(),
            date_column: None,
            debit_column: None,
            credit_column: None,
            signed_amount_column: None,
            balance_column: None,
            reference_column: None,
            description_column: None,
            currency_column: None,
            account_column: None,
            account_masking_rules: MaskingRules::default(),
            detection_priority: 0,
        }
    }
}

fn sep_char(s: &Option<String>) -> Option<char> {
    s.as_ref().and_then(|v| v.chars().next())
}

impl AdapterSpec {
    pub fn order(&self) -> OrderHint {
        match self.transaction_order.as_str() {
            "ascending" => OrderHint::Ascending,
            "descending" => OrderHint::Descending,
            _ => OrderHint::Auto,
        }
    }

    pub fn mode(&self) -> AmountMode {
        match self.amount_mode.as_str() {
            "debit_credit" => AmountMode::DebitCredit,
            "signed" => AmountMode::Signed,
            _ => AmountMode::Auto,
        }
    }

    /// Merge this adapter into a base `NormalizeOptions` (typically default
    /// options already carrying user CLI/web overrides for strict/redact…).
    /// User-provided values in `base` win over adapter values.
    pub fn apply(&self, base: &NormalizeOptions) -> Result<NormalizeOptions, EngineError> {
        let mut o = base.clone();
        o.adapter_id = self.id.clone();
        if o.currency.is_none() {
            o.currency = self.currency.clone();
        }
        if o.date_formats.is_empty() {
            o.date_formats = self.date_formats.clone();
        }
        if o.decimal_separator.is_none() {
            o.decimal_separator = sep_char(&self.decimal_separator);
        }
        if o.thousands_separator.is_none() {
            o.thousands_separator = sep_char(&self.thousands_separator);
        }
        if o.order == OrderHint::Auto {
            o.order = self.order();
        }
        if o.amount_mode == AmountMode::Auto {
            o.amount_mode = self.mode();
        }
        if o.rows_to_skip == 0 {
            o.rows_to_skip = self.rows_to_skip as usize;
        }
        o.mask_min_digits = self.account_masking_rules.min_digits as usize;
        o.mask_keep_last = self.account_masking_rules.keep_last as usize;
        o.footer_patterns
            .extend(self.footer_patterns.iter().cloned());

        for (field_name, aliases) in &self.header_aliases {
            let field = Field::parse(field_name).ok_or_else(|| EngineError::InvalidAdapter {
                id: self.id.clone(),
                reason: format!("unknown field in header_aliases: '{field_name}'"),
            })?;
            o.aliases
                .entry(field)
                .or_default()
                .extend(aliases.iter().cloned());
        }

        let columns: [(Field, &Option<String>); 9] = [
            (Field::Date, &self.date_column),
            (Field::Debit, &self.debit_column),
            (Field::Credit, &self.credit_column),
            (Field::Amount, &self.signed_amount_column),
            (Field::Balance, &self.balance_column),
            (Field::Reference, &self.reference_column),
            (Field::Description, &self.description_column),
            (Field::Currency, &self.currency_column),
            (Field::Account, &self.account_column),
        ];
        for (field, col) in columns {
            if let Some(name) = col {
                // User mapping (already in base) wins.
                o.mapping
                    .entry(field)
                    .or_insert_with(|| ColumnRef::parse(name));
            }
        }
        Ok(o)
    }
}
