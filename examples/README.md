# Examples

Practical recipes for StatementBridge. All commands run against the synthetic
fixtures in `fixtures/` — swap in your own files.

## CLI recipes

Convert a Spanish-style export (semicolon, comma decimals, debit/credit) and
pretty-inspect it first:

```sh
statementbridge inspect fixtures/csv/semicolon_comma_es.csv
statementbridge convert fixtures/csv/semicolon_comma_es.csv --output normalized.json
```

Normalized CSV instead of JSON:

```sh
statementbridge convert fixtures/csv/debit_credit_mdy.csv --format csv --output normalized.csv
```

Automation-friendly: JSON envelope on stdout, quiet, fixed timestamp for
reproducible artifacts, strict warnings:

```sh
statementbridge convert statement.csv --json -q \
  --imported-at 2026-01-01T00:00:00Z --strict > result.json
echo "exit: $?"   # 0 ok · 1 runtime · 2 usage · 3 invalid · 4 warnings with --strict
```

Pipe into `jq` — total of all debits:

```sh
statementbridge convert statement.csv -q | jq -r '.summary.total_debits'
```

List transactions flagged as possible duplicates:

```sh
statementbridge convert statement.csv -q \
  | jq -r '.transactions[] | select(.duplicate_kind == "possible") | [.date, .amount, .description] | @tsv'
```

Headerless file with explicit column mapping (1-based `#N` indexes):

```sh
statementbridge convert fixtures/csv/headerless.csv \
  --map date=#1 --map description=#2 --map amount=#3
```

Drop exact duplicate rows (first occurrence kept, every removal reported):

```sh
statementbridge convert statement.csv --remove-exact-duplicates -q | jq '.summary'
```

## Using the Rust library

See the "Use as a Rust library" section in the top-level README for a
compile-checked snippet (read bytes → detect adapter → normalize → export).
The crates are not on crates.io yet (planned for v0.2); use a git or path
dependency meanwhile.

## n8n / workflow automation

Until the dedicated n8n node exists (roadmap v0.2), call the CLI from an
Execute Command node and parse stdout:

```sh
statementbridge convert "$INPUT_FILE" --json -q --imported-at "$NOW"
```

The `--json` envelope (`{"ok":…,"command":…,"data":…}`) and the stable exit
codes are designed for exactly this kind of integration.
