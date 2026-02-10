use std::io::{self, Read, Seek, SeekFrom};

/// Check whether a reader points to a binary file by looking for null bytes
/// in the first 512 bytes. Resets the reader position to the start afterward.
pub fn is_binary_reader<R: Read + Seek>(reader: &mut R) -> io::Result<bool> {
    let mut header = [0u8; 512];
    let n = reader.read(&mut header)?;
    reader.seek(SeekFrom::Start(0))?;
    Ok(header[..n].contains(&0))
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
