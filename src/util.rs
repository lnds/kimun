use std::error::Error;
use std::fs::File;
use std::hash::Hasher;
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use std::time::SystemTime;

use crate::loc::counter::{LineKind, classify_reader};
use crate::loc::language::LanguageSpec;

pub type ClassifiedSource = (Vec<String>, Vec<LineKind>);

/// Check whether a reader points to a binary file by looking for null bytes
/// in the first 512 bytes. Resets the reader position to the start afterward.
pub fn is_binary_reader<R: Read + Seek>(reader: &mut R) -> io::Result<bool> {
    let mut header = [0u8; 512];
    let n = reader.read(&mut header)?;
    reader.seek(SeekFrom::Start(0))?;
    Ok(header[..n].contains(&0))
}

/// Compute a content hash for a file using streaming FNV via `DefaultHasher`.
/// Returns `None` if the file cannot be opened or read.
pub fn hash_file(path: &Path) -> Option<u64> {
    let file = File::open(path).ok()?;
    let mut reader = BufReader::new(file);
    let mut hasher = std::hash::DefaultHasher::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = reader.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        hasher.write(&buf[..n]);
    }
    Some(hasher.finish())
}

/// Replace the contents of string and char literals with spaces,
/// so that keywords/braces inside literals are not counted.
pub fn mask_strings(line: &str) -> String {
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut result = bytes.to_vec();
    let mut i = 0;

    while i < len {
        let ch = bytes[i];
        if ch == b'"' || ch == b'\'' {
            let quote = ch;
            i += 1; // skip opening quote
            while i < len {
                if bytes[i] == b'\\' {
                    // escape: mask both chars
                    result[i] = b' ';
                    i += 1;
                    if i < len {
                        result[i] = b' ';
                        i += 1;
                    }
                } else if bytes[i] == quote {
                    i += 1; // skip closing quote
                    break;
                } else {
                    result[i] = b' ';
                    i += 1;
                }
            }
        } else {
            i += 1;
        }
    }

    // SAFETY: we only replaced ASCII bytes with ASCII spaces
    String::from_utf8(result).unwrap_or_else(|_| line.to_string())
}

/// Read a source file, check for binary content, and classify lines.
/// Returns None for binary files. On success returns the split lines
/// and the per-line classification.
pub fn read_and_classify(path: &Path, spec: &LanguageSpec) -> io::Result<Option<ClassifiedSource>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    if is_binary_reader(&mut reader)? {
        return Ok(None);
    }

    let content = io::read_to_string(reader)?;
    let lines: Vec<String> = content.lines().map(String::from).collect();
    let kinds = classify_reader(content.as_bytes(), spec);

    Ok(Some((lines, kinds)))
}

/// Find the line index where `#[cfg(test)]` starts (for stripping inline test blocks).
/// Returns `lines.len()` if no such line is found.
pub fn find_test_block_start(lines: &[String]) -> usize {
    for (i, line) in lines.iter().enumerate() {
        if line.trim() == "#[cfg(test)]" {
            return i;
        }
    }
    lines.len()
}

/// Parse a duration string like "6m", "1y", "30d" into a Unix timestamp
/// representing that far back from now.
///
/// Approximations: 1 month = 30 days, 1 year = 365 days.
pub fn parse_since(s: &str) -> Result<i64, Box<dyn Error>> {
    let s = s.trim();
    if s.is_empty() {
        return Err("empty --since value".into());
    }

    let split_pos = s.find(|c: char| !c.is_ascii_digit()).ok_or_else(|| {
        format!("invalid --since value: {s:?} (no unit, expected e.g. 6m, 1y, 30d)")
    })?;

    let (num_str, unit) = s.split_at(split_pos);
    let n: u64 = num_str
        .parse()
        .map_err(|_| format!("invalid --since value: {s:?} (expected e.g. 6m, 1y, 30d)"))?;

    let seconds = match unit {
        "d" | "day" | "days" => n.checked_mul(86_400),
        "m" | "mo" | "month" | "months" => n.checked_mul(30 * 86_400),
        "y" | "yr" | "year" | "years" => n.checked_mul(365 * 86_400),
        _ => return Err(format!("unknown unit in --since: {s:?} (use d, m, or y)").into()),
    }
    .ok_or("--since value too large")?;

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs();

    let ts = now
        .checked_sub(seconds)
        .ok_or("--since value goes before Unix epoch")?;

    Ok(ts as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_strings_basic() {
        assert_eq!(
            mask_strings(r#"let s = "if x > 0";"#),
            r#"let s = "        ";"#
        );
        assert_eq!(
            mask_strings(r#"let c = '{'; if x {"#),
            r#"let c = ' '; if x {"#
        );
        assert_eq!(
            mask_strings(r#"let s = "he said \"hi\"";"#),
            r#"let s = "              ";"#
        );
    }

    #[test]
    fn mask_strings_empty() {
        assert_eq!(mask_strings(""), "");
    }

    #[test]
    fn mask_strings_no_strings() {
        assert_eq!(mask_strings("let x = 42;"), "let x = 42;");
    }

    #[test]
    fn mask_strings_raw_string() {
        // Python raw string: r prefix is just an identifier char, "..." is masked normally
        let result = mask_strings(r#"x = r"if|for|while""#);
        assert!(!result.contains("if|for|while"));
        assert!(result.contains("r")); // the r prefix is preserved
    }

    #[test]
    fn mask_strings_unclosed_string() {
        // Unclosed string: mask everything after the quote
        assert_eq!(mask_strings(r#"let s = "hello"#), r#"let s = "     "#);
    }
}
