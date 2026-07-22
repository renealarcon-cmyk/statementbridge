# Architecture

This document describes how StatementBridge is put together: the crates, the data flow, why the boundaries sit where they sit, and the two design properties everything else serves — determinism and honesty about assumptions.

Related reading: [canonical-schema.md](canonical-schema.md) (the output contract), [decisions.md](decisions.md) (why individual choices were made), [threat-model.md](threat-model.md) (security view of the same boundaries).

## Repository layout

```
statementbridge/
├── crates/
│   ├── statementbridge-core/       # canonical model, normalization, validation, export
│   ├── statementbridge-formats/    # bytes → RawTable (CSV, XLSX, encodings, limits)
│   ├── statementbridge-adapters/   # declarative adapter spec, validation, detection
│   ├── statementbridge-cli/        # `statementbridge` binary (clap)
│   └── statementbridge-wasm/       # wasm-bindgen bindings for the web app
├── apps/
│   └── web/                        # local-first React app using the WASM engine
├── adapters/                       # built-in adapter.json documents (embedded in the binary)
├── schemas/                        # JSON Schemas: canonical statement + adapter format
├── fixtures/                       # synthetic test files (csv/, xlsx/, golden/)
├── scripts/                        # test-all, lint-all, build-wasm, benchmark, fixture generator
└── docs/                           # this documentation
```

## Crate dependency graph

```
                 ┌─────────────────────┐
                 │ statementbridge-core │   no I/O, no network, no clock
                 └──────────┬──────────┘
             ┌──────────────┼──────────────┐
             ▼              ▼              │
┌────────────────────┐ ┌─────────────────┐ │
│statementbridge-    │ │statementbridge- │ │
│formats             │ │adapters         │ │
│(csv, xlsx, encodings)│(spec, detect,   │ │
└─────────┬──────────┘ │ validate)       │ │
          │            └───────┬─────────┘ │
          └───────┬────────────┘           │
                  ▼                        ▼
      ┌───────────────────┐    ┌───────────────────┐
      │statementbridge-cli │    │statementbridge-   │
      │(binary, does I/O)  │    │wasm (browser API) │
      └───────────────────┘    └───────────────────┘
```

Arrows point from dependent to dependency. `core` depends on nothing else in the workspace; `formats` and `adapters` depend only on `core`; the CLI and the WASM crate are thin shells that compose all three.

## Responsibilities per crate

### statementbridge-core

The engine. It owns:

- **The canonical model** (`model.rs`): `Statement`, `Transaction`, `Summary`, `Validation` — a direct Rust mirror of `schemas/statementbridge-v1.schema.json`. Money is `rust_decimal::Decimal` in memory and a decimal string in JSON; dates are `chrono::NaiveDate`.
- **The pipeline** (`normalize.rs`): header/column detection → format inference (dates, numbers, currency) → per-row parsing → chronological ordering → duplicate classification → balance-chain validation → summary.
- **Support modules**: `dates` (parsing and day-first/month-first inference), `numbers` (exact decimal parsing and separator inference), `header` (header detection and the English/Spanish alias table), `text` (Unicode normalization and matching keys), `fingerprint` (SHA-256 hashing, transaction ids), `dedupe` (exact/possible/repeated classification), `mask` (account masking, digit-run redaction), `issues` (stable error/warning codes and the per-code warning cap), `limits` (resource caps), `export` (canonical JSON and guarded CSV), `inspect` (detection-only report with masked samples).

Design guarantees stated in the crate root and enforced by tests:

- **No I/O, no network, no clock.** Callers hand it a `RawTable` (already-parsed rows) and options; it hands back a `Statement`. `imported_at` is caller input precisely so the engine never reads a clock.
- **Exact money.** Text parses directly to `Decimal`; floats never touch monetary values inside the engine.
- **Determinism.** Identical input plus identical options produces byte-identical output. All maps that affect output ordering are `BTreeMap`s; tie-breaks in every heuristic are defined.
- **Safe logging.** `tracing` events carry counts, codes and row numbers — never descriptions, references, amounts or account numbers.

### statementbridge-formats

Byte-level readers producing a `statementbridge_core::RawTable`. Pure functions over byte slices — callers do the file I/O:

