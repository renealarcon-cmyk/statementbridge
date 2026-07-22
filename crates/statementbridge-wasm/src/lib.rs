//! WebAssembly bindings for the StatementBridge engine.
//!
//! API contract (stable within 0.1.x): every function that can fail returns a
//! JSON *envelope string* instead of throwing:
//!
//! ```json
//! { "ok": true,  "data": … }
//! { "ok": false, "error": { "code": "…", "message": "…" } }
//! ```
//!
//! The module performs no I/O, no network access and holds no state: the web
//! app passes file bytes in, gets JSON out, and owns the memory lifecycle.
//! `imported_at` is caller input so outputs stay deterministic.

use serde::Deserialize;
use statementbridge_adapters as adapters;
use statementbridge_core::config::{
    AmountMode, ColumnRef, Field, Locale, NormalizeOptions, OrderHint,
};
use statementbridge_formats as formats;
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;

/// Options accepted by `inspect_file` / `convert_file` (all fields optional).
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct WasmOptions {
    adapter: Option<String>,
    currency: Option<String>,
    locale: Option<String>,
    date_format: Option<String>,
    decimal_separator: Option<String>,
    thousands_separator: Option<String>,
    order: Option<String>,
    mapping: Option<BTreeMap<String, String>>,
    sheet: Option<String>,
    delimiter: Option<String>,
    redact: bool,
    remove_exact_duplicates: bool,
    imported_at: Option<String>,
    max_file_bytes: Option<u64>,
    max_rows: Option<usize>,
}

fn ok(data: serde_json::Value) -> String {
    serde_json::json!({ "ok": true, "data": data }).to_string()
}

fn err(code: &str, message: &str) -> String {
    serde_json::json!({ "ok": false, "error": { "code": code, "message": message } }).to_string()
}

fn engine_err(e: &statementbridge_core::EngineError) -> String {
    err(e.code(), &e.to_string())
}

/// Install a panic hook that reports panics to the browser console (call once
/// at startup; optional but very helpful when debugging).
#[wasm_bindgen]
pub fn init() {
    console_error_panic_hook::set_once();
}

/// Engine version.
#[wasm_bindgen]
pub fn version() -> String {
    statementbridge_core::VERSION.to_string()
}

/// Canonical schema version of the produced statements.
#[wasm_bindgen]
pub fn schema_version() -> String {
    statementbridge_core::SCHEMA_VERSION.to_string()
}

/// List built-in adapters. Envelope with `data: [{id, version, display_name,
/// description, amount_mode}]`.
#[wasm_bindgen]
pub fn adapters_json() -> String {
    let items: Vec<serde_json::Value> = adapters::builtin_adapters()
        .iter()
        .map(|a| {
            serde_json::json!({
                "id": a.id,
                "version": a.version,
                "display_name": a.display_name,
                "description": a.description,
                "amount_mode": a.amount_mode,
            })
        })
        .collect();
    ok(serde_json::Value::Array(items))
}

struct Prepared {
    raw: statementbridge_core::RawTable,
    options: NormalizeOptions,
}

fn prepare(bytes: &[u8], file_name: &str, options_json: &str) -> Result<Prepared, String> {
    let opts: WasmOptions = if options_json.trim().is_empty() {
        WasmOptions::default()
    } else {
        serde_json::from_str(options_json)
            .map_err(|e| err("invalid_config", &format!("bad options JSON: {e}")))?
    };

    let defaults = statementbridge_core::Limits::default();
    let limits = statementbridge_core::Limits {
        // Browser default: 50 MiB unless the caller says otherwise.
        max_file_bytes: opts.max_file_bytes.unwrap_or(50 * 1024 * 1024),
        max_rows: opts.max_rows.unwrap_or(defaults.max_rows),
        ..defaults
    };

    let delimiter = match opts.delimiter.as_deref() {
        None => None,
        Some("tab") | Some("\\t") | Some("\t") => Some('\t'),
        Some(v) if v.chars().count() == 1 => v.chars().next(),
        Some(other) => {
            return Err(err(
                "invalid_config",
                &format!("delimiter '{other}' must be a single character or 'tab'"),
            ))
        }
    };

    let read_opts = formats::ReadOptions {
        delimiter,
        sheet: opts.sheet.clone(),
        limits,
    };
    let raw = formats::read_bytes(bytes, file_name, &read_opts).map_err(|e| engine_err(&e))?;

    let builtins = adapters::builtin_adapters();
    let spec = match &opts.adapter {
        Some(id) => adapters::find_adapter(id, &[]).map_err(|e| engine_err(&e))?,
        None => adapters::detect_adapter(&raw, &builtins)
            .cloned()
            .ok_or_else(|| err("adapter_not_found", "no adapter supports this file"))?,
    };

    let locale = match opts.locale.as_deref() {
        None => None,
        Some("en") => Some(Locale::En),
        Some("es") => Some(Locale::Es),
        Some(other) => {
            return Err(err(
                "invalid_config",
                &format!("unknown locale '{other}' (en, es)"),
            ))
        }
    };
    let order = match opts.order.as_deref() {
        None | Some("auto") => OrderHint::Auto,
        Some("ascending") => OrderHint::Ascending,
        Some("descending") => OrderHint::Descending,
        Some(other) => {
            return Err(err(
                "invalid_config",
                &format!("unknown order '{other}' (auto, ascending, descending)"),
            ))
        }
    };
    let mut mapping: BTreeMap<Field, ColumnRef> = BTreeMap::new();
    if let Some(m) = &opts.mapping {
        for (k, v) in m {
            let field = Field::parse(k)
                .ok_or_else(|| err("invalid_config", &format!("unknown field '{k}' in mapping")))?;
            if !v.trim().is_empty() {
                mapping.insert(field, ColumnRef::parse(v.trim()));
            }
        }
    }
    if let Some(c) = &opts.currency {
        let cu = c.trim().to_uppercase();
        if !cu.is_empty() && (cu.len() != 3 || !cu.bytes().all(|b| b.is_ascii_uppercase())) {
            return Err(err(
                "invalid_config",
                &format!("currency '{c}' must be a 3-letter ISO 4217 code"),
            ));
        }
    }

    let base = NormalizeOptions {
        currency: opts
            .currency
            .as_ref()
            .map(|c| c.trim().to_uppercase())
            .filter(|c| !c.is_empty()),
        locale,
        date_formats: opts.date_format.clone().into_iter().collect(),
        decimal_separator: opts
            .decimal_separator
            .as_ref()
            .and_then(|s| s.chars().next()),
        thousands_separator: opts
            .thousands_separator
            .as_ref()
            .and_then(|s| s.chars().next()),
        order,
        amount_mode: AmountMode::Auto,
        mapping,
        redact: opts.redact,
        remove_exact_duplicates: opts.remove_exact_duplicates,
        imported_at: opts.imported_at.clone(),
        limits,
        ..NormalizeOptions::default()
    };
    let options = spec.apply(&base).map_err(|e| engine_err(&e))?;
    Ok(Prepared { raw, options })
}

