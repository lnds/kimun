/// Language-specific import extraction for dependency graph analysis.
///
/// Each extractor returns raw import strings that the resolver will map to
/// project-relative file paths. Only potentially-internal references are
/// returned: relative imports (JS/TS). Go, Kaikai and Python imports are
/// returned for the resolver to filter: by module path (Go) or by what
/// exists in the project (Kaikai, Python). Elixir and Rust are read apart:
/// their dependencies are names to resolve over the whole graph.
use super::{kaikai, python};

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
pub fn extract_imports(language: &str, source: &str) -> Vec<String> {
    match language {
        "Python" => python::extract(source),
        "JavaScript" | "TypeScript" | "JSX" | "TSX" => extract_js(source),
        "Go" => extract_go(source),
        "Kaikai" => kaikai::extract(source),
        _ => vec![],
    }
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
