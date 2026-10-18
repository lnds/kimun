//! Filesystem walking with `.gitignore` support and test exclusion.
//!
//! Provides directory traversal that respects `.gitignore` rules, skips
//! `.git` directories, filters test directories/files when requested,
//! and detects source file languages by extension or shebang line.
//! Uses the `ignore` crate for efficient `.gitignore`-aware traversal.
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::WalkBuilder;

use crate::loc::language::{LanguageSpec, detect, detect_by_shebang};
use crate::test_blocks::rust_test_modules;
use crate::util::is_generated_file;

/// Filter that excludes files by extension, directory name, or glob pattern.
///
/// Built from the `--exclude-ext`, `--exclude-dir`, and `--exclude` CLI flags
/// and passed through every analysis pipeline. An empty filter (via `Default`)
/// is a no-op and imposes no overhead on the walk or file-matching paths.
#[derive(Clone, Debug, Default)]
pub struct ExcludeFilter {
    /// When set, only files with these extensions pass (allowlist mode).
    /// Stored lowercase without leading dot. Mutually exclusive with `extensions`.
    include_extensions: Option<HashSet<Box<str>>>,
    /// Lowercase extensions to exclude (without leading dot).
    extensions: HashSet<Box<str>>,
    /// Directory names to exclude (exact, case-sensitive match).
    dirs: HashSet<Box<str>>,
    /// Compiled glob patterns for file path matching.
    globs: Option<GlobSet>,
}

impl ExcludeFilter {
    /// Build a filter from extension, directory, and glob pattern slices.
    ///
    /// Extensions are normalized by stripping any leading dot and lowercasing,
    /// so `"JS"`, `".js"`, and `"js"` all match `.js` files.
    /// Directory names are stored as-is and matched case-sensitively.
    /// Glob patterns use standard glob syntax (e.g. `"*.min.js"`, `"vendor/**"`).
    /// Invalid glob patterns are reported to stderr and skipped.
    pub fn new(
        include_extensions: &[String],
        extensions: &[String],
        dirs: &[String],
        globs: &[String],
    ) -> Self {
        let normalize_exts = |exts: &[String]| -> HashSet<Box<str>> {
            exts.iter()
                .map(|e| e.trim_start_matches('.').to_lowercase().into_boxed_str())
                .filter(|e| !e.is_empty())
                .collect()
        };
        let include_extensions = if include_extensions.is_empty() {
            None
        } else {
            Some(normalize_exts(include_extensions))
        };
        let extensions = normalize_exts(extensions);
        let dirs = dirs.iter().map(|d| d.clone().into_boxed_str()).collect();
        let globs = Self::build_glob_set(globs);
        Self {
            include_extensions,
            extensions,
            dirs,
            globs,
        }
    }

