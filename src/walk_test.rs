use std::fs;
use std::path::Path;

use tempfile::tempdir;

use super::*;

/// Shorthand for the empty walk root used in unit tests where
/// paths are bare filenames (not rooted in a real directory).
const ROOT: &str = "";

// ── ExcludeFilter::new ─────────────────────────────────────────────────

#[test]
fn exclude_filter_empty() {
    let f = ExcludeFilter::new(&[], &[], &[], &[]);
    assert!(!f.excludes_dir("vendor"));
    assert!(!f.excludes_file(Path::new("foo.rs"), Path::new(ROOT)));
    assert!(f.is_empty());
}

#[test]
fn exclude_filter_is_empty() {
    let f = ExcludeFilter::new(&[], &["js".to_string()], &[], &[]);
    assert!(!f.is_empty());
    let f = ExcludeFilter::new(&[], &[], &["vendor".to_string()], &[]);
    assert!(!f.is_empty());
    let f = ExcludeFilter::new(&[], &[], &[], &["*.js".to_string()]);
    assert!(!f.is_empty());
    let f = ExcludeFilter::new(&["rs".to_string()], &[], &[], &[]);
    assert!(!f.is_empty());
}

// ── Include-ext (allowlist mode) ─────────────────────────────────────

#[test]
fn include_ext_only_matching_pass() {
    let f = ExcludeFilter::new(&["rs".to_string()], &[], &[], &[]);
    assert!(
        !f.excludes_file(Path::new("main.rs"), Path::new(ROOT)),
        "rs files should pass"
    );
    assert!(
        f.excludes_file(Path::new("app.js"), Path::new(ROOT)),
        "js files should be excluded"
    );
    assert!(
        f.excludes_file(Path::new("style.css"), Path::new(ROOT)),
        "css files should be excluded"
    );
}

#[test]
fn include_ext_case_insensitive() {
    let f = ExcludeFilter::new(&["RS".to_string()], &[], &[], &[]);
    assert!(!f.excludes_file(Path::new("main.rs"), Path::new(ROOT)));
    assert!(!f.excludes_file(Path::new("lib.RS"), Path::new(ROOT)));
    assert!(f.excludes_file(Path::new("app.js"), Path::new(ROOT)));
}

#[test]
fn include_ext_multiple() {
    let f = ExcludeFilter::new(&["rs".to_string(), "toml".to_string()], &[], &[], &[]);
    assert!(!f.excludes_file(Path::new("main.rs"), Path::new(ROOT)));
    assert!(!f.excludes_file(Path::new("Cargo.toml"), Path::new(ROOT)));
    assert!(f.excludes_file(Path::new("app.js"), Path::new(ROOT)));
}

#[test]
fn include_ext_excludes_extensionless_files() {
    let f = ExcludeFilter::new(&["rs".to_string()], &[], &[], &[]);
    assert!(
        f.excludes_file(Path::new("Makefile"), Path::new(ROOT)),
        "files without extension excluded when include list is set"
    );
}

#[test]
fn include_ext_integration() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
    fs::write(dir.path().join("lib.js"), "export {};").unwrap();
    fs::write(dir.path().join("style.css"), "body {}").unwrap();

    let filter = ExcludeFilter::new(&["rs".to_string()], &[], &[], &[]);
    let files = source_files(dir.path(), false, &filter);

    assert_eq!(files.len(), 1, "only rs files should pass");
    assert_eq!(files[0].0.file_name().unwrap().to_str().unwrap(), "main.rs");
}

#[test]
fn exclude_filter_extension_normalises_dot_and_case() {
    // ".JS", "JS", "js", ".js" should all match foo.js
    for raw in [".JS", "JS", "js", ".js"] {
        let f = ExcludeFilter::new(&[], &[raw.to_string()], &[], &[]);
        assert!(
            f.excludes_file(Path::new("foo.js"), Path::new(ROOT)),
            "'{raw}' should exclude foo.js"
        );
        assert!(
            f.excludes_file(Path::new("bar.JS"), Path::new(ROOT)),
            "'{raw}' should exclude bar.JS (case-insensitive extension)"
        );
        assert!(
            !f.excludes_file(Path::new("foo.rs"), Path::new(ROOT)),
            "should not exclude foo.rs"
        );
    }
}

#[test]
fn exclude_filter_multiple_extensions() {
    let exts = vec!["js".to_string(), "ts".to_string()];
    let f = ExcludeFilter::new(&[], &exts, &[], &[]);
    assert!(f.excludes_file(Path::new("app.js"), Path::new(ROOT)));
    assert!(f.excludes_file(Path::new("app.ts"), Path::new(ROOT)));
    assert!(!f.excludes_file(Path::new("app.rs"), Path::new(ROOT)));
}

