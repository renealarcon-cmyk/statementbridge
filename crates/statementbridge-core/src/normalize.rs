//! The normalization pipeline: RawTable + options → canonical Statement.
//!
//! Stages: header/column detection → format inference (dates, numbers,
//! currency) → per-row parsing → chronological ordering → duplicate
//! classification → balance-chain validation → summary.
//!
//! Every stage is deterministic and every assumption emits a warning.

use crate::config::{AmountMode, Field, NormalizeOptions, OrderHint};
use crate::dates;
use crate::dedupe;
use crate::fingerprint;
use crate::header::{self, HeaderDetection};
use crate::issues::{codes, Issue, WarningCollector};
use crate::mask;
use crate::model::*;
use crate::numbers::{self, NumberFormat};
use crate::raw::RawTable;
use crate::text;
use regex::Regex;
use rust_decimal::Decimal;
use std::collections::BTreeMap;

const DEFAULT_FOOTER_PATTERNS: [&str; 6] = [
    r"^\s*total(es)?\b",
    r"^\s*saldo\s+(final|inicial)\b",
    r"^\s*(closing|opening)\s+balance\b",
    r"^\s*balance\s+final\b",
    r"^\s*sum(a|mary)?\b",
    r"^\s*\*+\s*end\b",
];

const WARNING_CAP: u64 = 100;
const SAMPLE_ROWS: usize = 200;

/// Everything the engine detected before row parsing. Shared by `inspect`.
#[derive(Debug, Clone)]
pub struct Detection {
    pub header: HeaderDetection,
    pub columns: BTreeMap<Field, usize>,
    pub amount_mode: AmountMode,
    pub date_formats: Vec<String>,
    pub date_ambiguous: bool,
    pub number_format: NumberFormat,
    pub number_ambiguous: bool,
    pub currency_from_symbol: Option<&'static str>,
    pub errors: Vec<Issue>,
    pub warnings: Vec<Issue>,
}

