//! Declarations that are no function with a body: types, whose
//! parentheses hold a constructor or a base class, and abstract members.
//! Operator names, whose symbols would read as an assignment.

use super::*;

/// Keywords that declare a type, whose parentheses hold a constructor or
/// a base class rather than the parameters of a function.
const TYPE_KEYWORDS: &[&str] = &[
    "class",
    "struct",
    "record",
    "interface",
    "object",
    "enum",
    "union",
];

/// Whether a signature shaped like a C function declares none: a
/// preprocessor directive (`#define NAME(args) body`, which would otherwise
/// absorb the complexity of the code after it) or a type.
pub(super) fn declares_no_function(sig: &str) -> bool {
    sig.starts_with('#') || is_type_declaration(sig)
}

/// `line` with the `for` of a trait implementation (`impl Display for
/// Level {`) blanked: it names the type, it is no loop.
pub(crate) fn without_impl_for(line: &str) -> std::borrow::Cow<'_, str> {
    let declaration = line.trim_start();
    let implements = ["impl", "unsafe impl", "default impl"].iter().any(|word| {
        declaration.starts_with(&format!("{word} ")) || declaration.starts_with(&format!("{word}<"))
    });
    match line.find(" for ").filter(|_| implements) {
        Some(at) => {
            let mut out = line.to_string();
            out.replace_range(at + 1..at + 4, "   ");
            out.into()
        }
        None => line.into(),
    }
}

/// Whether a declaration names a type: a type keyword followed by the
/// name of the type (`class Repo(private val db: Db) {`, `record Point(int
/// x, int y) {`, `object Cache : Base(size) {`, `fun interface Action {`).
/// A C function returning a type (`struct node *make(int v) {`) names the
/// type and then the function, and is no type declaration.
pub(super) fn is_type_declaration(sig: &str) -> bool {
    let head = sig.find('(').map_or(sig, |at| &sig[..at]);
    let head = without_type_arguments(&head.replace("::", "__"));
    let words: Vec<&str> = head
        .split(|c: char| c.is_whitespace() || c == '*' || c == '&' || c == '{')
        .filter(|w| !w.is_empty())
        .collect();
    let Some(keyword) = words.iter().rposition(|w| TYPE_KEYWORDS.contains(w)) else {
        return false;
    };
    let after = &words[keyword + 1..];
    let returned_type = after.len() == 2 && after.iter().all(|w| is_identifier(w));
    // In C# `object` is a type a method returns (`object Get(int id)`); a
    // Kotlin object has parentheses only in what it extends.
    let least = if words[keyword] == "object" { 2 } else { 1 };
    after.len() >= least && !returned_type
}

/// `text` without the type arguments it holds (`Box<T, U>`).
pub(super) fn without_type_arguments(text: &str) -> String {
    let mut depth = 0usize;
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '<' => depth += 1,
            '>' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(ch),
            _ => {}
        }
    }
    out
}

/// `sig` with the symbols of an operator's name (`operator==`,
/// `operator()`) turned into `_`, so that they read as part of the name
/// and not as an assignment or the parameter list.
pub(super) fn without_operator_symbols(sig: &str) -> String {
    let Some(at) = operator_name_at(sig) else {
        return sig.to_string();
    };
    let start = at + "operator".len();
    let rest = &sig[start..];
    let gap = rest.len() - rest.trim_start().len();
    let rest = rest.trim_start();
    let call = usize::from(rest.starts_with("()")) * 2;
    let symbols = call
        + rest[call..]
            .bytes()
            .take_while(|b| b"=!<>+-*/%&|^~[],".contains(b))
            .count();
    let mut out = sig.to_string();
    out.replace_range(start + gap..start + gap + symbols, &"_".repeat(symbols));
    out
}

/// Where the word `operator` that names an operator starts.
fn operator_name_at(sig: &str) -> Option<usize> {
    sig.match_indices("operator").map(|(at, _)| at).find(|&at| {
        let before_ok = !sig[..at].ends_with(|c: char| c.is_alphanumeric() || c == '_');
        let next = sig[at + "operator".len()..].trim_start();
        before_ok && next.starts_with(|c: char| "=!<>+-*/%&|^~[(,".contains(c))
    })
}

/// Whether a declaration on one line, `masked`, read into `scan`, has no
/// body: its parameters are seen to close and neither it nor the line after
/// it goes on to a body (`fun get(id: Int): Row?` in an interface).
pub(super) fn without_body(
    scan: &BodyScan,
    masked: &str,
    next: Option<&(usize, &str)>,
    base_indent: usize,
) -> bool {
    let line = masked.trim_end();
    let closed = line.contains('(') && scan.paren_depth <= 0;
    let next_goes_on = next.is_some_and(|(_, next)| goes_on_from(next, base_indent));
    closed && !scan.found_open && !scan.expression_body && !goes_on(line) && !next_goes_on
}

/// Whether a signature goes on past the end of `line`.
fn goes_on(line: &str) -> bool {
    line.ends_with(['{', '=', ':', ',', '<', '(']) || line.ends_with("->")
}

/// Whether `next` goes on with a declaration made at `base_indent`.
fn goes_on_from(next: &str, base_indent: usize) -> bool {
    let trimmed = next.trim_start();
    indent_level(next) > base_indent
        || trimmed.starts_with(['{', '=', ':', '.', ')'])
        || ["->", "where"].iter().any(|word| trimmed.starts_with(word))
}
