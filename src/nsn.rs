use serde::Serialize;

use crate::error::IlsError;

#[derive(Debug, Clone, Serialize)]
pub struct Nsn {
    pub input: String,
    pub normalized: String,
    pub kind: NsnKind,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NsnKind {
    Nsn,
    Niin,
}

impl Nsn {
    pub fn parse(raw: &str) -> Result<Self, IlsError> {
        let input = raw.trim().to_owned();
        if input.is_empty() {
            return Err(IlsError::InvalidNsn {
                input,
                reason: "empty input",
            });
        }
        let digits: String = input
            .chars()
            .filter(|c| !matches!(c, '-' | ' ' | '\t'))
            .collect();
        if !digits.chars().all(|c| c.is_ascii_digit()) {
            return Err(IlsError::InvalidNsn {
                input,
                reason: "non-digit characters (after stripping hyphens and whitespace)",
            });
        }
        let kind = match digits.len() {
            13 => NsnKind::Nsn,
            9 => NsnKind::Niin,
            _ => {
                return Err(IlsError::InvalidNsn {
                    input,
                    reason: "must be 13 digits (NSN) or 9 digits (NIIN)",
                });
            }
        };
        Ok(Self {
            input,
            normalized: digits,
            kind,
        })
    }
}

#[derive(Debug)]
pub struct InputEntry {
    pub line: usize,
    pub raw: String,
    pub parsed: Result<Nsn, IlsError>,
}

pub fn parse_nsn_list(text: &str) -> Vec<InputEntry> {
    text.lines()
        .enumerate()
        .filter_map(|(i, line)| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                return None;
            }
            Some(InputEntry {
                line: i + 1,
                raw: trimmed.to_owned(),
                parsed: Nsn::parse(trimmed),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_formatted_nsn() {
        let n = Nsn::parse("4730-01-234-5678").expect("valid NSN");
        assert_eq!(n.normalized, "4730012345678");
        assert_eq!(n.kind, NsnKind::Nsn);
    }

    #[test]
    fn parses_unformatted_nsn() {
        let n = Nsn::parse("4730012345678").expect("valid NSN");
        assert_eq!(n.normalized, "4730012345678");
        assert_eq!(n.kind, NsnKind::Nsn);
    }

    #[test]
    fn parses_niin() {
        let n = Nsn::parse("012345678").expect("valid NIIN");
        assert_eq!(n.normalized, "012345678");
        assert_eq!(n.kind, NsnKind::Niin);
    }

    #[test]
    fn accepts_spaces_and_hyphens() {
        let n = Nsn::parse(" 4730 01-234 5678 ").expect("valid NSN");
        assert_eq!(n.normalized, "4730012345678");
    }

    #[test]
    fn rejects_alpha_characters() {
        assert!(Nsn::parse("ABCD01234567A").is_err());
    }

    #[test]
    fn rejects_wrong_length() {
        assert!(Nsn::parse("1234").is_err());
        assert!(Nsn::parse("12345678901234").is_err());
        assert!(Nsn::parse("1234567890").is_err());
    }

    #[test]
    fn rejects_empty() {
        assert!(Nsn::parse("").is_err());
        assert!(Nsn::parse("   ").is_err());
    }

    #[test]
    fn list_skips_comments_and_blanks() {
        let text = "# header\n\n4730-01-234-5678\n  \n# another comment\n012345678\n";
        let entries = parse_nsn_list(text);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].line, 3);
        assert_eq!(entries[1].line, 6);
    }

    #[test]
    fn list_preserves_invalid_entries() {
        let entries = parse_nsn_list("4730-01-234-5678\nBAD\n");
        assert_eq!(entries.len(), 2);
        assert!(entries[0].parsed.is_ok());
        assert!(entries[1].parsed.is_err());
    }
}
