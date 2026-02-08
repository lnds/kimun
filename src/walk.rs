use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use ignore::WalkBuilder;

use crate::loc::language::{LanguageSpec, detect_by_shebang};

/// Test directory names to exclude when `--exclude-tests` is active.
pub const TEST_DIRS: &[&str] = &["tests", "test", "__tests__", "spec"];

/// Check whether a file matches a test naming pattern based on its extension.
pub fn is_test_file(path: &Path) -> bool {
    let file_name = match path.file_name().and_then(|n| n.to_str()) {
        Some(n) => n,
        None => return false,
    };

    let Some(dot) = file_name.rfind('.') else {
        return false;
    };
    let ext = &file_name[dot + 1..];
    let base = &file_name[..dot];

    match ext {
        // suffix _test: Rust, Go, Python, Ruby, PHP, Elixir, Dart
        "rs" | "go" | "exs" | "dart" => base.ends_with("_test"),
        "py" => base.starts_with("test_") || base.ends_with("_test"),
        "rb" => base.ends_with("_test") || base.ends_with("_spec"),
        "php" => base.ends_with("Test") || base.ends_with("_test"),
        // double-ext .test./.spec.: JS/TS family
        "js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx" | "mts" | "cts" => {
            base.ends_with(".test") || base.ends_with(".spec")
        }
        // PascalCase suffixes: Java, Kotlin, C#, Swift, Scala
        "java" | "kt" | "kts" => base.ends_with("Test") || base.ends_with("Tests"),
        "cs" => base.ends_with("Test") || base.ends_with("Tests"),
        "swift" => base.ends_with("Test") || base.ends_with("Tests"),
        "scala" => base.ends_with("Test") || base.ends_with("Spec"),
        // C/C++
        "c" => base.ends_with("_test") || base.starts_with("test_") || base.ends_with("_unittest"),
        "cc" | "cpp" | "cxx" => {
            base.ends_with("_test")
                || base.starts_with("test_")
                || base.ends_with("_unittest")
                || base.ends_with("Test")
        }
        // Haskell
        "hs" => base.ends_with("Test") || base.ends_with("Spec"),
        _ => false,
    }
}

/// Try to detect a language by reading the shebang line of a file.
pub fn try_detect_shebang(path: &Path) -> Option<&'static LanguageSpec> {
    let file = File::open(path).ok()?;
    let mut reader = BufReader::new(file);
    let mut first_line = String::new();
    reader.read_line(&mut first_line).ok()?;
    detect_by_shebang(&first_line)
}

/// Build a directory walker that respects `.gitignore`, skips `.git`,
/// and optionally excludes test directories.
pub fn walk(path: &Path, exclude_tests: bool) -> ignore::Walk {
    WalkBuilder::new(path)
        .hidden(false)
        .follow_links(false)
        .filter_entry(move |entry| {
            if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                if entry.file_name() == ".git" {
                    return false;
                }
                if exclude_tests
                    && let Some(name) = entry.file_name().to_str()
                    && TEST_DIRS.contains(&name)
                {
                    return false;
                }
            }
            true
        })
        .build()
}
