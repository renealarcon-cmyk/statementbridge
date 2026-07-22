# Contributing to StatementBridge

Thank you for your interest in improving StatementBridge. This is a young,
alpha-stage project — contributions of every size are welcome: adapters for
new bank export formats, bug reports, documentation, translations, tests,
and code.

Please read this document once before opening your first pull request. It is
short, and one rule in it is non-negotiable.

## The one rule that matters most: no real financial data

**Never post or commit real bank statements, account numbers, or customer
data — anywhere in this project.** Not in fixtures, tests, issues, pull
requests, commit messages, screenshots, benchmark files, or logs.

- All fixtures and test data must be **fully synthetic**: invented business
  names, invented account numbers, invented amounts.
- Bug reports must reproduce with a **synthetic** file. If a bug only shows
  up with your real statement, rebuild a minimal synthetic file with the same
  *structure* (same delimiters, date format, column layout) and invented
  values until it reproduces.
- Adapter requests need the **header row only** — never data rows.
- If you accidentally push real data, contact the maintainers immediately and
  privately (GitHub **Security** tab → *Report a vulnerability* reaches them
  confidentially). The history will need to be rewritten; a follow-up commit
  that deletes the file is not enough.

## Development environment

You need:

| Tool | Version | Used for |
| --- | --- | --- |
| Rust (stable) | ≥ 1.79 (workspace MSRV) with `rustfmt` + `clippy` | core, formats, adapters, CLI, WASM |
| Node.js | ≥ 20 | web app (`apps/web`) |
| pnpm | ≥ 9 | web app package manager |
| Python 3 | any recent | regenerating synthetic fixtures |
| wasm-pack | optional | building the WASM package (`scripts/build-wasm.sh` prefers it; the script documents a `cargo` + `wasm-bindgen` fallback) |

## Quickstart

```sh
git clone https://github.com/statementbridge/statementbridge
cd statementbridge

# Build and test the Rust workspace
cargo build --workspace
cargo test --workspace

# Try the CLI against a synthetic fixture
cargo run -p statementbridge-cli -- inspect fixtures/csv/comma_dot_signed_iso.csv
cargo run -p statementbridge-cli -- convert fixtures/csv/semicolon_comma_es.csv --output /tmp/out.json
```

Web app (optional; requires Node, pnpm and the WASM package):

```sh
bash scripts/build-wasm.sh          # builds the WASM package into apps/web/src/lib/wasm/pkg/
pnpm --dir apps/web install
pnpm --dir apps/web dev             # local Vite dev server
```

## One-command checks

Run these before pushing — CI runs the same things:

```sh
scripts/test-all.sh     # cargo test --workspace + web tests (vitest)
scripts/lint-all.sh     # rustfmt, clippy -D warnings, ESLint, Prettier, tsc
```

Both scripts skip the web parts automatically if `apps/web/node_modules` does
not exist, so they work in a Rust-only checkout too.

The individual commands, if you prefer them:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Fixtures and golden files

- Fixtures are **generated, deterministic and synthetic**. To regenerate
  `fixtures/csv/` and `fixtures/xlsx/`:

  ```sh
  python3 scripts/generate_fixtures.py
  ```

  Large benchmark inputs (gitignored) are produced with
  `python3 scripts/generate_fixtures.py --large 100000`.

- Golden tests compare normalized output byte-for-byte against
  `fixtures/golden/*.json`. If your change intentionally alters normalized
  output, regenerate the goldens and **review the diff carefully** before
  committing:

  ```sh
  UPDATE_GOLDEN=1 cargo test -p statementbridge-cli --test golden
  ```

- Benchmarks are reproducible via `scripts/benchmark.sh`. Never paste
  benchmark numbers you did not actually measure.

## Contributing an adapter

Adapters are declarative JSON files — no Rust required. The walkthrough:

1. **Scaffold** a template (creates `adapters/<name>/adapter.json` plus a
   `README.md` explaining every field):

   ```sh
   cargo run -p statementbridge-cli -- init-adapter mybank-checking
   # or, with an installed binary: statementbridge init-adapter mybank-checking
   ```

2. **Fill in** the template: header aliases, rows to skip, footer patterns,
   date formats, decimal/thousands separators, amount mode (separate
   debit/credit columns vs. a single signed column), transaction order,
   currency. See `docs/adapter-authoring.md` for what each field means.

3. **Validate** it:

   ```sh
   cargo run -p statementbridge-cli -- adapters validate adapters/mybank-checking/adapter.json
   ```

4. **Create a synthetic fixture** that mimics the export's *layout* with
   invented data (either a hand-written file or an addition to
   `scripts/generate_fixtures.py`), and add a test that converts it with your
   adapter. Do not copy any real file, real name, or real number.

5. Run `scripts/test-all.sh` and `scripts/lint-all.sh`, then **open a pull
   request**. Mention which institution/export the adapter targets and
   confirm in the PR checklist that every byte of test data is synthetic.

If you cannot write the adapter yourself, open an
[adapter request issue](.github/ISSUE_TEMPLATE/adapter_request.yml) with the
header row and format details.

## Commit conventions

- [Conventional Commits](https://www.conventionalcommits.org/) are
  **recommended, not enforced**: `feat: …`, `fix: …`, `docs: …`, `test: …`,
  `chore: …`. If you don't use them, write a clear, imperative subject line.
- Keep commits focused; avoid mixing refactors with behavior changes.

## Developer Certificate of Origin (DCO)

This project uses the
[Developer Certificate of Origin, version 1.1](https://developercertificate.org/)
instead of a CLA. By signing off a commit you certify that you have the right
to submit the change under the project's Apache-2.0 license, as described in
that document.

Every commit must carry a `Signed-off-by` line matching the commit author:

```sh
git commit -s -m "fix: handle trailing semicolon in ES exports"
```

Forgot to sign off? Amend the last commit with `git commit --amend -s --no-edit`,
or fix a branch with `git rebase --signoff main`.

## Pull request process

1. Fork, create a topic branch, make your change.
2. Update documentation and add tests for user-visible behavior.
3. Add a line under `[Unreleased]` in `CHANGELOG.md` for user-visible changes.
4. Ensure `scripts/test-all.sh` and `scripts/lint-all.sh` pass locally.
5. Open the PR and fill in the template. CI must be green.
6. **One maintainer approval** is required to merge. Review follows lazy
   consensus (see `GOVERNANCE.md`); breaking changes to the canonical schema
   or the CLI contract need broader agreement.

Review is best-effort volunteer work — we aim to respond promptly but make no
response-time promises. A friendly ping after a week or two is welcome.

## Issue labels

The project uses these labels to organize work:

| Label | Meaning |
| --- | --- |
| `good first issue` | scoped, mentored, suitable as a first contribution |
| `adapter` | new or improved bank/processor format support |
| `documentation` | docs, guides, translations |
| `bug` | confirmed defects |
| `security` | hardening work that is safe to discuss publicly (vulnerabilities go through `SECURITY.md`, never public issues) |
| `help wanted` | maintainers would appreciate outside help |

## Releases

- Versioning follows [SemVer](https://semver.org/). While the project is
  `0.x` (alpha), minor versions may contain breaking changes; they are always
  documented in `CHANGELOG.md`.
- `CHANGELOG.md` follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
- Pushing a `v*` tag triggers the release workflow, which builds CLI binaries
  for Linux/macOS/Windows and creates a **draft** GitHub release. A
  maintainer reviews and publishes the draft manually — nothing is
  auto-published, and CI does not publish to crates.io or npm in 0.1.x.

## Conduct and security

- Be kind. The [Code of Conduct](CODE_OF_CONDUCT.md) applies in all project
  spaces.
- Report vulnerabilities privately as described in [SECURITY.md](SECURITY.md)
  — never in public issues.
