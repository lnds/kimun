//! The text of a file before a change, put back from the text after it.

use std::collections::HashSet;

use super::FileDiffStat;

impl FileDiffStat {
    /// The file as it was before the change, given `after`, the file as it
    /// is after it: the lines the change added taken out, those it deleted
    /// put back where they were.
    pub fn before(&self, after: &str) -> String {
        let added: HashSet<usize> = self.inserted_at.iter().copied().collect();
        let mut kept = after
            .lines()
            .enumerate()
            .filter(|(i, _)| !added.contains(&(i + 1)))
            .map(|(_, line)| line);
        let mut deleted = self.removed_at.iter().zip(&self.removed).peekable();
        let mut lines: Vec<&str> = Vec::new();
        loop {
            let line = match deleted.peek() {
                Some((at, _)) if **at == lines.len() + 1 => deleted.next().map(|(_, l)| l.as_str()),
                _ => kept
                    .next()
                    .or_else(|| deleted.next().map(|(_, l)| l.as_str())),
            };
            match line {
                Some(line) => lines.push(line),
                None => break,
            }
        }
        lines.iter().map(|l| format!("{l}\n")).collect()
    }
}
