# Roadmap

Where StatementBridge is going, and why each step comes when it does. Milestones are scoped by capability, not by date — this is a young open-source project and dates would be fiction.

Status legend: v0.1 is the current release line; everything after it is planned, not promised.

## v0.1 — the trustworthy core (current)

CSV and XLSX import, the canonical schema (v1.0), declarative adapters with auto-detection, the CLI, the local web app, validation (balance chain, per-row issues), duplicate detection, JSON/CSV export.

**Rationale:** everything else the project might become depends on one thing being true first — that the normalization core is exact (decimal money), deterministic, honest about assumptions, and safe against hostile files. v0.1 exists to make that core real and testable before any surface area grows around it.

## v0.2 — the ecosystem seed

- Better format detection (more evidence sources, better confidence reporting).
- A stable plugin API, so integrations can build on documented interfaces rather than internal crates.
- More community adapters, with the contribution flow smoothed by v0.1 feedback.
- n8n integration, bringing normalization into no-code automation pipelines.
- Published packages: crates.io for the Rust crates, npm for the WASM engine.

**Rationale:** v0.1 proves the tool; v0.2 makes it *adoptable*. Publishing packages and a plugin API is deliberately second — publishing first would freeze interfaces before real usage has criticized them. The declarative adapter format is expected to absorb most contributions; the plugin API exists for what data alone cannot express.

## v0.3 — more sources

- PDF statements with a text layer (a new reader in `statementbridge-formats` producing the same `RawTable`; the architecture reserved this slot from day one).
- Optional, strictly local OCR for image-only PDFs — optional because OCR dependencies are heavy and many users never need them; local because the privacy guarantee is non-negotiable.
- Payment-processor export support (processor exports are structurally different enough from bank statements to deserve dedicated adapters and fixtures).
- Reconciliation rules: configurable matching between normalized statements and external records.

**Rationale:** input breadth expands only after the pipeline behind it is stable, so each new source inherits validation, duplicates, provenance and export for free.

## v1.0 — the commitment

- Stable, documented API across library, CLI, WASM and adapter format.
- A documented compatibility policy (what upgrades may break, and when).
- An adapter conformance suite: automated verification that a contributed adapter behaves correctly against its fixtures.
- An integrations ecosystem (documented patterns for ERPs, accounting tools, automation platforms).
- A support policy stating what maintainers commit to.

**Rationale:** 1.0 is a promise, not a feature set. It ships when the interfaces have survived enough real-world use that promising stability is honest. See [versioning.md](versioning.md) for what stability will formally mean.

## Explicitly not planned

To keep expectations clear, these remain out of scope regardless of version: direct bank connections and credential storage, cloud processing of user documents, telemetry, AI-based accounting classification as a required component, and tax or financial advice features. Items may move from "not planned" only through an RFC-style discussion in the open.

## How to influence this roadmap

The roadmap follows demonstrated need, in the open:

- **Open an issue** describing the workflow you cannot accomplish — real use cases move items up faster than feature votes.
- **Contribute an adapter** ([docs/adapter-authoring.md](adapter-authoring.md)): the fastest way to expand coverage today, and adapter friction reports directly shape the v0.2 detection work.
- **Comment on milestone issues**: scope discussions happen on the tracker, and disagreement before implementation is the cheap kind.

Nothing here is a delivery commitment; it is a statement of intended direction that changes only in public.