pub(crate) fn detect_all(raw: &RawTable, opts: &NormalizeOptions) -> Detection {
    let hd = header::detect(raw, opts);
    let mut warnings = hd.warnings.clone();
    let mut errors = Vec::new();

    let mut columns: BTreeMap<Field, usize> = BTreeMap::new();
    for c in &hd.columns {
        if let Some(f) = c.field {
            columns.entry(f).or_insert(c.index);
        }
    }

    // Resolve amount mode.
    let has_dc = columns.contains_key(&Field::Debit) || columns.contains_key(&Field::Credit);
    let has_amount = columns.contains_key(&Field::Amount);
    let amount_mode = match opts.amount_mode {
        AmountMode::DebitCredit => AmountMode::DebitCredit,
        AmountMode::Signed => AmountMode::Signed,
        AmountMode::Auto => {
            if columns.contains_key(&Field::Debit) && columns.contains_key(&Field::Credit) {
                AmountMode::DebitCredit
            } else if has_amount {
                AmountMode::Signed
            } else if has_dc {
                AmountMode::DebitCredit
            } else {
                AmountMode::Signed // will fail below with no_amount_column
            }
        }
    };
    match amount_mode {
        AmountMode::DebitCredit if !has_dc => {
            errors.push(Issue::new(
                codes::NO_AMOUNT_COLUMN,
                "no debit/credit columns detected; map them manually (e.g. --map debit=<column> --map credit=<column>)",
            ));
        }
        AmountMode::Signed if !has_amount => {
            errors.push(Issue::new(
                codes::NO_AMOUNT_COLUMN,
                "no amount column detected; map it manually (e.g. --map amount=<column>)",
            ));
        }
        _ => {}
    }
    if !columns.contains_key(&Field::Date) {
        errors.push(Issue::new(
            codes::NO_DATE_COLUMN,
            "no date column detected; map it manually (e.g. --map date=<column>)",
        ));
    }

    // Sample data rows for inference.
    let mut date_samples: Vec<&str> = Vec::new();
    let mut number_samples: Vec<&str> = Vec::new();
    let mut currency_symbol: Option<&'static str> = None;
    let numeric_fields = [Field::Debit, Field::Credit, Field::Amount, Field::Balance];
    let mut sampled = 0usize;
    for r in hd.data_start..raw.rows.len() {
        if sampled >= SAMPLE_ROWS {
            break;
        }
        if raw.row_is_empty(r) {
            continue;
        }
        sampled += 1;
        if let Some(dc) = columns.get(&Field::Date) {
            let v = raw.cell(r, *dc).trim();
            if !v.is_empty() {
                date_samples.push(v);
            }
        }
        for f in numeric_fields {
            if let Some(ci) = columns.get(&f) {
                let v = raw.cell(r, *ci).trim();
                if !v.is_empty() {
                    number_samples.push(v);
                    if currency_symbol.is_none() {
                        currency_symbol = numbers::currency_from_symbol(v);
                    }
                }
            }
        }
    }

    let (date_formats, date_ambiguous) = if opts.date_formats.is_empty() {
        let inf = dates::infer_date_formats(&date_samples, opts.locale);
        (inf.formats, inf.ambiguous)
    } else {
        (opts.date_formats.clone(), false)
    };
    if date_ambiguous {
        warnings.push(Issue::new(
            codes::AMBIGUOUS_DATE_FORMAT,
            format!(
                "day/month order is ambiguous in this file; assuming '{}' — set --date-format to override",
                date_formats.first().map(String::as_str).unwrap_or("%Y-%m-%d")
            ),
        ));
    }

    let (number_format, number_ambiguous) = match (opts.decimal_separator, opts.thousands_separator)
    {
        (Some(d), t) => (
            NumberFormat {
                decimal: d,
                thousands: t,
            },
            false,
        ),
        (None, t) => {
            let (mut fmt, amb) = numbers::infer_number_format(&number_samples, opts.locale);
            if let Some(t) = t {
                fmt.thousands = Some(t);
            }
            (fmt, amb)
        }
    };
    if number_ambiguous {
        warnings.push(Issue::new(
            codes::AMBIGUOUS_NUMBER_FORMAT,
            format!(
                "decimal separator is ambiguous in this file; assuming '{}' — set --decimal-separator to override",
                number_format.decimal
            ),
        ));
    }

    // The symbol-derived currency is only used when nothing better exists.
    let symbol_currency_used = currency_symbol.is_some()
        && opts.currency.is_none()
        && !columns.contains_key(&Field::Currency);
    if let (true, Some(code)) = (symbol_currency_used, currency_symbol) {
        warnings.push(Issue::new(
            codes::CURRENCY_FROM_SYMBOL,
            format!("currency {code} inferred from a currency symbol in the amounts; pass an explicit currency to override"),
        ));
    }

    Detection {
        header: hd,
        columns,
        amount_mode,
        date_formats,
        date_ambiguous,
        number_format,
        number_ambiguous,
        currency_from_symbol: currency_symbol,
        errors,
        warnings,
    }
}

fn iso_currency(v: &str) -> Option<String> {
    let t = v.trim().to_uppercase();
    if t.len() == 3 && t.bytes().all(|b| b.is_ascii_uppercase()) {
        Some(t)
    } else {
        None
    }
}

