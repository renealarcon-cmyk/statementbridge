//! StatementBridge CLI implementation.
//!
//! Exit codes (stable contract):
//! * 0 — success; file is valid (or valid with warnings without `--strict`)
//! * 1 — runtime failure (unreadable file, bad adapter, I/O…)
//! * 2 — usage error (clap)
//! * 3 — validation failed (`validation.status == "invalid"`)
//! * 4 — warnings present and `--strict` was set
//!
//! Privacy: human output prints counts, codes, row numbers and masked
//! samples — never raw descriptions, references or account numbers.

use clap::{Args, Parser, Subcommand};
use statementbridge_adapters as adapters;
use statementbridge_core::config::{
    AmountMode, ColumnRef, Field, Locale, NormalizeOptions, OrderHint,
};
use statementbridge_core::export;
use statementbridge_core::model::{Statement, ValidationStatus};
use statementbridge_formats as formats;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub mod exit_codes {
    pub const OK: i32 = 0;
    pub const RUNTIME: i32 = 1;
    pub const USAGE: i32 = 2;
    pub const INVALID: i32 = 3;
    pub const STRICT_WARNINGS: i32 = 4;
}

#[derive(Parser, Debug)]
#[command(
    name = "statementbridge",
    version,
    about = "Normalize bank statement exports (CSV/XLSX) into a canonical, validated format — locally.",
    long_about = "StatementBridge converts messy bank/processor CSV and XLSX exports into a canonical,\n\
                  validated JSON/CSV format. Everything runs locally: no network, no telemetry.\n\n\
                  Exit codes: 0 ok · 1 runtime error · 2 usage · 3 validation failed · 4 warnings with --strict",
    propagate_version = true
)]
pub struct Cli {
    /// Machine-readable JSON envelope on stdout ({"ok":…,"data":…}).
    #[arg(long, global = true)]
    pub json: bool,

    /// Suppress non-essential human messages.
    #[arg(long, short, global = true)]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Show what the engine detects in a file (encoding, delimiter, columns…).
    Inspect(InspectArgs),
    /// Convert a statement into canonical JSON or normalized CSV.
    Convert(ConvertArgs),
    /// Validate a statement and report errors/warnings without exporting.
    Validate(ValidateArgs),
    /// Work with declarative adapters.
    #[command(subcommand)]
    Adapters(AdaptersCommand),
    /// Scaffold a new adapter template for contribution.
    InitAdapter(InitAdapterArgs),
}

#[derive(Subcommand, Debug)]
pub enum AdaptersCommand {
    /// List available adapters (built-in + --adapters-dir).
    List {
        /// Directory with extra adapters (<dir>/<name>/adapter.json).
        #[arg(long)]
        adapters_dir: Option<PathBuf>,
    },
    /// Validate an adapter.json file.
    Validate { path: PathBuf },
}

#[derive(Args, Debug, Clone)]
pub struct EngineArgs {
    /// Statement file (.csv or .xlsx).
    pub file: PathBuf,

    /// Adapter id (see `adapters list`) or path to an adapter.json file.
    #[arg(long)]
    pub adapter: Option<String>,

    /// Directory with extra adapters (<dir>/<name>/adapter.json).
    #[arg(long)]
    pub adapters_dir: Option<PathBuf>,

    /// ISO 4217 currency to assume when the file has none (e.g. USD).
    #[arg(long)]
    pub currency: Option<String>,

    /// Locale hint for ambiguous dates/numbers: en or es.
    #[arg(long)]
    pub locale: Option<String>,

    /// chrono date format, e.g. %d/%m/%Y (repeatable; tried in order).
    #[arg(long = "date-format")]
    pub date_format: Vec<String>,

    /// Decimal separator: "." or ",".
    #[arg(long)]
    pub decimal_separator: Option<char>,

    /// Thousands separator: "." "," "'" or " ".
    #[arg(long)]
    pub thousands_separator: Option<char>,

    /// Transaction order in the file: auto, ascending or descending.
    #[arg(long, default_value = "auto")]
    pub transaction_order: String,

    /// Map a column explicitly: FIELD=COLUMN (repeatable). FIELD is one of
    /// date, posted_at, description, reference, debit, credit, amount,
    /// balance, currency, account. COLUMN is a header name or #N (1-based).
    #[arg(long = "map", value_name = "FIELD=COLUMN")]
    pub map: Vec<String>,

