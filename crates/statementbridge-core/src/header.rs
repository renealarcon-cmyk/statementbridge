//! Header detection and column-role identification.
//!
//! Finds the real header row even when informational rows precede it, then
//! maps each column to a canonical field using a built-in English/Spanish
//! alias table merged with adapter/user aliases. Explicit mappings always win.

use crate::config::{ColumnRef, Field, NormalizeOptions};
use crate::issues::{codes, Issue};
use crate::raw::RawTable;
use crate::text::matching_key;
use std::collections::BTreeMap;

/// Built-in alias table (already in `matching_key` form: lowercase, no accents).
pub fn default_aliases() -> BTreeMap<Field, Vec<&'static str>> {
    let mut m = BTreeMap::new();
    m.insert(
        Field::Date,
        vec![
            "date",
            "fecha",
            "transaction date",
            "fecha operacion",
            "fecha de operacion",
            "posting date",
            "value date",
            "fecha valor",
            "f. valor",
            "f.valor",
            "fecha mov",
            "fecha movimiento",
            "operation date",
            "fecha transaccion",
            "fecha de transaccion",
            "dia",
            "fech",
        ],
    );
    m.insert(
        Field::PostedAt,
        vec![
            "posted",
            "posted date",
            "fecha contable",
            "booking date",
            "fecha aplicacion",
        ],
    );
    m.insert(
        Field::Description,
        vec![
            "description",
            "descripcion",
            "concepto",
            "detalle",
            "details",
            "memo",
            "narrative",
            "transaction details",
            "descripcion del movimiento",
            "concept",
            "detalle de la operacion",
            "particulars",
            "transaction description",
            "movimiento",
        ],
    );
    m.insert(
        Field::Reference,
        vec![
            "reference",
            "referencia",
            "ref",
            "ref.",
            "document",
            "documento",
            "doc",
            "check number",
            "numero de documento",
            "no documento",
            "n documento",
            "transaction id",
            "id transaccion",
            "folio",
            "comprobante",
            "num. ref",
        ],
    );
    m.insert(
        Field::Debit,
        vec![
            "debit",
            "debito",
            "cargo",
            "cargos",
            "withdrawal",
            "withdrawals",
            "debe",
            "salida",
            "salidas",
            "egreso",
            "egresos",
            "retiro",
            "retiros",
            "debit amount",
            "money out",
            "paid out",
            "importe cargo",
        ],
    );
    m.insert(
        Field::Credit,
        vec![
            "credit",
            "credito",
            "abono",
            "abonos",
            "deposit",
            "deposits",
            "haber",
            "entrada",
            "entradas",
            "ingreso",
            "ingresos",
            "deposito",
            "depositos",
            "credit amount",
            "money in",
            "paid in",
            "importe abono",
        ],
    );
    m.insert(
        Field::Amount,
        vec![
            "amount",
            "importe",
            "monto",
            "cantidad",
            "valor",
            "value",
            "amount (usd)",
            "importe (eur)",
            "monto (usd)",
            "signed amount",
            "transaction amount",
            "amt",
        ],
    );
    m.insert(
        Field::Balance,
        vec![
            "balance",
            "saldo",
            "running balance",
            "saldo disponible",
            "available balance",
            "new balance",
            "saldo actual",
            "closing balance",
            "saldo final",
            "balance (usd)",
        ],
    );
    m.insert(
        Field::Currency,
        vec!["currency", "moneda", "divisa", "ccy", "cur"],
    );
    m.insert(
        Field::Account,
        vec![
            "account",
            "cuenta",
            "account number",
            "numero de cuenta",
            "no cuenta",
            "n cuenta",
            "iban",
            "account id",
            "nro cuenta",
        ],
    );
    m
}

#[derive(Debug, Clone)]
pub struct ColumnInfo {
    /// 0-based column index.
    pub index: usize,
    /// Header text as read (may be synthetic `column_N` for headerless files).
    pub header: String,
    pub field: Option<Field>,
}

#[derive(Debug, Clone)]
pub struct HeaderDetection {
    /// 0-based row index of the header row, if one was found.
    pub header_row: Option<usize>,
    /// 0-based row index where data starts.
    pub data_start: usize,
    pub columns: Vec<ColumnInfo>,
    pub warnings: Vec<Issue>,
}