/// Run the full pipeline. Fatal structural problems (no date/amount columns)
/// produce a statement with `validation.status = "invalid"` and no
/// transactions rather than an `Err` — callers still get a full report.
pub fn normalize(raw: &RawTable, opts: &NormalizeOptions) -> Result<Statement, crate::EngineError> {
    let det = detect_all(raw, opts);
    let mut warnings = WarningCollector::new(WARNING_CAP);
    let mut errors: Vec<Issue> = det.errors.clone();
    for w in raw.meta.warnings.iter().chain(det.warnings.iter()) {
        warnings.push(w.clone());
    }

    let footer_res: Result<Vec<Regex>, regex::Error> = DEFAULT_FOOTER_PATTERNS
        .iter()
        .map(|p| (*p).to_string())
        .chain(opts.footer_patterns.iter().cloned())
        .map(|p| {
            regex::RegexBuilder::new(&p)
                .case_insensitive(true)
                .size_limit(1 << 20)
                .build()
        })
        .collect();
    let footers = footer_res
        .map_err(|e| crate::EngineError::InvalidConfig(format!("bad footer pattern: {e}")))?;

    let structural_ok = errors.is_empty();
    let mut txns: Vec<Transaction> = Vec::new();

    if structural_ok {
        parse_rows(
            raw,
            opts,
            &det,
            &footers,
            &mut txns,
            &mut errors,
            &mut warnings,
        );
    }

    // ---- Ordering ----------------------------------------------------------
    let order = detect_order(&txns, opts.order, &mut warnings);
    if order == OrderHint::Descending {
        txns.reverse();
    }

    // ---- Duplicates --------------------------------------------------------
    let dup = dedupe::classify(&mut txns, opts.remove_exact_duplicates, &mut warnings);

    // ---- Balance chain -----------------------------------------------------
    let balance_check = check_balance_chain(&txns, &mut errors);

    // ---- Account / currency aggregation ------------------------------------
    let mut currencies: Vec<String> = Vec::new();
    let mut accounts: Vec<String> = Vec::new();
    for t in &txns {
        if let Some(c) = &t.currency {
            if !currencies.contains(c) {
                currencies.push(c.clone());
            }
        }
    }
    for t in txns.iter() {
        if let Some(m) = t
            .warnings
            .iter()
            .find(|w| w.code == "__account_masked")
            .map(|w| w.message.clone())
        {
            if !accounts.contains(&m) {
                accounts.push(m);
            }
        }
    }
    // Strip internal markers.
    for t in txns.iter_mut() {
        t.warnings.retain(|w| w.code != "__account_masked");
    }
    if currencies.len() > 1 {
        warnings.push(Issue::new(
            codes::MULTIPLE_CURRENCIES,
            format!("multiple currencies present: {}", currencies.join(", ")),
        ));
    }
    if accounts.len() > 1 {
        warnings.push(Issue::new(
            codes::MULTIPLE_ACCOUNTS,
            format!("{} distinct masked accounts present", accounts.len()),
        ));
    }

    // ---- Summary -----------------------------------------------------------
    let mut total_debits = Decimal::ZERO;
    let mut total_credits = Decimal::ZERO;
    for t in &txns {
        if t.amount.is_sign_negative() {
            total_debits += -t.amount;
        } else {
            total_credits += t.amount;
        }
    }
    let opening = txns
        .iter()
        .find(|t| t.balance.is_some())
        .and_then(|t| t.balance.map(|b| b - t.amount));
    let closing = txns.iter().rev().find_map(|t| t.balance);
    let period = match (txns.first(), txns.last()) {
        (Some(f), Some(l)) => Some(Period {
            from: f.date.min(l.date),
            to: l.date.max(f.date),
        }),
        _ => None,
    };

    let summary = Summary {
        transaction_count: txns.len() as u32,
        total_debits: numbers::money(total_debits),
        total_credits: numbers::money(total_credits),
        opening_balance: opening,
        closing_balance: closing,
        exact_duplicates: dup.exact,
        possible_duplicates: dup.possible,
        duplicates_removed: dup.removed,
    };

    let account = Account {
        institution: None,
        account_name: None,
        account_masked: accounts.first().cloned(),
        currency: if currencies.len() == 1 {
            Some(currencies[0].clone())
        } else {
            opts.currency.clone()
        },
    };

    let warnings = warnings.finish();
    let any_txn_warnings = txns.iter().any(|t| !t.warnings.is_empty());
    let status = if !errors.is_empty() {
        ValidationStatus::Invalid
    } else if !warnings.is_empty() || any_txn_warnings {
        ValidationStatus::ValidWithWarnings
    } else {
        ValidationStatus::Valid
    };

    tracing::debug!(
        rows = raw.rows.len(),
        transactions = txns.len(),
        errors = errors.len(),
        warnings = warnings.len(),
        "normalization finished"
    );

    Ok(Statement {
        schema_version: SCHEMA_VERSION.to_string(),
        source: Source {
            file_name: raw.meta.file_name.clone(),
            format: raw.meta.format.clone(),
            adapter_id: opts.adapter_id.clone(),
            imported_at: opts.imported_at.clone(),
            content_hash: raw.meta.content_hash.clone(),
        },
        account,
        period,
        transactions: txns,
        summary,
        validation: Validation {
            status,
            errors,
            warnings,
            balance_check,
        },
    })
}

