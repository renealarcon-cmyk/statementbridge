# Architecture decision records

Short records of the design decisions that shaped StatementBridge, in ADR style: context, decision, consequences. Each one is visible in the code today; none is aspirational. Statuses are all **accepted** as of v0.1.0.

Format inspired by Michael Nygard's ADRs, kept deliberately brief. Newer decisions get appended with the next number.

---

## ADR-001: Money is exact decimals, serialized as strings

**Context.** IEEE-754 floats cannot represent most decimal amounts (`0.1 + 0.2 ≠ 0.3`), and JSON numbers are floats in almost every consumer. A tool whose whole purpose is trustworthy financial data cannot round-trip money through either.

**Decision.** All monetary values are `rust_decimal::Decimal` in memory and decimal *strings* in JSON (`"249.99"`), enforced by serde adapters and by the JSON Schema (`^-?\d+(\.\d+)?$` on string types). Parsing goes directly from text to `Decimal`; parsed scale is preserved (no silent rounding), and values with more than 12 decimal places are rejected rather than truncated.

**Consequences.** Consumers must parse strings into their own decimal types — mildly less convenient than JSON numbers, and deliberately so: the inconvenience is the guarantee. Summary totals render with a minimum of two decimals for readability. The one unavoidable float exposure is quarantined in ADR-007.

---

## ADR-002: `imported_at` is caller input; the engine never reads a clock

**Context.** Determinism (same input + same options ⇒ byte-identical output) is required for golden tests, output diffing, caching and audit. A timestamp generated inside the engine would break it for every run.

**Decision.** `NormalizeOptions.imported_at` is an optional caller-provided ISO-8601 string, recorded verbatim; `null` is valid. Only the CLI's `convert` command — an interactive convenience surface — defaults it to the current time, and `--imported-at` pins it. The library and WASM paths are deterministic by default.

**Consequences.** Reproducible pipelines get byte-stable outputs with one flag. The cost is a slightly surprising `"imported_at": null` in library output, documented in the schema reference. No other engine code path may read clocks or RNGs; the determinism test would catch regressions that affect output.

---

## ADR-003: Issue codes are a stable machine contract; messages are not

**Context.** Automation needs to react to specific problems ("fail the pipeline on `balance_mismatch`, tolerate `footer_row_skipped`"), but error text needs freedom to improve.

**Decision.** Every warning and error carries a stable snake_case `code` from a single catalog (`core/src/issues.rs`). Adding a code is a minor change; renaming or removing one is breaking. Messages are explicitly human-oriented and may change in any release. Messages may quote raw date/amount tokens (capped) but never descriptions, references or account values.

**Consequences.** Scripts key on `code` and survive upgrades; message wording can be improved without ceremony. The catalog is documented exhaustively in [canonical-schema.md](canonical-schema.md).

---

## ADR-004: Duplicates are classified exact / possible / repeated — and only exact ones are ever removed

**Context.** Real statements legitimately contain identical-looking rows (two same-day coffee purchases). A tool that auto-deletes "duplicates" corrupts financial data; a tool that flags everything is noise.

**Decision.** Three-way deterministic taxonomy: **exact** (identical full fingerprint, including reference/balance/account when present), **possible** (same date + amount + description, and *nothing* proves the rows distinct), **repeated** (same loose key but a differing reference or differing balance proves both happened — informational only, no warning). Removal exists only for exact duplicates, only on explicit request (`--remove-exact-duplicates`), always keeping the first occurrence, always logging each removal.

**Consequences.** The engine never destroys data on its own judgment. Human review is explicitly part of the possible-duplicate flow. The balance-chain error message closes the loop by suggesting removal when an exact duplicate breaks the chain.

---

## ADR-005: Ambiguous day/month order defaults to day-first, loudly

**Context.** `03/04/2026` is unresolvable in isolation. Evidence usually decides (any component > 12), but some files are ambiguous on every row, and refusing to parse them would make the tool useless for exactly the short, clean files users try first.

**Decision.** Inference uses whole-column evidence first. When both day-first and month-first parse *every* row, the engine picks day-first — the convention of most of the world and of the project's primary Spanish-speaking audience — unless `locale = en` is set, which picks month-first. Either way it emits `ambiguous_date_format` naming the assumed format, and `--date-format` overrides.

**Consequences.** A deterministic, documented guess instead of a refusal or a silent one. US-format files without evidence need `--locale en` or `--date-format` (or an adapter that pins it — the durable fix). Strict pipelines can treat the warning as failure via `--strict`.

---

## ADR-006: Windows-1252 vs ISO-8859-1 is decided by C1 bytes, and the assumption is labeled

**Context.** Non-UTF-8 Latin statements are common. The two encodings differ only in 0x80–0x9F: printable characters (€, quotes, dashes) in Windows-1252, control characters in ISO-8859-1. No detector can distinguish them beyond that range because they are byte-identical from 0xA0 up.

**Decision.** After BOM checks and strict UTF-8 validation fail: if any byte falls in 0x80–0x9F, label the file `windows-1252`; otherwise label it `iso-8859-1`. Both decode through the Windows-1252 table (identical for the bytes that can occur in the second case). Every non-UTF-8 decode emits `encoding_assumed`.