#[test]
fn exclude_filter_no_extension_file_not_excluded() {
    let f = ExcludeFilter::new(&[], &["rs".to_string()], &[], &[]);
    // Files without an extension should never be excluded by an ext filter
    assert!(!f.excludes_file(Path::new("Makefile"), Path::new(ROOT)));
    assert!(!f.excludes_file(Path::new("Dockerfile"), Path::new(ROOT)));
}

#[test]
fn exclude_filter_dir_exact_match() {
    let f = ExcludeFilter::new(&[], &[], &["vendor".to_string(), "dist".to_string()], &[]);
    assert!(f.excludes_dir("vendor"));
    assert!(f.excludes_dir("dist"));
    assert!(!f.excludes_dir("src"));
    assert!(!f.excludes_dir("Vendor")); // case-sensitive
}

#[test]
fn exclude_filter_compound_extension() {
    // "foo.min.js" has extension "js", not "min.js"
    // Excluding "js" should match; excluding "min.js" should not
    let f = ExcludeFilter::new(&[], &["js".to_string()], &[], &[]);
    assert!(
        f.excludes_file(Path::new("app.min.js"), Path::new(ROOT)),
        "foo.min.js has extension 'js' and should be excluded"
    );

    let f2 = ExcludeFilter::new(&[], &["min.js".to_string()], &[], &[]);
    assert!(
        !f2.excludes_file(Path::new("app.min.js"), Path::new(ROOT)),
        "compound 'min.js' is not a real extension and should not match"
    );
}

#[test]
fn exclude_filter_empty_string_extension_ignored() {
    // Passing an empty string or just "." should not create a match-all rule
    let f = ExcludeFilter::new(&[], &["".to_string(), ".".to_string()], &[], &[]);
    // Neither dirs nor extensions should match anything
    assert!(!f.excludes_dir("anything"));
    assert!(!f.excludes_file(Path::new("foo.rs"), Path::new(ROOT)));
}

#[test]
fn exclude_filter_combined_ext_and_dir() {
    let f = ExcludeFilter::new(
        &[],
        &["js".to_string(), "css".to_string()],
        &["vendor".to_string(), "dist".to_string()],
        &[],
    );
    // Extensions
    assert!(f.excludes_file(Path::new("app.js"), Path::new(ROOT)));
    assert!(f.excludes_file(Path::new("style.css"), Path::new(ROOT)));
    assert!(!f.excludes_file(Path::new("main.rs"), Path::new(ROOT)));
    // Directories
    assert!(f.excludes_dir("vendor"));
    assert!(f.excludes_dir("dist"));
    assert!(!f.excludes_dir("src"));
}

// ── Glob pattern matching ───────────────────────────────────────────────

#[test]
fn exclude_filter_glob_matches_filename() {
    let f = ExcludeFilter::new(&[], &[], &[], &["*.min.js".to_string()]);
    assert!(
        f.excludes_file(Path::new("app.min.js"), Path::new(ROOT)),
        "*.min.js should match app.min.js"
    );
    assert!(
        !f.excludes_file(Path::new("app.js"), Path::new(ROOT)),
        "*.min.js should not match app.js"
    );
}

#[test]
fn exclude_filter_glob_matches_path_pattern() {
    let f = ExcludeFilter::new(&[], &[], &[], &["vendor/**".to_string()]);
    assert!(
        f.excludes_file(Path::new("vendor/dep.rs"), Path::new(ROOT)),
        "vendor/** should match vendor/dep.rs"
    );
    assert!(
        f.excludes_file(Path::new("vendor/sub/dep.rs"), Path::new(ROOT)),
        "vendor/** should match nested vendor/sub/dep.rs"
    );
    assert!(
        !f.excludes_file(Path::new("src/main.rs"), Path::new(ROOT)),
        "vendor/** should not match src/main.rs"
    );
}

#[test]
fn exclude_filter_glob_multiple_patterns() {
    let f = ExcludeFilter::new(
        &[],
        &[],
        &[],
        &["*.min.js".to_string(), "*.bundle.js".to_string()],
    );
    assert!(f.excludes_file(Path::new("app.min.js"), Path::new(ROOT)));
    assert!(f.excludes_file(Path::new("main.bundle.js"), Path::new(ROOT)));
    assert!(!f.excludes_file(Path::new("app.js"), Path::new(ROOT)));
}

#[test]
fn exclude_filter_glob_combined_with_ext_and_dir() {
    let f = ExcludeFilter::new(
        &[],
        &["css".to_string()],
        &["dist".to_string()],
        &["*.min.js".to_string()],
    );
    assert!(
        f.excludes_file(Path::new("style.css"), Path::new(ROOT)),
        "ext filter"
    );
    assert!(f.excludes_dir("dist"), "dir filter");
    assert!(
        f.excludes_file(Path::new("app.min.js"), Path::new(ROOT)),
        "glob filter"
    );
    assert!(
        !f.excludes_file(Path::new("app.js"), Path::new(ROOT)),
        "not matched by any"
    );
}

