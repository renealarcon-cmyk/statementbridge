# Writing an adapter

Adapters are the intended way for the community to teach StatementBridge about specific bank and processor export formats — without writing any code. An adapter is one declarative JSON document (`adapter.json`) that pins down what the generic detection would otherwise have to guess: which headers to expect, what the date format is, which separator the bank uses, which rows to skip.

This guide covers when to write one, every field of the format, the `init-adapter` workflow, testing, and the submission checklist.

## When to write an adapter

Try the generic pipeline first:

```sh
statementbridge inspect your-export.csv
```

If the detected columns, date format and separators are right, you may not need an adapter at all — `generic-csv`, `generic-debit-credit` and `generic-signed-amount` cover a lot of real-world files. Write an adapter when:

- the export is **genuinely ambiguous** and you keep needing `--date-format`, `--decimal-separator` or `--map` flags (an adapter records them once);
- the bank uses **header names the alias table does not know** ("Movimiento", "Importe Cargo (EUR)", …);
- the export has a **fixed preamble** (`rows_to_skip`) or **footer lines** the defaults do not catch;
- you want the file to be **auto-detected** as this bank's format, so users of the same bank get correct results with zero flags.

An adapter cannot do things the engine cannot do: it configures the pipeline, it does not add parsing logic. If your bank's format needs new *behavior* (a new date syntax, a new container format), open an issue instead — that is an engine change.

## Quick start

```text
$ statementbridge init-adapter acmebank-checking
Created adapters/acmebank-checking/adapter.json and README.md.
Next steps:
  1. Fill in the columns/aliases for your bank's export.
  2. Validate: statementbridge adapters validate adapters/acmebank-checking/adapter.json
  3. Test against a *synthetic* fixture (never commit real statements).
```

`init-adapter` scaffolds a documented template plus a README with the contribution rules. It refuses to overwrite an existing directory. The adapter id must be 1–64 characters of `[a-z0-9-]`, not starting with `-`; pick something specific and stable: `acmebank-checking-csv`, not `bank1`.

Use the adapter immediately, before it is ever merged anywhere:

```sh
statementbridge convert statement.csv --adapter adapters/acmebank-checking/adapter.json
# or make a whole directory of adapters available and let detection choose:
statementbridge convert statement.csv --adapters-dir adapters/
```

Adapters loaded from a path or `--adapters-dir` win over built-ins on id collision, so you can also override a built-in locally.

## Anatomy of adapter.json

The formal contract is [`schemas/statementbridge-adapter-v1.schema.json`](../schemas/statementbridge-adapter-v1.schema.json); `statementbridge adapters validate` checks both the schema-level structure and semantic rules (regexes must compile, chrono patterns must render, separators must differ, mode/column combinations must be coherent). Unknown fields are rejected — typos fail loudly instead of being ignored.

A minimal valid adapter needs `id`, `version`, `display_name`, `description`, `supported_formats` and `amount_mode`; everything else has defaults.

### Identity

| Field | Type / allowed | Default | Meaning |
|---|---|---|---|
| `id` | `[a-z0-9-]`, ≤ 64 chars | — (required) | Stable identifier, used in `--adapter` and recorded in every output's `source.adapter_id`. Never change it once published. |
| `version` | `MAJOR.MINOR.PATCH` | `"0.1.0"` | Version of *this adapter document*, independent of the StatementBridge version. Bump it when you change the adapter. |
| `display_name` | string ≤ 120 | — (required) | Human name shown by `adapters list` and the web app. |
| `description` | string ≤ 1000 | — (required) | Which bank, which export screen/menu, which language. Write it for a stranger. |
| `supported_formats` | array of `"csv"` / `"xlsx"` | `["csv","xlsx"]` | Container formats this adapter applies to. Detection never offers an adapter for a format it does not list. |
| `locales` | array of `xx` or `xx-XX` | `[]` | Informational: languages/regions of the export. |

### Detection

