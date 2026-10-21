//! What a Markdown document refers to and the anchors it offers, read line
//! by line. Fenced code, front matter and HTML comments refer to nothing.

use std::collections::{HashMap, HashSet};

use serde::Serialize;

/// How a document refers to something.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Form {
    /// A link: `[text](target)`, a reference definition or `<a href>`.
    Link,
    /// An image: `![alt](target)` or `<img src>`.
    Image,
    /// A code span: `` `src/main.rs` ``.
    Code,
    /// A wikilink to a note by its name: `[[Note]]`, `[[Note|text]]`,
    /// `![[Note]]`; the target keeps a `#heading`.
    Note,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Reference {
    pub line: usize,
    pub target: String,
    pub form: Form,
}

/// The references of a document and the anchors its headings and HTML
/// elements give, as GitHub names them.
#[derive(Debug, Default)]
pub struct Scan {
    pub references: Vec<Reference>,
    pub anchors: HashSet<String>,
}

pub fn scan(text: &str) -> Scan {
    let mut scan = Scan::default();
    let mut slugs = Slugs::default();
    let mut fence: Option<(char, usize)> = None;
    let mut in_comment = false;
    let mut paragraph: Option<String> = None;
    let mut block = Block::default();
    let mut indented = Indented::default();
    let lines: Vec<&str> = text.lines().collect();
    for (index, raw) in lines.iter().enumerate().skip(front_matter_end(&lines)) {
        let number = index + 1;
        if let Some(open) = fence {
            if closes_fence(raw, open) {
                fence = None;
            }
            continue;
        }
        if indented.is_code(raw, block.chars.is_empty()) {
            continue;
        }
        if let Some(open) = opens_fence(raw) {
            block.flush(&mut scan);
            fence = Some(open);
            paragraph = None;
            continue;
        }
        let line = without_comments(raw, &mut in_comment);
        if let Some(heading) = atx_heading(&line) {
            block.flush(&mut scan);
            scan.anchors.insert(slugs.next(heading));
            let mut own = Block::default();
            own.push(&line, number);
            own.flush(&mut scan);
        } else if let Some(target) = definition(&line) {
            block.flush(&mut scan);
            scan.references.push(Reference {
                line: number,
                target,
                form: Form::Link,
            });
        } else if line.trim().is_empty() {
            block.flush(&mut scan);
        } else {
            if let Some(text) = paragraph.as_deref().filter(|_| is_setext_underline(&line)) {
                scan.anchors.insert(slugs.next(text));
            }
            block.push(&line, number);
        }
        paragraph = is_paragraph(&line).then_some(line);
    }
    block.flush(&mut scan);
    scan
}

/// Indented code blocks: lines four columns in that do not go on with a
/// paragraph or a list.
#[derive(Default)]
struct Indented {
    in_code: bool,
    in_list: bool,
}

impl Indented {
    /// Whether `line` is code; `after_break` tells that no paragraph is open.
    fn is_code(&mut self, line: &str, after_break: bool) -> bool {
        let blank = line.trim().is_empty();
        let indent: usize = line
            .chars()
            .take_while(|c| c.is_whitespace())
            .map(|c| if c == '\t' { 4 } else { 1 })
            .sum();
        if self.in_code && (blank || indent >= 4) {
            return true;
        }
        self.in_code = !blank && indent >= 4 && after_break && !self.in_list;
        if !blank && indent < 4 {
            let list = is_list_item(line.trim_start());
            self.in_list = list || (self.in_list && indent > 0);
        }
        self.in_code
    }
}

fn is_list_item(rest: &str) -> bool {
    let bullet =
        rest.len() > 1 && rest.starts_with(['-', '*', '+']) && rest[1..].starts_with([' ', '\t']);
    let numbered = rest.split_once(['.', ')']).is_some_and(|(n, _)| {
        !n.is_empty() && n.len() <= 9 && n.chars().all(|c| c.is_ascii_digit())
    });
    bullet || numbered
}

/// The lines of a paragraph read as one text, so that code spans and link
/// texts may go on from one line to the next; each character keeps its
/// line.
#[derive(Default)]
struct Block {
    chars: Vec<char>,
    lines: Vec<usize>,
}