- **Format detection** by magic bytes, never by extension: ZIP + `[Content_Types].xml` is XLSX; a ZIP that is not a workbook, an OLE container (legacy `.xls`) or NUL-dense binary data is rejected with an actionable error; everything else is treated as CSV.
- **Encoding detection** (`encoding.rs`): BOMs first (UTF-8, UTF-16 LE/BE), then strict UTF-8, then Windows-1252 vs ISO-8859-1 by a documented heuristic (bytes in 0x80–0x9F are printable in Windows-1252 but control characters in ISO-8859-1). Every non-plain-UTF-8 assumption emits an `encoding_assumed` warning.
- **CSV** (`csv_read.rs`): delimiter scoring over the first 40 lines (`,` `;` tab `|`), preferring the most consistent field count; low-confidence detection is reported. Blank lines are kept as empty row slots so `rows[i]` is always source line `i + 1` — this is what makes `original_row` provenance exact.
- **XLSX** (`xlsx_read.rs`): via calamine. Cell values only; macros, external links and embedded objects are never executed (they are simply never read). Formula cells contribute their cached value; error cells (`#N/A`, `#DIV/0!`) read as empty and are counted in a `cell_error` warning. The sheet with the most non-empty rows is chosen unless one is named explicitly.
- **Limits enforcement**: maximum file bytes (fail fast), maximum columns (fail fast — a 300-column row is not a statement), maximum rows (stop loudly with `rows_truncated`), plus a hard per-cell cap upstream of core's per-field cap.

### statementbridge-adapters

The declarative adapter system:

- `spec.rs` — the `AdapterSpec` struct mirroring `adapter.json` (`serde` with `deny_unknown_fields`), and `AdapterSpec::apply`, which merges an adapter into a base `NormalizeOptions`. Precedence is fixed: explicit user/CLI values win over adapter values, which win over engine inference.
- `validate.rs` — semantic validation beyond what a JSON Schema can express (regexes must compile, chrono patterns must render, separators must differ, mode/column combinations must be coherent).
- `detect.rs` — deterministic adapter scoring against the detected header row (see [adapter-authoring.md](adapter-authoring.md) for the exact scoring rules).
- Three built-in generic adapters are embedded into every binary at compile time from `adapters/`.

Adapters are configuration, never code: they cannot execute anything, and a hostile adapter file can at worst mis-map columns — which validation then reports.

### statementbridge-cli

The `statementbridge` binary. It owns everything the core refuses to do: reading files, writing files, reading the clock (default `imported_at` for `convert`), and process exit codes. Its human output prints counts, codes, row numbers and masked samples; its `--json` mode wraps every result in a stable `{"ok": …, "data"| "error": …}` envelope. Exit codes (0 ok, 1 runtime, 2 usage, 3 validation failed, 4 strict warnings) are part of the public contract.

### statementbridge-wasm

A thin `wasm-bindgen` layer exposing `inspect_file`, `convert_file`, `statement_to_csv`, `adapters_json`, `version` and `schema_version` to JavaScript. Every fallible function returns a JSON envelope string instead of throwing, so the web app has one uniform error path. The module holds no state and performs no I/O; the browser default file-size limit is 50 MiB (vs 100 MiB in the CLI) and is caller-overridable.

### apps/web

A React + TypeScript + Vite application that drives the WASM engine entirely in the browser: drag and drop, automatic detection, editable column mapping, normalization, a validation summary (valid rows, errors, warnings, duplicates, balance differences), a results table, JSON/CSV download, and a clear-memory control. No backend, no analytics, no network requests during processing, no persistence of documents.

## Data flow

