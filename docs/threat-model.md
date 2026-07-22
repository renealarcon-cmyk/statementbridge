# Threat model

StatementBridge parses financial files that come from many origins — bank portals, email attachments, files forwarded by clients — and produces files that get opened in spreadsheet applications. Both directions cross a trust boundary. This document names the assets, the boundaries, what an attacker can do, how the code answers, and what risk remains.

Scope note: **parsing untrusted statement files is in scope by design.** "Only open files you trust" is not a usable rule for accountants; the parser must assume hostility.

## Assets

1. **The user's financial data** — transactions, balances, account identifiers, in the input file, in memory, and in the outputs.
2. **The user's machine and browser session** — the process/tab must not become an execution vector for file-borne payloads.
3. **Downstream systems** — the spreadsheet or accounting software that opens StatementBridge's CSV export.
4. **Output integrity** — normalized amounts must be exactly what the file said (or an explicit error); silent corruption of financial data is treated as a security failure here, not just a bug.

## Trust boundaries

```
untrusted file ──► statementbridge-formats ──► core engine ──► exports ──► spreadsheet apps
untrusted adapter.json ──► adapters (validate) ──► options ────┘
user flags/mapping ─────────────────────────────► options
```

- Everything inside the engine is trusted code operating on untrusted *data*.
- Adapter files are untrusted *configuration* (a user may download one from a stranger).
- Outputs are data that will be interpreted by third-party software with known parsing quirks (Excel formula evaluation).

## Attacker capabilities and mitigations

### 1. Malicious or malformed statement file

**Capability:** the attacker fully controls the bytes of a file the user will open.

