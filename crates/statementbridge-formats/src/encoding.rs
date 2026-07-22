//! Text encoding detection and decoding.
//!
//! Order of evidence:
//! 1. BOMs: UTF-8, UTF-16 LE, UTF-16 BE.
//! 2. Strict UTF-8 validation.
//! 3. Bytes in 0x80–0x9F ⇒ Windows-1252 (those are letters there, control
//!    chars in ISO-8859-1).
//! 4. Otherwise high bytes ⇒ ISO-8859-1 (decoded via the Windows-1252 table,
//!    identical for 0xA0–0xFF).
//!
//! Any non-UTF-8 assumption is reported with an `encoding_assumed` warning.

use encoding_rs::{UTF_16BE, UTF_16LE, WINDOWS_1252};
use statementbridge_core::issues::{codes, Issue};
use statementbridge_core::EngineError;

pub struct Decoded {
    pub text: String,
    pub encoding: &'static str,
    pub warning: Option<Issue>,
}

pub fn decode(bytes: &[u8]) -> Result<Decoded, EngineError> {
    if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        let text = String::from_utf8(bytes[3..].to_vec()).map_err(|_| {
            EngineError::Encoding("UTF-8 BOM present but content is not valid UTF-8".into())
        })?;
        return Ok(Decoded {
            text,
            encoding: "utf-8-bom",
            warning: None,
        });
    }
    if bytes.starts_with(&[0xff, 0xfe]) {
        let (text, _, malformed) = UTF_16LE.decode(&bytes[2..]);
        if malformed {
            return Err(EngineError::Encoding("malformed UTF-16LE content".into()));
        }
        return Ok(Decoded {
            text: text.into_owned(),
            encoding: "utf-16le",
            warning: Some(Issue::new(
                codes::ENCODING_ASSUMED,
                "decoded as UTF-16LE (BOM)",
            )),
        });
    }
    if bytes.starts_with(&[0xfe, 0xff]) {
        let (text, _, malformed) = UTF_16BE.decode(&bytes[2..]);
        if malformed {
            return Err(EngineError::Encoding("malformed UTF-16BE content".into()));
        }
        return Ok(Decoded {
            text: text.into_owned(),
            encoding: "utf-16be",
            warning: Some(Issue::new(
                codes::ENCODING_ASSUMED,
                "decoded as UTF-16BE (BOM)",
            )),
        });
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        return Ok(Decoded {
            text: text.to_string(),
            encoding: "utf-8",
            warning: None,
        });
    }

    let has_c1 = bytes.iter().any(|b| (0x80..=0x9f).contains(b));
    let label = if has_c1 { "windows-1252" } else { "iso-8859-1" };
    let (text, _, _) = WINDOWS_1252.decode(bytes);
    Ok(Decoded {
        text: text.into_owned(),
        encoding: label,
        warning: Some(Issue::new(
            codes::ENCODING_ASSUMED,
            format!("file is not UTF-8; decoded as {label}"),
        )),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn utf8_plain_and_bom() {
        let d = decode("Fecha,Descripción".as_bytes()).unwrap();
        assert_eq!(d.encoding, "utf-8");
        let mut with_bom = vec![0xef, 0xbb, 0xbf];
        with_bom.extend_from_slice("Fecha".as_bytes());
        let d = decode(&with_bom).unwrap();
        assert_eq!(d.encoding, "utf-8-bom");
        assert_eq!(d.text, "Fecha");
    }

    #[test]
    fn windows_1252_detected() {
        // "Descripción" in Windows-1252: ó = 0xF3; plus € (0x80, a C1 byte).
        let bytes = b"Caf\xe9 \x80 5,00";
        let d = decode(bytes).unwrap();
        assert_eq!(d.encoding, "windows-1252");
        assert_eq!(d.text, "Café € 5,00");
        assert!(d.warning.is_some());
    }

    #[test]
    fn iso_8859_1_when_no_c1() {
        let bytes = b"Caf\xe9 con az\xfacar";
        let d = decode(bytes).unwrap();
        assert_eq!(d.encoding, "iso-8859-1");
        assert_eq!(d.text, "Café con azúcar");
    }
}