```
            file bytes (CLI: fs::read · web: File API)
                          │
                          ▼
        statementbridge-formats::read_bytes
          ├─ detect_format (magic bytes)
          ├─ decode (BOM / UTF-8 / 1252 / 8859-1)
          ├─ detect_delimiter | select sheet
          └─ enforce size/row/column limits
                          │
                          ▼
                      RawTable            rows exactly as read + metadata
                          │               (encoding, delimiter, sheet, sha-256,
                          │                reader warnings)
                          ▼
        statementbridge-adapters::detect_adapter   (or explicit --adapter)
                          │
                          ▼
              AdapterSpec::apply(base options)  →  NormalizeOptions
                          │
                          ▼
             statementbridge-core::normalize
               ├─ header & column detection (alias table + explicit mapping)
               ├─ inference: date formats, number format, currency symbol
               ├─ per-row parsing (dates, decimals, text cleanup, masking)
               ├─ chronological ordering (auto-detected, reversed if descending)
               ├─ duplicate classification (exact / possible / repeated)
               ├─ balance-chain validation
               └─ summary + validation status
                          │
                          ▼
                      Statement           canonical model, schema version 1.0
                          │
              ┌───────────┴───────────┐
              ▼                       ▼
   export::to_canonical_json   export::to_normalized_csv
   (byte-stable pretty JSON)   (RFC 4180, injection-guarded)
```

`inspect` runs the same pipeline up to and including detection, then stops and reports what it found, with sample rows masked for safe display.

## Why the boundaries sit where they do

- **`formats` is separate from `core`** so that new input formats plug in without touching normalization. A future PDF-with-text-layer reader (roadmap v0.3) is one new module in `statementbridge-formats` producing the same `RawTable`; an optional OCR path is a producer in front of that. Nothing downstream changes.
- **`core` does no I/O** so the exact same engine runs in the CLI, in tests, and in a browser via WASM — and so the security review surface for "code that can touch your disk or network" is confined to the CLI shell.
- **Adapters are data, not plugins**, so contributing support for a bank requires editing one JSON file, and loading a third-party adapter cannot execute code. A code-level plugin API is deliberately deferred to v0.2, after the data-level format has proven what it needs.
- **The CLI and WASM crates are thin** so behavior differences between surfaces stay near zero: both reduce their inputs to the same `NormalizeOptions` and call the same functions.

## Determinism design

Determinism — identical input and options produce byte-identical output — is a hard requirement, because normalized statements get hashed, diffed, cached and committed as golden test files. The design consequences:

- `imported_at` is caller input. The engine never reads the clock; only the CLI's `convert` fills in "now" as a convenience, and `--imported-at` overrides it. Library and WASM output is reproducible by default.
- Transaction ids derive from content: `sb1-` + first 16 hex chars of SHA-256(`fingerprint ":" original_row`). No UUIDs, no counters that depend on iteration order.
- Every heuristic has defined tie-breaks: delimiter candidates in fixed order, adapter scoring ties broken by priority then id, greedy column assignment sorted by (score, column, field), `BTreeMap` everywhere iteration order can leak into output.
- Pretty JSON key order is fixed by the struct definitions; export appends a trailing newline; CSV uses `\n` terminators regardless of platform.
- A dedicated test converts the same fixture twice and asserts byte equality, and golden tests pin full outputs for seven representative fixtures.

## Error philosophy

There are two distinct failure channels, and keeping them apart is deliberate:

1. **Fatal engine errors (`EngineError`)** — the file could not be processed at all: unsupported format, undecodable text, oversized input, broken adapter, invalid configuration. These are Rust `Err` values (CLI exit code 1) with actionable messages that never contain financial field values.
2. **Row-level and file-level issues (`Issue`)** — the file was processed, and here is everything wrong or assumed: `invalid_amount` on row 3, `ambiguous_date_format`, `balance_mismatch` at row 14. These land in `validation.errors` and `validation.warnings` of the output statement, each with a stable machine-readable `code`, so a user sees *all* problems in one pass instead of fixing them one crash at a time.

Two related rules:

- **Errors do not suppress output.** A statement with errors is still fully exported, marked `"status": "invalid"` (CLI exit code 3). Users can inspect exactly what failed; nothing is hidden, and nothing is silently "fixed".
- **Every assumption is auditable.** Whenever the engine guesses (encoding, delimiter, header row, date order, decimal separator, transaction order), it emits a warning naming the guess and the flag that overrides it. Warnings are capped at 100 per code with an explicit `warnings_capped` summary so a million-row file cannot balloon the report.

The full issue-code catalog is in [canonical-schema.md](canonical-schema.md).
