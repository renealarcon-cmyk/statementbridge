# Support

StatementBridge is alpha software maintained by volunteers. Support is
provided on a best-effort basis — there are no SLAs and no guaranteed
response times.

## Before asking

- Read the [README](README.md) quickstart and the guides in [`docs/`](docs/).
- Run `statementbridge --help` (every subcommand has its own `--help`).
- Run `statementbridge inspect <file>` — it shows what the engine detected
  (encoding, delimiter, header row, column mapping) and often explains why a
  conversion did not do what you expected.

## Where to ask

| Need | Where |
| --- | --- |
| Usage question or idea | GitHub **Discussions**, if enabled on the repository; otherwise open a GitHub issue |
| Bug | GitHub issue using the **Bug report** template |
| Support for a new bank/processor export | GitHub issue using the **Adapter request** template |
| Feature suggestion | GitHub issue using the **Feature request** template |
| Security vulnerability | **Never a public issue** — see [SECURITY.md](SECURITY.md) for private reporting |

## What to include

The more of this you provide, the faster someone can help:

- Version: output of `statementbridge --version` (or the commit hash).
- Operating system and how you run it (CLI, library, web app).
- The exact command you ran, including flags.
- What you expected, and what actually happened (exit code, error output).
- A **synthetic** sample file that reproduces the behavior.

## The data rule

**Never post real bank statements, account numbers, or customer data** — not
in issues, discussions, screenshots, or logs. Recreate the *structure* of
your file (same delimiter, date format, column layout) with invented names
and amounts until the problem reproduces, and share that synthetic file
instead. Maintainers will close reports containing real financial data and
ask for a scrubbed reproduction.

## What support is not

- StatementBridge is a data normalization tool, **not** financial,
  accounting, or tax advice. Verifying that normalized output is fit for
  your accounting purposes remains your responsibility.
- Volunteers answer when they can. If your business depends on an answer by a
  deadline, plan for that — the project makes no response-time promises.
