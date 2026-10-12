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
