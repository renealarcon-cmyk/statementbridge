//! Text normalization: Unicode NFC, whitespace cleanup, control-character
//! stripping, matching keys (lowercase + accent folding) and length capping.

use unicode_normalization::UnicodeNormalization;

const ZERO_WIDTH: [char; 4] = ['\u{200b}', '\u{200c}', '\u{200d}', '\u{feff}'];

/// Normalize free text (descriptions, references):
/// NFC, exotic spaces → space, control/zero-width chars removed, whitespace
/// collapsed, trimmed, capped at `max_chars` (char boundary).
/// Returns the text and whether it was truncated.
pub fn normalize_text(raw: &str, max_chars: usize) -> (String, bool) {
    let mut out = String::with_capacity(raw.len().min(max_chars * 4));
    let mut last_space = true; // leading spaces collapse away
    for c in raw.nfc() {
        let c = match c {
            '\u{00a0}' | '\u{202f}' | '\u{2007}' | '\u{2009}' | '\t' => ' ',
            _ => c,
        };
        if ZERO_WIDTH.contains(&c) || (c.is_control() && c != ' ') {
            continue;
        }
        if c.is_whitespace() {
            if !last_space {
                out.push(' ');
                last_space = true;
            }
        } else {
            out.push(c);
            last_space = false;
        }
    }
    while out.ends_with(' ') {
        out.pop();
    }
    let truncated = out.chars().count() > max_chars;
    if truncated {
        out = out.chars().take(max_chars).collect();
        out = out.trim_end().to_string();
    }
    (out, truncated)
}

/// Key used for header-alias matching and fingerprints: NFC → lowercase →
/// accents removed (NFD, combining marks dropped) → whitespace collapsed.
pub fn matching_key(raw: &str) -> String {
    let (clean, _) = normalize_text(raw, 512);
    clean
        .to_lowercase()
        .nfd()
        .filter(|c| !is_combining_mark(*c))
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_combining_mark(c: char) -> bool {
    matches!(c, '\u{0300}'..='\u{036f}' | '\u{1ab0}'..='\u{1aff}' | '\u{20d0}'..='\u{20ff}')
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn collapses_whitespace_and_strips_controls() {
        let (s, t) = normalize_text("  Pago\u{00a0} a\t\tproveedor\u{200b}\r\n ", 100);
        assert_eq!(s, "Pago a proveedor");
        assert!(!t);
    }

    #[test]
    fn truncates_at_char_boundary() {
        let (s, t) = normalize_text("ééééé", 3);
        assert_eq!(s, "ééé");
        assert!(t);
    }

    #[test]
    fn matching_key_folds_accents_and_case() {
        assert_eq!(matching_key("  DÉBITO  (EUR) "), "debito (eur)");
        assert_eq!(matching_key("Descripción"), "descripcion");
    }
}
