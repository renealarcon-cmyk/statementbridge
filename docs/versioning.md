# Versioning and compatibility policy

StatementBridge follows [Semantic Versioning 2.0.0](https://semver.org). This page defines what exactly is covered by that promise: which surfaces are public API, what counts as a breaking change on each, how the canonical schema version relates to crate versions, how deprecations work, and the MSRV policy.

Current state: **0.1.x, alpha.** Per SemVer (and Cargo's interpretation of it), during 0.x the *minor* version is the breaking-change indicator: 0.1 → 0.2 may break, 0.1.0 → 0.1.1 may not. The rules below are written for the 1.0 era and are already applied in spirit — the difference is only where the breaking bump lands while the leading zero remains.

## Public API surfaces

These are the contracts users may rely on. Anything not listed (internal module layout, private functions, exact human-readable message wording, log output) is not covered.

### 1. The canonical statement schema

The versioned document format defined by `schemas/statementbridge-v1.schema.json` and mirrored in [canonical-schema.md](canonical-schema.md).

- **Covered:** field names, types, required/nullable status, enum values (`valid`/`valid_with_warnings`/`invalid`, duplicate kinds, balance statuses), the decimal-string and date formats, the `sb1-` id shape, the fingerprint inputs, and the **issue-code catalog**.
- **Non-breaking (minor):** adding an optional field; adding a new issue code; adding a new enum value *where the schema and docs state the set may grow* (issue codes are the primary example).
- **Breaking (major):** removing or renaming a field or code; changing a type, sign convention, hashing input, or the meaning of an existing code; making a previously optional field required; changing an existing enum value.
- Message strings inside issues are explicitly *not* covered — key automation on `code`, never on `message`.

### 2. The CLI

- **Covered:** command names (`inspect`, `convert`, `validate`, `adapters list`, `adapters validate`, `init-adapter`), documented flags and their meanings, the exit-code table (0/1/2/3/4), the `--json` envelope shape (`{"ok": …, "data" | "error": {code, message}}`), and the machine-readable error codes.
- **Non-breaking:** adding commands or flags; adding fields to `--json` output objects; improving human-readable text (human output layout is informative, not contractual — scripts should use `--json`).
- **Breaking:** removing or renaming a command or flag; changing a flag's default in a way that changes outputs; changing an exit code's meaning; removing fields from `--json` output.

### 3. The Rust crates

The public items of `statementbridge-core`, `statementbridge-formats` and `statementbridge-adapters` (and the `statementbridge-wasm` exported functions), per the standard Rust API evolution rules.

- **Non-breaking:** adding items; adding fields to structs already marked non-exhaustive; new default-implemented behavior.
- **Breaking:** removing/renaming public items, changing signatures or public field sets, changing behavior that documented guarantees depend on — determinism, the no-I/O rule, and the money rules are documented guarantees, so violating them is breaking even if signatures compile.
- Until crates are published (planned v0.2), path/git users should pin a commit; the same rules are applied to the tree.

### 4. The adapter format

`schemas/statementbridge-adapter-v1.schema.json` plus the semantic validation rules.

- **Non-breaking:** adding optional fields with defaults; widening an accepted range; recognizing new alias-table entries.
- **Breaking:** removing or renaming a field; narrowing accepted values so that previously valid adapters fail validation; changing the meaning of an existing field.

## `schema_version` vs crate version

They are deliberately independent:

- **`schema_version`** (`"1.0"` today) identifies the *document format*. It changes only when the document contract changes: a compatible extension bumps it to `1.1`; an incompatible change bumps it to `2.0` **and** the schema ships as a new file (`statementbridge-v2.schema.json`) so v1 consumers keep a stable target.
- **Crate/CLI versions** (`0.1.0` today) track the software. Many software releases can emit the same schema version; that is the normal case.
- Every document states its own `schema_version`, so consumers can dispatch on it. A software release that changes the emitted `schema_version` says so prominently in the changelog.

## Deprecation policy

Nothing covered above disappears without a deprecation window:

1. The deprecation is announced in the changelog and documented at the old item (rustdoc `#[deprecated]`, CLI help text, schema `description`), naming the replacement.
2. The deprecated form keeps working for **at least one full minor release cycle** after the announcement.
3. Removal happens at the next breaking-change release (major, or 0.x minor while in 0.x), never in a patch.

Exception: a security fix may shorten the window; if so, the changelog states it explicitly and the release notes explain the forced timing.

## MSRV policy

- The minimum supported Rust version is declared in `Cargo.toml` (`rust-version`, currently **1.79**) and enforced by the toolchain when building.
- **Raising the MSRV is a minor change**, not a patch: it never happens in a patch release, and the changelog states the new floor and the reason.
- The MSRV is raised conservatively (when a dependency or a genuinely useful language feature requires it), not to chase the latest stable.

## Changelog and releases

- `CHANGELOG.md` follows the Keep a Changelog convention: every release lists Added / Changed / Deprecated / Removed / Fixed / Security, and breaking changes are called out explicitly at the top of the entry.
- Releases are cut from tags (`v0.1.0`). Nothing is published to registries automatically; publication to crates.io/npm is a deliberate step planned for v0.2 (see [roadmap.md](roadmap.md)).

## What alpha status means in practice

Until 1.0: the canonical schema and issue codes are treated as the most stable surface (changing them requires the strongest justification), the CLI contract next, and the Rust crate APIs are the most likely to move between 0.x minors. If you build on the crates today, pin versions and read changelogs; if you build on the JSON output and exit codes, upgrades should be uneventful — and if they are not, that is a bug in this policy's application, so please file an issue.
