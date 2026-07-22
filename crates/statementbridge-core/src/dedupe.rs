//! Deterministic duplicate classification.
//!
//! * **Exact**: identical full fingerprint (date, amount, description,
//!   reference, balance, account). Removed only when the caller asks
//!   (`remove_exact_duplicates`); the first occurrence is always kept.
//! * **Possible**: same date + amount + description, and no evidence the rows
//!   are distinct. Flagged for review, never removed automatically.
//! * **Repeated**: same date + amount + description but a distinct reference
//!   or a distinct balance proves both really happened (e.g. two identical
//!   coffee purchases). Informational only.

use crate::fingerprint;
use crate::issues::{codes, Issue, WarningCollector};
use crate::model::{DuplicateKind, Transaction};
use crate::text;
use std::collections::HashMap;

#[derive(Debug, Default, Clone, Copy)]
pub struct DupStats {
    pub exact: u32,
    pub possible: u32,
    pub removed: u32,
}

pub fn classify(
    txns: &mut Vec<Transaction>,
    remove_exact: bool,
    warnings: &mut WarningCollector,
) -> DupStats {
    let mut stats = DupStats::default();
    let mut by_fp: HashMap<String, usize> = HashMap::new();
    let mut by_loose: HashMap<String, usize> = HashMap::new();
    let mut decisions: Vec<(usize, DuplicateKind, u32)> = Vec::new();

    for (i, t) in txns.iter().enumerate() {
        let desc_key = text::matching_key(&t.description);
        let loose = fingerprint::loose_key(t.date, &t.amount, &desc_key);
        if let Some(&first) = by_fp.get(&t.fingerprint) {
            decisions.push((i, DuplicateKind::Exact, txns[first].original_row));
            continue;
        }
        if let Some(&first) = by_loose.get(&loose) {
            let p = &txns[first];
            let distinct_refs = match (&p.reference, &t.reference) {
                (Some(a), Some(b)) => a.to_lowercase() != b.to_lowercase(),
                _ => false,
            };
            let distinct_balance = match (p.balance, t.balance) {
                (Some(a), Some(b)) => a != b,
                _ => false,
            };
            let kind = if distinct_refs || distinct_balance {
                DuplicateKind::Repeated
            } else {
                DuplicateKind::Possible
            };
            decisions.push((i, kind, p.original_row));
        }
        by_fp.insert(t.fingerprint.clone(), i);
        by_loose.entry(loose).or_insert(i);
    }

    let mut remove: Vec<usize> = Vec::new();
    for (i, kind, of_row) in decisions {
        let row = txns[i].original_row;
        txns[i].duplicate_kind = Some(kind);
        txns[i].duplicate_of_row = Some(of_row);
        match kind {
            DuplicateKind::Exact => {
                stats.exact += 1;
                if remove_exact {
                    stats.removed += 1;
                    remove.push(i);
                    warnings.push(
                        Issue::new(
                            codes::EXACT_DUPLICATE_REMOVED,
                            format!("exact duplicate of row {of_row} removed"),
                        )
                        .at_row(row),
                    );
                } else {
                    txns[i].warnings.push(
                        Issue::new(
                            codes::EXACT_DUPLICATE,
                            format!("exact duplicate of row {of_row}"),
                        )
                        .at_row(row),
                    );
                }
            }
            DuplicateKind::Possible => {
                stats.possible += 1;
                txns[i].warnings.push(
                    Issue::new(
                        codes::POSSIBLE_DUPLICATE,
                        format!("possibly a duplicate of row {of_row}; review before importing"),
                    )
                    .at_row(row),
                );
            }
            DuplicateKind::Repeated => { /* informational only */ }
        }
    }

    if !remove.is_empty() {
        let removed: std::collections::HashSet<usize> = remove.into_iter().collect();
        let mut keep = Vec::with_capacity(txns.len() - removed.len());
        for (i, t) in txns.drain(..).enumerate() {
            if !removed.contains(&i) {
                keep.push(t);
            }
        }
        *txns = keep;
    }
    stats
}
