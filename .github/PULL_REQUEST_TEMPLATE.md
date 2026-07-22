<!--
Thanks for contributing to StatementBridge!

THE ONE RULE THAT MATTERS MOST: never post or commit real bank statements,
account numbers, or customer data — in code, fixtures, tests, screenshots,
or this description. All sample data must be fully synthetic.
-->

## Summary

<!-- What does this PR change? One or two sentences. -->

## Motivation

<!-- Why is this change needed? Link related issues, e.g. "Fixes #123". -->

## How was this tested?

<!-- Paste the commands you ran and their outcome, e.g.:

scripts/test-all.sh          # all green
cargo test -p statementbridge-core dedupe
UPDATE_GOLDEN=1 cargo test -p statementbridge-cli --test golden   # goldens updated intentionally, diff reviewed
-->

## Checklist

- [ ] **No real financial data anywhere in this PR** — every fixture, test
      input, example, and screenshot is fully synthetic (no real bank
      statements, account numbers, or customer data).
- [ ] Tests pass locally: `scripts/test-all.sh`
- [ ] Lint is clean: `scripts/lint-all.sh`
- [ ] Documentation updated if behavior changed.
- [ ] `CHANGELOG.md` updated under `[Unreleased]` for user-visible changes.
- [ ] All commits are signed off (`git commit -s`, DCO — see
      [CONTRIBUTING.md](CONTRIBUTING.md)).