    /// Returns `true` if no filters have been configured.
    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.include_extensions.is_none()
            && self.extensions.is_empty()
            && self.dirs.is_empty()
            && self.globs.is_none()
    }

    /// Compile glob patterns into a `GlobSet`, skipping invalid ones.
    fn build_glob_set(patterns: &[String]) -> Option<GlobSet> {
        if patterns.is_empty() {
            return None;
        }
        let mut builder = GlobSetBuilder::new();
        for pattern in patterns {
            match Glob::new(pattern) {
                Ok(g) => {
                    builder.add(g);
                }
                Err(e) => eprintln!("warning: invalid glob '{pattern}': {e}"),
            }
        }
        match builder.build() {
            Ok(set) => Some(set),
            Err(e) => {
                eprintln!("warning: failed to compile glob set: {e}");
                None
            }
        }
    }

    /// Returns `true` if a directory with this name should be excluded.
    pub fn excludes_dir(&self, name: &str) -> bool {
        self.dirs.contains(name)
    }

    /// Returns `true` if a file should be excluded by extension or glob pattern.
    ///
    /// Pass `walk_root` to ensure glob patterns match against relative paths
    /// regardless of whether the user supplied an absolute or relative analysis path.
    pub fn excludes_file(&self, path: &Path, walk_root: &Path) -> bool {
        self.excludes_by_extension(path) || self.excludes_by_glob(path, walk_root)
    }

    /// Check whether the file's extension matches any excluded extension,
    /// or fails to match an include-only list when one is configured.
    /// Uses case-insensitive comparison without allocating.
    fn excludes_by_extension(&self, path: &Path) -> bool {
        let ext = match path.extension().and_then(|e| e.to_str()) {
            Some(e) => e,
            None => {
                // Files without an extension: excluded if include list is set
                // (they can't match any included extension), otherwise kept.
                return self.include_extensions.is_some();
            }
        };
        // Include list: only matching extensions pass.
        if let Some(ref include) = self.include_extensions {
            return !include.iter().any(|inc| inc.eq_ignore_ascii_case(ext));
        }
        // Exclude list: matching extensions are excluded.
        !self.extensions.is_empty()
            && self
                .extensions
                .iter()
                .any(|excl| excl.eq_ignore_ascii_case(ext))
    }

    /// Check whether the file path matches any glob pattern.
    /// Normalises to a path relative to the walk root so that globs like
    /// `vendor/**` work regardless of whether the input was absolute.
    fn excludes_by_glob(&self, path: &Path, walk_root: &Path) -> bool {
        self.globs.as_ref().is_some_and(|g| {
            let relative = path.strip_prefix(walk_root).unwrap_or(path);
            g.is_match(relative)
        })
    }
}

/// Configuration for directory walking and file filtering.
///
/// Bundles the analysis root path, test inclusion flag, and exclude filter
/// into a single struct passed to all analysis modules. This eliminates
/// the need to thread three separate parameters through every `run()` function.
#[derive(Debug)]
pub struct WalkConfig<'a> {
    /// Root directory to analyze.
    pub path: &'a Path,
    /// Whether to include test files and directories.
    pub include_tests: bool,
    /// File and directory exclusion filter.
    pub filter: &'a ExcludeFilter,
}

impl<'a> WalkConfig<'a> {
    pub fn new(path: &'a Path, include_tests: bool, filter: &'a ExcludeFilter) -> Self {
        Self {
            path,
            include_tests,
            filter,
        }
    }

    /// Whether test files/directories should be excluded (inverse of `include_tests`).
    pub fn exclude_tests(&self) -> bool {
        !self.include_tests
    }

    /// Walk source files using this config.
    pub fn source_files(&self) -> Vec<(PathBuf, &'static LanguageSpec)> {
        source_files(self.path, self.exclude_tests(), self.filter)
    }

    /// Walk the source files that metrics of code read (see [`code_files`]).
    pub fn code_files(&self) -> Vec<(PathBuf, &'static LanguageSpec)> {
        code_files(self.path, self.exclude_tests(), self.filter)
    }

    /// Walk and analyze source files using this config. `f` is also told
    /// whether tests are left out, for those a file keeps inside it.
    pub fn collect_analysis<T>(
        &self,
        f: impl Fn(&Path, &LanguageSpec, bool) -> Result<Option<T>, Box<dyn std::error::Error>>,
    ) -> Vec<T> {
        let exclude_tests = self.exclude_tests();
        collect_analysis(self.path, exclude_tests, self.filter, |path, spec| {
            f(path, spec, exclude_tests)
        })
    }
}

/// Test directory names to exclude when `--exclude-tests` is active.
pub const TEST_DIRS: &[&str] = &["tests", "test", "__tests__", "spec"];

/// Whether a directory holds tests: one of `TEST_DIRS`, or a .NET test
/// project, named after what it tests (`App.Tests`, `Orders.UnitTests`,
/// `Api.IntegrationTest`).
pub fn is_test_dir(name: &str) -> bool {
    let dotnet = name
        .rsplit_once('.')
        .is_some_and(|(_, last)| last.ends_with("Tests") || last.ends_with("Test"));
    TEST_DIRS.contains(&name) || dotnet
}

