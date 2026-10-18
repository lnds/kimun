//! Functions of languages that close them with words, neither braces nor
//! indentation: the program units of Fortran and the routines of Pascal.
//! Both read code folded to lower case (`case_fold`).

use crate::detection::FunctionBody;
use crate::util::{indent_level, mask_strings};

/// The words of a line outside strings and comments, and outside
/// parentheses when `flat` is set, so that `real(kind=8) function f(x)`
/// reads `real function f`.
fn words(line: &str, comment: &str, flat: bool) -> Vec<String> {
    let masked = mask_strings(line, &[comment]);
    let mut words = Vec::new();
    let mut word = String::new();
    let mut depth = 0usize;
    for c in masked.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            if depth == 0 || !flat {
                word.push(c);
            }
            continue;
        }
        if !word.is_empty() {
            words.push(std::mem::take(&mut word));
        }
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    if !word.is_empty() {
        words.push(word);
    }
    words
}

/// Walk `code_lines`; where `declares` names a function, `end_of` gives the
/// index of its last line.
fn bodies<'a>(
    code_lines: &[(usize, &'a str)],
    mut declares: impl FnMut(usize) -> Option<String>,
    end_of: impl Fn(usize) -> usize,
) -> Vec<FunctionBody<'a>> {
    let mut functions = Vec::new();
    let mut i = 0;
    while i < code_lines.len() {
        let Some(name) = declares(i) else {
            i += 1;
            continue;
        };
        let end = end_of(i);
        functions.push(FunctionBody {
            name,
            start_line: code_lines[i].0 + 1,
            code_lines: code_lines[i..=end].to_vec(),
        });
        i = end + 1;
    }
    functions
}

// ── Fortran ─────────────────────────────────────────────────────────────

/// Words that may precede `subroutine` or `function` in a declaration:
/// prefixes and result types.
const FORTRAN_PREFIXES: &[&str] = &[
    "recursive",
    "non_recursive",
    "pure",
    "impure",
    "elemental",
    "module",
    "integer",
    "real",
    "double",
    "precision",
    "doubleprecision",
    "complex",
    "doublecomplex",
    "logical",
    "character",
    "type",
    "class",
];

const FORTRAN_UNITS: &[&str] = &["subroutine", "function", "program"];

/// The label of a Fortran statement, if it has one, and its words outside
/// parentheses.
pub fn fortran_statement(line: &str) -> (Option<String>, Vec<String>) {
    let mut words = words(line, "!", true);
    let labelled = words.first().is_some_and(|w| is_number(w));
    let label = labelled.then(|| words.remove(0));
    (label, words)
}

fn is_number(word: &str) -> bool {
    word.bytes().all(|b| b.is_ascii_digit())
}

/// The words of a Fortran statement, without its label.
fn fortran_words(line: &str) -> Vec<String> {
    fortran_statement(line).1
}

/// The name of the program unit a statement opens, if it opens one.
fn fortran_unit(words: &[String]) -> Option<String> {
    let at = words
        .iter()
        .position(|w| FORTRAN_UNITS.contains(&w.as_str()))?;
    let prefixed = words[..at]
        .iter()
        .all(|w| FORTRAN_PREFIXES.contains(&w.as_str()) || is_number(w));
    prefixed.then(|| words.get(at + 1).cloned()).flatten()
}

/// Whether a statement closes a program unit: `end` alone, or `end` with
/// the kind of unit.
fn closes_fortran_unit(words: &[String]) -> bool {
    match words {
        [end] => {
            end == "end" || ["endsubroutine", "endfunction", "endprogram"].contains(&end.as_str())
        }
        [end, kind, ..] if end == "end" => FORTRAN_UNITS.contains(&kind.as_str()),
        [end, ..] => ["endsubroutine", "endfunction", "endprogram"].contains(&end.as_str()),
        [] => false,
    }
}

