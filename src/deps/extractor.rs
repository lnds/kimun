/// Language-specific import extraction for dependency graph analysis.
///
/// Each extractor returns raw import strings that the resolver will map to
/// project-relative file paths. Only potentially-internal references are
/// returned: relative imports (JS/TS). Go, Kaikai and Python imports are
/// returned for the resolver to filter: by module path (Go) or by what
/// exists in the project (Kaikai, Python). Elixir and Rust are read apart:
/// their dependencies are names to resolve over the whole graph.
use super::{golang_source, kaikai, python};

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
            | "Go"
            | "Kaikai"
            | "Elixir"
            | "Elixir Script"
            | "C"
            | "C++"
            | "Java"
            | "C#"
    )
}

/// Extract raw import references from a source file.
/// Returns strings that the resolver will attempt to map to project files.
pub fn extract_imports(language: &str, source: &str) -> Vec<String> {
    match language {
        "Python" => python::extract(source),
        "JavaScript" | "TypeScript" | "JSX" => super::javascript::extract(source),
        "Go" => golang_source::read(source).imports,
        "Kaikai" => kaikai::extract(source),
        _ => vec![],
    }
}

#[cfg(test)]
#[path = "extractor_test.rs"]
mod tests;
