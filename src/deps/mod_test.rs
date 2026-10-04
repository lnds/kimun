use super::*;
use crate::cli::OutputMode;
use crate::walk::{ExcludeFilter, WalkConfig};
use std::fs;

// ── detect_go_module ─────────────────────────────────────────────────────────

#[test]
fn detect_go_module_returns_none_for_missing_file() {
    let dir = tempfile::tempdir().unwrap();
    assert!(detect_go_module(dir.path()).is_none());
}

#[test]
fn detect_go_module_returns_module_name() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("go.mod"),
        "module github.com/user/myproject\n\ngo 1.21\n",
    )
    .unwrap();
    assert_eq!(
        detect_go_module(dir.path()),
        Some("github.com/user/myproject".to_string())
    );
}

#[test]
fn detect_go_module_no_module_line() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("go.mod"), "go 1.21\n").unwrap();
    assert!(detect_go_module(dir.path()).is_none());
}

// ── run ──────────────────────────────────────────────────────────────────────

#[test]
fn run_on_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Table, false, "default", 20).unwrap();
}

#[test]
fn run_on_empty_dir_json() {
    let dir = tempfile::tempdir().unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Json, false, "default", 20).unwrap();
}

#[test]
fn run_on_rust_files_no_deps() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("main.rs"), "fn main() {}\n").unwrap();
    fs::write(dir.path().join("lib.rs"), "pub fn helper() {}\n").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Table, false, "default", 20).unwrap();
}

#[test]
fn run_on_rust_with_mod_declaration() {
    let dir = tempfile::tempdir().unwrap();
    // lib.rs declares mod foo
    fs::write(dir.path().join("lib.rs"), "mod foo;\n\npub fn bar() {}\n").unwrap();
    // foo.rs exists
    fs::write(dir.path().join("foo.rs"), "pub fn foo_fn() {}\n").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Table, false, "default", 20).unwrap();
}

#[test]
fn run_sort_by_fan_in() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("main.rs"), "fn main() {}\n").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Table, false, "fan-in", 20).unwrap();
}

#[test]
fn run_sort_by_fan_out() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("main.rs"), "fn main() {}\n").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Table, false, "fan-out", 20).unwrap();
}

#[test]
fn run_cycles_only_filter() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("main.rs"), "fn main() {}\n").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Table, true, "default", 20).unwrap();
}

#[test]
fn run_cycles_only_json() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("main.rs"), "fn main() {}\n").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Json, true, "default", 20).unwrap();
}

#[test]
fn run_with_go_module() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("go.mod"),
        "module github.com/example/project\n\ngo 1.21\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("main.go"),
        "package main\n\nfunc main() {}\n",
    )
    .unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Table, false, "default", 20).unwrap();
}

#[test]
fn run_unreadable_file_gracefully_handled() {
    // Write a file with null bytes so read_to_string fails
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("binary.rs"), b"fn main() \x00{}").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    // Should not panic — errors are handled gracefully
    run(&cfg, OutputMode::Table, false, "default", 20).unwrap();
}

#[test]
fn run_json_with_mod_deps() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("lib.rs"), "mod foo;\n").unwrap();
    fs::write(dir.path().join("foo.rs"), "pub fn f() {}\n").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Json, false, "default", 20).unwrap();
}

#[test]
fn run_short_format() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("main.rs"), "fn main() {}\n").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Short, false, "default", 20).unwrap();
}

#[test]
fn run_terse_format() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("main.rs"), "fn main() {}\n").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Terse, false, "default", 20).unwrap();
}

// ── Kaikai ───────────────────────────────────────────────────────────────────

fn write(root: &std::path::Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// (fan_in, fan_out, in_cycle) of the entry at `rel`.
fn metrics(result: &DepResult, rel: &str) -> (usize, usize, bool) {
    let entry = result
        .entries
        .iter()
        .find(|e| e.path == std::path::Path::new(rel))
        .unwrap_or_else(|| panic!("no entry for {rel}"));
    (entry.fan_in, entry.fan_out, entry.in_cycle)
}

#[test]
fn kaikai_imports_build_the_graph() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "app/main.kai",
        "import app.util.text\nimport app.b as b\n\n\
         fn main() : Unit / Stdout = Stdout.print(text.shout(b.name()))\n",
    );
    write(
        dir.path(),
        "app/b.kai",
        "import app.util.text.{shout}\nimport app.main\n\n\
         pub fn name() : String = shout(\"b\")\n",
    );
    write(
        dir.path(),
        "app/util/text.kai",
        "pub fn shout(s: String) : String = s\n",
    );
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let result = analyze(&cfg);

    assert_eq!(metrics(&result, "app/main.kai"), (1, 2, true));
    assert_eq!(metrics(&result, "app/b.kai"), (1, 2, true));
    assert_eq!(metrics(&result, "app/util/text.kai"), (2, 0, false));
    assert_eq!(
        result.cycles,
        vec![vec![
            PathBuf::from("app/b.kai"),
            PathBuf::from("app/main.kai")
        ]]
    );
    assert!(result.unsupported.is_empty());
}