/// Match strength of a header cell against an alias list.
fn match_score(cell_key: &str, alias_key: &str) -> u32 {
    if cell_key == alias_key {
        return 3;
    }
    if let Some(rest) = cell_key.strip_prefix(alias_key) {
        // "fecha valor" starts with "fecha"; require a word boundary.
        if rest.starts_with([' ', ':', '.', '(', '/']) {
            return 2;
        }
    }
    // Whole-word containment, e.g. "total amount (usd)" contains "amount".
    if cell_key
        .split([' ', '(', ')', '/', ':'])
        .any(|w| w == alias_key)
    {
        return 1;
    }
    0
}

/// Score a whole row as a potential header: how many distinct fields match.
fn row_field_matches(
    cells: &[String],
    aliases: &BTreeMap<Field, Vec<String>>,
) -> BTreeMap<Field, (u32, usize)> {
    // field -> (best score, column index); leftmost column wins ties.
    let mut out: BTreeMap<Field, (u32, usize)> = BTreeMap::new();
    for (ci, cell) in cells.iter().enumerate() {
        let key = matching_key(cell);
        if key.is_empty() {
            continue;
        }
        for (field, al) in aliases {
            let mut best = 0;
            for a in al {
                best = best.max(match_score(&key, a));
            }
            if best > 0 {
                let entry = out.entry(*field).or_insert((0, ci));
                if best > entry.0 {
                    *entry = (best, ci);
                }
            }
        }
    }
    out
}

fn merged_aliases(opts: &NormalizeOptions) -> BTreeMap<Field, Vec<String>> {
    let mut merged: BTreeMap<Field, Vec<String>> = BTreeMap::new();
    for (f, list) in default_aliases() {
        merged.insert(f, list.into_iter().map(matching_key).collect());
    }
    for (f, list) in &opts.aliases {
        let e = merged.entry(*f).or_default();
        // Adapter aliases first so they win equal-score ties.
        let mut v: Vec<String> = list.iter().map(|s| matching_key(s)).collect();
        v.append(e);
        *e = v;
    }
    merged
}