| Field | Type | Default | Meaning |
|---|---|---|---|
| `expected_headers` | array of strings | `[]` | Headers that **must** be present for this adapter to auto-match. Matched case- and accent-insensitively, and — when an entry is a canonical field name like `"debit"` — also through the built-in alias table, so `"debit"` matches a `Cargo` column. A missing expected header effectively disqualifies the adapter (see scoring below). |
| `header_aliases` | object: canonical field → array of strings | `{}` | Extra header names for this bank, merged over the built-in English/Spanish table; adapter aliases win equal-score ties. Keys must be canonical fields: `date`, `posted_at`, `description`, `reference`, `debit`, `credit`, `amount`, `balance`, `currency`, `account`. |
| `detection_priority` | integer 0–100 | `0` | Tie-breaker added to the detection score. Specific adapters should sit above the generic ones (`generic-debit-credit` is 50, `generic-signed-amount` 40, `generic-csv` 10) — the template uses 60. |

### Structure

| Field | Type | Default | Meaning |
|---|---|---|---|
| `rows_to_skip` | integer 0–100 | `0` | Rows to skip before header detection starts. Usually unnecessary — the engine finds headers after preambles on its own — but useful when a preamble row *looks like* a header. |
| `footer_patterns` | array of regex strings | `[]` | Case-insensitive regular expressions matching footer rows to skip, **added to** the built-in defaults (totals, `saldo final`, closing/opening balance lines, etc.). Each match is reported with a `footer_row_skipped` warning. |

### Formats

| Field | Type / allowed | Default | Meaning |
|---|---|---|---|
| `date_formats` | array of chrono patterns | `[]` (infer) | Tried in order, e.g. `["%d/%m/%Y"]`. Set this whenever the bank is consistent — it removes the day-first/month-first ambiguity entirely. |
| `decimal_separator` | `"."` or `","` or null | `null` (infer) | Decimal separator used by the export. |
| `thousands_separator` | `"."`, `","`, `"'"`, `" "` or null | `null` (infer) | Grouping separator. Must differ from `decimal_separator`. |
| `transaction_order` | `"auto"` / `"ascending"` / `"descending"` | `"auto"` | Fix the row order when the bank always exports one way. |
| `currency` | ISO 4217 code or null | `null` | Currency to assume when the file itself carries none. Only set it when the export genuinely never states a currency and is always in one. |

### Amounts and columns

| Field | Type / allowed | Default | Meaning |
|---|---|---|---|
| `amount_mode` | `"auto"` / `"debit_credit"` / `"signed"` | `"auto"` | `debit_credit`: separate debit and credit columns. `signed`: one amount column with +/− values. `auto` decides from the detected columns. |
| `date_column` | column ref or null | `null` | Explicit column mapping for the date. |
| `debit_column` / `credit_column` | column ref or null | `null` | Explicit debit/credit columns (only meaningful with `debit_credit` mode; setting them in `signed` mode is a validation error). |
| `signed_amount_column` | column ref or null | `null` | Explicit amount column (`signed` mode; rejected in `debit_credit` mode). |
| `balance_column`, `reference_column`, `description_column`, `currency_column`, `account_column` | column ref or null | `null` | Explicit mappings for the remaining fields. |

A **column ref** is either a header name (matched case- and accent-insensitively) or `#N` for the 1-based column position — `"#3"` is the third column. Positional refs make headerless exports workable.

Explicit columns are usually unnecessary: good `header_aliases` let detection do the work and survive the bank reordering its columns. Reach for explicit refs when headers are absent, duplicated or unusable.

### Masking

| Field | Type | Default | Meaning |
|---|---|---|---|
| `account_masking_rules` | `{ "min_digits": 4–34, "keep_last": 2–6 }` | `{ "min_digits": 6, "keep_last": 4 }` | Masking parameters applied to account-column values: a value with at least `min_digits` digits is masked down to `****` + its last `keep_last` digits. Values with fewer digits are not treated as account numbers. |

## How detection scoring works

When no `--adapter` is given, every candidate adapter is scored against the detected header row; the highest score wins. The rules, in order:

1. An adapter that does not list the file's format in `supported_formats` is excluded outright.
2. Each entry in `expected_headers` that is found in the header — directly (substring of a normalized header cell) or via the built-in alias table when the entry is a canonical field name — adds **+25**. Each expected header that is *missing* subtracts **1000**, which effectively disqualifies the adapter: expected headers are requirements, not suggestions.
3. Each `header_aliases` value found verbatim (normalized) in the header adds **+10**.
4. `detection_priority` (0–100) is added to the total.
5. Ties break by higher `detection_priority`, then by lexicographically smaller `id` — fully deterministic.