/// Directories of examples and benchmarks: code that is run to show or to
/// time the project, not shipped with it. Left out along with tests.
pub const EXAMPLE_DIRS: &[&str] = &[
    "examples",
    "example",
    "benches",
    "bench",
    "benchmarks",
    "benchsuite",
];

/// Whether a directory name holds tests, examples or benchmarks.
pub fn is_non_production_dir(name: &str) -> bool {
    is_test_dir(name) || EXAMPLE_DIRS.contains(&name)
}

/// Mapping from file extension to test-file naming patterns.
/// Used to identify and exclude test files based on naming conventions.
struct TestPattern {
    exts: &'static [&'static str],
    suffixes: &'static [&'static str],
    prefixes: &'static [&'static str],
}

/// Data-driven test file detection patterns, grouped by naming convention.
const TEST_PATTERNS: &[TestPattern] = &[
    // snake_case _test suffix: Rust, Go, Elixir, Dart
    TestPattern {
        exts: &["rs", "go", "exs", "dart"],
        suffixes: &["_test"],
        prefixes: &[],
    },
    // Python: test_ prefix or _test suffix
    TestPattern {
        exts: &["py"],
        suffixes: &["_test"],
        prefixes: &["test_"],
    },
    // Ruby: _test or _spec suffix
    TestPattern {
        exts: &["rb"],
        suffixes: &["_test", "_spec"],
        prefixes: &[],
    },
    // PHP: PascalCase Test or snake_case _test
    TestPattern {
        exts: &["php"],
        suffixes: &["Test", "_test"],
        prefixes: &[],
    },
    // JS/TS family: .test. or .spec. double extension
    TestPattern {
        exts: &["js", "jsx", "mjs", "cjs", "ts", "tsx", "mts", "cts"],
        suffixes: &[".test", ".spec"],
        prefixes: &[],
    },
    // JVM + C# + Swift: PascalCase Test/Tests suffix
    TestPattern {
        exts: &["java", "kt", "kts", "cs", "swift"],
        suffixes: &["Test", "Tests"],
        prefixes: &[],
    },
    // Scala: PascalCase Test or Spec suffix
    TestPattern {
        exts: &["scala"],
        suffixes: &["Test", "Spec"],
        prefixes: &[],
    },
    // C: snake_case only (no PascalCase Test)
    TestPattern {
        exts: &["c"],
        suffixes: &["_test", "_unittest"],
        prefixes: &["test_"],
    },
    // C++: snake_case + PascalCase Test
    TestPattern {
        exts: &["cc", "cpp", "cxx"],
        suffixes: &["_test", "_unittest", "Test"],
        prefixes: &["test_"],
    },
    // Haskell: PascalCase Test or Spec
    TestPattern {
        exts: &["hs"],
        suffixes: &["Test", "Spec"],
        prefixes: &[],
    },
];

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

    for pattern in TEST_PATTERNS {
        if !pattern.exts.contains(&ext) {
            continue;
        }
        if pattern.suffixes.iter().any(|s| base.ends_with(s)) {
            return true;
        }
        if pattern.prefixes.iter().any(|p| base.starts_with(p)) {
            return true;
        }
    }
    false
}

/// Try to detect a language by reading the shebang line of a file.
pub fn try_detect_shebang(path: &Path) -> Option<&'static LanguageSpec> {
    let file = File::open(path).ok()?;
    let mut reader = BufReader::new(file);
    let mut first_line = String::new();
    reader.read_line(&mut first_line).ok()?;
    detect_by_shebang(&first_line)
}

