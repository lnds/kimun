//! What `km tc` pairs: files of code alone, or with `--across` files of
//! different kinds, such as a document and the code it describes; and the
//! pairs it keeps.

use std::path::Path;

use super::TcOptions;
use super::analyzer::FileCoupling;
use crate::kind::Kind;
use crate::walk::{self, ExcludeFilter};

/// Whether a path of history is read: one of code, or across kinds one of
/// the kinds paired.
pub fn read(path: &Path, opts: &TcOptions<'_>) -> bool {
    let kinds = opts.filter.kinds();
    match opts.across {
        true => {
            !crate::util::is_generated(path)
                && kind_of(path, opts.filter).is_some_and(|kind| kinds.across(kind))
        }
        false => walk::is_code_path(path, opts.filter),
    }
}

/// The kind of a path of history.
pub fn kind_of(path: &Path, filter: &ExcludeFilter) -> Option<Kind> {
    filter.kinds().kind_of_path(path, Path::new(""))
}

/// Keep the pairs as strong as `--min-strength` asks and, with
/// `--across`, whose files are of different kinds.
pub fn keep_pairs(pairs: &mut Vec<FileCoupling>, opts: &TcOptions<'_>) {
    let strong = |p: &FileCoupling| opts.min_strength.is_none_or(|min| p.strength >= min);
    let across = |p: &FileCoupling| {
        !opts.across || kind_of(&p.file_a, opts.filter) != kind_of(&p.file_b, opts.filter)
    };
    pairs.retain(|p| strong(p) && across(p));
}