#[test]
fn exclude_filter_invalid_glob_skipped() {
    // An invalid glob pattern should not cause a panic, just be skipped
    let f = ExcludeFilter::new(&[], &[], &[], &["[invalid".to_string()]);
    // Should still work for other checks
    assert!(!f.excludes_file(Path::new("foo.rs"), Path::new(ROOT)));
}

#[test]
fn exclude_filter_empty_glob_is_noop() {
    let f = ExcludeFilter::new(&[], &[], &[], &[]);
    assert!(!f.excludes_file(Path::new("anything.rs"), Path::new(ROOT)));
}

// ── Glob: absolute-path correctness ─────────────────────────────────────

#[test]
fn exclude_filter_glob_with_absolute_path() {
    let f = ExcludeFilter::new(&[], &[], &[], &["vendor/**".to_string()]);
    let root = Path::new("/home/user/project");
    assert!(
        f.excludes_file(Path::new("/home/user/project/vendor/foo.rs"), root),
        "vendor/** should match when path is absolute"
    );
    assert!(
        !f.excludes_file(Path::new("/home/user/project/src/main.rs"), root),
        "should not match non-vendor files"
    );
}

#[test]
fn exclude_filter_glob_with_tempdir() {
    let dir = tempdir().unwrap();
    let vendor = dir.path().join("vendor");
    fs::create_dir(&vendor).unwrap();
    fs::write(vendor.join("dep.rs"), "// generated").unwrap();
    fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();

    let f = ExcludeFilter::new(&[], &[], &[], &["vendor/**".to_string()]);

    // tempdir paths are absolute — verify glob still works
    let vendor_file = vendor.join("dep.rs");
    assert!(
        f.excludes_file(&vendor_file, dir.path()),
        "vendor/** should match with absolute tempdir path"
    );
    assert!(
        !f.excludes_file(&dir.path().join("main.rs"), dir.path()),
        "should not match root files"
    );
}

// ── Integration: source_files respects ExcludeFilter ──────────────────

#[test]
fn source_files_excludes_extension() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
    fs::write(dir.path().join("lib.js"), "console.log('hi');").unwrap();

    let filter = ExcludeFilter::new(&[], &["js".to_string()], &[], &[]);
    let files = source_files(dir.path(), false, &filter);

    let names: Vec<_> = files
        .iter()
        .map(|(p, _)| p.file_name().unwrap().to_str().unwrap().to_string())
        .collect();

    assert!(
        names.contains(&"main.rs".to_string()),
        "main.rs should be included"
    );
    assert!(
        !names.contains(&"lib.js".to_string()),
        "lib.js should be excluded"
    );
}

#[test]
fn source_files_excludes_directory() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();

    let vendor = dir.path().join("vendor");
    fs::create_dir(&vendor).unwrap();
    fs::write(vendor.join("dep.rs"), "// generated").unwrap();

    let filter = ExcludeFilter::new(&[], &[], &["vendor".to_string()], &[]);
    let files = source_files(dir.path(), false, &filter);

    // Only main.rs at root should appear; dep.rs inside vendor should not
    assert_eq!(files.len(), 1, "only root file should be found");
    assert_eq!(files[0].0.file_name().unwrap().to_str().unwrap(), "main.rs");
}

#[test]
fn source_files_no_filter_unchanged() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
    fs::write(dir.path().join("lib.js"), "export {};").unwrap();

    let filter = ExcludeFilter::default();
    let files = source_files(dir.path(), false, &filter);

    assert_eq!(files.len(), 2, "both files should appear with empty filter");
}

#[test]
fn source_files_excludes_by_glob() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
    fs::write(dir.path().join("app.min.js"), "var x=1;").unwrap();
    fs::write(dir.path().join("lib.js"), "export {};").unwrap();

    let filter = ExcludeFilter::new(&[], &[], &[], &["*.min.js".to_string()]);
    let files = source_files(dir.path(), false, &filter);

    let names: Vec<_> = files
        .iter()
        .map(|(p, _)| p.file_name().unwrap().to_str().unwrap().to_string())
        .collect();

    assert!(names.contains(&"main.rs".to_string()));
    assert!(names.contains(&"lib.js".to_string()));
    assert!(
        !names.contains(&"app.min.js".to_string()),
        "app.min.js should be excluded by glob"
    );
}

#[test]
fn source_files_glob_with_absolute_path() {
    let dir = tempdir().unwrap();
    let vendor = dir.path().join("vendor");
    fs::create_dir(&vendor).unwrap();
    fs::write(vendor.join("dep.rs"), "// generated").unwrap();
    fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();

    // tempdir paths are absolute — this previously would have failed
    let filter = ExcludeFilter::new(&[], &[], &[], &["vendor/**".to_string()]);
    let files = source_files(dir.path(), false, &filter);

    assert_eq!(files.len(), 1, "only main.rs should be found");
    assert_eq!(files[0].0.file_name().unwrap().to_str().unwrap(), "main.rs");
}