    /// JSON file with a column mapping object, e.g. {"date":"Fecha"}.
    #[arg(long)]
    pub mapping: Option<PathBuf>,

    /// XLSX sheet name (default: the sheet with the most data).
    #[arg(long)]
    pub sheet: Option<String>,

    /// CSV delimiter: "," ";" "|" or "tab" (default: detected).
    #[arg(long)]
    pub delimiter: Option<String>,

    /// Treat warnings as failures (exit code 4).
    #[arg(long)]
    pub strict: bool,

    /// Mask long digit runs in descriptions/references in the output.
    #[arg(long)]
    pub redact: bool,

    /// Drop exact duplicate rows (the first occurrence is kept).
    #[arg(long)]
    pub remove_exact_duplicates: bool,

    /// Maximum file size in bytes (default 104857600 = 100 MiB).
    #[arg(long, value_name = "BYTES")]
    pub max_file_size: Option<u64>,

    /// Maximum rows to read (default 1000000).
    #[arg(long, value_name = "N")]
    pub max_rows: Option<usize>,

    /// Timestamp recorded as source.imported_at (ISO-8601). Defaults to now;
    /// pass a fixed value for reproducible outputs.
    #[arg(long, value_name = "ISO8601")]
    pub imported_at: Option<String>,
}

#[derive(Args, Debug)]
pub struct InspectArgs {
    #[command(flatten)]
    pub engine: EngineArgs,
}

#[derive(Args, Debug)]
pub struct ConvertArgs {
    #[command(flatten)]
    pub engine: EngineArgs,

    /// Output file. "-" or omitted writes the document to stdout.
    #[arg(long, short)]
    pub output: Option<PathBuf>,

    /// Output format: json (canonical) or csv (normalized).
    #[arg(long, default_value = "json")]
    pub format: String,
}

#[derive(Args, Debug)]
pub struct ValidateArgs {
    #[command(flatten)]
    pub engine: EngineArgs,
}

#[derive(Args, Debug)]
pub struct InitAdapterArgs {
    /// Adapter id: lowercase letters, digits and dashes (e.g. mybank-checking).
    pub name: String,

    /// Parent directory to create the adapter in (default: ./adapters).
    #[arg(long, default_value = "adapters")]
    pub dir: PathBuf,
}

// ---------------------------------------------------------------------------

struct Failure {
    code: String,
    message: String,
    exit: i32,
}

impl Failure {
    fn runtime(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
            exit: exit_codes::RUNTIME,
        }
    }
}

impl From<statementbridge_core::EngineError> for Failure {
    fn from(e: statementbridge_core::EngineError) -> Self {
        Failure {
            code: e.code().to_string(),
            message: e.to_string(),
            exit: exit_codes::RUNTIME,
        }
    }
}

fn emit_failure(cli_json: bool, command: &str, f: &Failure) {
    if cli_json {
        let env = serde_json::json!({
            "ok": false,
            "command": command,
            "error": { "code": f.code, "message": f.message }
        });
        println!("{env}");
    } else {
        eprintln!("error: {}", f.message);
    }
}

fn emit_ok(cli_json: bool, quiet: bool, command: &str, data: serde_json::Value, human: &str) {
    if cli_json {
        let env = serde_json::json!({ "ok": true, "command": command, "data": data });
        println!("{env}");
    } else if !human.is_empty() && !quiet {
        println!("{human}");
    }
}

pub fn run(cli: Cli) -> i32 {
    let json = cli.json;
    let quiet = cli.quiet;
    match cli.command {
        Command::Inspect(a) => cmd_inspect(json, quiet, a),
        Command::Convert(a) => cmd_convert(json, quiet, a),
        Command::Validate(a) => cmd_validate(json, quiet, a),
        Command::Adapters(a) => cmd_adapters(json, quiet, a),
        Command::InitAdapter(a) => cmd_init_adapter(json, quiet, a),
    }
}

// ---------------------------------------------------------------------------
// Option assembly

fn parse_locale(s: &Option<String>) -> Result<Option<Locale>, Failure> {
    match s.as_deref() {
        None => Ok(None),
        Some("en") => Ok(Some(Locale::En)),
        Some("es") => Ok(Some(Locale::Es)),
        Some(other) => Err(Failure::runtime(
            "invalid_config",
            format!("unknown locale '{other}' (expected: en, es)"),
        )),
    }
}

