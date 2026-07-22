# Canonical schema reference

This is the field-by-field reference for the canonical statement document produced by StatementBridge — the output of `statementbridge convert`, `statementbridge_core::normalize` and the WASM `convert_file` function.

The formal contract is [`schemas/statementbridge-v1.schema.json`](../schemas/statementbridge-v1.schema.json) (JSON Schema draft 2020-12). The golden tests validate every produced document against it. This page explains the same structure for humans, plus the rules a schema cannot express: money semantics, hashing, duplicate taxonomy and the balance check.

Current schema version: **`1.0`**. The schema version is independent of the crate version; see [versioning.md](versioning.md).

## Document structure

```json
{
  "schema_version": "1.0",
  "source":       { … },
  "account":      { … },
  "period":       { … } | null,
  "transactions": [ … ],
  "summary":      { … },
  "validation":   { … }
}
```

### `source`

Provenance of the input file.

| Field | Type | Meaning |
|---|---|---|
| `file_name` | string | Sanitized base name of the input (final path component only, control characters stripped, capped at 255 chars; empty becomes `"statement"`). |
| `format` | `"csv"` \| `"xlsx"` | Container format, detected from content, never from the extension. |
| `adapter_id` | string | Id of the adapter that drove normalization (`generic-csv`, `generic-debit-credit`, …). |
| `imported_at` | string \| null | ISO-8601 timestamp **supplied by the caller**. The engine never reads a clock; `null` means the caller wanted deterministic output. The CLI `convert` command defaults it to the current time and accepts `--imported-at` to fix it. |
| `content_hash` | string | SHA-256 of the raw input bytes, lowercase hex. Lets you tie any output back to the exact file that produced it. |

### `account`

Aggregated account information. Fields the engine cannot determine are `null` — never invented.

| Field | Type | Meaning |
|---|---|---|
| `institution` | string \| null | Reserved; not populated by the v0.1 engine. |
| `account_name` | string \| null | Reserved; not populated by the v0.1 engine. |
| `account_masked` | string \| null | Masked account identifier such as `****4821`, taken from a detected account column. Full account numbers are never emitted (schema enforces the `****` prefix). When several distinct accounts appear, the first is used and a `multiple_accounts` warning is emitted. |
| `currency` | string \| null | ISO 4217 code when exactly one currency was seen across transactions, else the caller/adapter-provided currency, else `null`. |

### `period`

`{ "from": "YYYY-MM-DD", "to": "YYYY-MM-DD" }` — the date range spanned by the normalized transactions (after chronological ordering), or `null` when the file yielded no valid transactions.

### `transactions[]`

One entry per normalized row, in ascending chronological order (descending files are reversed; see the ordering rules below).

| Field | Type | Meaning |
|---|---|---|
| `id` | string | Deterministic id: `sb1-` + first 16 hex chars of SHA-256(`fingerprint ":" original_row`). Stable across runs for the same file bytes and options; distinct even for byte-identical duplicate rows (the row number differs). Pattern: `^sb1-[0-9a-f]{16}$`. |
| `date` | string | Transaction date, ISO `YYYY-MM-DD`. Required; rows without a parseable date become errors, not transactions. |
| `posted_at` | string \| null | Secondary date (booking/value date) when the file has one. A second date-like column is automatically mapped here. Unparseable values are dropped with an `invalid_posted_at` warning. |
| `description` | string | Normalized free text: Unicode NFC, exotic spaces folded, control and zero-width characters removed, whitespace collapsed, trimmed, capped at the field limit (default 2,000 chars, `field_truncated` warning when hit). |
| `reference` | string \| null | Normalized reference/document number (capped at 256 chars). |
| `debit` | string \| null | Debit magnitude as a **non-negative** decimal string, set only when the movement is a debit. |
| `credit` | string \| null | Credit magnitude, same convention. `debit` and `credit` are never both set: a row with values in both columns is a `debit_and_credit_both_set` error. |
| `amount` | string | The canonical signed amount: **debits negative, credits positive**. Always present. |
| `balance` | string \| null | Running balance after this transaction, when the file provides one. |
| `currency` | string \| null | ISO 4217 code for this transaction: from a currency column if it holds a valid code, else the caller/adapter currency, else inferred from an unambiguous currency symbol in the amount (see below), else `null`. |
| `original_row` | integer | 1-based row number in the **source file** — including preamble rows, blank lines and (for XLSX) leading empty sheet rows, so you can open the original and look at exactly that line. |
| `fingerprint` | string | SHA-256 content fingerprint (see below). |
| `duplicate_of_row` | integer, optional | When this row was classified against an earlier row: that row's `original_row`. Omitted otherwise. |
| `duplicate_kind` | `"exact"` \| `"possible"` \| `"repeated"`, optional | Duplicate classification (see below). Omitted for unique rows. |
| `warnings` | Issue[] | Row-scoped warnings (truncation, invalid balance, duplicate flags, …). |