/// Walk the directory tree and return all recognized source files with their
/// detected language spec. Handles errors, filters non-files, excludes test
/// files when requested, applies the `ExcludeFilter`, and detects language by
/// extension or shebang.
pub fn source_files(
    path: &Path,
    exclude_tests: bool,
    filter: &ExcludeFilter,
) -> Vec<(PathBuf, &'static LanguageSpec)> {
    let mut result = Vec::new();
    for entry in walk(path, exclude_tests, filter) {
        let entry = match entry {
            Ok(e) => e,
            Err(err) => {
                eprintln!("warning: {err}");
                continue;
            }
        };
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }
        let file_path = entry.path();
        // File-level filtering (test names, extension/glob exclusion) is
        // handled by the walker's filter_entry callback — no checks needed here.
        let Some(spec) = detect(file_path).or_else(|| try_detect_shebang(file_path)) else {
            continue;
        };
        result.push((file_path.to_path_buf(), spec));
    }
    if exclude_tests {
        without_test_modules(&mut result);
    }
    result
}

/// `files` without the Rust modules declared under `#[cfg(test)]`.
fn without_test_modules(files: &mut Vec<(PathBuf, &'static LanguageSpec)>) {
    let test_modules = rust_test_modules(files);
    if !test_modules.is_empty() {
        files.retain(|(path, _)| !in_test_module(path, &test_modules));
    }
}

/// Whether a file is one of the Rust modules declared under `#[cfg(test)]`
/// or sits in the directory of one, where its submodules live.
fn in_test_module(path: &Path, test_modules: &HashSet<PathBuf>) -> bool {
    test_modules.iter().any(|module| {
        let dir = if module.file_name().is_some_and(|name| name == "mod.rs") {
            module.parent().unwrap_or(Path::new("")).to_path_buf()
        } else {
            module.with_extension("")
        };
        path == module || path.starts_with(dir)
    })
}

/// Whether a path, which may no longer exist, is code by its name alone:
/// not data or prose and not generated. For the paths of git history.
pub fn is_code_path(path: &Path) -> bool {
    !crate::util::is_generated(path) && detect(path).is_none_or(LanguageSpec::is_code)
}

/// The source files that metrics of code read: [`source_files`] without
/// data and prose ([`LanguageSpec::is_code`]) or generated files
/// ([`is_generated_file`]). Line counts and the dependency graph read
/// every source file instead.
pub fn code_files(
    path: &Path,
    exclude_tests: bool,
    filter: &ExcludeFilter,
) -> Vec<(PathBuf, &'static LanguageSpec)> {
    let mut files = source_files(path, exclude_tests, filter);
    files.retain(|(path, spec)| spec.is_code() && !is_generated_file(path));
    files
}

/// Walk the code files ([`code_files`]), analyze each with `f`, and collect
/// successful results. Handles the common Ok(Some)/Ok(None)/Err pattern
/// used across modules.
pub fn collect_analysis<T>(
    path: &Path,
    exclude_tests: bool,
    filter: &ExcludeFilter,
    f: impl Fn(&Path, &LanguageSpec) -> Result<Option<T>, Box<dyn std::error::Error>>,
) -> Vec<T> {
    let mut results = Vec::new();
    for (file_path, spec) in code_files(path, exclude_tests, filter) {
        match f(&file_path, spec) {
            Ok(Some(m)) => results.push(m),
            Ok(None) => {}
            Err(err) => {
                eprintln!("warning: {}: {err}", file_path.display());
            }
        }
    }
    results
}

/// Build a directory walker that respects `.gitignore`, skips `.git`,
/// and optionally excludes test directories, user-specified directories,
/// test-named files, and files matching extension/glob exclusion filters.
///
/// File-level filtering is done here (rather than in `source_files()`)
/// so that excluded files are never yielded, reducing downstream work.
pub(crate) fn walk(path: &Path, exclude_tests: bool, filter: &ExcludeFilter) -> ignore::Walk {
    let filter = filter.clone();
    let walk_root = path.to_path_buf();
    WalkBuilder::new(path)
        .hidden(false)
        .follow_links(false)
        .filter_entry(move |entry| keeps_entry(entry, &walk_root, exclude_tests, &filter))
        .build()
}

/// Whether the walk goes into a directory or yields a file.
fn keeps_entry(
    entry: &ignore::DirEntry,
    walk_root: &Path,
    exclude_tests: bool,
    filter: &ExcludeFilter,
) -> bool {
    let Some(ft) = entry.file_type() else {
        return true;
    };
    if ft.is_dir() {
        return keeps_dir(entry.file_name(), exclude_tests, filter);
    }
    let file_path = entry.path();
    !ft.is_file()
        || !((exclude_tests && is_test_file(file_path))
            || filter.excludes_file(file_path, walk_root))
}

/// Whether the walk goes into a directory of this name.
fn keeps_dir(name: &std::ffi::OsStr, exclude_tests: bool, filter: &ExcludeFilter) -> bool {
    if name == ".git" {
        return false;
    }
    let Some(name) = name.to_str() else {
        return true;
    };
    !filter.excludes_dir(name) && !(exclude_tests && is_non_production_dir(name))
}

/// Return the exclusion reason for an ancestor directory, if any.
/// Checks all path components for excluded directory names or test directories.
fn ancestor_exclusion_reason(
    path: &Path,
    exclude_tests: bool,
    filter: &ExcludeFilter,
) -> Option<&'static str> {
    let parent = path.parent()?;
    for component in parent.components() {
        if let std::path::Component::Normal(name) = component
            && let Some(name_str) = name.to_str()
        {
            if filter.excludes_dir(name_str) {
                return Some("directory");
            }
            if exclude_tests && is_test_dir(name_str) {
                return Some("test directory");
            }
            if exclude_tests && EXAMPLE_DIRS.contains(&name_str) {
                return Some("example or benchmark directory");
            }
        }
    }
    None
}