/// Detect header row and map columns. See module docs for the algorithm.
pub fn detect(raw: &RawTable, opts: &NormalizeOptions) -> HeaderDetection {
    let mut warnings = Vec::new();
    let aliases = merged_aliases(opts);
    let scan_from = opts.rows_to_skip.min(raw.rows.len());
    let scan_to = (scan_from + 50).min(raw.rows.len());

    let mut header_row: Option<usize> = None;
    for r in scan_from..scan_to {
        if raw.row_is_empty(r) {
            continue;
        }
        let matches = row_field_matches(&raw.rows[r], &aliases);
        let has_anchor = matches.contains_key(&Field::Date)
            || matches.contains_key(&Field::Amount)
            || (matches.contains_key(&Field::Debit) || matches.contains_key(&Field::Credit));
        if matches.len() >= 2 && has_anchor {
            header_row = Some(r);
            break;
        }
    }

    // Explicit index-based mapping can drive a headerless file.
    let index_mapped = !opts.mapping.is_empty()
        && opts
            .mapping
            .values()
            .all(|c| matches!(c, ColumnRef::Index(_)));

    let (header_row, data_start, headers): (Option<usize>, usize, Vec<String>) = match header_row {
        Some(r) => {
            if r > scan_from {
                warnings.push(Issue::new(
                    codes::PREAMBLE_ROWS_SKIPPED,
                    format!(
                        "{} informational row(s) before the header were skipped",
                        r - scan_from
                    ),
                ));
            }
            (
                Some(r),
                r + 1,
                raw.rows[r].iter().map(|c| c.trim().to_string()).collect(),
            )
        }
        None if index_mapped => {
            let width = raw.rows.get(scan_from).map(|r| r.len()).unwrap_or(0);
            let names = (0..width).map(|i| format!("column_{}", i + 1)).collect();
            (None, scan_from, names)
        }
        None => {
            // Assume the first non-empty scanned row is the header.
            let r = (scan_from..scan_to)
                .find(|r| !raw.row_is_empty(*r))
                .unwrap_or(scan_from);
            warnings.push(Issue::new(
                codes::HEADER_ASSUMED,
                format!(
                    "no header row matched known column names; assuming row {} is the header — check the column mapping",
                    r + 1
                ),
            ));
            (
                Some(r),
                r + 1,
                raw.rows
                    .get(r)
                    .map(|c| c.iter().map(|x| x.trim().to_string()).collect())
                    .unwrap_or_default(),
            )
        }
    };

    // Assign fields to columns.
    let mut columns: Vec<ColumnInfo> = headers
        .iter()
        .enumerate()
        .map(|(i, h)| ColumnInfo {
            index: i,
            header: h.clone(),
            field: None,
        })
        .collect();

    // 1. Alias-based candidates: (score, column, field), assigned greedily.
    let mut candidates: Vec<(u32, usize, Field)> = Vec::new();
    for (ci, h) in headers.iter().enumerate() {
        let key = matching_key(h);
        if key.is_empty() {
            continue;
        }
        for (field, al) in &aliases {
            let mut best = 0;
            for a in al {
                best = best.max(match_score(&key, a));
            }
            if best > 0 {
                candidates.push((best, ci, *field));
            }
        }
    }
    // Deterministic order: score desc, column asc, field order.
    candidates.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    let mut field_taken: BTreeMap<Field, usize> = BTreeMap::new();
    let mut col_taken: Vec<bool> = vec![false; columns.len()];
    for (_, ci, field) in &candidates {
        if col_taken[*ci] || field_taken.contains_key(field) {
            // Second date-like column becomes posted_at if free.
            if *field == Field::Date
                && !col_taken[*ci]
                && !field_taken.contains_key(&Field::PostedAt)
            {
                field_taken.insert(Field::PostedAt, *ci);
                col_taken[*ci] = true;
                columns[*ci].field = Some(Field::PostedAt);
            }
            continue;
        }
        field_taken.insert(*field, *ci);
        col_taken[*ci] = true;
        columns[*ci].field = Some(*field);
    }

    // 2. Explicit mapping overrides everything.
    for (field, cref) in &opts.mapping {
        let target = match cref {
            ColumnRef::Index(i) => Some(*i),
            ColumnRef::Name(name) => {
                let want = matching_key(name);
                columns.iter().position(|c| matching_key(&c.header) == want)
            }
        };
        if let Some(ci) = target {
            if ci < columns.len() {
                for c in columns.iter_mut() {
                    if c.field == Some(*field) {
                        c.field = None;
                    }
                }
                columns[ci].field = Some(*field);
            }
        } else {
            warnings.push(Issue::new(
                codes::HEADER_ASSUMED,
                format!(
                    "mapped column for field '{}' not found in the header",
                    field.name()
                ),
            ));
        }
    }

    HeaderDetection {
        header_row,
        data_start,
        columns,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::raw::RawMeta;

    fn table(rows: Vec<Vec<&str>>) -> RawTable {
        RawTable {
            rows: rows
                .into_iter()
                .map(|r| r.into_iter().map(String::from).collect())
                .collect(),
            meta: RawMeta::default(),
        }
    }

    fn opts() -> NormalizeOptions {
        NormalizeOptions::default()
    }

    #[test]
    fn detects_header_after_preamble() {
        let t = table(vec![
            vec!["Banco Ejemplo S.A."],
            vec!["Cuenta: ****1234"],
            vec![],
            vec!["Fecha", "Concepto", "Cargo", "Abono", "Saldo"],
            vec!["01/02/2026", "Pago", "10,00", "", "90,00"],
        ]);
        let d = detect(&t, &opts());
        assert_eq!(d.header_row, Some(3));
        assert_eq!(d.data_start, 4);
        let fields: Vec<_> = d.columns.iter().map(|c| c.field).collect();
        assert_eq!(
            fields,
            vec![
                Some(Field::Date),
                Some(Field::Description),
                Some(Field::Debit),
                Some(Field::Credit),
                Some(Field::Balance)
            ]
        );
        assert!(d
            .warnings
            .iter()
            .any(|w| w.code == codes::PREAMBLE_ROWS_SKIPPED));
    }

    #[test]
    fn second_date_column_becomes_posted_at() {
        let t = table(vec![vec![
            "Fecha operación",
            "Fecha valor",
            "Concepto",
            "Importe",
        ]]);
        let d = detect(&t, &opts());
        let fields: Vec<_> = d.columns.iter().map(|c| c.field).collect();
        assert_eq!(
            fields,
            vec![
                Some(Field::Date),
                Some(Field::PostedAt),
                Some(Field::Description),
                Some(Field::Amount)
            ]
        );
    }

    #[test]
    fn explicit_mapping_wins() {
        let t = table(vec![vec!["A", "B", "C"], vec!["2026-01-01", "x", "5.00"]]);
        let mut o = opts();
        o.mapping.insert(Field::Date, ColumnRef::Index(0));
        o.mapping.insert(Field::Description, ColumnRef::Index(1));
        o.mapping.insert(Field::Amount, ColumnRef::Index(2));
        let d = detect(&t, &o);
        assert_eq!(d.columns[0].field, Some(Field::Date));
        assert_eq!(d.columns[2].field, Some(Field::Amount));
    }
}
