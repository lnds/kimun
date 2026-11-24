//! Haskell type signatures (`f :: Int -> Int`), which describe a value and
//! run nothing, so Halstead leaves them out, with the lines they go on to.

/// Which of `lines` belong to a type signature.
pub fn signature_mask(lines: &[&str]) -> Vec<bool> {
    let mut mask = vec![false; lines.len()];
    let mut i = 0;
    while i < lines.len() {
        if !is_signature(lines[i]) {
            i += 1;
            continue;
        }
        let indent = indentation(lines[i]);
        mask[i] = true;
        i += 1;
        while i < lines.len() && indentation(lines[i]) > indent {
            mask[i] = true;
            i += 1;
        }
    }
    mask
}

/// Whether a line opens a signature: names, or operators in parentheses,
/// separated by commas, then `::`.
fn is_signature(line: &str) -> bool {
    let Some((names, _)) = line.split_once("::") else {
        return false;
    };
    let mut names = names.split(',').map(str::trim).peekable();
    names.peek().is_some_and(|n| !n.is_empty()) && names.all(is_bound_name)
}

/// A variable (`go`, `x'`) or an operator in parentheses (`(<+>)`).
fn is_bound_name(name: &str) -> bool {
    if let Some(op) = name.strip_prefix('(').and_then(|n| n.strip_suffix(')')) {
        return !op.is_empty() && op.chars().all(|c| "!#$%&*+./<=>?@\\^|-~:".contains(c));
    }
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_lowercase() || c == '_')
        && chars.all(|c| c.is_alphanumeric() || c == '_' || c == '\'')
}

fn indentation(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

#[cfg(test)]
#[path = "signatures_test.rs"]
mod tests;
