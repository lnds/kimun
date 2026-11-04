use super::*;
use crate::git::ChangeKind;

fn change(path: &str, lines: &[usize], removed: &[&str], inserted: &[&str]) -> FileDiffStat {
    FileDiffStat {
        path: PathBuf::from(path),
        old_path: Some(PathBuf::from(path)),
        kind: ChangeKind::Modified,
        added: inserted.len(),
        deleted: removed.len(),
        lines: lines.to_vec(),
        old_lines: lines.to_vec(),
        probe: Vec::new(),
        removed: removed.iter().map(|l| l.to_string()).collect(),
        inserted: inserted.iter().map(|l| l.to_string()).collect(),
    }
}

fn files(list: &[(&str, &str)]) -> Vec<(PathBuf, String)> {
    list.iter()
        .map(|(p, t)| (PathBuf::from(p), t.to_string()))
        .collect()
}

#[test]
fn literals_are_strings_atoms_and_numbers_that_tell() {
    assert_eq!(
        literals(r#"  @valid ["CL", "PE", 'MX', :pending, 2024, 1, "a", true, "x#{y}"]"#),
        ["CL", "PE", "MX", "pending", "2024"]
    );
    // Keywords, type annotations, apostrophes and names with digits are no values.
    assert!(literals("  key: value, x :: integer, don't v2").is_empty());
    assert_eq!(literals(r#""say \"hi\"""#), [r#"say "hi""#]);
    assert_eq!(literals("@codes ~w(CL PE)a"), ["CL", "PE"]);
    assert_eq!(literals(r#"["??", "->", ".tf"]"#), [".tf"]);
}

const GEO: &str =
    "defmodule Geo.Countries do\n  @valid [\"CL\", \"CO\", \"MX\", \"PE\", \"US\"]\nend\n";

fn geo_change() -> FileDiffStat {
    change(
        "apps/geo/lib/countries.ex",
        &[2],
        &["  @valid [\"CL\", \"CO\", \"MX\", \"PE\"]"],
        &["  @valid [\"CL\", \"CO\", \"MX\", \"PE\", \"US\"]"],
    )
}

#[test]
fn a_value_added_to_a_group_is_an_edit_of_it() {
    let edits = edits(&geo_change(), GEO);
    assert_eq!(edits.len(), 1);
    let set = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<BTreeSet<_>>();
    assert_eq!(edits[0].before, set(&["CL", "CO", "MX", "PE"]));
    assert_eq!(edits[0].added, set(&["US"]));
    assert!(edits[0].dropped.is_empty());
    assert_eq!(edits[0].line, 2);
}

#[test]
fn a_group_over_several_lines_counts_whole() {
    let head = "VALID = [\n    \"CL\",\n    \"CO\",\n    \"MX\",\n    \"AR\",\n]\n";
    // "PE" became "AR".
    let stat = change("geo.py", &[5], &["    \"PE\","], &["    \"AR\","]);
    let edits = edits(&stat, head);
    assert_eq!(edits.len(), 1);
    assert!(edits[0].before.contains("PE") && !edits[0].before.contains("AR"));
    assert_eq!(edits[0].dropped.iter().collect::<Vec<_>>(), ["PE"]);
}

#[test]
fn small_groups_and_unchanged_values_make_no_edit() {
    let message = change(
        "a.ex",
        &[1],
        &["log(\"Failed\")"],
        &["log(\"Failed again\")"],
    );
    assert!(edits(&message, "log(\"Failed again\")\n").is_empty());
    // The same values written another way.
    let reflowed = change(
        "a.ex",
        &[1],
        &["[\"CL\", \"CO\",", "\"MX\"]"],
        &["[\"CL\", \"CO\", \"MX\"]"],
    );
    assert!(edits(&reflowed, "[\"CL\", \"CO\", \"MX\"]\n").is_empty());
}

#[test]
fn a_copy_left_behind_is_found_and_a_comment_ties_it() {
    let edits = edits(&geo_change(), GEO);
    let others = files(&[
        (
            "apps/billing/lib/codes.ex",
            "defmodule Billing.Codes do\n  # Mirrors Geo.Countries, keep in sync.\n  @codes ~w(CL CO MX PE)a\n  @codes2 [:CL, :CO, :MX, :PE]\nend\n",
        ),
        (
            "apps/web/assets/countries.js",
            "export const COUNTRIES = ['CL', 'CO', 'MX', 'PE'];\n",
        ),
        // Already updated.
        (
            "apps/api/countries.py",
            "COUNTRIES = ['CL', 'CO', 'MX', 'PE', 'US']\n",
        ),
        // Too few values in common.
        (
            "apps/x/other.ex",
            "@codes [\"CL\", \"AR\", \"BR\", \"UY\"]\n",
        ),
    ]);
    let found = copies(&edits, &others);
    let summary: Vec<(&str, usize, bool)> = found
        .iter()
        .map(|c| (c.file.to_str().unwrap(), c.line, c.mentions))
        .collect();
    assert_eq!(
        summary,
        [
            ("apps/billing/lib/codes.ex", 3, true),
            ("apps/web/assets/countries.js", 1, false),
        ]
    );
    assert_eq!(found[0].missing, ["US"]);
    assert_eq!(found[0].matched, ["CL", "CO", "MX", "PE"]);
    assert_eq!(found[0].of, PathBuf::from("apps/geo/lib/countries.ex"));
}

#[test]
fn a_copy_that_still_holds_a_dropped_value_is_behind() {
    let head = "STATES = [:draft, :sent, :paid]\n";
    let stat = change(
        "lib/order.ex",
        &[1],
        &["STATES = [:draft, :sent, :paid, :void]"],
        &["STATES = [:draft, :sent, :paid]"],
    );
    let edits = edits(&stat, head);
    let found = copies(
        &edits,
        &files(&[
            (
                "web/order.ts",
                "const STATES = ['draft', 'sent', 'paid', 'void'];\n",
            ),
            ("web/done.ts", "const STATES = ['draft', 'sent', 'paid'];\n"),
        ]),
    );
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].file, PathBuf::from("web/order.ts"));
    assert_eq!(found[0].kept, ["void"]);
    assert!(found[0].missing.is_empty());
}

#[test]
fn the_block_names_each_copy_and_what_it_lacks() {
    let edits = edits(&geo_change(), GEO);
    let found = copies(
        &edits,
        &files(&[(
            "apps/billing/lib/codes.ex",
            "# Keep in sync with countries.ex\n@codes [\"CL\", \"CO\", \"MX\", \"PE\"]\n",
        )]),
    );
    assert_eq!(
        crate::impact::copies_report::render(&found)[2..4],
        [
            " apps/billing/lib/codes.ex:2  4 of 5 values of apps/geo/lib/countries.ex:2, says it mirrors it",
            "            lacks US",
        ]
    );
    assert!(crate::impact::copies_report::render(&[]).is_empty());
}
