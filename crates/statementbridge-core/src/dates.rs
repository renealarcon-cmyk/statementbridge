//! Date parsing and format inference.
//!
//! Supported out of the box: ISO dates, `DD/MM/YYYY`, `MM/DD/YYYY`, dashes and
//! dots, 2-digit years, `01 Feb 2026`-style month names (English and Spanish),
//! and datetime cells (time part is dropped for `date`).
//!
//! DD/MM vs MM/DD ambiguity is resolved with evidence from the whole column
//! (any day > 12 decides). When a file is *genuinely* ambiguous the engine
//! picks day-first (or the locale convention) and emits
//! `ambiguous_date_format` — configure `--date-format` to silence it.

use chrono::NaiveDate;

/// Candidate formats, tried in listed order during inference.
pub const CANDIDATES: [&str; 16] = [
    "%Y-%m-%d",
    "%Y/%m/%d",
    "%d/%m/%Y",
    "%m/%d/%Y",
    "%d-%m-%Y",
    "%m-%d-%Y",
    "%d.%m.%Y",
    "%Y.%m.%d",
    "%d/%m/%y",
    "%m/%d/%y",
    "%Y%m%d",
    "%d %b %Y",
    "%d-%b-%Y",
    "%b %d, %Y",
    "%B %d, %Y",
    "%d %B %Y",
];

const ES_MONTHS: [(&str, &str); 24] = [
    ("enero", "January"),
    ("febrero", "February"),
    ("marzo", "March"),
    ("abril", "April"),
    ("mayo", "May"),
    ("junio", "June"),
    ("julio", "July"),
    ("agosto", "August"),
    ("septiembre", "September"),
    ("octubre", "October"),
    ("noviembre", "November"),
    ("diciembre", "December"),
    ("ene", "Jan"),
    ("feb", "Feb"),
    ("mar", "Mar"),
    ("abr", "Apr"),
    ("may", "May"),
    ("jun", "Jun"),
    ("jul", "Jul"),
    ("ago", "Aug"),
    ("sep", "Sep"),
    ("set", "Sep"),
    ("oct", "Oct"),
    ("dic", "Dec"),
];

/// Replace Spanish month names/abbreviations with English so chrono's `%b/%B`
/// can parse them. Whole-word, case-insensitive, "de" connectors dropped
/// ("3 de enero de 2026" → "3 January 2026").
fn fold_spanish_months(s: &str) -> String {
    let lower = s.to_lowercase();
    let mut words: Vec<String> = Vec::new();
    for w in lower.split_whitespace() {
        if w == "de" || w == "del" {
            continue;
        }
        let mapped = ES_MONTHS
            .iter()
            .find(|(es, _)| *es == w.trim_end_matches(['.', ',']))
            .map(|(_, en)| (*en).to_string());
        words.push(mapped.unwrap_or_else(|| w.to_string()));
    }
    words.join(" ")
}

/// Try to parse one cell with the given formats (in order). Handles datetime
/// suffixes by also trying the first whitespace/'T'-separated token.
pub fn parse_date(raw: &str, formats: &[String]) -> Option<NaiveDate> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }
    // RFC3339 / ISO datetime fast path.
    if s.len() >= 19 {
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
            return Some(dt.date_naive());
        }
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
            return Some(dt.date());
        }
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
            return Some(dt.date());
        }
    }
    let candidates: [&str; 2] = [s, s.split(['T', ' ']).next().unwrap_or(s)];
    for cand in candidates {
        let folded;
        let cand = if cand.bytes().any(|b| b.is_ascii_alphabetic()) {
            folded = fold_spanish_months(cand);
            folded.as_str()
        } else {
            cand
        };
        for fmt in formats {
            if let Ok(d) = NaiveDate::parse_from_str(cand, fmt) {
                return Some(d);
            }
        }
    }
    None
}

/// Outcome of format inference over a column of samples.
pub struct DateInference {
    /// Formats to use, best first.
    pub formats: Vec<String>,
    /// True when day-first vs month-first could not be decided from data.
    pub ambiguous: bool,
}

