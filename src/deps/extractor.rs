/// Language-specific import extraction for dependency graph analysis.
///
/// Each extractor returns raw import strings that the resolver will map to
/// project-relative file paths. Only potentially-internal references are
/// returned: relative imports (JS/TS/Python) and module declarations (Rust).
/// Go and Kaikai imports are returned verbatim for the resolver to filter:
/// by module path (Go) or by what exists in the project (Kaikai).
use std::path::Path;

use super::kaikai;

/// Whether imports can be extracted and resolved for this language.
/// Files in any other language have no measured dependencies.
pub fn is_supported(language: &str) -> bool {
    matches!(
        language,
        "Rust"
            | "Python"
            | "JavaScript"
            | "TypeScript"
            | "JSX"
            | "TSX"
            | "Go"
            | "Kaikai"
            | "Elixir"
            | "Elixir Script"
    )
}

/// Extract raw import references from a source file.
/// Returns strings that the resolver will attempt to map to project files.
pub fn extract_imports(path: &Path, language: &str, source: &str) -> Vec<String> {
    match language {
        "Rust" => extract_rust(path, source),
        "Python" => extract_python(source),
        "JavaScript" | "TypeScript" | "JSX" | "TSX" => extract_js(source),
        "Go" => extract_go(source),
        "Kaikai" => kaikai::extract(source),
        _ => vec![],
    }
}

/// Rust: extract `mod foo;` declarations (external `mod foo {}` inline are skipped).
/// The file path is used to compute the correct relative base for resolution.
fn extract_rust(_path: &Path, source: &str) -> Vec<String> {
    let mut imports = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        // Skip inline modules (contain `{`) and non-semicolon-terminated lines.
        if !trimmed.ends_with(';') || trimmed.contains('{') {
            continue;
        }
        if let Some(rest) = strip_visibility(trimmed).strip_prefix("mod ") {
            let name = rest.trim_end_matches(';').trim();
            if !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                imports.push(name.to_string());
            }
        }
    }
    imports
}

/// Drop a leading visibility qualifier: `pub`, `pub(crate)`, `pub(in path)`.
fn strip_visibility(decl: &str) -> &str {
    let Some(rest) = decl.strip_prefix("pub") else {
        return decl;
    };
    let rest = match rest.strip_prefix('(') {
        Some(scoped) => scoped.split_once(')').map_or(rest, |(_, after)| after),
        None => rest,
    };
    if rest.starts_with(char::is_whitespace) {
        rest.trim_start()
    } else {
        decl
    }
}

/// Python: extract relative imports (`from .foo import bar`, `from . import bar`).
/// Absolute imports are skipped — they may be external packages.
fn extract_python(source: &str) -> Vec<String> {
    let mut imports = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(module) = trimmed
            .strip_prefix("from ")
            .and_then(|r| r.split_whitespace().next())
        {
            // Only relative imports start with `.`
            if module.starts_with('.') && module != "." {
                imports.push(module.to_string());
            }
        }
    }
    imports
}

/// JavaScript/TypeScript: extract relative import/require paths (`./foo`, `../bar`).
/// Absolute module specifiers (bare `foo`, `@scope/pkg`) are external — skipped.
fn extract_js(source: &str) -> Vec<String> {
    let mut imports = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        // Skip comment lines
        if trimmed.starts_with("//") || trimmed.starts_with("*") || trimmed.starts_with("/*") {
            continue;
        }
        if let Some(path) = find_relative_string(trimmed) {
            imports.push(path);
        }
    }
    imports
}

/// Find the first relative path string literal (`'./…'`, `"../…"`) on a line.
fn find_relative_string(line: &str) -> Option<String> {
    for &q in &['"', '\'', '`'] {
        let mut search = line;
        while let Some(start) = search.find(q) {
            let inner = &search[start + 1..];
            if (inner.starts_with("./") || inner.starts_with("../"))
                && let Some(end) = inner.find(q)
            {
                return Some(inner[..end].to_string());
            }
            // Advance past this quote and keep looking
            search = &search[start + 1..];
            if search.is_empty() {
                break;
            }
        }
    }
    None
}

/// Go: extract all quoted import paths (both single-line and block imports).
/// The resolver filters to project-internal paths using the go.mod module name.
fn extract_go(source: &str) -> Vec<String> {
    let mut imports = Vec::new();
    let mut in_block = false;

    for line in source.lines() {
        let trimmed = line.trim();

        if trimmed == "import (" {
            in_block = true;
            continue;
        }
        if in_block && trimmed == ")" {
            in_block = false;
            continue;
        }

        let candidate = if in_block {
            Some(trimmed)
        } else {
            trimmed.strip_prefix("import ").map(str::trim)
        };

        if let Some(s) = candidate {
            // Handle optional alias: `alias "path"` or just `"path"`
            if let Some(path) = extract_quoted(s).filter(|p| !p.is_empty()) {
                imports.push(path);
            }
        }
    }
    imports
}

/// Extract the content of the last `"…"` on a line (handles alias prefix).
fn extract_quoted(s: &str) -> Option<String> {
    let start = s.rfind('"')?;
    let before = &s[..start];
    let start2 = before.rfind('"')? + 1;
    Some(before[start2..].to_string())
}

#[cfg(test)]
#[path = "extractor_test.rs"]
mod tests;