/// Whether a statement opens or closes an interface block, whose units
/// declare procedures without a body.
fn fortran_interface(words: &[String]) -> Option<bool> {
    match words {
        [w, ..] if w == "interface" => Some(true),
        [a, w, ..] if a == "abstract" && w == "interface" => Some(true),
        [w, ..] if w == "endinterface" => Some(false),
        [e, w, ..] if e == "end" && w == "interface" => Some(false),
        _ => None,
    }
}

/// The program units of Fortran: `subroutine`, `function` and `program`,
/// to the `end` that closes them. Units contained in one are part of it;
/// those of an interface block are no functions.
pub fn fortran<'a>(code_lines: &[(usize, &'a str)]) -> Vec<FunctionBody<'a>> {
    let statements: Vec<Vec<String>> = code_lines.iter().map(|(_, l)| fortran_words(l)).collect();
    let mut in_interface = false;
    let declares = |i: usize| {
        if let Some(opens) = fortran_interface(&statements[i]) {
            in_interface = opens;
        }
        if in_interface {
            return None;
        }
        fortran_unit(&statements[i])
    };
    let end_of = |start: usize| {
        let mut depth = 0usize;
        for (at, words) in statements.iter().enumerate().skip(start) {
            if fortran_unit(words).is_some() {
                depth += 1;
            } else if closes_fortran_unit(words) {
                depth -= 1;
                if depth == 0 {
                    return at;
                }
            }
        }
        statements.len() - 1
    };
    bodies(code_lines, declares, end_of)
}

// ── Pascal ──────────────────────────────────────────────────────────────

const PASCAL_ROUTINES: &[&str] = &[
    "procedure",
    "function",
    "constructor",
    "destructor",
    "operator",
];

/// The routine a line declares, as the keyword's position among its words.
fn pascal_routine(words: &[String]) -> Option<usize> {
    let keyword = |w: &String| PASCAL_ROUTINES.contains(&w.as_str());
    match words {
        [first, ..] if keyword(first) => Some(0),
        [class, second, ..] if class == "class" && keyword(second) => Some(1),
        _ => None,
    }
}

/// Whether a declaration says its body is elsewhere.
fn without_body(words: &[String]) -> bool {
    words.iter().any(|w| w == "forward" || w == "external")
}

/// The name a routine declaration gives, with the type of a method:
/// `tcounter.inc`.
fn pascal_name(line: &str, keyword: &str) -> String {
    let after = line
        .find(keyword)
        .map_or("", |at| &line[at + keyword.len()..]);
    let name: String = after
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '.')
        .collect();
    if name.is_empty() {
        "<anonymous>".to_string()
    } else {
        name
    }
}

/// The structured type a line opens in a type section (`= class`,
/// `= record`, `= interface`…), unless it is declared in that line alone
/// (`= class;`, `= class(tbase);`, `= class of tbase;`).
fn opens_type(line: &str) -> bool {
    let masked = mask_strings(line, &["//"]);
    let Some(eq) = masked.find('=') else {
        return false;
    };
    let rest = masked[eq + 1..].trim_start();
    let rest = rest.strip_prefix("packed").map_or(rest, str::trim_start);
    let Some(kind) = ["class", "object", "record", "interface", "dispinterface"]
        .iter()
        .find(|k| rest.starts_with(*k))
    else {
        return false;
    };
    let after = &rest[kind.len()..];
    if after.starts_with(|c: char| c.is_ascii_alphanumeric() || c == '_') {
        return false;
    }
    let after = after.trim_start();
    let after = match after.strip_prefix('(') {
        Some(inner) => inner
            .find(')')
            .map_or("", |at| &inner[at + 1..])
            .trim_start(),
        None => after,
    };
    !(after.starts_with(';') || after.starts_with("of "))
}

/// How a word changes the blocks of a routine: `begin` and `asm` open its
/// own, `case`, `try` and `record` open others; `end` closes one.
#[derive(Clone, Copy, PartialEq)]
enum Block {
    Own,
    Other,
}