**Consequences.** `Café € 5,00` decodes correctly; pure Latin-1 text decodes correctly; the reported encoding name is honest about what the evidence supports. Genuinely different legacy encodings (e.g. Cyrillic code pages) are out of scope for v0.1 and would decode wrongly — but visibly, with the warning pointing at the assumption.

---

## ADR-007: XLSX numbers convert via shortest round-trip formatting, and this is documented as the single float exposure

**Context.** The XLSX format stores numbers as IEEE-754 doubles — a property of the format, not a choice available to any reader. Pretending otherwise would be dishonest; parsing the doubles sloppily (fixed-precision printing) would manufacture noise like `1234.5599999999999`.

**Decision.** Each numeric cell is converted with Rust's shortest round-trip float formatting — the shortest string that uniquely identifies the stored double, so a cell holding the double nearest `1234.56` yields `"1234.56"` — and that string is parsed as an exact `Decimal` downstream. The caveat is documented in the schema reference and the README rather than hidden.

**Consequences.** CSV input never touches floats; XLSX input touches them exactly once, at the format boundary, with the least-surprising conversion available. Precision beyond a double's capacity was already lost inside the XLSX file itself and cannot be recovered by any tool.

---

## ADR-008: CSV exports guard formula injection with an apostrophe prefix

**Context.** Spreadsheet applications execute cell content beginning with `=`, `+`, `-` or `@` as formulas (CSV injection, CWE-1236). Statement descriptions are attacker-influenced text (counterparties choose their own transfer descriptions), and StatementBridge output is destined for Excel/LibreOffice.

**Decision.** On CSV export, text cells (description, reference) starting with `=`, `+`, `-`, `@`, tab or carriage return get a leading single quote — the convention spreadsheets interpret as "literal text". Engine-generated numeric/date columns are not guarded because they cannot carry payloads.

**Consequences.** Payloads like `=HYPERLINK(...)` arrive inert. The cost: legitimate text beginning with `-` (or `+`) also gains an apostrophe; visible but harmless, and preferable to the tool deciding what is "really" a payload. The canonical JSON export is unaffected (JSON strings are not evaluated by spreadsheets).

---

## ADR-009: Adapters are declarative JSON, not code plugins

**Context.** Format coverage must scale through community contribution. Code plugins have a security cost (executing strangers' code over financial data), a review cost, and an API-stability cost the project cannot pay at 0.x.

**Decision.** An adapter is one `adapter.json` — pure configuration validated by a JSON Schema plus semantic checks (`deny_unknown_fields`, compiled regex limits, renderable chrono patterns, bounded numeric ranges). Built-ins are embedded at compile time; extras load from disk and can shadow built-ins by id. There is no code-execution path through an adapter.

**Consequences.** Contributing a bank format is editing JSON — reviewable in minutes, testable with `adapters validate`, safe to download. The expressiveness ceiling is real: behavior the schema cannot express requires an engine change. A stable *code* plugin API is deferred to v0.2 (see [roadmap.md](roadmap.md)), to be designed against the accumulated evidence of what data alone could not do.

---

## ADR-010: A dedicated `wasm-release` profile, size-optimized

**Context.** The web app downloads the engine on first load; browser payload size is a user-facing cost. The native CLI wants speed; WASM wants size. One release profile cannot serve both.

**Decision.** The workspace defines `[profile.wasm-release]` (inherits release; `opt-level = "s"`, full LTO, `codegen-units = 1`, `panic = "abort"`) used only for the WASM build, while native release keeps thin LTO and symbol stripping. `scripts/build-wasm.sh` additionally applies `wasm-opt -Os` when available.

**Consequences.** Neither target subsidizes the other's constraints. `panic = "abort"` is acceptable in WASM because the bindings return error envelopes instead of unwinding across the boundary, and a console panic hook covers diagnostics. Real sizes are measured, not asserted — see [benchmarks.md](benchmarks.md).

---

## ADR-011: File-level warnings are capped at 100 per code, with an explicit summary

**Context.** A million-row file with a systematic issue (every row missing an amount) would otherwise generate a million warning objects — memory pressure, unusable reports, and a denial-of-service vector through report inflation.

**Decision.** File-level warnings pass through a collector that keeps at most 100 instances per code while counting all of them; at the end, one `warnings_capped` entry per capped code states exactly how many more were suppressed. Errors and per-transaction warnings are not silently dropped by this mechanism; the cap governs the file-level warning list, and the total counts remain visible.

**Consequences.** Reports stay bounded and readable for pathological inputs, and nothing disappears without a trace — the suppression itself is a coded, countable warning. Automation that counts occurrences should read the `warnings_capped` totals rather than the list length.

---

## ADR-012: Blank lines occupy row slots so `original_row` matches the source file exactly

**Context.** Provenance is only useful if `original_row: 14` means line 14 of the file the user can open. The `csv` crate skips blank lines and does not count them in its positions, and XLSX ranges may start below row 1 — both silently shift row numbers.

**Decision.** The CSV reader maps record byte offsets back to physical line numbers and pads the row vector with empty rows so `rows[i]` is always source line `i + 1`; the XLSX reader pads up to the sheet range's real starting row. The engine skips empty rows during processing but never renumbers around them.

**Consequences.** Every reported row number — in errors, warnings, duplicate references and `original_row` — can be checked against the original file with a text editor or spreadsheet showing line numbers. The cost is a few empty vectors in memory and reader code that is more careful than the naive loop; the golden and reader tests pin the behavior.
