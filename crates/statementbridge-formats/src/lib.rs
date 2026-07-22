//! # statementbridge-formats
//!
//! Byte-level readers that turn CSV/XLSX files into a
//! [`statementbridge_core::RawTable`]. Pure functions over byte slices: no
//! filesystem paths, no network, no clocks — callers do the I/O.
//!
//! Responsibilities:
//! * format detection (magic bytes, not file extensions),
//! * encoding detection (UTF-8, UTF-8 BOM, UTF-16 LE/BE BOM, Windows-1252,
//!   ISO-8859-1),
//! * CSV delimiter detection (`,` `;` `\t` `|`),
//! * XLSX sheet selection (densest sheet wins; others reported),
//! * enforcing size/row/column/field limits with loud warnings.

mod csv_read;
mod encoding;
mod xlsx_read;

use statementbridge_core::fingerprint::sha256_hex;
use statementbridge_core::{EngineError, Limits, RawTable};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileFormat {
    Csv,
    Xlsx,
}

impl FileFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            FileFormat::Csv => "csv",
            FileFormat::Xlsx => "xlsx",
        }
    }
}

/// Options for reading raw bytes.
#[derive(Debug, Clone, Default)]
pub struct ReadOptions {
    /// Force a CSV delimiter instead of detecting one.
    pub delimiter: Option<char>,
    /// XLSX sheet to read (by name). Default: the densest non-empty sheet.
    pub sheet: Option<String>,
    pub limits: Limits,
}

/// Detect the container format from content, never from the file name.
pub fn detect_format(bytes: &[u8]) -> Result<FileFormat, EngineError> {
    if bytes.is_empty() {
        return Err(EngineError::EmptyFile);
    }
    if bytes.starts_with(b"PK\x03\x04") {
        // ZIP container: XLSX must contain [Content_Types].xml.
        let head = &bytes[..bytes.len().min(64 * 1024)];
        if find_subslice(head, b"[Content_Types].xml").is_some()
            || find_subslice(bytes, b"[Content_Types].xml").is_some()
        {
            return Ok(FileFormat::Xlsx);
        }
        return Err(EngineError::UnsupportedFormat(
            "ZIP archive that is not an XLSX workbook".to_string(),
        ));
    }
    if bytes.starts_with(&[0xd0, 0xcf, 0x11, 0xe0]) {
        return Err(EngineError::UnsupportedFormat(
            "legacy .xls (OLE) is not supported; re-save the file as .xlsx or CSV".to_string(),
        ));
    }
    // Reject clearly binary data (NUL density) unless it's a UTF-16 BOM file.
    let has_utf16_bom = bytes.starts_with(&[0xff, 0xfe]) || bytes.starts_with(&[0xfe, 0xff]);
    if !has_utf16_bom {
        let probe = &bytes[..bytes.len().min(4096)];
        let nuls = probe.iter().filter(|b| **b == 0).count();
        if nuls * 10 > probe.len() {
            return Err(EngineError::UnsupportedFormat(
                "binary data; expected a CSV or XLSX statement export".to_string(),
            ));
        }
    }
    Ok(FileFormat::Csv)
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Read bytes into a `RawTable`, detecting format, encoding and delimiter.
pub fn read_bytes(
    bytes: &[u8],
    file_name: &str,
    opts: &ReadOptions,
) -> Result<RawTable, EngineError> {
    if bytes.len() as u64 > opts.limits.max_file_bytes {
        return Err(EngineError::FileTooLarge {
            size: bytes.len() as u64,
            limit: opts.limits.max_file_bytes,
        });
    }
    let format = detect_format(bytes)?;
    let mut table = match format {
        FileFormat::Csv => csv_read::read(bytes, opts)?,
        FileFormat::Xlsx => xlsx_read::read(bytes, opts)?,
    };
    table.meta.file_name = sanitize_file_name(file_name);
    table.meta.format = format.as_str().to_string();
    table.meta.content_hash = sha256_hex(bytes);
    table.meta.byte_len = bytes.len() as u64;
    if table
        .rows
        .iter()
        .all(|r| r.iter().all(|c| c.trim().is_empty()))
    {
        return Err(EngineError::EmptyFile);
    }
    tracing::debug!(
        format = table.meta.format,
        rows = table.rows.len(),
        truncated = table.meta.truncated,
        "file read"
    );
    Ok(table)
}

/// Keep only the final path component and drop control characters, so a
/// hostile file name cannot smuggle paths or escape sequences into reports.
pub fn sanitize_file_name(name: &str) -> String {
    let base = name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(name)
        .chars()
        .filter(|c| !c.is_control())
        .take(255)
        .collect::<String>();
    if base.is_empty() {
        "statement".to_string()
    } else {
        base
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn detects_formats() {
        assert!(matches!(
            detect_format(b"a,b,c\n1,2,3\n"),
            Ok(FileFormat::Csv)
        ));
        assert!(matches!(detect_format(b""), Err(EngineError::EmptyFile)));
        assert!(matches!(
            detect_format(b"PK\x03\x04junk"),
            Err(EngineError::UnsupportedFormat(_))
        ));
        assert!(matches!(
            detect_format(&[0xd0, 0xcf, 0x11, 0xe0, 0x00]),
            Err(EngineError::UnsupportedFormat(_))
        ));
    }

    #[test]
    fn sanitizes_file_names() {
        assert_eq!(sanitize_file_name("../../etc/passwd"), "passwd");
        assert_eq!(sanitize_file_name("C:\\Users\\x\\st.csv"), "st.csv");
        assert_eq!(sanitize_file_name("nor\u{7}mal.csv"), "normal.csv");
        assert_eq!(sanitize_file_name(""), "statement");
    }
}
