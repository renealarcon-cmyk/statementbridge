//! Golden tests: fixtures are normalized through the library (no CLI, no
//! clock: `imported_at = None`) and compared byte-for-byte against
//! `fixtures/golden/*.json`. Every golden output must also validate against
//! the canonical JSON Schema.
//!
//! Regenerate after an intentional engine change:
//! `UPDATE_GOLDEN=1 cargo test -p statementbridge-cli --test golden`

use statementbridge_adapters as adapters;
use statementbridge_core::export::to_canonical_json;
use statementbridge_core::NormalizeOptions;
use statementbridge_formats as formats;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

fn normalize_fixture(rel: &str, tweak: impl Fn(&mut NormalizeOptions)) -> String {
    let root = repo_root();
    let path = root.join(rel);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let raw = formats::read_bytes(
        &bytes,
        path.file_name()
            .and_then(|s| s.to_str())
            .expect("file name"),
        &formats::ReadOptions::default(),
    )
    .expect("read fixture");
    let candidates = adapters::builtin_adapters();
    let spec = adapters::detect_adapter(&raw, &candidates).expect("adapter");
    let mut opts = spec.apply(&NormalizeOptions::default()).expect("options");
    tweak(&mut opts);
    let st = statementbridge_core::normalize(&raw, &opts).expect("normalize");
    to_canonical_json(&st).expect("json")
}

fn check_schema(json: &str) {
    let schema_text =
        std::fs::read_to_string(repo_root().join("schemas/statementbridge-v1.schema.json"))
            .expect("schema file");
    let schema: serde_json::Value = serde_json::from_str(&schema_text).expect("schema json");
    let validator = jsonschema::validator_for(&schema).expect("compile schema");
    let instance: serde_json::Value = serde_json::from_str(json).expect("output json");
    let errors: Vec<String> = validator
        .iter_errors(&instance)
        .map(|e| e.to_string())
        .collect();
    assert!(errors.is_empty(), "schema violations: {errors:#?}");
}

fn golden(rel_fixture: &str, golden_name: &str, tweak: impl Fn(&mut NormalizeOptions)) {
    let produced = normalize_fixture(rel_fixture, tweak);
    check_schema(&produced);
    let golden_path = repo_root().join("fixtures/golden").join(golden_name);
    if std::env::var("UPDATE_GOLDEN").is_ok() {
        std::fs::create_dir_all(golden_path.parent().expect("parent")).expect("mkdir");
        std::fs::write(&golden_path, &produced).expect("write golden");
        return;
    }
    let expected = std::fs::read_to_string(&golden_path).unwrap_or_else(|e| {
        panic!(
            "missing golden {} ({e}); run with UPDATE_GOLDEN=1",
            golden_path.display()
        )
    });
    assert_eq!(
        produced, expected,
        "golden mismatch for {rel_fixture}; run UPDATE_GOLDEN=1 if the change is intentional"
    );
}

#[test]
fn golden_comma_dot_signed_iso() {
    golden(
        "fixtures/csv/comma_dot_signed_iso.csv",
        "comma_dot_signed_iso.json",
        |_| {},
    );
}

#[test]
fn golden_semicolon_comma_es() {
    golden(
        "fixtures/csv/semicolon_comma_es.csv",
        "semicolon_comma_es.json",
        |_| {},
    );
}

#[test]
fn golden_debit_credit_mdy() {
    golden(
        "fixtures/csv/debit_credit_mdy.csv",
        "debit_credit_mdy.json",
        |_| {},
    );
}

#[test]
fn golden_descending() {
    golden("fixtures/csv/descending.csv", "descending.json", |_| {});
}

#[test]
fn golden_exact_duplicates() {
    golden(
        "fixtures/csv/exact_duplicates.csv",
        "exact_duplicates.json",
        |_| {},
    );
}

#[test]
fn golden_balance_inconsistent() {
    golden(
        "fixtures/csv/balance_inconsistent.csv",
        "balance_inconsistent.json",
        |_| {},
    );
}

#[test]
fn golden_multisheet_xlsx() {
    golden(
        "fixtures/xlsx/multisheet.xlsx",
        "multisheet_xlsx.json",
        |_| {},
    );
}

#[test]
fn determinism_two_runs_identical() {
    let a = normalize_fixture("fixtures/csv/semicolon_comma_es.csv", |_| {});
    let b = normalize_fixture("fixtures/csv/semicolon_comma_es.csv", |_| {});
    assert_eq!(a, b);
}

#[test]
fn windows1252_decodes() {
    let out = normalize_fixture("fixtures/csv/windows1252.csv", |_| {});
    assert!(
        out.contains("Café La Terraza"),
        "cp1252 text must decode: {out}"
    );
    check_schema(&out);
}

