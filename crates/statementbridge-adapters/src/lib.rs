//! # statementbridge-adapters
//!
//! Declarative adapters: JSON documents describing one bank/processor export
//! format (columns, aliases, date/number formats, footer patterns…).
//! Adapters contain configuration only — never code, never real bank data.
//!
//! The three built-in generics ship embedded in the binary:
//! `generic-csv` (full auto-detection), `generic-debit-credit` and
//! `generic-signed-amount`.

mod detect;
mod spec;
mod validate;

pub use detect::detect_adapter;
pub use spec::{AdapterSpec, MaskingRules};
pub use validate::validate_spec;

use statementbridge_core::EngineError;

/// Built-in adapters, embedded at compile time from `adapters/`.
pub fn builtin_adapters() -> Vec<AdapterSpec> {
    let sources = [
        include_str!("../../../adapters/generic-csv/adapter.json"),
        include_str!("../../../adapters/generic-debit-credit/adapter.json"),
        include_str!("../../../adapters/generic-signed-amount/adapter.json"),
    ];
    sources
        .iter()
        .map(|s| serde_json::from_str::<AdapterSpec>(s).expect("built-in adapter must parse"))
        .collect()
}

/// Parse one adapter from JSON, with structural validation.
pub fn parse_adapter(json: &str) -> Result<AdapterSpec, EngineError> {
    let spec: AdapterSpec =
        serde_json::from_str(json).map_err(|e| EngineError::InvalidAdapter {
            id: "<unparsed>".to_string(),
            reason: e.to_string(),
        })?;
    let problems = validate_spec(&spec);
    if let Some(first) = problems.first() {
        return Err(EngineError::InvalidAdapter {
            id: spec.id.clone(),
            reason: format!("{} ({} problem(s) total)", first, problems.len()),
        });
    }
    Ok(spec)
}

/// Find an adapter by id among built-ins plus any extras (extras win on id
/// collision so users can override a built-in).
pub fn find_adapter(id: &str, extras: &[AdapterSpec]) -> Result<AdapterSpec, EngineError> {
    extras
        .iter()
        .chain(builtin_adapters().iter())
        .find(|a| a.id == id)
        .cloned()
        .ok_or_else(|| EngineError::AdapterNotFound(id.to_string()))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn builtins_parse_and_validate() {
        let all = builtin_adapters();
        assert_eq!(all.len(), 3);
        for a in &all {
            assert!(
                validate_spec(a).is_empty(),
                "adapter {} invalid: {:?}",
                a.id,
                validate_spec(a)
            );
        }
        assert!(all.iter().any(|a| a.id == "generic-csv"));
    }

    #[test]
    fn find_prefers_extras() {
        let mut custom = builtin_adapters()[0].clone();
        custom.display_name = "override".to_string();
        let found = find_adapter("generic-csv", &[custom.clone()]).unwrap();
        assert_eq!(found.display_name, "override");
        assert!(find_adapter("nope", &[]).is_err());
    }
}
