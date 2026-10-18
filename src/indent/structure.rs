//! Indentation depth read as structure rather than layout: a line that
//! continues a statement is not nested deeper than one level below it.

use super::analyzer::indent_depth;

/// What a line sits inside: the parentheses or brackets opened on a line
/// of the given depth, or a block.
enum Open {
    Paren(usize),
    Block,
}

/// What is known about the statement being read, line by line.
#[derive(Default)]
struct Statements {
    stack: Vec<Open>,
    /// Depth of the last line that started a statement.
    base: usize,
    /// Whether the last line ended in an operator that continues it.
    continues: bool,
}

impl Statements {
    /// The structural depth of a line at layout depth `raw`.
    fn depth(&mut self, raw: usize, trimmed: &str) -> usize {
        if raw == 0 && !trimmed.starts_with([')', ']', '}']) {
            // A new top-level statement: whatever was left open was misread.
            self.stack.clear();
        }
        if let Some(Open::Paren(opener)) = self.stack.last() {
            return raw.min(opener + 1);
        }
        if self.continues || starts_with_operator(trimmed) {
            return raw.min(self.base + 1);
        }
        self.base = raw;
        raw
    }

    /// Take in what a line at `depth` opens and closes.
    fn read(&mut self, trimmed: &str, depth: usize) {
        for ch in trimmed.chars() {
            match ch {
                '(' | '[' => self.stack.push(Open::Paren(depth)),
                '{' => self.stack.push(Open::Block),
                ')' | ']' => self.pop_until(|o| matches!(o, Open::Paren(_))),
                '}' => self.pop_until(|o| matches!(o, Open::Block)),
                _ => {}
            }
        }
        if closes_block(trimmed) {
            self.pop_until(|o| matches!(o, Open::Block));
        } else if opens_block(trimmed) {
            self.stack.push(Open::Block);
        }
        self.continues = ends_with_operator(trimmed);
    }

    /// Pop down to and including the innermost entry `is_match` takes;
    /// nothing when there is none.
    fn pop_until(&mut self, is_match: impl Fn(&Open) -> bool) {
        if let Some(at) = self.stack.iter().rposition(is_match) {
            self.stack.truncate(at);
        }
    }
}

/// The depth of each code line as structure, not layout. A line that
/// continues a statement (inside parentheses or brackets, after a line
/// ending in an operator, or starting with one) is at most one level deeper
/// than the line it continues: arguments aligned under an opening
/// parenthesis are not nesting. Inside a block opened within parentheses,
/// such as the body of a closure, depths count again from that block.
pub fn structural_depths(code: &[&str], tab_width: usize) -> Vec<usize> {
    let mut statements = Statements::default();
    code.iter()
        .map(|line| {
            let text = code_text(line);
            let trimmed = text.trim();
            let depth = statements.depth(indent_depth(line, tab_width), trimmed);
            statements.read(trimmed, depth);
            depth
        })
        .collect()
}

/// Whether a line closes a block opened by a keyword: `end`.
fn closes_block(trimmed: &str) -> bool {
    trimmed == "end" || trimmed.starts_with("end ") || trimmed.starts_with("end)")
}

/// Whether a line ends opening a block whose body follows it, by a keyword
/// rather than a brace: `do`, `->`, `=>`.
fn opens_block(trimmed: &str) -> bool {
    trimmed == "do"
        || trimmed.ends_with(" do")
        || trimmed.ends_with("->")
        || trimmed.ends_with("=>")
}

/// Operators that, ending a line, leave the statement for the next one.
const TRAILING_OPERATORS: &[&str] = &["::", "&&", "||", "\\", "|>", "|"];

/// Operators that, starting a line, continue the statement before it.
const LEADING_OPERATORS: &[&str] = &[".", "|", "&&", "||"];

fn ends_with_operator(trimmed: &str) -> bool {
    TRAILING_OPERATORS.iter().any(|op| trimmed.ends_with(op))
}

fn starts_with_operator(trimmed: &str) -> bool {
    LEADING_OPERATORS.iter().any(|op| trimmed.starts_with(op)) && !trimmed.starts_with("..")
}

/// A line without the contents of its string and character literals and
/// without a trailing comment (`//`, or `#` followed by a space), so that
/// brackets and operators in them count for nothing.
fn code_text(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::with_capacity(line.len());
    let mut i = 0;
    while i < chars.len() {
        if starts_comment(&chars, i) {
            break;
        }
        if chars[i] == '"' {
            out.push_str("\"\"");
            i = string_end(&chars, i + 1) + 1;
        } else if let Some(len) = char_literal_len(&chars, i) {
            i += len;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// Whether a trailing comment starts at `i`.
fn starts_comment(chars: &[char], i: usize) -> bool {
    let next = chars.get(i + 1);
    (chars[i] == '/' && next == Some(&'/')) || (chars[i] == '#' && next == Some(&' '))
}

/// Where the string literal whose content starts at `i` closes, or the end
/// of the line.
fn string_end(chars: &[char], mut i: usize) -> usize {
    while i < chars.len() && chars[i] != '"' {
        i += if chars[i] == '\\' { 2 } else { 1 };
    }
    i
}

/// The length of a character literal at `i` ('x' or '\x'), if one starts
/// there. A lone quote, such as a Rust lifetime, is none.
fn char_literal_len(chars: &[char], i: usize) -> Option<usize> {
    if chars[i] != '\'' {
        return None;
    }
    if chars.get(i + 2) == Some(&'\'') {
        return Some(3);
    }
    (chars.get(i + 1) == Some(&'\\') && chars.get(i + 3) == Some(&'\'')).then_some(4)
}

#[cfg(test)]
#[path = "structure_test.rs"]
mod tests;