| Attack | Mitigation (where in the code) |
|---|---|
| CSV cells crafted to execute as formulas when the *output* is opened in Excel/LibreOffice (CSV injection, CWE-1236) | On export, text cells (description, reference) beginning with `=`, `+`, `-`, `@`, tab or carriage return are prefixed with a single quote so spreadsheets treat them as text (`core/src/export.rs::csv_guard`, covered by unit, integration and fixture tests). Numeric and date columns are engine-generated and cannot carry payloads. |
| Resource exhaustion: multi-GB files, million-column rows, unbounded cell sizes | Explicit limits, all overridable and all loud: max file bytes (100 MiB CLI / 50 MiB web defaults — refuse fast), max 256 columns (refuse — that is not a statement), max 1,000,000 rows (stop with a `rows_truncated` warning), 32 KiB hard per-cell cap at the reader plus a 2,000-char per-field cap with `field_truncated` warnings (`core/src/limits.rs`, `formats/src/*`). |
| XLSX with macros, external links, embedded objects | Never executed and never followed: the reader (calamine) extracts cell values only; macro streams and link targets are simply not read (`formats/src/xlsx_read.rs`). Formula cells contribute only their cached value; nothing is evaluated. |
| ZIP-based container games (a ZIP that is not a workbook; legacy OLE `.xls`; binary blobs renamed `.csv`) | Format detection is content-based (magic bytes plus `[Content_Types].xml` check), never extension-based; non-XLSX ZIPs, OLE containers and NUL-dense binary data are rejected with typed errors (`formats/src/lib.rs::detect_format`). |
| Encoding tricks (mislabeled encodings, BOM abuse, malformed UTF-16) | Deterministic decode order (BOMs → strict UTF-8 → Windows-1252/ISO-8859-1 heuristic); malformed UTF-16 and BOM-contradicting content are hard errors; every assumption is surfaced as `encoding_assumed` (`formats/src/encoding.rs`). |
| Hostile file names (`../../etc/passwd`, control characters, oversized names) | The reported file name is reduced to its final path component, control characters stripped, capped at 255 chars (`formats::sanitize_file_name`); the CLI writes only to the path the user provided, refuses to overwrite the input file, and requires the output directory to already exist (no path creation on the attacker's behalf). |
| Amount-corruption attempts (letters glued into numbers, double separators, separators after the decimal point) | Parse-or-error: invalid amounts are per-row errors, never silently "fixed" (`core/src/numbers.rs`); a property test asserts the parser never panics on arbitrary input. |
| Content designed to leak through logs or error messages | Issue messages may quote only date/amount tokens, capped at 24 characters; descriptions, references and account values are never quoted; `tracing` events carry counts and row numbers only; `inspect` samples mask 8+-digit runs and cap cell previews (`core/src/issues.rs` doc contract, `core/src/inspect.rs`, CLI tests). |

### 2. Malicious adapter file

**Capability:** the attacker supplies an `adapter.json` the user loads via `--adapter` or `--adapters-dir`.

Adapters are pure data — there is deliberately no plugin code path, so an adapter cannot execute anything. Remaining vectors and answers:

| Attack | Mitigation |
|---|---|
| Pathological regexes in `footer_patterns` (compile bombs) | Regexes are compiled with a 1 MiB size limit and must compile at validation time; invalid patterns are rejected (`adapters/src/validate.rs`, `core/src/normalize.rs`). The regex engine used (the `regex` crate) has linear-time matching by design — no catastrophic backtracking. |
| Panicking chrono format strings | Every `date_formats` entry is test-rendered during validation; non-renderable patterns are rejected. |
| Structural nonsense (unknown fields, absurd `rows_to_skip`, colliding separators, contradictory mode/column combinations) | `deny_unknown_fields` deserialization plus semantic validation with bounded numeric ranges; `adapters validate` exits non-zero with the reasons. |
| Deliberate mis-mapping (an adapter that maps the balance column to `amount`) | Cannot be fully prevented — configuration is meant to change mapping. Countermeasures: the output names the adapter (`source.adapter_id`), `inspect` shows the resulting column mapping before conversion, and balance-chain validation tends to fail loudly on nonsense mappings. Review third-party adapters like you would review a config file. |

### 3. Malicious output consumer path

The normalized CSV will be opened in spreadsheet software. Besides the formula-injection guard above, the export uses RFC 4180 quoting via the `csv` crate (no hand-rolled escaping) and UTF-8 with `\n` terminators, minimizing parser disagreement between writer and reader.

## Design-level defenses

- **No `unsafe` code** anywhere in the workspace (`unsafe_code = "deny"` in workspace lints); memory-safety bugs in this codebase are limited to what safe Rust allows.
- **No network capability** in the engine, and none in the CLI either — exfiltration would require a supply-chain compromise, not a parsing bug.
- **No I/O in the engine**: file-system attack surface is confined to the small CLI shell.
- **Determinism as a defense**: identical inputs produce identical outputs, so any tampering along the way is detectable by hash comparison (`source.content_hash` for inputs; byte-stable JSON for outputs).
- **Fail loud, not silent**: every limit hit, every assumption, every skipped row produces a coded warning or error. An attacker cannot rely on quiet degradation to hide manipulation.

## Residual risks

Named honestly, because they exist:

1. **Upstream parser bugs.** XLSX parsing (calamine), ZIP handling and CSV tokenization are third-party crates; a memory-safety or logic bug there is inherited. Mitigations: dependency updates and `cargo audit` runs, no `unsafe` locally, and size limits reducing exposure — not elimination.
2. **No process sandboxing.** The CLI runs with the user's full privileges; the library runs in the host process. StatementBridge does not fork a sandboxed worker. The browser/WASM build is the most isolated deployment; on the CLI, users parsing highly suspect files can add their own OS-level sandboxing.
3. **Heuristic ambiguity is not fully solvable.** A file where day/month order or decimal separators are genuinely undecidable gets a documented, warned assumption. An attacker-controlled file could exploit a wrong assumption to shift dates or scales — but only within what the emitted `ambiguous_*` warnings disclose. Strict mode (`--strict`) turns those warnings into failures for pipelines that must not guess.
4. **Balance-check tolerance.** The half-unit-at-finest-scale tolerance could mask a manipulation smaller than half a cent per step. It exists to absorb representation noise; reducing it below one representable unit would produce false alarms.
5. **The `-` formula guard has a usability cost.** Text starting with `-` is apostrophe-prefixed in CSV exports even when harmless; this is accepted in favor of not classifying payloads ourselves.
6. **Masked identifiers are still partial identifiers.** `****4821` plus dates and amounts can be enough to correlate an account across leaked outputs. Masking reduces exposure; it is not anonymization.

## Out of scope

- Attacks on the user's machine, OS, browser or spreadsheet application itself.
- Malicious modifications of the StatementBridge source or binaries (supply-chain of *this* project is addressed by review + CI, not by the threat model of the tool).
- Network attackers: there is no network component.
- Confidentiality of the *outputs* once the user shares them (see [privacy.md](privacy.md)).
- Cryptographic verification of statement authenticity — StatementBridge validates internal coherence, not whether the bank really issued the file.

## Reporting

Report suspected vulnerabilities privately per [SECURITY.md](../SECURITY.md) rather than opening a public issue. Reproduction cases must use synthetic files — never real statements.
