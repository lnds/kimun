use super::*;

/// The 1-based numbers of the lines of `src` inside a test block.
fn masked(src: &str) -> Vec<usize> {
    let lines: Vec<String> = src.lines().map(String::from).collect();
    let mask = test_block_mask(&lines);
    (1..=lines.len()).filter(|n| mask[n - 1]).collect()
}

#[test]
fn nothing_is_masked_without_the_attribute() {
    assert!(masked("fn foo() {}\nfn bar() {}\n").is_empty());
    assert!(masked("").is_empty());
}

#[test]
fn items_under_the_attribute_one_after_another_are_all_masked() {
    let src = "\
#[cfg(test)]
use std::fmt;
#[cfg(test)]
use std::io;
fn kept() {}
";
    assert_eq!(masked(src), [1, 2, 3, 4]);
}

#[test]
fn code_after_a_test_module_is_kept() {
    let src = "\
pub fn before() -> u32 {
    1
}

#[cfg(test)]
mod tests {
    #[test]
    fn works() {
        assert_eq!(super::before(), 1);
    }
}

pub fn after() -> u32 {
    2
}
";
    assert_eq!(masked(src), vec![5, 6, 7, 8, 9, 10, 11]);
}

#[test]
fn item_without_braces_ends_at_its_semicolon() {
    let src = "\
#[cfg(test)]
use std::collections::HashMap;

  #[cfg(test)]
#[path = \"foo_test.rs\"]
mod tests;

pub fn kept() {}
";
    assert_eq!(masked(src), vec![1, 2, 4, 5, 6]);
}

#[test]
fn field_under_the_attribute_does_not_take_the_rest_of_the_struct() {
    let src = "\
pub struct Config {
    pub name: String,
    #[cfg(test)]
    pub probe: Vec<u32>,
    pub size: usize,
}
#[cfg(test)] const LIMIT: usize = 3;
";
    assert_eq!(masked(src), vec![3, 4, 7]);
    let last = "\
pub struct Config {
    #[cfg(test)]
    pub probe: fn(u32) -> u32
}
pub fn kept() {}
";
    assert_eq!(masked(last), vec![2, 3]);
}

#[test]
fn function_with_parameters_on_several_lines_and_a_where_clause() {
    let src = "\
#[cfg(test)]
fn helper<T>(
    first: T,
    second: [u8; 2],
) -> T
where
    T: Clone,
{
    first.clone()
}
pub fn kept() {}
";
    assert_eq!(masked(src), (1..=10).collect::<Vec<_>>());
}

#[test]
fn braces_and_attributes_inside_literals_and_comments_count_for_nothing() {
    let src = "\
// #[cfg(test)]
pub fn doc() -> &'static str {
    \"
#[cfg(test)]
\"
}
#[cfg(test)]
mod tests {
    const OPEN: char = '{';
    const TEXT: &str = \"}\"; // }
    fn id<'a>(x: &'a str) -> &'a str { x }
}
pub fn kept() {}
";
    assert_eq!(masked(src), vec![7, 8, 9, 10, 11, 12]);
}

#[test]
fn unclosed_block_runs_to_the_end() {
    assert_eq!(
        masked("fn a() {}\n#[cfg(test)]\nmod tests {\n    fn t() {}\n"),
        vec![2, 3, 4]
    );
}

#[test]
fn test_block_lines_become_blank() {
    let lines: Vec<String> = "fn a() {}\n#[cfg(test)]\nmod tests;\n// note\nfn b() {}\n"
        .lines()
        .map(String::from)
        .collect();
    let kinds = [
        LineKind::Code,
        LineKind::Code,
        LineKind::Code,
        LineKind::Comment,
        LineKind::Code,
    ];
    assert_eq!(
        without_test_blocks(&lines, &kinds),
        vec![
            LineKind::Code,
            LineKind::Blank,
            LineKind::Blank,
            LineKind::Comment,
            LineKind::Code,
        ]
    );
}