fn parse_order(s: &str) -> Result<OrderHint, Failure> {
    match s {
        "auto" => Ok(OrderHint::Auto),
        "ascending" | "asc" => Ok(OrderHint::Ascending),
        "descending" | "desc" => Ok(OrderHint::Descending),
        other => Err(Failure::runtime(
            "invalid_config",
            format!("unknown transaction order '{other}' (expected: auto, ascending, descending)"),
        )),
    }
}

fn parse_delimiter(s: &Option<String>) -> Result<Option<char>, Failure> {
    match s.as_deref() {
        None => Ok(None),
        Some("tab") | Some("\\t") | Some("\t") => Ok(Some('\t')),
        Some(v) if v.chars().count() == 1 => Ok(v.chars().next()),
        Some(other) => Err(Failure::runtime(
            "invalid_config",
            format!("delimiter '{other}' must be a single character or 'tab'"),
        )),
    }
}

fn parse_mappings(args: &EngineArgs) -> Result<BTreeMap<Field, ColumnRef>, Failure> {
    let mut out: BTreeMap<Field, ColumnRef> = BTreeMap::new();
    if let Some(path) = &args.mapping {
        let text = std::fs::read_to_string(path).map_err(|e| {
            Failure::runtime(
                "io_error",
                format!("cannot read mapping file {}: {e}", path.display()),
            )
        })?;
        let obj: BTreeMap<String, String> = serde_json::from_str(&text).map_err(|e| {
            Failure::runtime(
                "invalid_config",
                format!("mapping file must be a JSON object of field→column: {e}"),
            )
        })?;
        for (k, v) in obj {
            let field = Field::parse(&k).ok_or_else(|| {
                Failure::runtime(
                    "invalid_config",
                    format!("unknown field '{k}' in mapping file"),
                )
            })?;
            out.insert(field, ColumnRef::parse(&v));
        }
    }
    for m in &args.map {
        let Some((k, v)) = m.split_once('=') else {
            return Err(Failure::runtime(
                "invalid_config",
                format!("--map '{m}' must be FIELD=COLUMN (e.g. --map date=Fecha)"),
            ));
        };
        let field = Field::parse(k.trim()).ok_or_else(|| {
            Failure::runtime(
                "invalid_config",
                format!(
                    "unknown field '{}' in --map (expected one of: {})",
                    k.trim(),
                    Field::ALL
                        .iter()
                        .map(|f| f.name())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            )
        })?;
        out.insert(field, ColumnRef::parse(v.trim()));
    }
    Ok(out)
}

fn load_extra_adapters(dir: &Option<PathBuf>) -> Result<Vec<adapters::AdapterSpec>, Failure> {
    let Some(dir) = dir else {
        return Ok(Vec::new());
    };
    let mut specs = Vec::new();
    let entries = std::fs::read_dir(dir).map_err(|e| {
        Failure::runtime(
            "io_error",
            format!("cannot read adapters dir {}: {e}", dir.display()),
        )
    })?;
    let mut paths: Vec<PathBuf> = Vec::new();
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            let f = p.join("adapter.json");
            if f.is_file() {
                paths.push(f);
            }
        } else if p.extension().map(|e| e == "json").unwrap_or(false) {
            paths.push(p);
        }
    }
    paths.sort();
    for p in paths {
        let text = std::fs::read_to_string(&p).map_err(|e| {
            Failure::runtime("io_error", format!("cannot read {}: {e}", p.display()))
        })?;
        let spec = adapters::parse_adapter(&text).map_err(Failure::from)?;
        specs.push(spec);
    }
    Ok(specs)
}

struct Prepared {
    raw: statementbridge_core::RawTable,
    options: NormalizeOptions,
    adapter_display: String,
}

