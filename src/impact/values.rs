//! The literal values a line of code holds, in any language: quoted
//! strings without interpolation, the words of a word sigil (`~w(CL PE)`),
//! atoms (`:pending`, read as `pending`) and numbers.

/// Values too common to tell one group from another.
const COMMON: [&str; 16] = [
    "true", "false", "nil", "null", "none", "ok", "error", "id", "name", "type", "value", "key",
    "data", "self", "the", "and",
];

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Whether a value is worth matching: long enough, more than punctuation
/// (an operator such as `??`), and not too common.
fn telling(value: &str) -> bool {
    value.chars().count() >= 2
        && value.chars().any(char::is_alphanumeric)
        && !COMMON.contains(&value.to_lowercase().as_str())
}

/// The delimiter that closes one opened with `open`.
fn closer(open: char) -> Option<char> {
    match open {
        '(' => Some(')'),
        '[' => Some(']'),
        '{' => Some('}'),
        '<' => Some('>'),
        '/' | '|' | '"' | '\'' => Some(open),
        _ => None,
    }
}

/// A line read value by value.
struct Reader {
    chars: Vec<char>,
    at: usize,
    values: Vec<String>,
}

impl Reader {
    fn peek(&self, ahead: usize) -> Option<char> {
        self.chars.get(self.at + ahead).copied()
    }

    /// Whether the character before the current one belongs to a name.
    fn after_ident(&self) -> bool {
        self.at > 0 && is_ident(self.chars[self.at - 1])
    }

    /// The characters from the current one while `keep` holds them.
    fn take_while(&self, from: usize, keep: impl Fn(char) -> bool) -> String {
        self.chars[from..]
            .iter()
            .take_while(|&&c| keep(c))
            .collect()
    }

    /// A quoted string: kept unless it interpolates. One the line does not
    /// close ends the reading.
    fn string(&mut self, quote: char) {
        let mut text = String::new();
        let mut i = self.at + 1;
        while let Some(&c) = self.chars.get(i) {
            if c == quote {
                if !text.contains("#{") && !text.contains("${") {
                    self.values.push(text);
                }
                self.at = i + 1;
                return;
            }
            if c == '\\' {
                text.extend(self.chars.get(i + 1));
                i += 2;
            } else {
                text.push(c);
                i += 1;
            }
        }
        self.at = self.chars.len();
    }

    /// The words of a word sigil, whose delimiter follows `~w`.
    fn word_sigil(&mut self) {
        let Some(close) = self.peek(2).and_then(closer) else {
            self.at += 1;
            return;
        };
        let body = self.take_while(self.at + 3, |c| c != close);
        self.values
            .extend(body.split_whitespace().map(str::to_string));
        self.at += 3 + body.chars().count() + 1;
    }

    /// An atom, whose name follows the colon.
    fn atom(&mut self) {
        let name = self.take_while(self.at + 1, is_ident);
        self.at += 1;
        if name.starts_with(|c: char| c.is_alphabetic() || c == '_') {
            self.at += name.chars().count();
            self.values.push(name);
        }
    }

    fn number(&mut self) {
        let number = self.take_while(self.at, |c| c.is_ascii_digit() || matches!(c, '.' | '_'));
        self.at += number.chars().count();
        self.values.push(number.trim_end_matches('.').to_string());
    }

    /// Read the value at the current character, or step over it.
    fn step(&mut self) {
        let c = self.chars[self.at];
        let free = !self.after_ident();
        match c {
            '"' | '\'' if free => self.string(c),
            '~' if matches!(self.peek(1), Some('w' | 'W')) => self.word_sigil(),
            ':' if free && (self.at == 0 || self.chars[self.at - 1] != ':') => self.atom(),
            c if free && c.is_ascii_digit() => self.number(),
            _ => self.at += 1,
        }
    }
}

/// The telling literal values of a line, in order.
pub fn literals(line: &str) -> Vec<String> {
    let mut reader = Reader {
        chars: line.chars().collect(),
        at: 0,
        values: Vec::new(),
    };
    while reader.at < reader.chars.len() {
        reader.step();
    }
    reader.values.retain(|v| telling(v));
    reader.values
}