#[test]
fn kaikai_external_and_commented_imports_add_no_edges() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "main.kai",
        "import core.list\n# import helper\n\
         #[doc(\"\"\"\nimport helper\n\"\"\")]\npub fn main() : Unit = ()\n",
    );
    write(dir.path(), "helper.kai", "pub fn help() : Unit = ()\n");
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let result = analyze(&cfg);

    assert_eq!(metrics(&result, "main.kai"), (0, 0, false));
    assert_eq!(metrics(&result, "helper.kai"), (0, 0, false));
}

#[test]
fn kaikai_package_directory_import() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "consumer/main.kai", "import mid\n");
    write(dir.path(), "mid/kai.toml", "name = \"mid\"\n");
    write(dir.path(), "mid/mid.kai", "pub fn m() : Unit = ()\n");
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let result = analyze(&cfg);

    assert_eq!(metrics(&result, "consumer/main.kai"), (0, 1, false));
    assert_eq!(metrics(&result, "mid/mid.kai"), (1, 0, false));
}

// ── unsupported languages ────────────────────────────────────────────────────

#[test]
fn unsupported_languages_are_left_out_of_the_graph() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "lib.rs", "mod foo;\n");
    write(dir.path(), "foo.rs", "pub fn f() {}\n");
    write(dir.path(), "build.sh", "echo hi\n");
    write(dir.path(), "deploy.sh", "echo bye\n");
    write(dir.path(), "Main.java", "class Main {}\n");
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let result = analyze(&cfg);

    let analysed: Vec<&str> = result.entries.iter().map(|e| e.language.as_str()).collect();
    assert_eq!(analysed, vec!["Rust", "Rust"]);
    assert_eq!(
        result.unsupported,
        vec![
            UnsupportedLanguage {
                language: "Bourne Shell".to_string(),
                files: 2,
            },
            UnsupportedLanguage {
                language: "Java".to_string(),
                files: 1,
            },
        ]
    );
}

#[test]
fn run_on_unsupported_languages_only() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "build.sh", "echo hi\n");
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    for mode in [OutputMode::Table, OutputMode::Json, OutputMode::Short] {
        run(&cfg, mode, false, "default", 20).unwrap();
    }
}

// ── sorting and selection ────────────────────────────────────────────────────

fn entry(path: &str, fan_in: usize, fan_out: usize, in_cycle: bool) -> DepEntry {
    DepEntry {
        path: PathBuf::from(path),
        language: "Rust".to_string(),
        fan_in,
        fan_out,
        in_cycle,
    }
}

fn sorted_paths(sort_by: &str) -> Vec<String> {
    let mut entries = vec![
        entry("c.rs", 2, 2, false),
        entry("b.rs", 3, 2, false),
        entry("a.rs", 1, 5, false),
    ];
    sort_entries(&mut entries, sort_by);
    entries
        .iter()
        .map(|e| e.path.display().to_string())
        .collect()
}

#[test]
fn sort_by_fan_in_orders_by_fan_in_descending() {
    assert_eq!(sorted_paths("fan-in"), vec!["b.rs", "c.rs", "a.rs"]);
}

#[test]
fn sort_by_fan_out_keeps_ties_in_their_original_order() {
    assert_eq!(sorted_paths("fan-out"), vec!["a.rs", "c.rs", "b.rs"]);
}

#[test]
fn default_sort_breaks_fan_out_ties_by_fan_in() {
    assert_eq!(sorted_paths("default"), vec!["a.rs", "b.rs", "c.rs"]);
}

#[test]
fn visible_entries_are_the_cycle_members_or_the_first_ones() {
    let entries = vec![
        entry("clean.rs", 0, 3, false),
        entry("a.rs", 1, 1, true),
        entry("b.rs", 1, 1, true),
    ];
    let paths = |cycles_only| -> Vec<String> {
        visible_entries(&entries, cycles_only, 1)
            .iter()
            .map(|e| e.path.display().to_string())
            .collect()
    };
    assert_eq!(paths(true), vec!["a.rs", "b.rs"]);
    assert_eq!(paths(false), vec!["clean.rs"]);
}