impl Block {
    fn push(&mut self, line: &str, number: usize) {
        for c in line.chars().chain(['\n']) {
            self.chars.push(c);
            self.lines.push(number);
        }
    }

    fn flush(&mut self, scan: &mut Scan) {
        inline(&self.chars, &self.lines, scan);
        self.chars.clear();
        self.lines.clear();
    }
}

/// The lines of a YAML front matter, without its fences.
pub fn front_matter(text: &str) -> Vec<&str> {
    let lines: Vec<&str> = text.lines().collect();
    match front_matter_end(&lines) {
        0 => Vec::new(),
        end => lines[1..end - 1].to_vec(),
    }
}

/// The line where the body starts, past a YAML front matter.
fn front_matter_end(lines: &[&str]) -> usize {
    if lines.first().map(|l| l.trim_end()) != Some("---") {
        return 0;
    }
    lines
        .iter()
        .skip(1)
        .position(|l| matches!(l.trim_end(), "---" | "..."))
        .map_or(0, |end| end + 2)
}

fn indent_ok(line: &str) -> Option<&str> {
    let trimmed = line.trim_start_matches(' ');
    (line.len() - trimmed.len() <= 3).then_some(trimmed)
}

fn opens_fence(line: &str) -> Option<(char, usize)> {
    let rest = indent_ok(line)?;
    let mark = rest.chars().next().filter(|c| *c == '`' || *c == '~')?;
    let run = rest.chars().take_while(|c| *c == mark).count();
    let info = &rest[run..];
    (run >= 3 && !(mark == '`' && info.contains('`'))).then_some((mark, run))
}

fn closes_fence(line: &str, (mark, run): (char, usize)) -> bool {
    indent_ok(line).is_some_and(|rest| {
        let count = rest.chars().take_while(|c| *c == mark).count();
        count >= run && rest[count..].trim().is_empty()
    })
}

/// `line` without what HTML comments hold, carrying an open comment over.
fn without_comments(line: &str, in_comment: &mut bool) -> String {
    let mut kept = String::new();
    let mut rest = line;
    loop {
        if *in_comment {
            match rest.find("-->") {
                Some(end) => {
                    rest = &rest[end + 3..];
                    *in_comment = false;
                }
                None => return kept,
            }
        } else {
            match rest.find("<!--") {
                Some(start) => {
                    kept.push_str(&rest[..start]);
                    rest = &rest[start + 4..];
                    *in_comment = true;
                }
                None => {
                    kept.push_str(rest);
                    return kept;
                }
            }
        }
    }
}

fn atx_heading(line: &str) -> Option<&str> {
    let rest = indent_ok(line)?;
    let level = rest.chars().take_while(|c| *c == '#').count();
    let text = &rest[level..];
    if !(1..=6).contains(&level) || !(text.is_empty() || text.starts_with([' ', '\t'])) {
        return None;
    }
    let text = text.trim();
    let closed = text.trim_end_matches('#');
    Some(if closed.is_empty() || closed.ends_with([' ', '\t']) {
        closed.trim_end()
    } else {
        text
    })
}

fn is_setext_underline(line: &str) -> bool {
    indent_ok(line).is_some_and(|rest| {
        let rest = rest.trim_end();
        !rest.is_empty() && (rest.chars().all(|c| c == '=') || rest.chars().all(|c| c == '-'))
    })
}

/// Whether a line may be the text of a setext heading.
fn is_paragraph(line: &str) -> bool {
    let Some(rest) = indent_ok(line) else {
        return false;
    };
    let other = is_list_item(rest) || rest.starts_with(['-', '*', '+', '>', '|']);
    !rest.trim().is_empty() && !other && atx_heading(line).is_none()
}

/// The target of a reference definition: `[label]: target`.
fn definition(line: &str) -> Option<String> {
    let rest = indent_ok(line)?.strip_prefix('[')?;
    if rest.starts_with('^') {
        return None;
    }
    let (label, after) = rest.split_once("]:")?;
    if label.is_empty() || label.contains(']') {
        return None;
    }
    let target = after.split_whitespace().next()?;
    Some(
        target
            .trim_start_matches('<')
            .trim_end_matches('>')
            .to_string(),
    )
}