fn prepare(args: &EngineArgs, default_imported_at: bool) -> Result<Prepared, Failure> {
    let bytes = std::fs::read(&args.file).map_err(|e| {
        Failure::runtime(
            "io_error",
            format!("cannot read {}: {e}", args.file.display()),
        )
    })?;

    let mut limits = statementbridge_core::Limits::default();
    if let Some(m) = args.max_file_size {
        limits.max_file_bytes = m;
    }
    if let Some(m) = args.max_rows {
        limits.max_rows = m;
    }

    let read_opts = formats::ReadOptions {
        delimiter: parse_delimiter(&args.delimiter)?,
        sheet: args.sheet.clone(),
        limits,
    };
    let file_name = args
        .file
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("statement");
    let raw = formats::read_bytes(&bytes, file_name, &read_opts).map_err(Failure::from)?;

    let extras = load_extra_adapters(&args.adapters_dir)?;

    // Resolve the adapter: explicit path > explicit id > auto-detection.
    let spec = match &args.adapter {
        Some(sel) => {
            let as_path = Path::new(sel);
            if as_path.is_file() {
                let text = std::fs::read_to_string(as_path).map_err(|e| {
                    Failure::runtime(
                        "io_error",
                        format!("cannot read adapter {}: {e}", as_path.display()),
                    )
                })?;
                adapters::parse_adapter(&text).map_err(Failure::from)?
            } else {
                adapters::find_adapter(sel, &extras).map_err(Failure::from)?
            }
        }
        None => {
            let mut candidates = extras.clone();
            candidates.extend(adapters::builtin_adapters());
            adapters::detect_adapter(&raw, &candidates)
                .cloned()
                .ok_or_else(|| {
                    Failure::runtime("adapter_not_found", "no adapter supports this file format")
                })?
        }
    };

    // Base options from CLI flags (these win over adapter values).
    let mut base = NormalizeOptions {
        currency: args.currency.clone().map(|c| c.to_uppercase()),
        locale: parse_locale(&args.locale)?,
        date_formats: args.date_format.clone(),
        decimal_separator: args.decimal_separator,
        thousands_separator: args.thousands_separator,
        order: parse_order(&args.transaction_order)?,
        amount_mode: AmountMode::Auto,
        mapping: parse_mappings(args)?,
        strict: args.strict,
        redact: args.redact,
        remove_exact_duplicates: args.remove_exact_duplicates,
        limits,
        ..NormalizeOptions::default()
    };
    if let Some(c) = &base.currency {
        if c.len() != 3 || !c.bytes().all(|b| b.is_ascii_uppercase()) {
            return Err(Failure::runtime(
                "invalid_config",
                format!("currency '{c}' must be a 3-letter ISO 4217 code (e.g. USD, EUR)"),
            ));
        }
    }
    base.imported_at = match &args.imported_at {
        Some(ts) => Some(ts.clone()),
        None if default_imported_at => {
            Some(chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
        }
        None => None,
    };

    let options = spec.apply(&base).map_err(Failure::from)?;
    let adapter_display = format!("{} v{}", spec.id, spec.version);
    Ok(Prepared {
        raw,
        options,
        adapter_display,
    })
}

fn status_exit(st: &Statement, strict: bool) -> i32 {
    match st.validation.status {
        ValidationStatus::Invalid => exit_codes::INVALID,
        ValidationStatus::ValidWithWarnings if strict => exit_codes::STRICT_WARNINGS,
        _ => exit_codes::OK,
    }
}

fn summary_lines(st: &Statement, adapter: &str) -> String {
    let v = &st.validation;
    let b = &v.balance_check;
    let status = match v.status {
        ValidationStatus::Valid => "valid",
        ValidationStatus::ValidWithWarnings => "valid (with warnings)",
        ValidationStatus::Invalid => "INVALID",
    };
    let balance = match b.status {
        statementbridge_core::model::BalanceStatus::Consistent => {
            format!("consistent ({} pairs checked)", b.checked_pairs)
        }
        statementbridge_core::model::BalanceStatus::Inconsistent => format!(
            "INCONSISTENT (first break at row {}, {} mismatch(es))",
            b.first_mismatch_row.unwrap_or(0),
            b.mismatches
        ),
        statementbridge_core::model::BalanceStatus::NotAvailable => "not available".to_string(),
    };
    let period = st
        .period
        .map(|p| format!("{} → {}", p.from, p.to))
        .unwrap_or_else(|| "—".to_string());
    let mut out = format!(
        "File:         {}\n\
         Adapter:      {}\n\
         Status:       {}\n\
         Period:       {}\n\
         Transactions: {}\n\
         Debits:       {}\n\
         Credits:      {}\n\
         Balance:      {}\n\
         Duplicates:   {} exact, {} possible{}\n\
         Errors:       {}\n\
         Warnings:     {}",
        st.source.file_name,
        adapter,
        status,
        period,
        st.summary.transaction_count,
        st.summary.total_debits,
        st.summary.total_credits,
        balance,
        st.summary.exact_duplicates,
        st.summary.possible_duplicates,
        if st.summary.duplicates_removed > 0 {
            format!(" ({} removed)", st.summary.duplicates_removed)
        } else {
            String::new()
        },
        v.errors.len(),
        v.warnings.len(),
    );
    for e in v.errors.iter().take(10) {
        out.push_str(&format!(
            "\n  error [{}]{}: {}",
            e.code,
            e.row.map(|r| format!(" row {r}")).unwrap_or_default(),
            e.message
        ));
    }
    if v.errors.len() > 10 {
        out.push_str(&format!("\n  … and {} more errors", v.errors.len() - 10));
    }
    for w in v.warnings.iter().take(10) {
        out.push_str(&format!(
            "\n  warning [{}]{}: {}",
            w.code,
            w.row.map(|r| format!(" row {r}")).unwrap_or_default(),
            w.message
        ));
    }
    if v.warnings.len() > 10 {
        out.push_str(&format!(
            "\n  … and {} more warnings",
            v.warnings.len() - 10
        ));
    }
    out
}

// ---------------------------------------------------------------------------
// Commands

fn cmd_inspect(json: bool, quiet: bool, a: InspectArgs) -> i32 {
    let prepared = match prepare(&a.engine, false) {
        Ok(p) => p,
        Err(f) => {
            emit_failure(json, "inspect", &f);
            return f.exit;
        }
    };
    let report = statementbridge_core::inspect(&prepared.raw, &prepared.options);
    if json {
        let data = serde_json::to_value(&report).unwrap_or(serde_json::Value::Null);
        emit_ok(json, quiet, "inspect", data, "");
        return exit_codes::OK;
    }
    let mut out = format!(
        "File:      {}\nFormat:    {}{}{}\nAdapter:   {}\nRows:      {} total, {} data\nHeader:    {}\nDates:     {}\nNumbers:   decimal '{}'{}\n",
        report.file_name,
        report.format,
        report
            .encoding
            .as_ref()
            .map(|e| format!(" · encoding {e}"))
            .unwrap_or_default(),
        report
            .delimiter
            .as_ref()
            .map(|d| format!(" · delimiter '{d}'"))
            .unwrap_or_default(),
        prepared.adapter_display,
        report.total_rows,
        report.data_rows,
        report
            .header_row
            .map(|r| format!("row {r}"))
            .unwrap_or_else(|| "not found (headerless mode)".to_string()),
        report.date_format.clone().unwrap_or_else(|| "—".to_string()),
        report.decimal_separator,
        report
            .thousands_separator
            .as_ref()
            .map(|t| format!(", thousands '{t}'"))
            .unwrap_or_default(),
    );
    if let Some(sheet) = &report.sheet_name {
        out.push_str(&format!(
            "Sheet:     {} (of: {})\n",
            sheet,
            report.sheet_names.join(", ")
        ));
    }
    out.push_str("Columns:\n");
    for c in &report.columns {
        out.push_str(&format!(
            "  {:>3}  {:<28} {}\n",
            c.index,
            truncate(&c.header, 28),
            c.field
                .clone()
                .map(|f| format!("→ {f}"))
                .unwrap_or_else(|| "(unmapped)".to_string())
        ));
    }
    if !report.sample_rows.is_empty() {
        out.push_str("Sample (masked):\n");
        for r in report.sample_rows.iter().take(5) {
            out.push_str(&format!("  {}\n", truncate(&r.join(" | "), 120)));
        }
    }
    for e in &report.errors {
        out.push_str(&format!("error [{}]: {}\n", e.code, e.message));
    }
    for w in &report.warnings {
        out.push_str(&format!("warning [{}]: {}\n", w.code, w.message));
    }
    // The report *is* inspect's output: print it even under --quiet
    // (--quiet silences decoration, never the requested data).
    let _ = quiet;
    print!("{out}");
    if report.errors.is_empty() {
        exit_codes::OK
    } else {
        exit_codes::INVALID
    }
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(n.saturating_sub(1)).collect();
        t.push('…');
        t
    }
}