#[allow(clippy::too_many_arguments)]
fn parse_rows(
    raw: &RawTable,
    opts: &NormalizeOptions,
    det: &Detection,
    footers: &[Regex],
    txns: &mut Vec<Transaction>,
    errors: &mut Vec<Issue>,
    warnings: &mut WarningCollector,
) {
    let col = |f: Field| det.columns.get(&f).copied();
    let get = |r: usize, f: Field| col(f).map(|c| raw.cell(r, c)).unwrap_or("");
    let colname = |f: Field| -> String {
        col(f)
            .and_then(|c| det.header.columns.get(c))
            .map(|c| c.header.clone())
            .unwrap_or_else(|| f.name().to_string())
    };
    let headerless = det.header.header_row.is_none();
    let last_data_row = (det.header.data_start..raw.rows.len())
        .rev()
        .find(|r| !raw.row_is_empty(*r));

    for r in det.header.data_start..raw.rows.len() {
        if raw.row_is_empty(r) {
            continue;
        }
        let row_no = (r + 1) as u32;
        let first_cell = raw.rows[r]
            .iter()
            .find(|c| !c.trim().is_empty())
            .map(String::as_str)
            .unwrap_or("");
        let joined = raw.rows[r].join(" ");
        if footers
            .iter()
            .any(|re| re.is_match(first_cell) || re.is_match(joined.trim()))
        {
            warnings.push(
                Issue::new(codes::FOOTER_ROW_SKIPPED, "footer/summary row skipped").at_row(row_no),
            );
            continue;
        }

        let date_raw = get(r, Field::Date);
        let date = dates::parse_date(date_raw, &det.date_formats);

        // Amounts.
        let mut row_warnings: Vec<Issue> = Vec::new();
        let (debit, credit, amount) = match det.amount_mode {
            AmountMode::DebitCredit => {
                let d_raw = get(r, Field::Debit).trim();
                let c_raw = get(r, Field::Credit).trim();
                let d = parse_opt_amount(d_raw, &det.number_format);
                let c = parse_opt_amount(c_raw, &det.number_format);
                match (d, c) {
                    (Some(Err(e)), _) => {
                        errors.push(
                            Issue::new(codes::INVALID_AMOUNT, format!("invalid debit: {e}"))
                                .at_row(row_no)
                                .at_column(colname(Field::Debit)),
                        );
                        continue;
                    }
                    (_, Some(Err(e))) => {
                        errors.push(
                            Issue::new(codes::INVALID_AMOUNT, format!("invalid credit: {e}"))
                                .at_row(row_no)
                                .at_column(colname(Field::Credit)),
                        );
                        continue;
                    }
                    (None, None) => {
                        handle_row_without_amount(
                            r,
                            row_no,
                            date.is_some(),
                            headerless,
                            det,
                            last_data_row,
                            warnings,
                            errors,
                        );
                        continue;
                    }
                    (d, c) => {
                        let dv = d.and_then(Result::ok).unwrap_or(Decimal::ZERO);
                        let cv = c.and_then(Result::ok).unwrap_or(Decimal::ZERO);
                        if !dv.is_zero() && !cv.is_zero() {
                            errors.push(
                                Issue::new(
                                    codes::DEBIT_AND_CREDIT_SET,
                                    "row has values in both debit and credit; not allowed by the debit/credit rule",
                                )
                                .at_row(row_no),
                            );
                            continue;
                        }
                        let mut dv = dv;
                        let mut cv = cv;
                        if dv.is_sign_negative() && !dv.is_zero() {
                            // A negative debit is economically a credit.
                            row_warnings.push(
                                Issue::new(
                                    codes::NEGATIVE_DEBIT_NORMALIZED,
                                    "negative debit treated as credit",
                                )
                                .at_row(row_no),
                            );
                            cv = -dv;
                            dv = Decimal::ZERO;
                        }
                        if cv.is_sign_negative() && !cv.is_zero() {
                            row_warnings.push(
                                Issue::new(
                                    codes::NEGATIVE_DEBIT_NORMALIZED,
                                    "negative credit treated as debit",
                                )
                                .at_row(row_no),
                            );
                            dv = -cv;
                            cv = Decimal::ZERO;
                        }
                        let amount = cv - dv;
                        let debit = if dv.is_zero() { None } else { Some(dv) };
                        let credit = if cv.is_zero() { None } else { Some(cv) };
                        (debit, credit, amount)
                    }
                }
            }
            AmountMode::Auto | AmountMode::Signed => {
                let a_raw = get(r, Field::Amount).trim();
                match parse_opt_amount(a_raw, &det.number_format) {
                    None => {
                        handle_row_without_amount(
                            r,
                            row_no,
                            date.is_some(),
                            headerless,
                            det,
                            last_data_row,
                            warnings,
                            errors,
                        );
                        continue;
                    }
                    Some(Err(e)) => {
                        errors.push(
                            Issue::new(codes::INVALID_AMOUNT, format!("invalid amount: {e}"))
                                .at_row(row_no)
                                .at_column(colname(Field::Amount)),
                        );
                        continue;
                    }
                    Some(Ok(a)) => {
                        let debit = if a.is_sign_negative() && !a.is_zero() {
                            Some(-a)
                        } else {
                            None
                        };
                        let credit = if a.is_sign_negative() || a.is_zero() {
                            None
                        } else {
                            Some(a)
                        };
                        (debit, credit, a)
                    }
                }
            }
        };

        // Date (required).
        let Some(date) = date else {
            if date_raw.trim().is_empty() {
                errors.push(
                    Issue::new(codes::MISSING_DATE, "row has an amount but no date")
                        .at_row(row_no)
                        .at_column(colname(Field::Date)),
                );
            } else if headerless && r == det.header.data_start {
                // Headerless mode: a first row that parses as neither date nor
                // amount is almost certainly a header after all.
                warnings.push(
                    Issue::new(
                        codes::HEADER_ASSUMED,
                        "first row treated as header (date did not parse)",
                    )
                    .at_row(row_no),
                );
            } else {
                errors.push(
                    Issue::new(
                        codes::INVALID_DATE,
                        format!(
                            "could not parse date '{}' with format '{}'",
                            date_raw.trim().chars().take(24).collect::<String>(),
                            det.date_formats.first().map(String::as_str).unwrap_or("?")
                        ),
                    )
                    .at_row(row_no)
                    .at_column(colname(Field::Date)),
                );
            }
            continue;
        };

        // Optional fields.
        let posted_at = {
            let v = get(r, Field::PostedAt).trim();
            if v.is_empty() {
                None
            } else {
                let p = dates::parse_date(v, &det.date_formats);
                if p.is_none() {
                    row_warnings.push(
                        Issue::new(
                            codes::INVALID_POSTED_AT,
                            "posted date did not parse; left empty",
                        )
                        .at_row(row_no),
                    );
                }
                p
            }
        };

        let (mut description, trunc) =
            text::normalize_text(get(r, Field::Description), opts.limits.max_field_chars);
        if trunc {
            row_warnings
                .push(Issue::new(codes::FIELD_TRUNCATED, "description truncated").at_row(row_no));
        }
        if description.is_empty() && col(Field::Description).is_some() {
            row_warnings
                .push(Issue::new(codes::DESCRIPTION_MISSING, "empty description").at_row(row_no));
        }

        let (mut reference, rtrunc) = text::normalize_text(
            get(r, Field::Reference),
            opts.limits.max_field_chars.min(256),
        );
        if rtrunc {
            row_warnings
                .push(Issue::new(codes::FIELD_TRUNCATED, "reference truncated").at_row(row_no));
        }

        let balance = {
            let v = get(r, Field::Balance).trim();
            if v.is_empty() {
                None
            } else {
                match numbers::parse_decimal(v, &det.number_format) {
                    Ok(b) => Some(b),
                    Err(e) => {
                        row_warnings.push(
                            Issue::new(codes::INVALID_BALANCE, format!("invalid balance: {e}"))
                                .at_row(row_no)
                                .at_column(colname(Field::Balance)),
                        );
                        None
                    }
                }
            }
        };

        let currency = {
            let v = get(r, Field::Currency).trim();
            if v.is_empty() {
                opts.currency
                    .clone()
                    .or_else(|| det.currency_from_symbol.map(String::from))
            } else {
                match iso_currency(v) {
                    Some(c) => Some(c),
                    None => {
                        row_warnings.push(
                            Issue::new(
                                codes::UNKNOWN_CURRENCY,
                                "currency value is not an ISO 4217 code",
                            )
                            .at_row(row_no),
                        );
                        opts.currency.clone()
                    }
                }
            }
        };

        let account_masked = {
            let v = get(r, Field::Account).trim();
            if v.is_empty() {
                None
            } else {
                mask::mask_account(v, opts.mask_min_digits, opts.mask_keep_last)
            }
        };

        if opts.redact {
            description = mask::redact_digit_runs(&description, 8);
            reference = mask::redact_digit_runs(&reference, 8);
        }

        let desc_key = text::matching_key(&description);
        let fp = fingerprint::transaction_fingerprint(
            date,
            &amount,
            &desc_key,
            if reference.is_empty() {
                None
            } else {
                Some(&reference)
            },
            balance.as_ref(),
            account_masked.as_deref(),
        );
        let id = fingerprint::transaction_id(&fp, row_no);

        // Internal marker so aggregation can see the mask without a field.
        let mut tw = row_warnings;
        if let Some(m) = &account_masked {
            tw.push(Issue::new("__account_masked", m.clone()));
        }

        txns.push(Transaction {
            id,
            date,
            posted_at,
            description,
            reference: if reference.is_empty() {
                None
            } else {
                Some(reference)
            },
            debit,
            credit,
            amount,
            balance,
            currency,
            original_row: row_no,
            fingerprint: fp,
            duplicate_of_row: None,
            duplicate_kind: None,
            warnings: tw,
        });
    }
}

