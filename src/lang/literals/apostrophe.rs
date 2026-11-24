//! What a `'` is in a language where it opens no string.

use super::is_word_byte;

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Apostrophe {
    /// A character literal, or the quote of a lifetime or a label (Rust).
    Character,
    /// After an operand, part of a name (Haskell's `x'`) or an operator on
    /// it (Julia's adjoint `A'`); a character literal elsewhere.
    Prime,
    /// Code: Clojure quotes a form with it, and writes characters as `\a`.
    Code,
}

impl Apostrophe {
    pub(crate) fn of(language: &str) -> Self {
        match language {
            "Haskell" | "OCaml" | "F#" | "Julia" => Apostrophe::Prime,
            "Clojure" => Apostrophe::Code,
            _ => Apostrophe::Character,
        }
    }

    /// Whether the `'` at `at` of `bytes` is code.
    pub(crate) fn is_code(self, bytes: &[u8], at: usize) -> bool {
        match self {
            Apostrophe::Character => false,
            Apostrophe::Prime => at
                .checked_sub(1)
                .is_some_and(|b| is_word_byte(bytes[b]) || b")]'".contains(&bytes[b])),
            Apostrophe::Code => true,
        }
    }
}