fn cmd_convert(json: bool, quiet: bool, a: ConvertArgs) -> i32 {
    if a.format != "json" && a.format != "csv" {
        let f = Failure::runtime(
            "invalid_config",
            format!("unknown output format '{}' (expected: json, csv)", a.format),
        );
        emit_failure(json, "convert", &f);
        return f.exit;
    }
    let prepared = match prepare(&a.engine, true) {
        Ok(p) => p,
        Err(f) => {
            emit_failure(json, "convert", &f);
            return f.exit;
        }
    };
    let statement = match statementbridge_core::normalize(&prepared.raw, &prepared.options) {
        Ok(s) => s,
        Err(e) => {
            let f = Failure::from(e);
            emit_failure(json, "convert", &f);
            return f.exit;
        }
    };
    let document = match if a.format == "csv" {
        export::to_normalized_csv(&statement)
    } else {
        export::to_canonical_json(&statement)
    } {
        Ok(d) => d,
        Err(e) => {
            let f = Failure::from(e);
            emit_failure(json, "convert", &f);
            return f.exit;
        }
    };

    match &a.output {
        Some(path) if path.as_os_str() != "-" => {
            // Refuse to clobber the input file and require an existing parent.
            let same = std::fs::canonicalize(path)
                .ok()
                .zip(std::fs::canonicalize(&a.engine.file).ok())
                .map(|(o, i)| o == i)
                .unwrap_or(false);
            if same {
                let f = Failure::runtime("invalid_config", "refusing to overwrite the input file");
                emit_failure(json, "convert", &f);
                return f.exit;
            }
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() && !parent.is_dir() {
                    let f = Failure::runtime(
                        "io_error",
                        format!("output directory {} does not exist", parent.display()),
                    );
                    emit_failure(json, "convert", &f);
                    return f.exit;
                }
            }
            if let Err(e) = std::fs::write(path, document.as_bytes()) {
                let f =
                    Failure::runtime("io_error", format!("cannot write {}: {e}", path.display()));
                emit_failure(json, "convert", &f);
                return f.exit;
            }
            let human = format!(
                "{}\n\nWrote {} ({} bytes)",
                summary_lines(&statement, &prepared.adapter_display),
                path.display(),
                document.len()
            );
            if json {
                let data = serde_json::json!({
                    "output": path.display().to_string(),
                    "bytes": document.len(),
                    "statement": statement_summary_json(&statement),
                });
                emit_ok(json, quiet, "convert", data, "");
            } else if !quiet {
                println!("{human}");
            }
        }
        _ => {
            // Document to stdout; summary to stderr so pipes stay clean.
            print!("{document}");
            if !json && !quiet {
                eprintln!("{}", summary_lines(&statement, &prepared.adapter_display));
            }
        }
    }
    status_exit(&statement, prepared.options.strict)
}

