//! Where a Python statement runs, as far as imports are concerned: when the
//! module loads, later in a function, or never, under `if TYPE_CHECKING:`.
//! Read by indentation, over the lines of code of a file in order.

use super::graph::EdgeKind;
use super::python::keyword;

/// A block whose statements do not run when the module loads.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Block {
    Function,
    TypeChecking,
}

/// The blocks open at a line, with the indentation of the line that
/// opened each one.
#[derive(Debug, Default)]
pub struct Blocks {
    open: Vec<(usize, Block)>,
    /// Brackets still open from the lines before.
    depth: usize,
    /// The line before ended in a backslash.
    continued: bool,
}

impl Blocks {
    /// Read the next line of code and tell how a statement that starts on it
    /// runs. A line that goes on a statement started before changes nothing.
    pub fn line(&mut self, line: &str) -> EdgeKind {
        let fresh = self.depth == 0 && !self.continued;
        let text = line.trim_start();
        if fresh && !text.is_empty() {
            let indent = line.len() - text.len();
            while self.open.last().is_some_and(|&(at, _)| at >= indent) {
                self.open.pop();
            }
            let kind = self.kind();
            if let Some(block) = opens(text) {
                self.open.push((indent, block));
            }
            self.read_brackets(line);
            return kind;
        }
        self.read_brackets(line);
        self.kind()
    }

    /// How a statement inside the blocks open runs.
    fn kind(&self) -> EdgeKind {
        let within = |block| self.open.iter().any(|&(_, b)| b == block);
        if within(Block::TypeChecking) {
            EdgeKind::Type
        } else if within(Block::Function) {
            EdgeKind::Deferred
        } else {
            EdgeKind::Load
        }
    }

    fn read_brackets(&mut self, line: &str) {
        self.depth = balance(line, self.depth);
        self.continued = line.trim_end().ends_with('\\');
    }
}

/// The block a line opens, if it opens one that does not run on load.
fn opens(text: &str) -> Option<Block> {
    let def = keyword(text, "async").map_or(text, str::trim_start);
    if keyword(def, "def").is_some() {
        return Some(Block::Function);
    }
    let condition = keyword(text, "if")?.trim().strip_suffix(':')?.trim();
    let name = condition.rsplit('.').next().unwrap_or(condition);
    let dotted = condition
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '.');
    (name == "TYPE_CHECKING" && dotted).then_some(Block::TypeChecking)
}

/// The brackets open after `line`, given those open before it. Strings
/// that start and end on the line are skipped.
fn balance(line: &str, mut depth: usize) -> usize {
    let mut quote = None;
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        match (quote, c) {
            (Some(_), '\\') => {
                chars.next();
            }
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(c),
            (None, '(' | '[' | '{') => depth += 1,
            (None, ')' | ']' | '}') => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    depth
}

#[cfg(test)]
#[path = "python_blocks_test.rs"]
mod tests;
