//! What every command reads a source file with: the languages kimun knows
//! (`language.rs`, detection and `LanguageSpec`) and the classifier of its
//! lines as blank, comment or code (`counter.rs`), which reads literals as
//! each language writes them (`literals/`, `strings.rs`).
pub(crate) mod counter;
mod fsm;
mod lang_macro;
pub(crate) mod language;
pub(crate) mod literals;
mod strings;
