//! # statementbridge-core
//!
//! Core engine for [StatementBridge](https://github.com/statementbridge/statementbridge):
//! turns messy bank/processor exports (already parsed into a [`raw::RawTable`])
//! into the canonical, validated statement model defined by
//! `schemas/statementbridge-v1.schema.json`.
//!
//! Design guarantees:
//!
//! * **Local-first**: this crate never performs I/O or network access. Callers
//!   hand it bytes/rows; it hands back data.
//! * **Exact money**: all monetary values are [`rust_decimal::Decimal`] and are
//!   serialized as decimal *strings* in JSON. Floats are never used for money.
//! * **Deterministic**: identical input + identical options ⇒ byte-identical
//!   output. Nothing here reads clocks or RNGs (`imported_at` is caller input).
//! * **Auditable**: every assumption or correction produces a warning with a
//!   stable machine-readable code (see [`issues::codes`]).
//! * **Safe logging**: `tracing` events emitted by this crate contain counts,
//!   codes and row numbers — never descriptions, references, amounts or
//!   account numbers.

pub mod config;
pub mod dates;
pub mod dedupe;
pub mod error;
pub mod export;
pub mod fingerprint;
pub mod header;
pub mod inspect;
pub mod issues;
pub mod limits;
pub mod mask;
pub mod model;
pub mod normalize;
pub mod numbers;
pub mod raw;
pub mod text;

pub use config::{AmountMode, ColumnRef, Field, Locale, NormalizeOptions, OrderHint};
pub use error::EngineError;
pub use inspect::{inspect, InspectReport};
pub use issues::Issue;
pub use limits::Limits;
pub use model::{Statement, Transaction, SCHEMA_VERSION};
pub use normalize::normalize;
pub use raw::RawTable;

/// Crate version, re-exported for CLIs and bindings.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
