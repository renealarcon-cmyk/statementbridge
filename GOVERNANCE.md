# Governance

StatementBridge is a young project. This document describes how it is
actually run today — simply and honestly — and how that will evolve as the
community grows.

## Current model

The project is maintained by a small group of maintainers: the people with
commit access to `statementbridge/statementbridge`.

Decisions are made by **lazy consensus**:

- Changes are proposed in the open, as issues or pull requests.
- If no maintainer objects within a reasonable window (about a week for
  significant changes; less for trivial ones), the proposal proceeds.
- Objections are resolved by discussion in the issue or PR. If discussion
  stalls, the maintainers decide together; when they cannot agree, the
  **status quo wins** until agreement is reached.

Day-to-day decisions (bug fixes, docs, new adapters, refactors) need one
maintainer approval on the PR plus green CI — see
[CONTRIBUTING.md](CONTRIBUTING.md).

## Changes that require explicit maintainer consensus

Some changes are too consequential for lazy consensus. These require the
active agreement of the maintainers (not merely an absence of objections):

- **Breaking changes to the canonical schema**
  (`schemas/statementbridge-v1.schema.json`) or to the documented CLI
  contract (commands, flags, exit codes, the `--json` envelope).
- **Changes to the security policy** ([SECURITY.md](SECURITY.md)) or to the
  threat model's guarantees.
- **Changes to this governance document.**
- **License changes** (the project is Apache-2.0).
- **Publishing a release** (drafts are prepared by CI; a maintainer reviews
  and publishes manually).

## Becoming a maintainer

There is no application form. Maintainers are added by invitation, extended
by the existing maintainers (by lazy consensus among them) to contributors
who have shown, over a sustained period:

- consistently high-quality contributions — code, adapters, documentation,
  reviews, or issue triage;
- sound judgment on scope, compatibility, and the project's privacy
  principles;
- constructive collaboration in line with the
  [Code of Conduct](CODE_OF_CONDUCT.md).

Maintainers who become inactive for an extended period may, after a friendly
check-in, be moved to emeritus status. Emeritus maintainers are welcome back
at any time.

## Evolution

This model fits a small project. As the community grows, we expect to evolve
it — for example, defined roles (adapter reviewers, docs maintainers), a
clearer decision process for contested changes, or formal voting. Changes are
proposed as pull requests to this file and follow the consensus rule above.
