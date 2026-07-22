//! CLI integration tests (black-box, via the built binary).

use assert_cmd::Command;
use predicates::prelude::*;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("root")
}

fn fixture(rel: &str) -> String {
    root().join(rel).display().to_string()
}

fn bin() -> Command {
    Command::cargo_bin("statementbridge").expect("binary")
}

#[test]
fn convert_valid_file_exit_0_and_stdout_json() {
    let out = bin()
        .args([
            "convert",
            &fixture("fixtures/csv/comma_dot_signed_iso.csv"),
            "--imported-at",
            "2026-01-01T00:00:00Z",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).expect("stdout is the canonical JSON");
    assert_eq!(v["schema_version"], "1.0");
    assert_eq!(v["summary"]["transaction_count"], 5);
    assert_eq!(v["validation"]["status"], "valid");
    assert_eq!(v["account"]["currency"], "USD");
}

#[test]
fn convert_deterministic_with_fixed_timestamp() {
    let run = || {
        bin()
            .args([
                "convert",
                &fixture("fixtures/csv/semicolon_comma_es.csv"),
                "--imported-at",
                "2026-01-01T00:00:00Z",
                "-q",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone()
    };
    assert_eq!(
        run(),
        run(),
        "same input + options must produce identical bytes"
    );
}

#[test]
fn validate_inconsistent_balance_exits_3() {
    bin()
        .args([
            "validate",
            &fixture("fixtures/csv/balance_inconsistent.csv"),
        ])
        .assert()
        .code(3)
        .stdout(predicate::str::contains("INCONSISTENT"));
}

#[test]
fn strict_warnings_exit_4() {
    // File is valid but produces warnings (preamble + footer rows skipped).
    bin()
        .args([
            "validate",
            &fixture("fixtures/csv/semicolon_comma_es.csv"),
            "--strict",
        ])
        .assert()
        .code(4);
}

#[test]
fn validate_json_envelope() {
    let out = bin()
        .args([
            "validate",
            &fixture("fixtures/csv/comma_dot_signed_iso.csv"),
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).expect("json envelope");
    assert_eq!(v["ok"], true);
    assert_eq!(v["command"], "validate");
    assert_eq!(v["data"]["summary"]["status"], "valid");
}

#[test]
fn inspect_reports_detection() {
    let out = bin()
        .args([
            "inspect",
            &fixture("fixtures/csv/windows1252.csv"),
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).expect("json");
    assert_eq!(v["data"]["encoding"], "windows-1252");
    assert_eq!(v["data"]["delimiter"], ";");
    assert_eq!(v["data"]["decimal_separator"], ",");
}

#[test]
fn inspect_masks_long_digit_runs_in_samples() {
    let dir = tempfile::tempdir().expect("tmp");
    let file = dir.path().join("acct.csv");
    std::fs::write(
        &file,
        "Date,Description,Amount\n2026-01-01,Transfer to 12345678901234,-5.00\n",
    )
    .expect("write");
    let out = bin()
        .args(["inspect", file.to_str().expect("path"), "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).expect("utf8");
    assert!(
        !text.contains("12345678901234"),
        "full account digits must not appear: {text}"
    );
    assert!(text.contains("****1234"));
}

#[test]
fn convert_csv_format_guards_injection() {
    let out = bin()
        .args([
            "convert",
            &fixture("fixtures/csv/injection_attempt.csv"),
            "--format",
            "csv",
            "-q",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).expect("utf8");
    assert!(text.contains("'=HYPERLINK"));
    assert!(!text.lines().skip(1).any(|l| l.starts_with('=')));
}

#[test]
fn convert_writes_output_file() {
    let dir = tempfile::tempdir().expect("tmp");
    let out_path = dir.path().join("normalized.json");
    bin()
        .args([
            "convert",
            &fixture("fixtures/csv/comma_dot_signed_iso.csv"),
            "--output",
            out_path.to_str().expect("path"),
            "-q",
        ])
        .assert()
        .success();
    let text = std::fs::read_to_string(&out_path).expect("output written");
    assert!(text.contains("\"schema_version\": \"1.0\""));
}

#[test]
fn convert_refuses_missing_output_dir() {
    bin()
        .args([
            "convert",
            &fixture("fixtures/csv/comma_dot_signed_iso.csv"),
            "--output",
            "/nonexistent-dir-sb/out.json",
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("does not exist"));
}

#[test]
fn convert_refuses_overwriting_input() {
    let dir = tempfile::tempdir().expect("tmp");
    let file = dir.path().join("in.csv");
    std::fs::write(&file, "Date,Description,Amount\n2026-01-01,x,1.00\n").expect("write");
    bin()
        .args([
            "convert",
            file.to_str().expect("p"),
            "--output",
            file.to_str().expect("p"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "refusing to overwrite the input file",
        ));
}

#[test]
fn remove_exact_duplicates_flag() {
    let out = bin()
        .args([
            "convert",
            &fixture("fixtures/csv/exact_duplicates.csv"),
            "--remove-exact-duplicates",
            "-q",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).expect("json");
    assert_eq!(v["summary"]["transaction_count"], 2);
    assert_eq!(v["summary"]["duplicates_removed"], 1);
}

#[test]
fn redact_masks_descriptions() {
    let dir = tempfile::tempdir().expect("tmp");
    let file = dir.path().join("r.csv");
    std::fs::write(
        &file,
        "Date,Description,Amount\n2026-01-01,Wire from account 998877665544,-9.00\n",
    )
    .expect("write");
    let out = bin()
        .args(["convert", file.to_str().expect("p"), "--redact", "-q"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).expect("utf8");
    assert!(!text.contains("998877665544"));
    assert!(text.contains("****5544"));
}

#[test]
fn map_flags_drive_headerless_files() {
    let out = bin()
        .args([
            "convert",
            &fixture("fixtures/csv/headerless.csv"),
            "--map",
            "date=#1",
            "--map",
            "description=#2",
            "--map",
            "amount=#3",
            "-q",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).expect("json");
    assert_eq!(v["summary"]["transaction_count"], 2);
}

#[test]
fn xlsx_sheet_selection() {
    let out = bin()
        .args([
            "inspect",
            &fixture("fixtures/xlsx/multisheet.xlsx"),
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).expect("json");
    assert_eq!(v["data"]["sheet_name"], "Movimientos");
    let out2 = bin()
        .args(["convert", &fixture("fixtures/xlsx/multisheet.xlsx"), "-q"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v2: serde_json::Value = serde_json::from_slice(&out2).expect("json");
    assert_eq!(v2["summary"]["transaction_count"], 4);
}

#[test]
fn adapters_list_shows_builtins() {
    let out = bin()
        .args(["adapters", "list", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).expect("json");
    let ids: Vec<&str> = v["data"]
        .as_array()
        .expect("array")
        .iter()
        .filter_map(|a| a["id"].as_str())
        .collect();
    assert!(ids.contains(&"generic-csv"));
    assert!(ids.contains(&"generic-debit-credit"));
    assert!(ids.contains(&"generic-signed-amount"));
}

#[test]
fn adapters_validate_good_and_bad() {
    bin()
        .args([
            "adapters",
            "validate",
            &fixture("adapters/generic-csv/adapter.json"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("is valid"));

    let dir = tempfile::tempdir().expect("tmp");
    let bad = dir.path().join("bad.json");
    std::fs::write(&bad, r#"{"id":"BAD ID","version":"x","display_name":"","description":"","supported_formats":[],"amount_mode":"nope"}"#).expect("write");
    bin()
        .args(["adapters", "validate", bad.to_str().expect("p")])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("invalid adapter"));
}

#[test]
fn init_adapter_scaffolds_and_validates() {
    let dir = tempfile::tempdir().expect("tmp");
    bin()
        .args([
            "init-adapter",
            "acme-bank",
            "--dir",
            dir.path().to_str().expect("p"),
        ])
        .assert()
        .success();
    let adapter = dir.path().join("acme-bank/adapter.json");
    assert!(adapter.is_file());
    assert!(dir.path().join("acme-bank/README.md").is_file());
    // The generated template must itself be a valid adapter.
    bin()
        .args(["adapters", "validate", adapter.to_str().expect("p")])
        .assert()
        .success();
    // Refuses to overwrite.
    bin()
        .args([
            "init-adapter",
            "acme-bank",
            "--dir",
            dir.path().to_str().expect("p"),
        ])
        .assert()
        .code(1);
    // Rejects hostile names.
    bin()
        .args([
            "init-adapter",
            "../escape",
            "--dir",
            dir.path().to_str().expect("p"),
        ])
        .assert()
        .code(1);
}

#[test]
fn unsupported_binary_data_exits_1() {
    let dir = tempfile::tempdir().expect("tmp");
    let file = dir.path().join("blob.csv");
    std::fs::write(&file, [0u8, 159, 146, 150, 0, 0, 1, 2, 3, 0, 0, 0]).expect("write");
    bin()
        .args(["convert", file.to_str().expect("p")])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("binary data"));
}

#[test]
fn json_error_envelope_on_failure() {
    let out = bin()
        .args(["convert", "/definitely/missing.csv", "--json"])
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).expect("json");
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"]["code"], "io_error");
}

#[test]
fn file_size_limit_enforced() {
    bin()
        .args([
            "convert",
            &fixture("fixtures/csv/comma_dot_signed_iso.csv"),
            "--max-file-size",
            "10",
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("file too large"));
}