fn parse_opt_amount(raw: &str, fmt: &NumberFormat) -> Option<Result<Decimal, String>> {
    let t = raw.trim();
    if t.is_empty() || t == "-" || t == "--" {
        return None;
    }
    Some(numbers::parse_decimal(t, fmt))
}

#[allow(clippy::too_many_arguments)]
fn handle_row_without_amount(
    r: usize,
    row_no: u32,
    has_date: bool,
    headerless: bool,
    det: &Detection,
    last_data_row: Option<usize>,
    warnings: &mut WarningCollector,
    errors: &mut Vec<Issue>,
) {
    if headerless && r == det.header.data_start && !has_date {
        warnings.push(
            Issue::new(
                codes::HEADER_ASSUMED,
                "first row treated as header (no date, no amount)",
            )
            .at_row(row_no),
        );
    } else if !has_date && last_data_row == Some(r) {
        // Trailing narrative/total row that escaped the footer patterns.
        warnings.push(
            Issue::new(
                codes::FOOTER_ROW_SKIPPED,
                "trailing row without date or amount skipped",
            )
            .at_row(row_no),
        );
    } else {
        warnings.push(
            Issue::new(
                codes::MISSING_AMOUNT,
                "row skipped: no debit/credit/amount value",
            )
            .at_row(row_no),
        );
        let _ = errors; // rows without amounts are warnings, not errors
    }
}

