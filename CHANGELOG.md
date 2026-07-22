# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). While the project
is in 0.x (alpha), minor versions may contain breaking changes; they are
always documented here.

## [Unreleased]

## [0.1.0] - 2026-07-22

First public alpha: local-first normalization of bank statement CSV/XLSX
exports into a canonical, validated format.

### Added

- **CSV import** with delimiter detection (comma, semicolon, tab, pipe) and
  encoding detection (UTF-8, UTF-8 with BOM, Windows-1252 / ISO-8859-1).
- **XLSX import** with multiple-sheet support (`--sheet`, defaulting to the
  sheet with the most data), date-cell handling, and formula cells
  contributing their cached values.
- **Structure detection**: the real header row is found even after
  informational preamble rows, and footer/total rows are excluded from the
  data.
- **Column identification** through an English/Spanish header alias table,
  with manual overrides via `--map FIELD=COLUMN` and `--mapping file.json`.
- **Canonical data model and versioned JSON Schema v1.0**
  (`schemas/statementbridge-v1.schema.json`). Monetary amounts are exact
  decimals serialized as strings — never floats — with debits negative and
  credits positive in the signed `amount`.
- **Deterministic output**: stable transaction ids and SHA-256 fingerprints;
  identical input and configuration produce byte-identical output
  (`--imported-at` pins the timestamp for reproducibility).
- **Duplicate detection** classifying exact duplicates, possible duplicates,
  and legitimately repeated transactions; `--remove-exact-duplicates`
  drops exact duplicates on request (first occurrence kept).
- **Balance-chain validation**: transaction order (ascending/descending) is
  detected automatically, the opening/amount/closing relationship is checked
  pairwise, and the first row where the chain breaks is reported.
- **Declarative adapter system** validated by a formal schema
  (`schemas/statementbridge-adapter-v1.schema.json`), three built-in generic
  adapters (`generic-csv`, `generic-debit-credit`,
  `generic-signed-amount`), loading of external adapters via
  `--adapter` / `--adapters-dir`, and an `init-adapter` command that
  scaffolds a documented template for contributors.
- **CLI** (`statementbridge`) with `inspect`, `convert` (canonical JSON or
  normalized CSV), `validate`, `adapters list`, `adapters validate`, and
  `init-adapter`; a machine-readable `--json` envelope for automation; and a
  stable exit-code contract (0 ok, 1 runtime error, 2 usage, 3 validation
  failed, 4 warnings with `--strict`).
- **Local web app** (English/Spanish) running the Rust core through
  WebAssembly entirely in the browser: drag-and-drop import, editable column
  mapping, validation summary, JSON/CSV download — no backend, no network
  calls during processing, no telemetry.
- **Security hardening**: formula-injection guard on CSV exports
  (CWE-1236), configurable file-size/row/field limits, `--redact` masking of
  long digit runs, account-number masking, refusal to overwrite the input
  file, and human-facing output designed to avoid leaking raw financial
  data.
- **Testing**: unit, property-based, golden, and CLI integration tests over a
  fully synthetic, deterministic fixture set (22 scenarios including
  encodings, locales, duplicates, inconsistent balances, formula cells, and
  injection attempts), plus a reproducible benchmark script
  (`scripts/benchmark.sh`).
- **Continuous integration**: GitHub Actions running rustfmt, clippy, the
  Rust test suite, a Linux/macOS/Windows CLI matrix, `cargo audit`, and web
  lint/tests/build; Dependabot updates; and a tag-triggered workflow that
  drafts (never auto-publishes) GitHub releases.

[Unreleased]: https://github.com/statementbridge/statementbridge/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/statementbridge/statementbridge/releases/tag/v0.1.0