### `summary`

| Field | Type | Meaning |
|---|---|---|
| `transaction_count` | integer | Number of transactions in the output (after any requested duplicate removal). |
| `total_debits` | string | Sum of debit magnitudes, non-negative decimal string, minimum two decimal places. |
| `total_credits` | string | Sum of credit magnitudes, same convention. |
| `opening_balance` | string \| null | Derived from the first transaction that carries a balance: `balance − amount`. `null` when no balances exist. |
| `closing_balance` | string \| null | Balance of the last transaction that carries one. |
| `exact_duplicates` | integer | Rows classified as exact duplicates. |
| `possible_duplicates` | integer | Rows flagged as possible duplicates. |
| `duplicates_removed` | integer | Exact duplicates actually removed (only with `remove_exact_duplicates`). |

### `validation`

| Field | Type | Meaning |
|---|---|---|
| `status` | `"valid"` \| `"valid_with_warnings"` \| `"invalid"` | `invalid` when any error exists; `valid_with_warnings` when there are no errors but any file-level or row-level warning exists; `valid` otherwise. **An invalid statement is still fully exported** — the status tells you to look at `errors`, it does not suppress output. |
| `errors` | Issue[] | Everything that made rows unusable or the file inconsistent, each with a stable `code`, a message, and usually a `row`/`column` anchor. |
| `warnings` | Issue[] | File-level assumptions and skipped rows. Capped at 100 per code, with a `warnings_capped` entry summarizing suppressed counts. |
| `balance_check` | object | Result of the balance-chain check (below). |

`balance_check`:

| Field | Meaning |
|---|---|
| `status` | `consistent` (all checked pairs match), `inconsistent` (at least one mismatch), `not_available` (no balance column, or fewer than two consecutive balances). |
| `checked_pairs` | Number of consecutive (previous balance, amount, balance) pairs verified. |
| `mismatches` | Total mismatching pairs. |
| `first_mismatch_row` | `original_row` of the first break, or `null`. |
| `expected` / `actual` | At the first break: the balance implied by the chain vs the balance the file states. |

### Issue objects

```json
{ "code": "invalid_amount", "message": "invalid amount: …", "row": 5, "column": "Importe" }
```

`code` is a stable machine contract: key automation on it, never on `message` (messages are human-oriented and may improve between versions). `row` is 1-based and refers to the source file; `column` is the source header name when known. Privacy rule: messages may quote raw *date* and *amount* tokens (capped at 24 characters — needed to fix files) but never descriptions, references, account numbers or counterparty names.

## Money rules

1. **Never floats.** Monetary values are decimal strings in JSON (`"249.99"`, pattern `^-?\d+(\.\d+)?$`) and `rust_decimal::Decimal` in memory. Text parses directly to `Decimal`; nothing monetary passes through `f64` inside the engine.
2. **Sign convention.** `amount` is signed: debit ⇒ negative, credit ⇒ positive. `debit`/`credit` hold non-negative magnitudes and are mutually exclusive. In debit/credit files, `amount = credit − debit`. A *negative* value in a debit column is economically a credit and is normalized as one, with a `negative_debit_normalized` warning (and symmetrically for negative credits).
3. **Scale preservation, no silent rounding.** The engine does not round parsed values: `"250.00"` stays `250.00`, `"0.005"` stays `0.005`. Parsing rejects more than 12 decimal places. Summary totals are rendered with at least two decimal places.
4. **Accepted numeric syntax.** Leading `-`, trailing `-` (common in bank exports), accounting parentheses `(1.50)`, leading `+`, the Unicode minus, NBSP/thin-space groupings, apostrophe groupings (`1'234.56`), and currency symbols or ISO-like codes at the edges (`$100`, `EUR 99,50`, `1.250,00 €`). Letters glued inside the digits (`12abc`) are an error — silently ignoring them would corrupt amounts. A thousands separator appearing *after* the decimal separator is an error.
5. **Separator inference is deterministic.** With both `.` and `,` present, the rightmost is the decimal separator. A single separator followed by 1–2 digits is decimal evidence; a repeated separator is thousands evidence; a lone 3-digit group (`1.234`) is genuinely ambiguous. When no evidence decides, the engine uses the locale convention (`es` ⇒ `,` decimal) or defaults to `.` decimal, and emits `ambiguous_number_format`. Pin `--decimal-separator` to silence it.
6. **Currency is never invented.** A currency column must hold an ISO 4217 alphabetic code to be used (`unknown_currency` warning otherwise). Without a column, the caller/adapter currency applies. As a last resort the engine infers a currency from an *unambiguous* symbol found in amounts — `€`→EUR, `£`→GBP, `₡`→CRC, `₹`→INR, `₩`→KRW. `$` and `¥` are used by many countries and are never guessed.
7. **The XLSX float caveat.** The XLSX file format stores numbers as IEEE-754 doubles — that is a property of the format, not of this tool. StatementBridge converts each numeric cell with shortest round-trip formatting (the string that uniquely identifies the double, so a cell displaying `1234.56` yields `"1234.56"`) and parses that as an exact decimal. This is the only place floats can appear in the pipeline. Amounts that were entered with more precision than a double can carry were already lossy inside the XLSX file itself. CSV input never touches floats at all.