/// Links, images, code spans and HTML elements of a run of text, whose
/// characters are on `lines`.
fn inline(chars: &[char], lines: &[usize], scan: &mut Scan) {
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '\\' => i += 2,
            '`' => {
                let (content, end) = code_span(chars, i);
                if let Some(content) = content.filter(|c| !c.is_empty()) {
                    scan.references.push(Reference {
                        line: lines[i],
                        target: content,
                        form: Form::Code,
                    });
                }
                i = end;
            }
            '[' if chars.get(i + 1) == Some(&'[') => i = wikilink(chars, lines, i, scan),
            '[' => i = link(chars, lines, i, scan),
            '<' => i = html_tag(chars, lines, i, scan),
            _ => i += 1,
        }
    }
}

/// The code span opened at `start` and where scanning goes on: its content,
/// trimmed, or none when the backticks are never closed.
fn code_span(chars: &[char], start: usize) -> (Option<String>, usize) {
    let run = chars[start..].iter().take_while(|c| **c == '`').count();
    let mut i = start + run;
    while i < chars.len() {
        if chars[i] == '`' {
            let closing = chars[i..].iter().take_while(|c| **c == '`').count();
            if closing == run {
                let content: String = chars[start + run..i]
                    .iter()
                    .map(|c| if *c == '\n' { ' ' } else { *c })
                    .collect();
                return (Some(content.trim().to_string()), i + run);
            }
            i += closing;
        } else {
            i += 1;
        }
    }
    (None, start + run)
}

/// The wikilink opened by the `[[` at `start`; returns where scanning goes
/// on. Its target ends at `|`, before the text shown.
fn wikilink(chars: &[char], lines: &[usize], start: usize, scan: &mut Scan) -> usize {
    let inner = &chars[start + 2..];
    let Some(len) = inner
        .windows(2)
        .position(|w| w == [']', ']'])
        .filter(|len| !inner[..*len].contains(&'\n'))
    else {
        return start + 2;
    };
    let text: String = inner[..len].iter().collect();
    let target = text.split('|').next().unwrap_or("").trim().to_string();
    if !target.is_empty() {
        scan.references.push(Reference {
            line: lines[start],
            target,
            form: Form::Note,
        });
    }
    start + 2 + len + 2
}

/// The link or image opened by the `[` at `start`, recorded with what its
/// text refers to; returns where scanning goes on.
fn link(chars: &[char], lines: &[usize], start: usize, scan: &mut Scan) -> usize {
    let Some(close) = closing_bracket(chars, start) else {
        return start + 1;
    };
    if chars.get(close + 1) != Some(&'(') {
        return start + 1;
    }
    let Some((target, end)) = destination(chars, close + 2) else {
        return start + 1;
    };
    let form = if start > 0 && chars[start - 1] == '!' {
        Form::Image
    } else {
        Form::Link
    };
    inline(&chars[start + 1..close], &lines[start + 1..close], scan);
    if !target.is_empty() {
        scan.references.push(Reference {
            line: lines[start],
            target,
            form,
        });
    }
    end
}

/// The `]` that closes the `[` at `start`, past nested brackets and code.
fn closing_bracket(chars: &[char], start: usize) -> Option<usize> {
    let mut depth = 0;
    let mut i = start;
    while let Some(&c) = chars.get(i) {
        i = match c {
            '`' => code_span(chars, i).1,
            '\\' => i + 2,
            ']' if depth == 1 => return Some(i),
            '[' | ']' => {
                depth += if c == '[' { 1 } else { -1 };
                i + 1
            }
            _ => i + 1,
        };
    }
    None
}

/// The destination of a link whose `(` ends before `start`, and where the
/// link ends.
fn destination(chars: &[char], start: usize) -> Option<(String, usize)> {
    let mut i = start;
    while chars.get(i).is_some_and(|c| c.is_whitespace()) {
        i += 1;
    }
    let mut target = String::new();
    if chars.get(i) == Some(&'<') {
        i += 1;
        while let Some(&c) = chars.get(i).filter(|c| **c != '>') {
            target.push(c);
            i += 1;
        }
        i += 1;
    } else {
        let mut depth = 0;
        while let Some(&c) = chars.get(i) {
            if c.is_whitespace() || (c == ')' && depth == 0) {
                break;
            }
            depth += match c {
                '(' => 1,
                ')' => -1,
                _ => 0,
            };
            target.push(c);
            i += 1;
        }
    }
    let mut quote = None;
    let end = chars.get(i..)?.iter().position(|&c| {
        match (quote, c) {
            (None, ')') => return true,
            (None, '"' | '\'') => quote = Some(c),
            (Some(q), c) if c == q => quote = None,
            _ => {}
        }
        false
    })?;
    Some((target, i + end + 1))
}

