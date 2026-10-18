use super::*;
use crate::loc::language::detect;
use std::path::Path;

fn fold(file: &str, lines: &[&str]) -> Vec<String> {
    let lines: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
    folded(&lines, detect(Path::new(file)).unwrap())
}

#[test]
fn folded_code_is_lower_case_and_keeps_its_length() {
    let folded = fold("a.pas", &["  IF X THEN Begin"]);
    assert_eq!(folded, ["  if x then begin"]);
}

#[test]
fn fortran_ends_of_constructs_are_joined() {
    let folded = fold(
        "a.f90",
        &[
            "END IF",
            "end  do",
            "End Select",
            "endif",
            "end subroutine f",
        ],
    );
    assert_eq!(
        folded,
        [
            "endif ",
            "enddo  ",
            "endselect ",
            "endif",
            "end subroutine f"
        ]
    );
}

#[test]
fn pascal_end_and_else_stay_apart() {
    assert_eq!(fold("a.pas", &["end else if x"]), ["end else if x"]);
}

#[test]
fn an_identifier_ending_in_end_is_no_end() {
    assert_eq!(fold("a.f90", &["call append if"]), ["call append if"]);
}

#[test]
fn names_are_read_back_as_written() {
    let lines = vec!["procedure TCounter.Inc;".to_string()];
    assert_eq!(name_at("tcounter.inc", &lines, 1), "TCounter.Inc");
    assert_eq!(name_at("<file>", &lines, 1), "<file>");
    assert_eq!(name_at("x", &lines, 9), "x");
}