## Date rules

- Output dates are always ISO `YYYY-MM-DD`. Datetime cells are accepted; the time part is dropped.
- Sixteen formats are recognized out of the box, including ISO, slash/dash/dot variants of day-first and month-first, two-digit years, compact `YYYYMMDD`, and month names — English and Spanish (`15 ene 2026`, `3 de enero de 2026`).
- **DD/MM vs MM/DD** is resolved by evidence over the whole date column: any component greater than 12 decides the order. When *every* row is ambiguous (all components ≤ 12), the engine picks day-first — the convention in most of the world, including Spanish-speaking countries — unless `--locale en` is set, which picks month-first. Either way it emits `ambiguous_date_format` naming the assumption; `--date-format` overrides and silences it.
- Rows whose date cannot be parsed at all become `invalid_date` errors (the raw token is quoted, capped at 24 chars); rows with an amount but an empty date cell become `missing_date` errors.

## Fingerprint and id

The **fingerprint** identifies the logical content of a transaction. It is the SHA-256 (lowercase hex) of the following normalized values, joined with the ASCII unit separator (U+001F):

```
date (ISO) · amount (trailing zeros stripped, so 1.50 ≡ 1.500)
· description matching key (lowercase, accents folded, whitespace collapsed)
· reference (trimmed, lowercase; empty when absent)
· balance (normalized; empty when absent)
· masked account (empty when absent)
```

Identical logical rows produce identical fingerprints on every platform and every run.

The **transaction id** additionally binds the row position, so that even byte-identical duplicate rows get distinct ids:

```
id = "sb1-" + first 16 hex chars of SHA-256(fingerprint + ":" + original_row)
```

Both are pure functions of file content and options — no clocks, no randomness.

## Duplicate kinds

Classification is deterministic and happens after ordering:

| Kind | Rule | What happens |
|---|---|---|
| `exact` | Identical full fingerprint (date, amount, description, reference, balance, account) as an earlier row. | Counted in `summary.exact_duplicates` and flagged with an `exact_duplicate` warning on the row. Removed **only** when the caller passes `remove_exact_duplicates` (`--remove-exact-duplicates`); the first occurrence is always kept and each removal is logged as `exact_duplicate_removed`. |
| `possible` | Same date + amount + description as an earlier row, and nothing (no differing reference, no differing balance) proves the rows are distinct. | Flagged with a `possible_duplicate` warning for human review. **Never removed automatically.** |
| `repeated` | Same date + amount + description, but a distinct reference or a distinct running balance proves both movements really happened (two identical coffee purchases). | Informational only: `duplicate_kind: "repeated"` is set, no warning, never removed. |

## Balance check semantics

When transactions carry balances, the engine verifies the chain over every pair of consecutive transactions that both have one:

```
expected_balance[i] = balance[previous] + amount[i]
|expected_balance[i] − balance[i]| ≤ tolerance  →  pair consistent
```

- **Tolerance** is half a unit at the finest decimal scale present in the file's amounts and balances (minimum scale 2). A file with two-decimal values gets a tolerance of `0.005` — enough to absorb representation noise, never enough to hide a real error.
- **Order matters, so order is detected first**: the date sequence decides ascending vs descending; for same-date files the balance chain itself is tried in both directions and the direction that fits wins (`transaction_order_assumed` warning when assumed). Descending files are reversed so the canonical output is always ascending. `--transaction-order` overrides detection.
- The **first mismatch** is reported as a `balance_mismatch` *error* with expected vs actual values and the row number; all further mismatches are counted in `balance_check.mismatches` but not repeated as individual errors. When the mismatching row is also an exact duplicate, the message suggests `--remove-exact-duplicates` — a duplicated row is the most common cause of a broken chain.
- A single broken row therefore makes the file `invalid`, with the cause named precisely — but the export still contains every transaction, and the user decides what to do.

## Issue code catalog

Codes are stable identifiers: adding a code is a minor change; renaming or removing one is breaking (see [versioning.md](versioning.md)).

### File and structure level (in `validation.warnings`, unless noted)