This is why `generic-csv` (no expectations, priority 10) is the universal fallback, and why a file with `Cargo`/`Abono` columns lands on `generic-debit-credit` (its expected `debit`/`credit` match through the alias table, +50 plus priority 50).

Practical consequence for authors: list in `expected_headers` only what is *always* present in the bank's export. One over-specified header makes your adapter silently unselectable.

## Testing your adapter

1. **Build a synthetic fixture.** Recreate the bank's layout — same columns, same separators, same preamble/footer shape — with invented data: fake business names, fake amounts, an invented account number. Never start from a real statement and "anonymize" it; write it from scratch. Keep it small (5–10 rows) and include one edge the adapter must handle (a footer line, a thousands separator, a negative amount).

2. **Validate the document:**

   ```text
   $ statementbridge adapters validate adapters/acmebank-checking/adapter.json
   adapter 'acmebank-checking' v0.1.0 is valid
   ```

   Exit code 0 on success, 3 with the first problem (and a total count) on failure.

3. **Check detection picks it** (this is the part that regresses most easily):

   ```sh
   statementbridge inspect fixture.csv --adapters-dir adapters/
   # the "Adapter:" line must name your adapter
   ```

4. **Convert and read the whole output**, not just the status:

   ```sh
   statementbridge convert fixture.csv --adapter adapters/acmebank-checking/adapter.json
   ```

   Verify: dates are the ISO dates you meant, debits are negative in `amount`, the balance check is `consistent` if your fixture has coherent balances, and there are no leftover warnings about assumptions your adapter should have pinned (`ambiguous_date_format`, `ambiguous_number_format` mean your adapter should set `date_formats` / separators).

5. **Exercise `--strict`** if you want CI-grade confidence: with a well-pinned adapter and a clean fixture, `statementbridge validate fixture.csv --adapter … --strict` should exit 0.

## Submission checklist

Before opening a pull request:

- [ ] `id` is lowercase, specific and final; `display_name` and `description` let a stranger identify the export.
- [ ] `statementbridge adapters validate` passes.
- [ ] A synthetic fixture exists and converts correctly; detection selects the adapter with `--adapters-dir`.
- [ ] **Data is 100% synthetic**: no real statements (even "anonymized"), no real account numbers or IBANs, no customer or employee names, no real transaction narratives copied from your own statement.
- [ ] No bank logos, trademarks or scraped artwork anywhere.
- [ ] `expected_headers` lists only always-present headers; `detection_priority` is above the generics (typically 60) and below 100.
- [ ] Formats that the bank keeps consistent (`date_formats`, separators, `transaction_order`) are pinned; everything genuinely variable is left to inference.
- [ ] The adapter README generated by `init-adapter` is updated, not deleted.

Naming convention: `<institution>-<account-or-product>[-<format>]`, all lowercase — `acmebank-checking`, `acmebank-business-xlsx`. One adapter per distinct export layout; if the bank's CSV and XLSX differ structurally, ship two adapters.

## Reference: a filled-in example

The scaffolded template, with the placeholders it asks you to replace:

```json
{
  "id": "acmebank-checking",
  "version": "0.1.0",
  "display_name": "My Bank — checking account export",
  "description": "Describe the export this adapter handles: which bank, which screen/menu produces it, CSV or XLSX, language.",
  "supported_formats": ["csv"],
  "locales": ["en"],
  "expected_headers": ["date", "amount"],
  "header_aliases": {
    "date": ["Transaction Date"],
    "description": ["Details"],
    "amount": ["Amount (USD)"]
  },
  "rows_to_skip": 0,
  "footer_patterns": ["^\\s*grand total\\b"],
  "date_formats": ["%m/%d/%Y"],
  "decimal_separator": ".",
  "thousands_separator": ",",
  "transaction_order": "auto",
  "currency": null,
  "amount_mode": "signed",
  "date_column": null,
  "debit_column": null,
  "credit_column": null,
  "signed_amount_column": null,
  "balance_column": null,
  "reference_column": null,
  "description_column": null,
  "currency_column": null,
  "account_column": null,
  "account_masking_rules": { "min_digits": 6, "keep_last": 4 },
  "detection_priority": 60
}
```

For how adapter values combine with CLI flags and engine inference (user flags win over adapter values, which win over inference), see [architecture.md](architecture.md); for what the normalized output means, see [canonical-schema.md](canonical-schema.md).
