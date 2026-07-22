# Privacy

StatementBridge processes financial documents, so its privacy posture is a design constraint, not a feature toggle. This page states exactly what is processed, where, what never happens, and what remains your responsibility. The security counterpart — who might attack this and how the code defends — is [threat-model.md](threat-model.md).

## What is processed

The input to every operation is the byte content of one statement file (CSV or XLSX) that you explicitly provide, plus the options you set. From it the tool derives dates, amounts, descriptions, references, balances, currencies and (when present) account identifiers — that is the job.

## Where processing happens

Everywhere StatementBridge runs, processing is local and in memory:

- **Library (`statementbridge-core`, `-formats`, `-adapters`)**: pure functions over byte slices and in-memory tables. The engine crates perform no file I/O, no network access and never read the system clock. What you pass in is all they ever see.
- **CLI**: reads the file you name, writes the output you name (stdout by default). Nothing else is read or written. There is no update check, no crash reporter, no usage ping — the binary has no networking code at all.
- **Web app**: the file is read with the browser File API and handed to the same Rust engine compiled to WebAssembly, inside your browser tab. There is no backend and no upload endpoint; the "Your file never leaves this device" notice describes the architecture, not a policy promise.

## What never happens

- **No network traffic during processing.** No telemetry, no analytics, no error reporting to third parties, in any component.
- **No storage of your documents.** The CLI writes only the output file you request. The web app keeps the file and results in page memory only: nothing is written to `localStorage`, `sessionStorage`, IndexedDB or cookies, and the clear/reset control drops the in-memory references so the browser can reclaim them. Closing the tab has the same effect.
- **No credentials, ever.** StatementBridge does not connect to banks, does not implement Open Banking, and has nowhere to store a password or token. You export the file from your bank yourself.
- **No full account numbers in the output.** Values from a detected account column are reduced to `****` plus the last four digits; the canonical schema formally requires the `****` prefix on `account_masked`. Full account numbers are never emitted in any output field the engine populates.
- **No financial values in logs or errors.** Diagnostic (`tracing`) events carry counts, issue codes and row numbers only. Error and warning messages may quote a raw *date* or *amount* token (capped in length — you need it to fix the file) but never descriptions, references, account numbers or counterparty names.

## Masking and redaction features

Beyond the defaults, two opt-in layers reduce what the *output* reveals:

- **`--redact`** rewrites every run of 8 or more consecutive digits inside descriptions and references to `****` plus the last four digits — useful when narrative text embeds card or account numbers ("PAYMENT CARD 4111111111111111" becomes "PAYMENT CARD ****1111"). Shorter runs (invoice numbers, dates) are left alone.
- **`inspect` sample rows are always masked**: the preview applies the same 8-digit-run masking and caps each cell at 60 characters, so inspection output is safe to paste into an issue — though you should still read it before sharing.

## What you should still be careful with

Local processing does not make the *outputs* harmless. The normalized JSON and CSV contain your transactions — dates, amounts, descriptions, references, balances. Treat them with the same care as the original statement:

- **Sharing an output file is sharing your financial data.** Redaction (`--redact`) reduces embedded numbers but does not anonymize descriptions or amounts.
- **`content_hash` identifies your input.** It is a SHA-256 of the file bytes — it reveals nothing by itself, but two parties holding the same file can match it.
- **Bug reports:** never attach a real statement, even partially. Reproduce the problem with a synthetic file (see `fixtures/` for the style); maintainers will not accept real data, per the contribution policy.
- **Your machine is the perimeter.** StatementBridge inherits the security of wherever you run it: an unencrypted disk, a shared computer or a compromised browser affects the files regardless of what this tool does.
- **The web app is strongest offline-capable.** After the page has loaded, processing needs no connectivity; if you want the extra assurance, load the app, then disconnect before dropping a file — the flow works identically.

## What this tool is not

StatementBridge normalizes data. It does not provide financial, accounting, tax or legal advice, and its validation statuses ("valid", "inconsistent balance") are statements about data coherence, not about the correctness of your finances. Review outputs before using them in accounting systems.

## Verifying these claims

This is open source; do not take the above on faith:

- The engine crates contain no networking dependencies — inspect `Cargo.toml`/`Cargo.lock`.
- The workspace forbids `unsafe` code (`unsafe_code = "deny"`).
- CLI tests assert that sample output masks long digit runs and that CSV exports are injection-guarded.
- The web app can be audited in the browser: the network panel shows no requests during processing.