| Code | Meaning |
|---|---|
| `encoding_assumed` | The file was not plain UTF-8; it was decoded as UTF-16 (BOM), Windows-1252 or ISO-8859-1, as stated in the message. |
| `delimiter_low_confidence` | CSV delimiter detection lacked consistent evidence; the assumed delimiter is named — pass an explicit delimiter if columns look wrong. |
| `header_assumed` | No row matched known column names and a row was assumed to be the header; also emitted when an explicitly mapped column is missing, or when a headerless file's first row turned out to be a header after all. |
| `preamble_rows_skipped` | Informational rows before the detected header were skipped (count in the message). |
| `footer_row_skipped` | A totals/summary row (or a trailing row with neither date nor amount) was skipped. |
| `rows_truncated` | Reading stopped at the row limit; raise `--max-rows` to process the whole file. |
| `multiple_sheets` | The XLSX workbook has several non-empty sheets; the densest was chosen — pass `--sheet` to override. |
| `no_date_column` | **Error.** No date column detected; map it manually (`--map date=<column>`). |
| `no_amount_column` | **Error.** No amount (or debit/credit) column detected; map manually. |
| `ambiguous_date_format` | Day/month order could not be decided from the data; the assumption is named — set `--date-format` to override. |
| `ambiguous_number_format` | The decimal separator could not be decided from the data; the assumption is named — set `--decimal-separator` to override. |
| `transaction_order_assumed` | Transaction order was not determinable from dates; it was inferred from the balance chain or defaulted to ascending. |
| `currency_inferred_from_symbol` | The currency was inferred from an unambiguous symbol (€, £, ₡, ₹, ₩) found in the amounts, because no currency column or explicit option was provided. `$` and `¥` are never inferred. |
| `multiple_currencies` | More than one ISO currency appears in the file; `account.currency` is then only set from an explicit option. |
| `multiple_accounts` | More than one distinct masked account appears; the first is used for `account.account_masked`. |
| `warnings_capped` | More than 100 warnings of one code occurred; this entry states how many were suppressed. |
| `missing_amount` | A row had no debit/credit/amount value and was skipped (a warning, not an error — subtotal and narrative lines are common). |

### Row level

| Code | Where | Meaning |
|---|---|---|
| `invalid_date` | `validation.errors` | The date cell did not parse with the detected/configured format; the row was not converted. |
| `missing_date` | `validation.errors` | The row has an amount but an empty date cell; the row was not converted. |
| `invalid_amount` | `validation.errors` | A debit/credit/amount cell was not a parseable number; the row was not converted. Amounts are never silently corrected. |
| `debit_and_credit_both_set` | `validation.errors` | The row has non-zero values in both debit and credit; not allowed by the debit/credit rule, row not converted. |
| `negative_debit_normalized` | transaction `warnings` | A negative debit was treated as a credit (or a negative credit as a debit) — auditable, not silent. |
| `invalid_balance` | transaction `warnings` | The balance cell did not parse; the transaction keeps its amount but loses its balance. |
| `invalid_posted_at` | transaction `warnings` | The secondary date did not parse and was left empty. |
| `field_truncated` | transaction `warnings` | A description/reference exceeded the field limit and was truncated at a character boundary. |
| `description_missing` | transaction `warnings` | A description column exists but the cell is empty. |
| `unknown_currency` | transaction `warnings` | The currency cell is not an ISO 4217 code; the configured currency (if any) was used instead. |
| `cell_error` | `validation.warnings` | XLSX cells contained spreadsheet errors (`#N/A`, `#DIV/0!`, …) and were read as empty (count in the message). |

### Duplicates

| Code | Where | Meaning |
|---|---|---|
| `exact_duplicate` | transaction `warnings` | This row is an exact duplicate of the named earlier row (kept in the output). |
| `possible_duplicate` | transaction `warnings` | This row may duplicate the named earlier row; review before importing. |
| `exact_duplicate_removed` | `validation.warnings` | An exact duplicate of the named row was removed at the caller's request. |

### Balance chain

| Code | Where | Meaning |
|---|---|---|
| `balance_mismatch` | `validation.errors` | The balance chain breaks at the named row: expected vs actual balances are stated. |

## Fatal error codes (`EngineError`)

These never appear inside a statement — they are returned when the file could not be processed at all (CLI exit code 1, JSON envelope `error.code`):

`unsupported_format`, `empty_file`, `file_too_large`, `too_many_columns`, `encoding_error`, `csv_error`, `xlsx_error`, `adapter_not_found`, `invalid_adapter`, `invalid_config`, `io_error`.

## Validating documents yourself

Any JSON Schema draft 2020-12 validator can check a produced document against `schemas/statementbridge-v1.schema.json`. The repository's golden tests do exactly this for every fixture output, so the schema and the engine cannot drift apart unnoticed.
