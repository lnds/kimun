//! Sequences of boolean operators: +1 for the first operator of a
//! sequence, +1 each time the operator changes. A sequence goes on through
//! parentheses and onto the next line when one of the two lines has an
//! operator at the edge they share; a comma, a `;` or a brace ends it.

use super::markers::CognitiveMarkers;

/// The boolean sequence being read, from one line to the next.
#[derive(Default)]
pub(super) struct Booleans {
    last: Option<&'static str>,
    /// The line read last ended with an operator.
    goes_on: bool,
}

impl Booleans {
    /// The increments of a line.
    pub(super) fn line(&mut self, stripped: &str, markers: &CognitiveMarkers) -> usize {
        let bytes = stripped.trim().as_bytes();
        if !(self.goes_on
            || bytes
                .first()
                .is_some_and(|_| operator_at(bytes, 0, markers).is_some()))
        {
            self.last = None;
        }
        let mut count = 0;
        let mut i = 0;
        let mut ends_with_operator = false;
        while i < bytes.len() {
            let Some(op) = operator_at(bytes, i, markers) else {
                if b"{};,".contains(&bytes[i]) {
                    self.last = None;
                }
                ends_with_operator &= bytes[i].is_ascii_whitespace();
                i += 1;
                continue;
            };
            count += usize::from(self.last != Some(op));
            self.last = Some(op);
            ends_with_operator = true;
            i += op.len();
        }
        self.goes_on = ends_with_operator;
        count
    }
}

/// The increments of a line read on its own.
pub(super) fn count_boolean_sequences(stripped: &str, markers: &CognitiveMarkers) -> usize {
    Booleans::default().line(stripped, markers)
}

/// The boolean operator that starts at byte `i`, if any; a word operator
/// (`and`, `or`) only as a whole word.
fn operator_at(bytes: &[u8], i: usize, markers: &CognitiveMarkers) -> Option<&'static str> {
    let op = *markers
        .boolean_operators
        .iter()
        .find(|op| bytes[i..].starts_with(op.as_bytes()))?;
    let is_word = op.starts_with(|c: char| c.is_alphabetic());
    let word_byte = |b: &u8| b.is_ascii_alphanumeric() || *b == b'_';
    let bounded =
        (i == 0 || !word_byte(&bytes[i - 1])) && !bytes.get(i + op.len()).is_some_and(word_byte);
    (!is_word || bounded).then_some(op)
}