/// Words that, unindented, start a section of a unit or a routine of its
/// own: nothing before them is still open. Conditional compilation can
/// leave a `begin` or a `record` unmatched; these bound the damage.
const PASCAL_SECTIONS: &[&str] = &[
    "implementation",
    "initialization",
    "finalization",
    "type",
    "var",
    "const",
    "resourcestring",
    "threadvar",
    "procedure",
    "function",
    "constructor",
    "destructor",
    "operator",
    "class",
];

/// Whether a line starts a section: unindented, with one of
/// `PASCAL_SECTIONS` or an `interface` of its own.
fn starts_section(line: &str, words: &[String]) -> bool {
    let unindented = !line.starts_with([' ', '\t']);
    unindented
        && match words {
            [w] if w == "interface" => true,
            [w, ..] => PASCAL_SECTIONS.contains(&w.as_str()),
            [] => false,
        }
}

/// The index of the last line of the routine declared at `start`: the
/// `end` of its `begin` block, after those of the routines it declares.
/// Within the block, a section or routine indented no deeper than the
/// declaration ends it on the line before. Before it, a header with the
/// routine's own name is another version of it, under conditional
/// compilation, not a routine it declares.
fn pascal_end(code_lines: &[(usize, &str)], statements: &[Vec<String>], start: usize) -> usize {
    let mut pending = 1usize;
    let mut blocks: Vec<Block> = Vec::new();
    let depth = indent_level(code_lines[start].1);
    let name_of = |at: usize| {
        let keyword = pascal_routine(&statements[at])?;
        Some(pascal_name(code_lines[at].1, &statements[at][keyword]))
    };
    let own = name_of(start);
    for (at, words) in statements.iter().enumerate().skip(start + 1) {
        let line = code_lines[at].1;
        let routine = pascal_routine(words).is_some();
        if !blocks.is_empty()
            && indent_level(line) <= depth
            && (routine || starts_section(line, words))
        {
            return at - 1;
        }
        if blocks.is_empty() && routine && !without_body(words) && name_of(at) != own {
            pending += 1;
        }
        for word in words {
            match word.as_str() {
                "begin" | "asm" => blocks.push(Block::Own),
                "case" | "try" | "record" => blocks.push(Block::Other),
                "end" => {
                    let closes_own = blocks.pop() == Some(Block::Own) && blocks.is_empty();
                    pending -= usize::from(closes_own);
                    if pending == 0 {
                        return at;
                    }
                }
                _ => {}
            }
        }
    }
    statements.len() - 1
}

/// The routines of Pascal, from their header to the `end` of their block.
/// Those the `interface` section declares, the methods a class type
/// declares and `forward` or `external` ones have no body here.
pub fn pascal<'a>(code_lines: &[(usize, &'a str)]) -> Vec<FunctionBody<'a>> {
    let statements: Vec<Vec<String>> = code_lines
        .iter()
        .map(|(_, l)| words(l, "//", false))
        .collect();
    let mut in_interface = false;
    let mut type_depth = 0usize;
    let declares = |i: usize| {
        let words = &statements[i];
        if starts_section(code_lines[i].1, words) {
            type_depth = 0;
        }
        let opens = opens_type(code_lines[i].1);
        if type_depth > 0 || opens {
            let records = words.iter().filter(|w| *w == "record").count();
            let ends = words.iter().filter(|w| *w == "end").count();
            type_depth = (type_depth + records.max(usize::from(opens))).saturating_sub(ends);
            return None;
        }
        match words.as_slice() {
            [w] if w == "interface" => in_interface = true,
            [w] if w == "implementation" => in_interface = false,
            _ => {}
        }
        let keyword = pascal_routine(words)?;
        if in_interface || without_body(words) {
            return None;
        }
        Some(pascal_name(code_lines[i].1, &words[keyword]))
    };
    bodies(code_lines, declares, |start| {
        pascal_end(code_lines, &statements, start)
    })
}

#[cfg(test)]
#[path = "unit_bodies_test.rs"]
mod tests;