/// The HTML element opened at `start`: its `href` and `src` are references,
/// its `id` and `name` anchors. Returns where scanning goes on.
fn html_tag(chars: &[char], lines: &[usize], start: usize, scan: &mut Scan) -> usize {
    let line = lines[start];
    let name_len = chars[start + 1..]
        .iter()
        .take_while(|c| c.is_ascii_alphanumeric())
        .count();
    let after_name = chars.get(start + 1 + name_len);
    if name_len == 0 || !after_name.is_some_and(|c| c.is_whitespace() || matches!(c, '>' | '/')) {
        return start + 1;
    }
    let Some(len) = chars[start..].iter().position(|c| *c == '>') else {
        return start + 1;
    };
    let tag: String = chars[start..start + len].iter().collect();
    for (attribute, value) in attributes(&tag) {
        match attribute.as_str() {
            "href" => scan.references.push(Reference {
                line,
                target: value,
                form: Form::Link,
            }),
            "src" => scan.references.push(Reference {
                line,
                target: value,
                form: Form::Image,
            }),
            "id" | "name" => {
                scan.anchors.insert(value);
            }
            _ => {}
        }
    }
    start + len + 1
}

/// The quoted attributes of a tag, names in lower case.
fn attributes(tag: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut rest = tag;
    while let Some(eq) = rest.find('=') {
        let name: String = rest[..eq]
            .trim_end()
            .rsplit(|c: char| c.is_whitespace())
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        let after = rest[eq + 1..].trim_start();
        let Some(quote) = after.chars().next().filter(|c| *c == '"' || *c == '\'') else {
            rest = &rest[eq + 1..];
            continue;
        };
        let Some(len) = after[1..].find(quote) else {
            break;
        };
        found.push((name, after[1..1 + len].to_string()));
        rest = &after[len + 2..];
    }
    found
}

/// Anchors of headings as GitHub gives them: a repeated heading takes
/// `-1`, `-2` and so on.
#[derive(Default)]
struct Slugs {
    seen: HashMap<String, usize>,
}

impl Slugs {
    fn next(&mut self, heading: &str) -> String {
        let base = slug(heading);
        let count = self.seen.entry(base.clone()).or_insert(0);
        let anchor = if *count == 0 {
            base
        } else {
            format!("{base}-{count}")
        };
        *count += 1;
        anchor
    }
}

/// The anchor of a heading: its text as shown, in lower case, without
/// punctuation but `-` and `_`, spaces as `-`.
pub fn slug(heading: &str) -> String {
    plain(heading)
        .trim()
        .to_lowercase()
        .chars()
        .filter_map(|c| match c {
            ' ' => Some('-'),
            c if c.is_alphanumeric() || c == '-' || c == '_' => Some(c),
            _ => None,
        })
        .collect()
}

/// The text of a heading as shown: links as their text, without HTML tags.
fn plain(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut shown = String::new();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '[' => {
                let target = closing_bracket(&chars, i)
                    .filter(|close| chars.get(close + 1) == Some(&'('))
                    .and_then(|close| destination(&chars, close + 2).map(|(_, end)| (close, end)));
                if let Some((close, end)) = target {
                    shown.push_str(&plain(&chars[i + 1..close].iter().collect::<String>()));
                    i = end;
                    continue;
                }
                shown.push('[');
            }
            '<' => {
                if let Some(len) = chars[i..].iter().position(|c| *c == '>') {
                    i += len + 1;
                    continue;
                }
                shown.push('<');
            }
            c => shown.push(c),
        }
        i += 1;
    }
    shown
}

#[cfg(test)]
#[path = "markdown_test.rs"]
mod tests;
