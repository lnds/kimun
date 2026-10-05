//! Where a patch touches a file, on each side of it.

use std::error::Error;

use git2::{DiffLine, Patch};

/// How many added lines are kept to recognise a text that holds the change.
const PROBE_LINES: usize = 3;

/// The lines a patch touches, from 1, in the file after it (`new`) and
/// before it (`old`), and a few of the lines it adds.
#[derive(Default)]
pub(super) struct Touched {
    pub new: Vec<usize>,
    pub old: Vec<usize>,
    pub probe: Vec<(usize, String)>,
}

/// The next line to come on each side while a hunk is read.
struct Cursor {
    new: usize,
    old: usize,
}

impl Touched {
    /// Record one line of a hunk. An addition touches, on the old side, the
    /// line it is inserted at; a deletion, on the new side, the line that
    /// now stands where the deleted ones were.
    fn record(&mut self, line: &DiffLine, at: &mut Cursor) {
        let origin = line.origin();
        if matches!(origin, '+' | '-') {
            self.new.push(at.new);
            self.old.push(at.old);
        }
        if origin == '+' {
            let text = String::from_utf8_lossy(line.content());
            self.probe.push((at.new, text.trim_end().to_string()));
        }
        at.new += usize::from(matches!(origin, '+' | ' '));
        at.old += usize::from(matches!(origin, '-' | ' '));
    }

    /// The lines `patch` touches.
    pub fn of(patch: &Patch) -> Result<Self, Box<dyn Error>> {
        let mut touched = Self::default();
        for hunk in 0..patch.num_hunks() {
            let (header, count) = patch.hunk(hunk)?;
            let mut at = Cursor {
                new: (header.new_start() as usize).max(1),
                old: (header.old_start() as usize).max(1),
            };
            for line in 0..count {
                touched.record(&patch.line_in_hunk(hunk, line)?, &mut at);
            }
        }
        for lines in [&mut touched.new, &mut touched.old] {
            lines.sort_unstable();
            lines.dedup();
        }
        // Blank lines say nothing about a text; keep the telling ones.
        touched.probe.retain(|(_, text)| !text.trim().is_empty());
        touched.probe.truncate(PROBE_LINES);
        Ok(touched)
    }
}