/// Inspect a file: detection report without full normalization.
#[wasm_bindgen]
pub fn inspect_file(bytes: &[u8], file_name: &str, options_json: &str) -> String {
    match prepare(bytes, file_name, options_json) {
        Ok(p) => {
            let report = statementbridge_core::inspect(&p.raw, &p.options);
            match serde_json::to_value(&report) {
                Ok(v) => {
                    let mut v = v;
                    v["adapter_id"] = serde_json::json!(p.options.adapter_id);
                    ok(v)
                }
                Err(e) => err("io_error", &format!("serialize failed: {e}")),
            }
        }
        Err(envelope) => envelope,
    }
}

/// Convert a file into the canonical statement (full pipeline).
#[wasm_bindgen]
pub fn convert_file(bytes: &[u8], file_name: &str, options_json: &str) -> String {
    match prepare(bytes, file_name, options_json) {
        Ok(p) => match statementbridge_core::normalize(&p.raw, &p.options) {
            Ok(st) => match serde_json::to_value(&st) {
                Ok(v) => ok(v),
                Err(e) => err("io_error", &format!("serialize failed: {e}")),
            },
            Err(e) => engine_err(&e),
        },
        Err(envelope) => envelope,
    }
}

/// Render a canonical statement (JSON string) as normalized CSV with formula
/// injection guards. Envelope with `data: "<csv>"`.
#[wasm_bindgen]
pub fn statement_to_csv(statement_json: &str) -> String {
    let st: statementbridge_core::Statement = match serde_json::from_str(statement_json) {
        Ok(s) => s,
        Err(e) => return err("invalid_config", &format!("not a canonical statement: {e}")),
    };
    match statementbridge_core::export::to_normalized_csv(&st) {
        Ok(csv) => ok(serde_json::Value::String(csv)),
        Err(e) => engine_err(&e),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn envelope_shapes() {
        let v: serde_json::Value = serde_json::from_str(&adapters_json()).expect("json");
        assert_eq!(v["ok"], true);
        assert!(v["data"].as_array().map(|a| a.len() == 3).unwrap_or(false));

        let csv = b"Date,Description,Amount\n2026-01-01,Test,5.00\n";
        let out = convert_file(csv, "t.csv", "{}");
        let v: serde_json::Value = serde_json::from_str(&out).expect("json");
        assert_eq!(v["ok"], true);
        assert_eq!(v["data"]["summary"]["transaction_count"], 1);

        let bad = convert_file(csv, "t.csv", "{\"locale\": \"xx\"}");
        let v: serde_json::Value = serde_json::from_str(&bad).expect("json");
        assert_eq!(v["ok"], false);
        assert_eq!(v["error"]["code"], "invalid_config");

        let st = serde_json::from_str::<serde_json::Value>(&out).expect("json")["data"].to_string();
        let csv_out = statement_to_csv(&st);
        let v: serde_json::Value = serde_json::from_str(&csv_out).expect("json");
        assert_eq!(v["ok"], true);
        assert!(v["data"].as_str().expect("csv").starts_with("id,date"));
    }
}