pub fn infer_date_formats(
    samples: &[&str],
    locale: Option<crate::config::Locale>,
) -> DateInference {
    let non_empty: Vec<&str> = samples
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if non_empty.is_empty() {
        return DateInference {
            formats: vec!["%Y-%m-%d".into()],
            ambiguous: false,
        };
    }
    let mut best: Option<(usize, &str)> = None; // (hits, fmt)
    let mut full: Vec<&str> = Vec::new();
    for fmt in CANDIDATES {
        let fmts = [fmt.to_string()];
        let hits = non_empty
            .iter()
            .filter(|s| parse_date(s, &fmts).is_some())
            .count();
        if hits == non_empty.len() {
            full.push(fmt);
        }
        if best.map(|(h, _)| hits > h).unwrap_or(hits > 0) {
            best = Some((hits, fmt));
        }
    }

    let day_first = ["%d/%m/%Y", "%d-%m-%Y", "%d.%m.%Y", "%d/%m/%y"];
    let month_first = ["%m/%d/%Y", "%m-%d-%Y", "%m/%d/%y"];
    let full_day = full.iter().any(|f| day_first.contains(f));
    let full_month = full.iter().any(|f| month_first.contains(f));

    if full_day && full_month {
        // Both parse every row ⇒ all component values ≤ 12: genuinely ambiguous.
        let pick = match locale {
            Some(crate::config::Locale::En) => month_first[0],
            _ => day_first[0], // day-first covers es + most of the world
        };
        // Preserve the concrete separator variant that actually matched.
        let pick = full
            .iter()
            .find(|f| {
                (matches!(locale, Some(crate::config::Locale::En)) && month_first.contains(f))
                    || (!matches!(locale, Some(crate::config::Locale::En)) && day_first.contains(f))
            })
            .copied()
            .unwrap_or(pick);
        return DateInference {
            formats: vec![pick.to_string()],
            ambiguous: true,
        };
    }

    if let Some(first_full) = full.first() {
        return DateInference {
            formats: vec![(*first_full).to_string()],
            ambiguous: false,
        };
    }

    // No single format covers everything: use the best one; rows that fail
    // will surface as per-row `invalid_date` errors.
    let fmt = best.map(|(_, f)| f).unwrap_or("%Y-%m-%d");
    DateInference {
        formats: vec![fmt.to_string()],
        ambiguous: false,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn p(s: &str, fmt: &str) -> Option<NaiveDate> {
        parse_date(s, &[fmt.to_string()])
    }

    #[test]
    fn parses_common_formats() {
        assert_eq!(
            p("2026-01-15", "%Y-%m-%d"),
            NaiveDate::from_ymd_opt(2026, 1, 15)
        );
        assert_eq!(
            p("15/01/2026", "%d/%m/%Y"),
            NaiveDate::from_ymd_opt(2026, 1, 15)
        );
        assert_eq!(
            p("01/15/2026", "%m/%d/%Y"),
            NaiveDate::from_ymd_opt(2026, 1, 15)
        );
        assert_eq!(
            p("15.01.2026", "%d.%m.%Y"),
            NaiveDate::from_ymd_opt(2026, 1, 15)
        );
        assert_eq!(
            p("20260115", "%Y%m%d"),
            NaiveDate::from_ymd_opt(2026, 1, 15)
        );
    }

    #[test]
    fn parses_spanish_month_names() {
        assert_eq!(
            p("15 ene 2026", "%d %b %Y"),
            NaiveDate::from_ymd_opt(2026, 1, 15)
        );
        assert_eq!(
            p("3 de enero de 2026", "%d %B %Y"),
            NaiveDate::from_ymd_opt(2026, 1, 3)
        );
    }

    #[test]
    fn drops_time_component() {
        assert_eq!(
            p("2026-01-15 13:45:00", "%Y-%m-%d"),
            NaiveDate::from_ymd_opt(2026, 1, 15)
        );
        assert_eq!(
            p("2026-01-15T13:45:00Z", "%Y-%m-%d"),
            NaiveDate::from_ymd_opt(2026, 1, 15)
        );
    }

    #[test]
    fn infers_day_first_with_evidence() {
        let inf = infer_date_formats(&["13/01/2026", "02/01/2026"], None);
        assert_eq!(inf.formats, vec!["%d/%m/%Y".to_string()]);
        assert!(!inf.ambiguous);
    }

    #[test]
    fn infers_month_first_with_evidence() {
        let inf = infer_date_formats(&["01/13/2026", "01/02/2026"], None);
        assert_eq!(inf.formats, vec!["%m/%d/%Y".to_string()]);
        assert!(!inf.ambiguous);
    }

    #[test]
    fn flags_genuine_ambiguity() {
        let inf = infer_date_formats(&["01/02/2026", "03/04/2026"], None);
        assert!(inf.ambiguous);
        assert_eq!(inf.formats, vec!["%d/%m/%Y".to_string()]); // day-first default
        let inf_en = infer_date_formats(&["01/02/2026"], Some(crate::config::Locale::En));
        assert_eq!(inf_en.formats, vec!["%m/%d/%Y".to_string()]);
    }

    #[test]
    fn rejects_invalid() {
        assert_eq!(p("31/02/2026", "%d/%m/%Y"), None);
        assert_eq!(p("hello", "%Y-%m-%d"), None);
    }
}