fn statement_summary_json(st: &Statement) -> serde_json::Value {
    serde_json::json!({
        "status": st.validation.status,
        "transaction_count": st.summary.transaction_count,
        "errors": st.validation.errors.len(),
        "warnings": st.validation.warnings.len(),
        "exact_duplicates": st.summary.exact_duplicates,
        "possible_duplicates": st.summary.possible_duplicates,
        "balance_check": st.validation.balance_check.status,
    })
}

fn cmd_validate(json: bool, quiet: bool, a: ValidateArgs) -> i32 {
    let prepared = match prepare(&a.engine, false) {
        Ok(p) => p,
        Err(f) => {
            emit_failure(json, "validate", &f);
            return f.exit;
        }
    };
    let statement = match statementbridge_core::normalize(&prepared.raw, &prepared.options) {
        Ok(s) => s,
        Err(e) => {
            let f = Failure::from(e);
            emit_failure(json, "validate", &f);
            return f.exit;
        }
    };
    if json {
        let data = serde_json::json!({
            "summary": statement_summary_json(&statement),
            "validation": statement.validation,
        });
        emit_ok(json, quiet, "validate", data, "");
    } else if !quiet {
        println!("{}", summary_lines(&statement, &prepared.adapter_display));
    }
    status_exit(&statement, prepared.options.strict)
}

