use super::*;

fn change(path: &str, kind: ChangeKind, removed: &[&str]) -> FileDiffStat {
    FileDiffStat {
        path: PathBuf::from(path),
        old_path: Some(PathBuf::from(path)),
        kind,
        added: 0,
        deleted: removed.len(),
        lines: vec![1],
        old_lines: vec![1],
        probe: Vec::new(),
        removed: removed.iter().map(|l| l.to_string()).collect(),
    }
}

#[test]
fn a_public_function_gone_from_the_file_is_removed() {
    let head = "defmodule Geo do\n  def name(c), do: c\nend\n";
    let removed = [
        "  def valid_countries, do: @valid",
        "  defp helper, do: 1",
        "  def name(c), do: c.name",
    ];
    let stat = change("lib/geo.ex", ChangeKind::Modified, &removed);
    // `name` only changed; a private function is nobody's to call.
    assert_eq!(
        functions_removed("Elixir", &stat, head)
            .into_iter()
            .collect::<Vec<_>>(),
        ["valid_countries"]
    );

    let python = change(
        "shop/prices.py",
        ChangeKind::Modified,
        &["def tax(x):", "    return x"],
    );
    assert_eq!(
        functions_removed("Python", &python, "RATE = 2\n")
            .into_iter()
            .collect::<Vec<_>>(),
        ["tax"]
    );
    // Languages that do not tell functions remove none.
    let go = change("a.go", ChangeKind::Modified, &["func F() {}"]);
    assert!(functions_removed("Go", &go, "").is_empty());
}

#[test]
fn a_deleted_file_is_kept_as_it_was_unless_it_moved() {
    let gone = change(
        "lib/old.ex",
        ChangeKind::Deleted,
        &["defmodule Old do", "end"],
    );
    let moved = change(
        "lib/moved.ex",
        ChangeKind::Deleted,
        &["defmodule Kept do", "end"],
    );
    let script = change("src/util.js", ChangeKind::Deleted, &["export const f = 1"]);
    let prose = change("notes.md", ChangeKind::Deleted, &["# Notes"]);
    let head = vec![(
        PathBuf::from("lib/new_place.ex"),
        "defmodule Kept do\nend\n".to_string(),
    )];
    let deleted = deleted_files(&[&gone, &moved, &script, &prose], &head);
    assert_eq!(
        deleted,
        [
            (
                PathBuf::from("lib/old.ex"),
                "defmodule Old do\nend\n".to_string()
            ),
            (
                PathBuf::from("src/util.js"),
                "export const f = 1\n".to_string()
            ),
        ]
    );
}
