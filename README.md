# StatementBridge

Normalize bank statement exports (CSV/XLSX) into a canonical, validated format — entirely on your machine.

**Status: alpha (v0.1.0).** The core pipeline works end to end and is tested, but the project is young: expect rough edges, and read the [limitations](#limitations) before relying on it.

*Documentación en español: [README.es.md](README.es.md).*

## The problem

Every bank, payment processor and ERP exports transactions differently: different column names ("Fecha", "Transaction Date", "Posting Date"), different date orders (`01/02/2026` — which month?), different decimal separators (`1.234,56` vs `1,234.56`), different encodings (UTF-8, Windows-1252), debit/credit split across two columns or folded into one signed amount, preamble rows before the header, totals rows after the data. Accountants, finance teams and developers spend hours cleaning these files by hand before they can import them anywhere — and manual cleaning silently introduces errors into financial data.

## The solution

StatementBridge is an open-source, local-first toolkit that reads those messy CSV and XLSX exports and produces one canonical, machine-validated statement format:

- It detects encoding, delimiter, the real header row, column roles, date format and number format — and tells you what it assumed.
- It normalizes every transaction into a fixed schema: ISO dates, exact decimal amounts (never floats), a canonical sign convention (debits negative, credits positive), provenance back to the original row.
- It validates the result: balance-chain consistency, duplicate detection, per-row errors and warnings with stable machine-readable codes.
- It exports canonical JSON (validated by a published JSON Schema) or normalized CSV.
- It is usable three ways: as a Rust library, as a command-line tool, and as a local web application compiled to WebAssembly.
- Everything is deterministic: the same file plus the same options always produces byte-identical output.

## Your data stays local

Bank statements are sensitive. StatementBridge is built so they never leave your machine:

- The core engine performs **no network access and no file I/O** — callers hand it bytes, it hands back data.
- The CLI reads and writes only the files you name. No telemetry, no update checks, no phoning home.
- The web app processes files **in memory, in your browser**, via WebAssembly. There is no backend and no upload; the UI states "Your file never leaves this device" because that is how it is built.
- Account numbers are masked to `****NNNN` in the output; full account numbers are never emitted. Error messages and logs never quote descriptions, references or account numbers.

Details: [docs/privacy.md](docs/privacy.md) and [docs/threat-model.md](docs/threat-model.md).

## Features (v0.1.0)

- **Input**: CSV and XLSX. Format detected from content (magic bytes), never from the file extension. Legacy `.xls` is rejected with an actionable message.
- **CSV**: delimiter auto-detection (`,` `;` tab `|`), encodings UTF-8, UTF-8 BOM, UTF-16 LE/BE (BOM), Windows-1252 and ISO-8859-1 (heuristic, always reported as a warning).
- **XLSX**: read via [calamine](https://crates.io/crates/calamine); values only — macros and external links are never executed. Sheet auto-selection with override.
- **Detection**: real header row found even after informational preamble rows; columns identified through a built-in English/Spanish alias table; footer/total rows skipped with warnings; explicit column mapping when detection is not enough.
- **Normalization**: dates (many formats, day-first/month-first resolved by evidence), amounts with exact decimals, thousands/decimal separators inferred deterministically, Unicode text cleanup, currency as ISO 4217 only (never invented).
- **Amount modes**: separate debit/credit columns or a single signed amount column, auto-detected or forced.
- **Validation**: balance-chain check with first-mismatch reporting, ascending/descending order detection, per-row errors and warnings with stable codes.
- **Duplicates**: deterministic fingerprints distinguish exact duplicates, possible duplicates (flagged, never auto-removed) and legitimately repeated transactions. Exact duplicates are removed only when you ask.
- **Export**: canonical JSON (schema-validated) and normalized CSV with formula-injection guards.
- **Adapters**: declarative `adapter.json` documents describing a bank's export format — no code required to add one.
- **Safety**: file-size/row/column/field limits, account masking, optional redaction of digit runs, hostile-filename sanitization.

## Installation

### Requirements

- Rust 1.79 or newer (for the CLI and library).
- Node.js 20 or newer plus [pnpm](https://pnpm.io) (only for the web app).

Developed and tested on Linux. The code has no platform-specific dependencies, but treat macOS and Windows as less exercised until the CI platform matrix has been running for a while.

### CLI (from source)

```sh
git clone https://github.com/renealarcon-cmyk/statementbridge
cd statementbridge
cargo install --path crates/statementbridge-cli
# or, without installing:
cargo build --release   # binary at target/release/statementbridge
```

### Web app (development)

```sh
scripts/build-wasm.sh              # compile the engine to WebAssembly
pnpm --dir apps/web install
pnpm --dir apps/web dev            # local dev server; processing stays in the browser
```

## Sixty-second example

The repository ships fully synthetic fixtures. This one imitates a Spanish bank export: semicolon-delimited, decimal commas, preamble rows before the header, debit/credit columns and a totals row at the end.

Inspect it first — see what the engine detects before converting anything:

```text
$ statementbridge inspect fixtures/csv/semicolon_comma_es.csv
File:      semicolon_comma_es.csv
Format:    csv · encoding utf-8 · delimiter ';'
Adapter:   generic-debit-credit v1.0.0
Rows:      8 total, 5 data
Header:    row 3
Dates:     %d/%m/%Y
Numbers:   decimal ',', thousands '.'
Columns:
    1  Fecha                        → date
    2  Concepto                     → description
    3  Cargo                        → debit
    4  Abono                        → credit
    5  Saldo                        → balance
Sample (masked):
  02/01/2026 | Depósito inicial |  | 1.500,00 | 1.500,00
  05/01/2026 | Pago proveedor Ferretería Central | 249,99 |  | 1.250,01
  09/01/2026 | Factura consultoría proyecto Delta |  | 875,50 | 2.125,51
  12/01/2026 | Suscripción software | 49,00 |  | 2.076,51
  TOTAL |  | 298,99 | 2.375,50 |
warning [preamble_rows_skipped]: 2 informational row(s) before the header were skipped
```

Convert it to canonical JSON:

```text
$ statementbridge convert fixtures/csv/semicolon_comma_es.csv --output normalized.json
File:         semicolon_comma_es.csv
Adapter:      generic-debit-credit v1.0.0
Status:       valid (with warnings)
Period:       2026-01-02 → 2026-01-12
Transactions: 4
Debits:       298.99
Credits:      2375.50
Balance:      consistent (3 pairs checked)
Duplicates:   0 exact, 0 possible
Errors:       0
Warnings:     2
  warning [preamble_rows_skipped]: 2 informational row(s) before the header were skipped
  warning [footer_row_skipped] row 8: footer/summary row skipped

Wrote normalized.json (3057 bytes)
```

The first transaction in `normalized.json` (`02/01/2026` with `1.500,00` in the *Abono* column became an ISO date and an exact decimal credit):

```json
{
  "id": "sb1-7b78fb4a32dfa75a",
  "date": "2026-01-02",
  "posted_at": null,
  "description": "Depósito inicial",
  "reference": null,
  "debit": null,
  "credit": "1500.00",
  "amount": "1500.00",
  "balance": "1500.00",
  "currency": null,
  "original_row": 4,
  "fingerprint": "4a123e9e170bd955508d8866060121c594c55a1bed8fa1b234d469ef0ca86b6f",
  "warnings": []
}
```

Or produce a normalized CSV instead:

```text
$ statementbridge convert fixtures/csv/comma_dot_signed_iso.csv --format csv --output normalized.csv
```

```csv
id,date,posted_at,description,reference,debit,credit,amount,balance,currency,original_row,duplicate_kind
sb1-dc5c32510a41d116,2026-01-02,,Opening deposit,TRX-1001,,1500.00,1500.00,1500.00,USD,2,
sb1-efdc8924e1684f20,2026-01-05,,Office supplies — Acme Tools,TRX-1002,249.99,,-249.99,1250.01,USD,3,
```

## CLI usage

```text
statementbridge inspect  <file>                 # show what the engine detects
statementbridge convert  <file> [--output f]    # canonical JSON (default) or --format csv
statementbridge validate <file>                 # report errors/warnings without exporting
statementbridge adapters list                   # built-in + --adapters-dir adapters
statementbridge adapters validate <adapter.json>
statementbridge init-adapter <name>             # scaffold an adapter for contribution
```

Frequently used options (run `statementbridge <command> --help` for the full list):

| Option | Purpose |
|---|---|
| `--adapter <id-or-path>` | Use a specific adapter instead of auto-detection |
| `--map date=Fecha --map amount=#4` | Explicit column mapping (header name or `#N`, 1-based) |
| `--date-format %d/%m/%Y` | Pin the date format (repeatable, tried in order) |
| `--decimal-separator , --thousands-separator .` | Pin number formats |
| `--currency EUR` | Currency to assume when the file has none |
| `--locale en\|es` | Tie-break hint for genuinely ambiguous dates/numbers |
| `--transaction-order auto\|ascending\|descending` | Override order detection |
| `--strict` | Treat warnings as failures (exit code 4) |
| `--redact` | Mask long digit runs in descriptions/references |
| `--remove-exact-duplicates` | Drop exact duplicate rows (first occurrence kept) |
| `--imported-at 2026-01-01T00:00:00Z` | Fixed timestamp for byte-reproducible output |
| `--json` | Machine-readable envelope `{"ok":…,"data":…}` on stdout |
| `--quiet` | Suppress non-essential human messages |

Exit codes are a stable contract, suitable for scripts and CI:

| Code | Meaning |
|---|---|
| 0 | Success (valid, or valid with warnings without `--strict`) |
| 1 | Runtime failure (unreadable file, bad adapter, I/O) |
| 2 | Usage error (unknown flag or argument) |
| 3 | Validation failed (`validation.status == "invalid"`) |
| 4 | Warnings present and `--strict` was set |

`convert` prints the document to stdout and the human summary to stderr, so piping stays clean; `--output file` writes the document and refuses to overwrite the input file.

## Using StatementBridge as a Rust library

The engine is three focused crates: `statementbridge-formats` (bytes → raw table), `statementbridge-adapters` (declarative configuration), `statementbridge-core` (normalization, validation, export). None of them perform I/O — you read the bytes, they do the rest:

```rust
use statementbridge_adapters as adapters;
use statementbridge_core::{export, normalize, NormalizeOptions};
use statementbridge_formats as formats;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Read the file bytes yourself; the engine never touches the filesystem.
    let bytes = std::fs::read("statement.csv")?;

    // 2. Parse bytes into a raw table (format, encoding and delimiter detected).
    let raw = formats::read_bytes(&bytes, "statement.csv", &formats::ReadOptions::default())?;

    // 3. Pick an adapter (auto-detected here) and resolve options.
    let candidates = adapters::builtin_adapters();
    let spec = adapters::detect_adapter(&raw, &candidates)
        .ok_or("no adapter supports this file")?;
    let options = spec.apply(&NormalizeOptions::default())?;

    // 4. Normalize and export.
    let statement = normalize(&raw, &options)?;
    println!("status: {:?}", statement.validation.status);
    let json = export::to_canonical_json(&statement)?;
    std::fs::write("normalized.json", json)?;
    Ok(())
}
```

The crates are not published to crates.io yet (planned for v0.2); use them as path or git dependencies for now. The same engine is exposed to JavaScript through `statementbridge-wasm` (functions `inspect_file`, `convert_file`, `statement_to_csv`, returning JSON envelopes).

## The canonical format

Every conversion produces one JSON document validated by [`schemas/statementbridge-v1.schema.json`](schemas/statementbridge-v1.schema.json) (schema version `1.0`). The essentials:

- **Money is exact.** All monetary values are decimal *strings* (`"249.99"`), parsed and computed with `rust_decimal`. Floats are never used for money.
- **One sign convention.** `amount` is the canonical signed value: debits negative, credits positive. `debit`/`credit` carry the non-negative magnitudes.
- **Provenance.** Every transaction records `original_row` (1-based row in the source file) and the source object records the SHA-256 of the input bytes.
- **Determinism.** `id` is derived from a SHA-256 fingerprint of the normalized fields plus the row number; the same input and options always yield the same ids, byte for byte.
- **Nothing is hidden.** `validation.errors` and `validation.warnings` list every problem with a stable `code`, a human message and the affected row; a file with errors is exported anyway, marked `"status": "invalid"`.

```json
{
  "schema_version": "1.0",
  "source": {
    "file_name": "semicolon_comma_es.csv",
    "format": "csv",
    "adapter_id": "generic-debit-credit",
    "imported_at": null,
    "content_hash": "fdcdf38ae0c5860e51fb5de7f2bde5b5f3f2e3f7e99aa91d79478e0845acd5c5"
  },
  "account": { "institution": null, "account_name": null, "account_masked": null, "currency": null },
  "period": { "from": "2026-01-02", "to": "2026-01-12" },
  "transactions": [ "…" ],
  "summary": {
    "transaction_count": 4,
    "total_debits": "298.99",
    "total_credits": "2375.50",
    "opening_balance": "0.00",
    "closing_balance": "2076.51",
    "exact_duplicates": 0,
    "possible_duplicates": 0,
    "duplicates_removed": 0
  },
  "validation": {
    "status": "valid_with_warnings",
    "errors": [],
    "warnings": [ "…" ],
    "balance_check": { "status": "consistent", "checked_pairs": 3, "mismatches": 0,
                       "first_mismatch_row": null, "expected": null, "actual": null }
  }
}
```

Full field-by-field reference, issue-code catalog and the fingerprint algorithm: [docs/canonical-schema.md](docs/canonical-schema.md).

## Adapters

An adapter is a single declarative JSON file describing one bank or processor export: expected headers, extra header aliases, date formats, separators, amount mode, footer patterns. No code. Three generic adapters ship built in:

| id | Handles |
|---|---|
| `generic-csv` | Full auto-detection fallback (most files) |
| `generic-debit-credit` | Separate debit/credit columns (cargo/abono, debe/haber, withdrawal/deposit) |
| `generic-signed-amount` | One amount column with positive/negative values |

The right adapter is auto-detected from the header row; you can always force one with `--adapter`. To contribute an adapter for your bank:

```sh
statementbridge init-adapter acmebank-checking
# edit adapters/acmebank-checking/adapter.json
statementbridge adapters validate adapters/acmebank-checking/adapter.json
```

The scaffold includes a documented template and a README with the contribution rules (synthetic fixtures only — never real statements). Full guide: [docs/adapter-authoring.md](docs/adapter-authoring.md).

## The web app

`apps/web` contains a local-first web application (React + TypeScript + Vite) that runs the same Rust engine compiled to WebAssembly. The flow: drop a file, review the automatic detection and the first rows, adjust the column mapping and date/currency settings if needed, normalize, review the summary (valid transactions, errors, warnings, duplicates, balance differences), browse the results table, download JSON or CSV, and clear the file from memory with one click. English and Spanish interface. No backend, no analytics, no network requests during processing, no persistence of your documents.

## Limitations

Honest list, current as of v0.1.0:

- **Alpha software.** The canonical schema aims to be stable, but APIs may still change between 0.x minor versions (see [docs/versioning.md](docs/versioning.md)).
- **No PDF, no OCR.** Only CSV and XLSX. PDF support is designed for (a future reader in `statementbridge-formats`) but not implemented. Legacy `.xls` is rejected — re-save as `.xlsx` or CSV.
- **No bank connections.** No Open Banking, no credential storage, no scraping. You export the file from your bank; StatementBridge normalizes it.
- **Heuristics can be wrong.** Delimiter, encoding, date-order and separator detection are deterministic and evidence-based, but a genuinely ambiguous file gets a documented assumption plus a warning — review warnings, or pin formats explicitly with flags or an adapter.
- **XLSX numbers pass through floats once.** The XLSX format itself stores numbers as IEEE-754 doubles. Values are converted with shortest round-trip formatting and then handled as exact decimals; this is inherent to XLSX and documented in [docs/canonical-schema.md](docs/canonical-schema.md).
- **Formula cells use cached values.** Formulas are never evaluated; a formula cell without a cached value reads as empty and is reported.
- **One statement per run.** No multi-file merging, no reconciliation against invoices, no accounting classification.
- **Not published yet.** No crates.io/npm packages until v0.2; install from source.
- **Not financial advice.** StatementBridge normalizes data. It does not interpret it, and its output should be reviewed before being used for accounting or tax purposes.

## Security and privacy

Parsing untrusted files is in scope: statements come from many sources and pass through many hands. Defenses include CSV formula-injection guards on export, size/row/column/field limits, no macro or link execution for XLSX, filename sanitization, account masking, and error messages that never quote free-text field contents. Read [docs/privacy.md](docs/privacy.md) for the data-handling model and [docs/threat-model.md](docs/threat-model.md) for assets, attacker capabilities, mitigations and residual risks. Report vulnerabilities privately as described in [SECURITY.md](SECURITY.md).

## Roadmap

- **v0.1 (current)**: CSV, XLSX, canonical schema, declarative adapters, CLI, local web app, validation, duplicates, export.
- **v0.2**: better format detection, stable plugin API, more community adapters, n8n integration, npm and crates.io packages.
- **v0.3**: PDF with text layer, optional local OCR, payment-processor imports, reconciliation rules.
- **v1.0**: stable API, documented compatibility, adapter conformance suite, integrations ecosystem, support policy.

Details and rationale: [docs/roadmap.md](docs/roadmap.md).

## Contributing

Contributions are welcome — adapters for new banks are the easiest way to help and require no Rust. Start with [CONTRIBUTING.md](CONTRIBUTING.md) and [docs/adapter-authoring.md](docs/adapter-authoring.md). One command runs all tests, one runs all lints:

```sh
scripts/test-all.sh
scripts/lint-all.sh
```

Never include real bank data in issues, fixtures or pull requests. All fixtures must be synthetic.

## Documentation map

| Document | Contents |
|---|---|
| [docs/architecture.md](docs/architecture.md) | Crate layout, data flow, design boundaries |
| [docs/canonical-schema.md](docs/canonical-schema.md) | Field-by-field output reference, money rules, issue codes |
| [docs/adapter-authoring.md](docs/adapter-authoring.md) | How to write and submit an adapter |
| [docs/privacy.md](docs/privacy.md) | What is processed, where, and what never happens |
| [docs/threat-model.md](docs/threat-model.md) | Assets, attackers, mitigations, residual risks |
| [docs/benchmarks.md](docs/benchmarks.md) | Reproducible benchmark methodology (no fabricated numbers) |
| [docs/decisions.md](docs/decisions.md) | Architecture decision records |
| [docs/versioning.md](docs/versioning.md) | SemVer policy, public API surfaces, MSRV |
| [docs/roadmap.md](docs/roadmap.md) | Where the project is going |

## License

Apache License 2.0. See [LICENSE](LICENSE) and [NOTICE](NOTICE).