fn cmd_adapters(json: bool, quiet: bool, cmd: AdaptersCommand) -> i32 {
    match cmd {
        AdaptersCommand::List { adapters_dir } => {
            let extras = match load_extra_adapters(&adapters_dir) {
                Ok(e) => e,
                Err(f) => {
                    emit_failure(json, "adapters list", &f);
                    return f.exit;
                }
            };
            let builtins = adapters::builtin_adapters();
            if json {
                let items: Vec<serde_json::Value> = extras
                    .iter()
                    .map(|a| (a, "extra"))
                    .chain(builtins.iter().map(|a| (a, "builtin")))
                    .map(|(a, src)| {
                        serde_json::json!({
                            "id": a.id,
                            "version": a.version,
                            "display_name": a.display_name,
                            "description": a.description,
                            "amount_mode": a.amount_mode,
                            "source": src,
                        })
                    })
                    .collect();
                emit_ok(
                    json,
                    quiet,
                    "adapters list",
                    serde_json::Value::Array(items),
                    "",
                );
            } else {
                // The list *is* the output: print it even under --quiet.
                for (a, src) in extras
                    .iter()
                    .map(|a| (a, "extra"))
                    .chain(builtins.iter().map(|a| (a, "builtin")))
                {
                    println!(
                        "{:<26} {:<8} {:<8} {}",
                        a.id, a.version, src, a.display_name
                    );
                }
            }
            exit_codes::OK
        }
        AdaptersCommand::Validate { path } => {
            let text = match std::fs::read_to_string(&path) {
                Ok(t) => t,
                Err(e) => {
                    let f = Failure::runtime(
                        "io_error",
                        format!("cannot read {}: {e}", path.display()),
                    );
                    emit_failure(json, "adapters validate", &f);
                    return f.exit;
                }
            };
            match adapters::parse_adapter(&text) {
                Ok(spec) => {
                    let msg = format!("adapter '{}' v{} is valid", spec.id, spec.version);
                    emit_ok(
                        json,
                        quiet,
                        "adapters validate",
                        serde_json::json!({ "id": spec.id, "version": spec.version, "valid": true }),
                        &msg,
                    );
                    exit_codes::OK
                }
                Err(e) => {
                    let f = Failure {
                        code: "invalid_adapter".into(),
                        message: e.to_string(),
                        exit: exit_codes::INVALID,
                    };
                    emit_failure(json, "adapters validate", &f);
                    f.exit
                }
            }
        }
    }
}

const ADAPTER_TEMPLATE: &str = include_str!("templates/adapter-template.json");
const ADAPTER_README: &str = include_str!("templates/adapter-README.md");

fn cmd_init_adapter(json: bool, quiet: bool, a: InitAdapterArgs) -> i32 {
    let name = a.name.trim();
    let ok = !name.is_empty()
        && name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !name.starts_with('-');
    if !ok {
        let f = Failure::runtime(
            "invalid_config",
            format!("adapter name '{name}' must be 1-64 chars of [a-z0-9-] and not start with '-'"),
        );
        emit_failure(json, "init-adapter", &f);
        return f.exit;
    }
    let dir = a.dir.join(name);
    if dir.exists() {
        let f = Failure::runtime(
            "io_error",
            format!("{} already exists; refusing to overwrite", dir.display()),
        );
        emit_failure(json, "init-adapter", &f);
        return f.exit;
    }
    if let Err(e) = std::fs::create_dir_all(&dir) {
        let f = Failure::runtime("io_error", format!("cannot create {}: {e}", dir.display()));
        emit_failure(json, "init-adapter", &f);
        return f.exit;
    }
    let adapter_json = ADAPTER_TEMPLATE.replace("__ADAPTER_ID__", name);
    let readme = ADAPTER_README.replace("__ADAPTER_ID__", name);
    for (file, content) in [
        ("adapter.json", adapter_json.as_str()),
        ("README.md", readme.as_str()),
    ] {
        if let Err(e) = std::fs::write(dir.join(file), content) {
            let f = Failure::runtime("io_error", format!("cannot write {file}: {e}"));
            emit_failure(json, "init-adapter", &f);
            return f.exit;
        }
    }
    let msg = format!(
        "Created {}/adapter.json and README.md.\nNext steps:\n  1. Fill in the columns/aliases for your bank's export.\n  2. Validate: statementbridge adapters validate {}/adapter.json\n  3. Test against a *synthetic* fixture (never commit real statements).",
        dir.display(),
        dir.display()
    );
    emit_ok(
        json,
        quiet,
        "init-adapter",
        serde_json::json!({ "path": dir.display().to_string() }),
        &msg,
    );
    exit_codes::OK
}