/// Print files that would be excluded by the current filter configuration.
/// Used by `--list-excluded` for debugging filter rules.
pub fn print_excluded_files(
    path: &Path,
    exclude_tests: bool,
    filter: &ExcludeFilter,
) -> Result<(), Box<dyn std::error::Error>> {
    // Walk with filter applied only for dirs; check files manually to report excluded ones
    let no_filter = ExcludeFilter::default();
    let test_modules = if exclude_tests {
        rust_test_modules(&source_files(path, false, &no_filter))
    } else {
        HashSet::new()
    };
    let mut count = 0;
    let files = walk(path, false, &no_filter)
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_some_and(|ft| ft.is_file()));
    for entry in files {
        let file_path = entry.path();
        let reasons = exclusion_reasons(file_path, path, exclude_tests, filter, &test_modules);
        if !reasons.is_empty() {
            let relative = file_path.strip_prefix(path).unwrap_or(file_path);
            println!("{} ({})", relative.display(), reasons.join(", "));
            count += 1;
        }
    }
    println!("\n{count} file(s) excluded");
    Ok(())
}

/// Why a file under `root` is left out, if it is.
fn exclusion_reasons(
    file_path: &Path,
    root: &Path,
    exclude_tests: bool,
    filter: &ExcludeFilter,
    test_modules: &HashSet<PathBuf>,
) -> Vec<&'static str> {
    let mut reasons = Vec::new();
    if exclude_tests && is_test_file(file_path) {
        reasons.push("test file");
    }
    if filter.excludes_by_extension(file_path) {
        reasons.push("extension");
    }
    if let Some(reason) = ancestor_exclusion_reason(file_path, exclude_tests, filter) {
        reasons.push(reason);
    }
    if filter.excludes_by_glob(file_path, root) {
        reasons.push("glob");
    }
    if in_test_module(file_path, test_modules) {
        reasons.push("test module");
    }
    reasons
}

#[cfg(test)]
#[path = "walk_test.rs"]
mod tests;