/// Detect chronological order. Evidence: date sequence first, then the
/// balance chain direction; ties default to ascending.
fn detect_order(
    txns: &[Transaction],
    hint: OrderHint,
    warnings: &mut WarningCollector,
) -> OrderHint {
    match hint {
        OrderHint::Ascending | OrderHint::Descending => return hint,
        OrderHint::Auto => {}
    }
    if txns.len() < 2 {
        return OrderHint::Ascending;
    }
    let mut inc = 0u32;
    let mut dec = 0u32;
    for w in txns.windows(2) {
        match w[1].date.cmp(&w[0].date) {
            std::cmp::Ordering::Greater => inc += 1,
            std::cmp::Ordering::Less => dec += 1,
            std::cmp::Ordering::Equal => {}
        }
    }
    if inc > dec {
        return OrderHint::Ascending;
    }
    if dec > inc {
        return OrderHint::Descending;
    }
    // Same-date file: use the balance chain to decide.
    let asc_hits = chain_matches(txns, false);
    let desc_hits = chain_matches(txns, true);
    if desc_hits > asc_hits {
        warnings.push(Issue::new(
            codes::ORDER_ASSUMED,
            "transaction order assumed descending from the balance chain",
        ));
        return OrderHint::Descending;
    }
    if asc_hits == desc_hits && asc_hits == 0 && txns.len() > 1 {
        warnings.push(Issue::new(
            codes::ORDER_ASSUMED,
            "transaction order could not be detected; assuming ascending (use --transaction-order to override)",
        ));
    }
    OrderHint::Ascending
}