#[test]
fn utf8_bom_decodes() {
    let out = normalize_fixture("fixtures/csv/utf8_bom.csv", |_| {});
    assert!(out.contains("Café y almuerzo"));
}

#[test]
fn similar_rows_are_repeated_not_duplicates() {
    let out = normalize_fixture("fixtures/csv/similar_not_duplicates.csv", |_| {});
    let v: serde_json::Value = serde_json::from_str(&out).expect("json");
    // Distinct references (POS-9001 vs POS-9002) → legitimately repeated.
    // Row 4 (no reference) vs the referenced ones → possible duplicate.
    // Rows 4 and 5 are byte-identical with no distinguishing evidence at all
    // → exact duplicate of each other (correct: nothing proves them apart).
    assert_eq!(v["summary"]["possible_duplicates"], 1, "{out}");
    assert_eq!(v["summary"]["exact_duplicates"], 1);
    let kinds: Vec<&str> = v["transactions"]
        .as_array()
        .expect("array")
        .iter()
        .filter_map(|t| t["duplicate_kind"].as_str())
        .collect();
    assert!(kinds.contains(&"repeated"));
    assert!(kinds.contains(&"possible"));
    assert!(kinds.contains(&"exact"));
}

#[test]
fn remove_exact_duplicates_keeps_first() {
    let out = normalize_fixture("fixtures/csv/exact_duplicates.csv", |o| {
        o.remove_exact_duplicates = true;
    });
    let v: serde_json::Value = serde_json::from_str(&out).expect("json");
    assert_eq!(v["summary"]["transaction_count"], 2);
    assert_eq!(v["summary"]["duplicates_removed"], 1);
    check_schema(&out);
}

#[test]
fn csv_export_escapes_injection() {
    let root = repo_root();
    let bytes = std::fs::read(root.join("fixtures/csv/injection_attempt.csv")).expect("fixture");
    let raw = formats::read_bytes(
        &bytes,
        "injection_attempt.csv",
        &formats::ReadOptions::default(),
    )
    .expect("read");
    let builtins = adapters::builtin_adapters();
    let spec = adapters::detect_adapter(&raw, &builtins).expect("adapter");
    let opts = spec.apply(&NormalizeOptions::default()).expect("options");
    let st = statementbridge_core::normalize(&raw, &opts).expect("normalize");
    let csv_out = statementbridge_core::export::to_normalized_csv(&st).expect("csv");
    for line in csv_out.lines().skip(1) {
        // No text cell may start with a formula trigger after the quote.
        assert!(
            !line.contains(",=") || line.contains(",'="),
            "unescaped formula in: {line}"
        );
    }
    assert!(
        csv_out.contains("'=HYPERLINK"),
        "must escape leading '=': {csv_out}"
    );
    assert!(
        csv_out.contains("'+cmd"),
        "must escape leading '+': {csv_out}"
    );
    assert!(
        csv_out.contains("'@SUM"),
        "must escape leading '@': {csv_out}"
    );
}

#[test]
fn balance_inconsistency_reports_first_break() {
    let out = normalize_fixture("fixtures/csv/balance_inconsistent.csv", |_| {});
    let v: serde_json::Value = serde_json::from_str(&out).expect("json");
    assert_eq!(v["validation"]["status"], "invalid");
    assert_eq!(v["validation"]["balance_check"]["status"], "inconsistent");
    assert_eq!(v["validation"]["balance_check"]["first_mismatch_row"], 3);
    assert_eq!(v["validation"]["balance_check"]["expected"], "800.00");
    assert_eq!(v["validation"]["balance_check"]["actual"], "850.00");
}

#[test]
fn invalid_amounts_reported_per_row() {
    let out = normalize_fixture("fixtures/csv/invalid_amounts.csv", |_| {});
    let v: serde_json::Value = serde_json::from_str(&out).expect("json");
    assert_eq!(v["validation"]["status"], "invalid");
    assert_eq!(v["summary"]["transaction_count"], 2);
    let errs = v["validation"]["errors"].as_array().expect("errors");
    assert_eq!(errs.len(), 1);
    assert_eq!(errs[0]["code"], "invalid_amount");
    assert_eq!(errs[0]["row"], 3);
}

#[test]
fn headerless_with_index_mapping() {
    use statementbridge_core::config::{ColumnRef, Field};
    let out = normalize_fixture("fixtures/csv/headerless.csv", |o| {
        o.mapping.insert(Field::Date, ColumnRef::Index(0));
        o.mapping.insert(Field::Description, ColumnRef::Index(1));
        o.mapping.insert(Field::Amount, ColumnRef::Index(2));
    });
    let v: serde_json::Value = serde_json::from_str(&out).expect("json");
    assert_eq!(v["summary"]["transaction_count"], 2, "{out}");
}
