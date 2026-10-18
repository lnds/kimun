use crate::loc::counter::LineKind;

use super::*;

fn lines(s: &str) -> Vec<String> {
    s.lines().map(String::from).collect()
}

fn all_code(n: usize) -> Vec<LineKind> {
    vec![LineKind::Code; n]
}

#[test]
fn magic_number_detected() {
    let ls = lines("    timeout(3600);");
    let kinds = vec![LineKind::Code];
    let smells = detect_magic_numbers(&ls, &kinds, &["//"][..]);
    assert_eq!(smells.len(), 1);
    assert_eq!(smells[0].kind, SmellKind::MagicNumber);
}

#[test]
fn trivial_numbers_not_detected() {
    let ls = lines("    x = 0;\n    y = 1;\n    z = 2;\n    w = -1;");
    let kinds = all_code(4);
    let smells = detect_magic_numbers(&ls, &kinds, &["//"][..]);
    assert!(smells.is_empty());
}

#[test]
fn const_declaration_not_detected() {
    let ls = lines("const MAX: i32 = 100;");
    let kinds = vec![LineKind::Code];
    let smells = detect_magic_numbers(&ls, &kinds, &["//"][..]);
    assert!(smells.is_empty());
}

#[test]
fn let_declaration_not_detected() {
    let ls = lines("let threshold = 42;");
    let kinds = vec![LineKind::Code];
    let smells = detect_magic_numbers(&ls, &kinds, &["//"][..]);
    assert!(smells.is_empty());
}

#[test]
fn readonly_declaration_not_detected() {
    // Shell: `readonly NAME=16` is the idiomatic constant declaration.
    let ls = lines("readonly FINGERPRINT_LEN=16");
    let kinds = vec![LineKind::Code];
    let smells = detect_magic_numbers(&ls, &kinds, &["#"][..]);
    assert!(smells.is_empty());
}

#[test]
fn declare_r_declaration_not_detected() {
    // Shell: `declare -r NAME=N` is another way to declare a readonly constant.
    let ls = lines("declare -r MAX_RETRIES=42");
    let kinds = vec![LineKind::Code];
    let smells = detect_magic_numbers(&ls, &kinds, &["#"][..]);
    assert!(smells.is_empty());
}

#[test]
fn export_declaration_not_detected() {
    // Shell: `export NAME=N` at file top is commonly used for config values.
    let ls = lines("export PORT=8443");
    let kinds = vec![LineKind::Code];
    let smells = detect_magic_numbers(&ls, &kinds, &["#"][..]);
    assert!(smells.is_empty());
}

#[test]
fn local_r_declaration_not_detected() {
    // Shell: `local -r NAME=N` — function-scoped readonly constant.
    let ls = lines("    local -r TIMEOUT=30");
    let kinds = vec![LineKind::Code];
    let smells = detect_magic_numbers(&ls, &kinds, &["#"][..]);
    assert!(smells.is_empty());
}

#[test]
fn decl_keyword_not_matched_as_substring() {
    // "deleteable" contains "let " as substring — should NOT be excluded
    let ls = lines("    is_deleteable(3600);");
    let kinds = vec![LineKind::Code];
    let smells = detect_magic_numbers(&ls, &kinds, &["//"][..]);
    assert_eq!(
        smells.len(),
        1,
        "substring match should not suppress detection"
    );
}

#[test]
fn hex_literal_detected() {
    let ls = lines("    mask = 0xFF;");
    let kinds = vec![LineKind::Code];
    let smells = detect_magic_numbers(&ls, &kinds, &["//"][..]);
    assert_eq!(smells.len(), 1);
}

#[test]
fn scientific_notation_detected() {
    let ls = lines("    threshold = 1.5e10;");
    let kinds = vec![LineKind::Code];
    let smells = detect_magic_numbers(&ls, &kinds, &["//"][..]);
    assert_eq!(smells.len(), 1);
}

#[test]
fn digit_separator_detected_as_magic_number() {
    // `1_000_000` is a non-trivial literal with Rust/Python/Swift digit separators
    let ls = lines("    timeout(1_000_000);");
    let kinds = vec![LineKind::Code];
    let smells = detect_magic_numbers(&ls, &kinds, &["//"][..]);
    assert_eq!(
        smells.len(),
        1,
        "1_000_000 should be detected as a magic number"
    );
}

#[test]
fn hex_with_digit_separator_detected() {
    let ls = lines("    mask = 0xFF_FF;");
    let kinds = vec![LineKind::Code];
    let smells = detect_magic_numbers(&ls, &kinds, &["//"][..]);
    assert_eq!(
        smells.len(),
        1,
        "0xFF_FF should be detected as a magic number"
    );
}

// ── Commented-out code ──

/// The lines of `src` reported as holding a magic number.
fn flagged(src: &str) -> Vec<usize> {
    let ls = lines(src);
    detect_magic_numbers(&ls, &all_code(ls.len()), &["//", "#"][..])
        .into_iter()
        .map(|s| s.line)
        .collect()
}

#[test]
fn constants_with_modifiers_are_declarations() {
    let src = "\
  private static final int PEEKED_BEGIN_ARRAY = 3;
  public const int BufferSize = 1024;
  int x = y * 3600;";
    assert_eq!(flagged(src), [3]);
}

#[test]
fn enum_members_are_part_of_the_declaration() {
    let src = "\
public enum JsonToken
{
    StartArray = 3,
    PropertyName = 4,
}
enum class color : unsigned {
  alice_blue = 0xF0F8FF,
};
int after = width * 3600;";
    assert_eq!(flagged(src), [9]);
}

#[test]
fn go_const_block_and_python_constants() {
    let src = "\
const (
	ErrorTypeBind ErrorType = 1 << 63
	MaxRetries = 5
)
MAX_RETRIES = 5
return x * 86400";
    assert_eq!(flagged(src), [6]);
}

#[test]
fn table_elements_are_data() {
    let src = "\
  static constexpr unsigned long pow10_significands[] = {
      0x81ceb32c4b43fcf5, 0xa2425ff75e14fc32,
      0xfd87b5f28300ca0e};
  return n * 1000;";
    assert_eq!(flagged(src), [4]);
}

#[test]
fn literal_union_types_are_types() {
    let src = "\
export type ClientErrorStatusCode =
  | 400
  | 404
const code = status + 300";
    assert!(flagged(src).is_empty());
}

#[test]
fn named_keys_and_parameter_defaults_name_their_value() {
    let src = "\
locals_without_parens = [
  connect: 3,
  delete: 4
]
def redirect(location: str, code: int = 303) -> str:
    return location * 7";
    assert_eq!(flagged(src), [6]);
}

#[test]
fn function_body_after_a_declaration_keyword_is_still_read() {
    let src = "\
  static int scale(int x = 1) {
    return x * 3600;
  }
export function f(a) {
  return a * 86400;
}";
    assert_eq!(flagged(src), [2, 5]);
}
