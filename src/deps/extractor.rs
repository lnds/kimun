/// Language-specific import extraction for dependency graph analysis.
///
/// Each extractor returns raw import strings that the resolver will map to
/// project-relative file paths. Only potentially-internal references are
/// returned: relative imports (JS/TS/Python) and module declarations (Rust).
/// Go and Kaikai imports are returned verbatim for the resolver to filter:
/// by module path (Go) or by what exists in the project (Kaikai).
use std::path::Path;

use crate::loc::counter::{LineKind, classify_reader};
use crate::loc::language::languages;
use crate::string_mask::multi_line_string_mask;

/// Whether imports can be extracted and resolved for this language.
/// Files in any other language have no measured dependencies.
pub fn is_supported(language: &str) -> bool {
    matches!(
        language,
        "Rust" | "Python" | "JavaScript" | "TypeScript" | "JSX" | "TSX" | "Go" | "Kaikai"
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
        "Kaikai" => extract_kaikai(source),
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
        // Strip visibility qualifiers, then look for `mod <name>;`
        let bare = trimmed
            .trim_start_matches("pub(crate) ")
            .trim_start_matches("pub(super) ")
            .trim_start_matches("pub(in ")
            .trim_start_matches("pub ");
        if let Some(rest) = bare.strip_prefix("mod ") {
            let name = rest.trim_end_matches(';').trim();
            if !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                imports.push(name.to_string());
            }
        }
    }
    imports
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

/// Kaikai: extract the dotted module path of each `import` line
/// (`import a.b.c`, `import a.b as x`, `import a.b.{f, g}`).
///
/// The line classifier and the multi-line string mask are reused so that an
/// `import` written inside a `#[doc(...)]` block or a triple-quoted string is
/// not taken for a real one.
fn extract_kaikai(source: &str) -> Vec<String> {
    let Some(spec) = languages().iter().find(|s| s.name == "Kaikai") else {
        return vec![];
    };
    let normalized = source.replace("\r\n", "\n");
    let lines: Vec<String> = normalized.lines().map(String::from).collect();
    let kinds = classify_reader(normalized.as_bytes(), spec);
    let in_string = multi_line_string_mask(&lines, spec);

    lines
        .iter()
        .zip(kinds)
        .zip(in_string)
        .filter(|((_, kind), masked)| *kind == LineKind::Code && !masked)
        .filter_map(|((line, _), _)| kaikai_import_path(line))
        .map(str::to_string)
        .collect()
}

/// The module path of a Kaikai `import` line, without its alias or its
/// selective list: `import a.b.{f, g}` → `a.b`.
fn kaikai_import_path(line: &str) -> Option<&str> {
    let rest = line.trim_start().strip_prefix("import")?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let rest = rest.trim_start();
    let end = rest
        .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '.'))
        .unwrap_or(rest.len());
    // The selective form leaves the dot that precedes `{`.
    let path = rest[..end].trim_end_matches('.');
    (!path.is_empty() && path.split('.').all(|seg| !seg.is_empty())).then_some(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn rust_mod_declarations() {
        let src = r#"
pub mod analyzer;
mod report;
pub(crate) mod utils;
mod inline { fn foo() {} }  // should be skipped (has {)
// mod commented_out;
"#;
        let result = extract_rust(&PathBuf::from("src/lib.rs"), src);
        assert_eq!(result, vec!["analyzer", "report", "utils"]);
    }

    #[test]
    fn python_relative_imports() {
        let src =
            "from .foo import bar\nfrom . import baz\nfrom ..utils import helper\nimport os\n";
        let result = extract_python(src);
        assert_eq!(result, vec![".foo", "..utils"]);
    }

    #[test]
    fn js_relative_imports() {
        let src = r#"
import foo from './foo';
import { bar } from '../bar';
import external from 'lodash';
const x = require('./utils');
"#;
        let result = extract_js(src);
        assert_eq!(result, vec!["./foo", "../bar", "./utils"]);
    }

    #[test]
    fn go_block_import() {
        let src = r#"
import (
    "fmt"
    "github.com/user/project/pkg/foo"
    alias "github.com/user/project/internal/bar"
)
"#;
        let result = extract_go(src);
        assert_eq!(
            result,
            vec![
                "fmt",
                "github.com/user/project/pkg/foo",
                "github.com/user/project/internal/bar",
            ]
        );
    }

    #[test]
    fn kaikai_import_forms() {
        let src = "\
import compiler.ast
import loop
import core.list as list
import mathlib.{add, mul}
import app.util.text.{shout}

fn main() : Unit / Stdout = Stdout.print(\"hi\")
";
        assert_eq!(
            extract_kaikai(src),
            vec![
                "compiler.ast",
                "loop",
                "core.list",
                "mathlib",
                "app.util.text"
            ]
        );
    }

    #[test]
    fn kaikai_import_with_trailing_comment() {
        assert_eq!(
            extract_kaikai("import compiler.ast # the tree\n"),
            vec!["compiler.ast"]
        );
    }

    #[test]
    fn kaikai_import_in_comment_is_skipped() {
        let src = "# import compiler.ast\n  # import compiler.lexer\nimport compiler.parser\n";
        assert_eq!(extract_kaikai(src), vec!["compiler.parser"]);
    }

    #[test]
    fn kaikai_import_in_doc_block_is_skipped() {
        let src = "\
import compiler.ast

#[doc(\"\"\"
Usage:

import compiler.parser
\"\"\")]
pub fn parse() : Unit = ()
";
        assert_eq!(extract_kaikai(src), vec!["compiler.ast"]);
    }

    #[test]
    fn kaikai_import_in_multi_line_string_is_skipped() {
        let src = "\
import compiler.ast

fn sample() : String = \"\"\"
import compiler.parser
\"\"\"
";
        assert_eq!(extract_kaikai(src), vec!["compiler.ast"]);
    }

    #[test]
    fn kaikai_import_inside_single_line_string_is_skipped() {
        let src = "let s = \"import compiler.ast\"\n";
        assert!(extract_kaikai(src).is_empty());
    }

    #[test]
    fn kaikai_names_starting_with_import_are_not_imports() {
        let src = "importer.run()\nimport_all(xs)\nimport ?\n";
        assert!(extract_kaikai(src).is_empty());
    }

    #[test]
    fn kaikai_is_extracted_through_the_dispatcher() {
        let result = extract_imports(&PathBuf::from("app/main.kai"), "Kaikai", "import app.b\n");
        assert_eq!(result, vec!["app.b"]);
    }

    #[test]
    fn supported_languages() {
        for lang in ["Rust", "Python", "TypeScript", "TSX", "Go", "Kaikai"] {
            assert!(is_supported(lang), "{lang} should be supported");
        }
        for lang in ["Bash", "TOML", "Java"] {
            assert!(!is_supported(lang), "{lang} should not be supported");
        }
    }
}