/// A Rust source of three lines of code followed by a test module of
/// eleven, which holds a function of five parameters and a nested branch.
pub(crate) const WITH_TEST_MODULE: &str = "\
pub fn double(x: u32) -> u32 {
    x * 2
}

#[cfg(test)]
mod tests {
    fn sum(a: u32, b: u32, c: u32, d: u32, e: u32) -> u32 {
        for n in 0..a {
            if n > b {
                return c + d + e;
            }
        }
        0
    }
}
";

/// `WITH_TEST_MODULE` written to a file of a directory of its own.
pub(crate) fn source_with_test_module() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("lib.rs");
    std::fs::write(&path, WITH_TEST_MODULE).unwrap();
    (dir, path)
}

fn code_lines_of(language_file: &str, exclude_tests: bool) -> usize {
    let spec = crate::loc::language::detect(std::path::Path::new(language_file)).unwrap();
    let lines: Vec<String> = WITH_TEST_MODULE.lines().map(String::from).collect();
    let kinds = crate::loc::counter::classify_reader(WITH_TEST_MODULE.as_bytes(), spec);
    production_kinds(&lines, kinds, spec, exclude_tests)
        .iter()
        .filter(|kind| **kind == LineKind::Code)
        .count()
}

#[test]
fn production_kinds_leave_the_test_module_out_only_when_asked() {
    assert_eq!(code_lines_of("lib.rs", true), 3);
    assert_eq!(code_lines_of("lib.rs", false), 14);
}

#[test]
fn production_kinds_touch_no_other_language() {
    assert_eq!(
        code_lines_of("lib.kt", true),
        code_lines_of("lib.kt", false)
    );
}

fn rust_files(dir: &Path, files: &[(&str, &str)]) -> Vec<(PathBuf, &'static LanguageSpec)> {
    let rust = crate::loc::language::detect(Path::new("a.rs")).unwrap();
    files
        .iter()
        .map(|(name, text)| {
            let path = dir.join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, text).unwrap();
            (path, rust)
        })
        .collect()
}

#[test]
fn test_modules_declared_without_a_body() {
    let dir = tempfile::tempdir().unwrap();
    let files = rust_files(
        dir.path(),
        &[
            (
                "src/lib.rs",
                "pub fn f() {}\n\n#[cfg(test)]\nmod testutil;\n\nmod real;\n",
            ),
            ("src/testutil.rs", "pub fn helper() {}\n"),
            ("src/real.rs", "#[cfg(test)]\npub(crate) mod fixtures;\n"),
            ("src/real/fixtures/mod.rs", "pub fn data() {}\n"),
        ],
    );
    let modules = rust_test_modules(&files);
    let expected: HashSet<PathBuf> = [
        dir.path().join("src/testutil.rs"),
        dir.path().join("src/real/fixtures/mod.rs"),
    ]
    .into_iter()
    .collect();
    assert_eq!(modules, expected);
}

#[test]
fn test_module_through_a_path_attribute() {
    let dir = tempfile::tempdir().unwrap();
    let files = rust_files(
        dir.path(),
        &[
            (
                "src/walk.rs",
                "#[cfg(test)]\n#[path = \"walk_test.rs\"]\nmod tests;\n",
            ),
            ("src/walk_test.rs", "fn t() {}\n"),
        ],
    );
    assert_eq!(
        rust_test_modules(&files),
        HashSet::from([dir.path().join("src/walk_test.rs")])
    );
}

#[test]
fn inline_test_module_names_no_file() {
    let dir = tempfile::tempdir().unwrap();
    let files = rust_files(
        dir.path(),
        &[
            (
                "src/lib.rs",
                "#[cfg(test)]\nmod tests {\n    mod inner;\n}\n",
            ),
            ("src/tests.rs", "fn t() {}\n"),
        ],
    );
    assert!(rust_test_modules(&files).is_empty());
}