fn chain_matches(txns: &[Transaction], reversed: bool) -> u32 {
    let mut hits = 0;
    let iter: Vec<&Transaction> = if reversed {
        txns.iter().rev().collect()
    } else {
        txns.iter().collect()
    };
    for w in iter.windows(2) {
        if let (Some(pb), Some(cb)) = (w[0].balance, w[1].balance) {
            if pb + w[1].amount == cb {
                hits += 1;
            }
        }
    }
    hits
}

/// Verify balance[i] == balance[i-1] + amount[i] over consecutive rows that
/// both carry balances. Tolerance: half a unit at the finest scale seen.
fn check_balance_chain(txns: &[Transaction], errors: &mut Vec<Issue>) -> BalanceCheck {
    let mut check = BalanceCheck::default();
    let mut max_scale = 2u32;
    for t in txns {
        max_scale = max_scale.max(t.amount.scale());
        if let Some(b) = t.balance {
            max_scale = max_scale.max(b.scale());
        }
    }
    // 0.5 * 10^-max_scale
    let tolerance = Decimal::new(5, max_scale + 1);

    let mut prev: Option<&Transaction> = None;
    for t in txns {
        if let Some(p) = prev {
            if let (Some(pb), Some(cb)) = (p.balance, t.balance) {
                check.checked_pairs += 1;
                let expected = pb + t.amount;
                let diff = (expected - cb).abs();
                if diff > tolerance {
                    check.mismatches += 1;
                    if check.first_mismatch_row.is_none() {
                        check.first_mismatch_row = Some(t.original_row);
                        check.expected = Some(expected);
                        check.actual = Some(cb);
                        let hint = if t.duplicate_kind == Some(DuplicateKind::Exact) {
                            " (row is an exact duplicate; consider --remove-exact-duplicates)"
                        } else {
                            ""
                        };
                        errors.push(
                            Issue::new(
                                codes::BALANCE_MISMATCH,
                                format!(
                                    "balance chain breaks at row {}: expected {} but file says {}{}",
                                    t.original_row, expected, cb, hint
                                ),
                            )
                            .at_row(t.original_row),
                        );
                    }
                }
            }
        }
        if t.balance.is_some() {
            prev = Some(t);
        }
    }
    check.status = if check.checked_pairs == 0 {
        BalanceStatus::NotAvailable
    } else if check.mismatches == 0 {
        BalanceStatus::Consistent
    } else {
        BalanceStatus::Inconsistent
    };
    check
}
