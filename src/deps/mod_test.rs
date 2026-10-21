use super::*;
use crate::cli::OutputMode;
use crate::walk::{ExcludeFilter, WalkConfig};
use std::fs;

// ── run ──────────────────────────────────────────────────────────────────────

#[test]
fn run_on_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run_table(&cfg, OutputMode::Table, false, "default", Some(20)).unwrap();
}

#[test]
fn run_on_empty_dir_json() {
    let dir = tempfile::tempdir().unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run_table(&cfg, OutputMode::Json, false, "default", Some(20)).unwrap();
}

#[test]
fn run_on_rust_files_no_deps() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("main.rs"), "fn main() {}\n").unwrap();
    fs::write(dir.path().join("lib.rs"), "pub fn helper() {}\n").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run_table(&cfg, OutputMode::Table, false, "default", Some(20)).unwrap();
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
    run_table(&cfg, OutputMode::Table, false, "default", Some(20)).unwrap();
}

#[test]
fn run_sort_by_fan_in() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("main.rs"), "fn main() {}\n").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run_table(&cfg, OutputMode::Table, false, "fan-in", Some(20)).unwrap();
}

#[test]
fn run_sort_by_fan_out() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("main.rs"), "fn main() {}\n").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run_table(&cfg, OutputMode::Table, false, "fan-out", Some(20)).unwrap();
}

#[test]
fn run_cycles_only_filter() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("main.rs"), "fn main() {}\n").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run_table(&cfg, OutputMode::Table, true, "default", Some(20)).unwrap();
}

#[test]
fn run_cycles_only_json() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("main.rs"), "fn main() {}\n").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run_table(&cfg, OutputMode::Json, true, "default", Some(20)).unwrap();
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
    run_table(&cfg, OutputMode::Table, false, "default", Some(20)).unwrap();
}

#[test]
fn run_unreadable_file_gracefully_handled() {
    // Write a file with null bytes so read_to_string fails
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("binary.rs"), b"fn main() \x00{}").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    // Should not panic — errors are handled gracefully
    run_table(&cfg, OutputMode::Table, false, "default", Some(20)).unwrap();
}

#[test]
fn run_json_with_mod_deps() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("lib.rs"), "mod foo;\n").unwrap();
    fs::write(dir.path().join("foo.rs"), "pub fn f() {}\n").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run_table(&cfg, OutputMode::Json, false, "default", Some(20)).unwrap();
}

#[test]
fn run_short_format() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("main.rs"), "fn main() {}\n").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run_table(&cfg, OutputMode::Short, false, "default", Some(20)).unwrap();
}

#[test]
fn run_terse_format() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("main.rs"), "fn main() {}\n").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run_table(&cfg, OutputMode::Terse, false, "default", Some(20)).unwrap();
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

#[test]
fn kaikai_files_of_a_package_have_only_the_edges_of_their_imports() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "lib/kai.toml", "name = \"lib\"\n");
    write(
        dir.path(),
        "lib/one.kai",
        "import two\n\npub fn f() : Int = two.g()\n",
    );
    write(dir.path(), "lib/two.kai", "pub fn g() : Int = 1\n");
    write(dir.path(), "lib/three.kai", "pub fn h() : Int = 3\n");
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let result = analyze(&cfg);

    assert_eq!(metrics(&result, "lib/one.kai"), (0, 1, false));
    assert_eq!(metrics(&result, "lib/two.kai"), (1, 0, false));
    assert_eq!(metrics(&result, "lib/three.kai"), (0, 0, false));
}

// ── unsupported languages ────────────────────────────────────────────────────

#[test]
fn unsupported_languages_are_left_out_of_the_graph() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "lib.rs", "mod foo;\n");
    write(dir.path(), "foo.rs", "pub fn f() {}\n");
    write(dir.path(), "build.sh", "echo hi\n");
    write(dir.path(), "deploy.sh", "echo bye\n");
    write(dir.path(), "Main.kt", "class Main\n");
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
                language: "Kotlin".to_string(),
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
        run_table(&cfg, mode, false, "default", Some(20)).unwrap();
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
        ..DepEntry::default()
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

#[test]
fn json_lists_the_edges_that_leave_the_files_listed() {
    let entries = [entry("a.rs", 0, 1, false), entry("b.rs", 1, 1, false)];
    let edge = |from: &str, to: &str| edges::Edge {
        from: PathBuf::from(from),
        to: PathBuf::from(to),
        kind: graph::EdgeKind::Load,
    };
    let edges = [edge("a.rs", "b.rs"), edge("b.rs", "c.rs")];
    assert_eq!(
        edges::leaving(&edges, &[&entries[0]]),
        vec![edge("a.rs", "b.rs")]
    );
}

#[test]
fn go_modules_in_subdirectories_resolve_from_where_they_are() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "libs/core/go.mod", "module example.com/core\n");
    write(dir.path(), "libs/core/model/model.go", "package model\n");
    write(dir.path(), "libs/core/model/field.go", "package model\n");
    write(
        dir.path(),
        "libs/core/model/model_test.go",
        "package model\n",
    );
    write(dir.path(), "libs/core/model/sql/sql.go", "package sql\n");
    write(dir.path(), "svc/api/go.mod", "module example.com/api\n");
    write(
        dir.path(),
        "svc/api/main.go",
        "package main\n\nimport (\n\t\"fmt\"\n\t\"example.com/core/model\"\n)\n",
    );
    let filter = ExcludeFilter::default();
    // Tests are walked too: a test file is still not what a package offers.
    let cfg = WalkConfig::new(dir.path(), true, &filter);
    let result = analyze(&cfg);

    assert_eq!(metrics(&result, "svc/api/main.go"), (0, 2, false));
    assert_eq!(metrics(&result, "libs/core/model/model.go"), (1, 0, false));
    assert_eq!(metrics(&result, "libs/core/model/field.go"), (1, 0, false));
    assert_eq!(
        metrics(&result, "libs/core/model/model_test.go"),
        (0, 0, false)
    );
    assert_eq!(
        metrics(&result, "libs/core/model/sql/sql.go"),
        (0, 0, false)
    );
}

#[test]
fn jsx_and_tsx_files_enter_the_graph() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "src/App.tsx",
        "import { Button } from './Button';\nimport { label } from './label';\n",
    );
    write(
        dir.path(),
        "src/Button.jsx",
        "import { label } from './label';\n",
    );
    write(dir.path(), "src/label.ts", "export const label = 'ok';\n");
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let result = analyze(&cfg);

    assert_eq!(metrics(&result, "src/App.tsx"), (0, 2, false));
    assert_eq!(metrics(&result, "src/Button.jsx"), (1, 1, false));
    assert_eq!(metrics(&result, "src/label.ts"), (2, 0, false));
    assert!(result.unsupported.is_empty());
}

#[test]
fn typescript_aliases_and_workspace_packages_add_edges() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "apps/web/tsconfig.json",
        "{\n  // aliases\n  \"compilerOptions\": { \"paths\": { \"@/*\": [\"./src/*\"] } },\n}\n",
    );
    write(
        dir.path(),
        "apps/web/src/app.tsx",
        "import { Button } from '@/components/Button';\nimport { slug } from '@acme/utils';\nimport React from 'react';\n",
    );
    write(
        dir.path(),
        "apps/web/src/components/Button.tsx",
        "export const Button = 1;\n",
    );
    write(
        dir.path(),
        "packages/utils/package.json",
        "{\"name\": \"@acme/utils\", \"main\": \"dist/index.js\"}\n",
    );
    write(
        dir.path(),
        "packages/utils/src/index.ts",
        "export const slug = 1;\n",
    );
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let result = analyze(&cfg);

    assert_eq!(metrics(&result, "apps/web/src/app.tsx"), (0, 2, false));
    assert_eq!(
        metrics(&result, "apps/web/src/components/Button.tsx"),
        (1, 0, false)
    );
    assert_eq!(
        metrics(&result, "packages/utils/src/index.ts"),
        (1, 0, false)
    );
}

// ── imports that do not load, and those that name no file ─────────────────────

fn analyze_files(files: &[(&str, &str)]) -> DepResult {
    let dir = tempfile::tempdir().unwrap();
    for (path, text) in files {
        write(dir.path(), path, text);
    }
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    analyze(&cfg)
}

fn entry_of<'a>(result: &'a DepResult, path: &str) -> &'a DepEntry {
    let found = result.entries.iter().find(|e| e.path == Path::new(path));
    found.unwrap_or_else(|| panic!("{path} not in the graph"))
}

#[test]
fn typescript_import_type_makes_no_cycle() {
    let result = analyze_files(&[
        (
            "a.ts",
            "import type { B } from './b'\nexport function make(): B { return { n: 1 } }\n",
        ),
        (
            "b.ts",
            "import { make } from './a'\nexport interface B { n: number }\nexport const value = make()\n",
        ),
    ]);
    assert!(result.cycles.is_empty());
    let a = entry_of(&result, "a.ts");
    assert_eq!((a.fan_out, a.type_fan_out), (1, 1));
    assert_eq!(result.totals.type_edges, 1);
}

#[test]
fn python_type_checking_and_function_imports_make_no_cycle() {
    let result = analyze_files(&[
        ("pkg/__init__.py", ""),
        (
            "pkg/app.py",
            "from typing import TYPE_CHECKING\n\nif TYPE_CHECKING:\n    from pkg.ctx import Context\n\n\ndef run(ctx: \"Context\") -> None:\n    print(ctx)\n",
        ),
        (
            "pkg/ctx.py",
            "from pkg.app import run\n\n\nclass Context:\n    def go(self) -> None:\n        run(self)\n",
        ),
        (
            "pkg/helpers.py",
            "def current():\n    from pkg.views import render\n    return render()\n",
        ),
        (
            "pkg/views.py",
            "from pkg.helpers import current\n\n\ndef render():\n    return 1\n",
        ),
    ]);
    assert!(result.cycles.is_empty(), "{:?}", result.cycles);
    assert_eq!(entry_of(&result, "pkg/app.py").type_fan_out, 1);
    assert_eq!(entry_of(&result, "pkg/helpers.py").deferred_fan_out, 1);
}

#[test]
fn a_quoted_include_of_no_file_is_unresolved() {
    let result = analyze_files(&[
        (
            "src/main.c",
            "#include \"util.h\"\n#include \"builtin.inc\"\n#include <stdio.h>\n\nint main(void) { return util(); }\n",
        ),
        ("src/util.h", "int util(void);\n"),
    ]);
    assert_eq!(
        entry_of(&result, "src/main.c").unresolved,
        vec!["builtin.inc"]
    );
    assert_eq!(result.totals.unresolved, 1);
    assert_eq!(result.totals.files_with_unresolved, 1);
}

#[test]
fn a_relative_import_of_no_script_is_unresolved() {
    let result = analyze_files(&[(
        "a.js",
        "import x from './missing'\nimport './style.css'\nimport React from 'react'\nexport default x\n",
    )]);
    assert_eq!(entry_of(&result, "a.js").unresolved, vec!["./missing"]);
}

#[test]
fn a_relative_python_import_of_no_module_is_unresolved() {
    let result = analyze_files(&[
        ("pkg/__init__.py", ""),
        (
            "pkg/a.py",
            "from .gone import x\nfrom .b import y\nimport requests\n",
        ),
        ("pkg/b.py", "y = 1\n"),
    ]);
    assert_eq!(entry_of(&result, "pkg/a.py").unresolved, vec![".gone"]);
}

#[test]
fn a_go_import_of_a_package_of_the_module_with_no_files_is_unresolved() {
    let result = analyze_files(&[
        ("go.mod", "module example.com/app\n"),
        (
            "main.go",
            "package main\n\nimport (\n\t\"fmt\"\n\t\"example.com/app/gen\"\n\t\"example.com/app/util\"\n)\n",
        ),
        ("util/util.go", "package util\n"),
    ]);
    assert_eq!(
        entry_of(&result, "main.go").unresolved,
        vec!["example.com/app/gen"]
    );
}

#[test]
fn matrix_prints_as_a_table_or_json() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "a.py", "import b\n");
    write(dir.path(), "b.py", "x = 1\n");
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    matrix_output::run(&cfg, OutputMode::Table, &matrix_args(), &Expand::Depth(1)).unwrap();
    matrix_output::run(&cfg, OutputMode::Json, &matrix_args(), &Expand::Depth(1)).unwrap();
    let error =
        matrix_output::run(&cfg, OutputMode::Short, &matrix_args(), &Expand::Depth(1)).unwrap_err();
    assert!(
        error
            .to_string()
            .starts_with("--matrix prints a table or JSON")
    );
}

#[test]
fn matrix_focus_on_a_missing_directory_fails() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "a.py", "x = 1\n");
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let focus = Expand::Focus(PathBuf::from("nowhere"));
    assert!(matrix_output::run(&cfg, OutputMode::Table, &matrix_args(), &focus).is_err());
}

fn matrix_args() -> crate::cli::DepsArgs {
    crate::cli::DepsArgs {
        cycles_only: false,
        sort_by: "fan-out".to_string(),
        top: None,
        matrix: true,
        depth: None,
        focus: None,
        html: None,
    }
}

#[test]
fn matrix_html_is_written_to_the_file_given() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "a.py", "import b\n");
    write(dir.path(), "b.py", "x = 1\n");
    let out = tempfile::tempdir().unwrap();
    let page = out.path().join("deps.html");
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let args = crate::cli::DepsArgs {
        html: Some(page.clone()),
        ..matrix_args()
    };
    matrix_output::run(&cfg, OutputMode::Table, &args, &Expand::Depth(1)).unwrap();
    let html = fs::read_to_string(page).unwrap();
    // Paths are shown as every report shows them, here unrelated to a base.
    assert!(html.contains(r#"a.py",""#) && html.contains(r#"b.py"],"#));
    assert!(html.contains(r#""edges":[[0,1,true,false]]"#));
}

#[test]
fn documents_enter_the_graph_only_when_kind_asks() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "README.md",
        "[guide](docs/guide.md) [[notes]] [x](gone.md)\n",
    );
    write(dir.path(), "docs/guide.md", "[back](../README.md)\n");
    write(dir.path(), "docs/notes.md", "# Notes\n");
    write(dir.path(), "src/main.rs", "fn main() {}\n");

    let plain = ExcludeFilter::default();
    let result = analyze(&WalkConfig::new(dir.path(), true, &plain));
    assert!(result.entries.iter().all(|e| e.language != "Markdown"));

    let kinds = crate::kind::Selection::new(&[crate::kind::Kind::Docs], &Default::default());
    let docs = ExcludeFilter::default().with_kinds(kinds);
    let result = analyze(&WalkConfig::new(dir.path(), true, &docs));
    assert_eq!(metrics(&result, "README.md"), (1, 2, true));
    assert_eq!(metrics(&result, "docs/notes.md"), (1, 0, false));
    let readme = result
        .entries
        .iter()
        .find(|e| e.path == std::path::Path::new("README.md"))
        .unwrap();
    assert_eq!(readme.unresolved, ["gone.md"]);
}

#[test]
fn terraform_modules_enter_the_graph_when_kind_asks() {
    let dir = tempfile::tempdir().unwrap();
    let module = |source: &str| format!("module \"m\" {{\n  source = \"{source}\"\n}}\n");
    write(
        dir.path(),
        "modules/net/main.tf",
        "resource \"x\" \"y\" {}\n",
    );
    write(dir.path(), "modules/net/outputs.tf", "output \"id\" {}\n");
    write(
        dir.path(),
        "envs/prod/main.tf",
        &module("../../modules/net"),
    );
    write(
        dir.path(),
        "envs/prod/gone.tf",
        &module("../../modules/gone"),
    );

    let plain = ExcludeFilter::default();
    let result = analyze(&WalkConfig::new(dir.path(), true, &plain));
    assert!(result.entries.iter().all(|e| e.language != "Terraform"));

    let kinds = crate::kind::Selection::new(&[crate::kind::Kind::Infra], &Default::default());
    let infra = ExcludeFilter::default().with_kinds(kinds);
    let result = analyze(&WalkConfig::new(dir.path(), true, &infra));
    assert_eq!(metrics(&result, "envs/prod/main.tf"), (0, 2, false));
    assert_eq!(metrics(&result, "modules/net/outputs.tf"), (1, 0, false));
    let gone = result
        .entries
        .iter()
        .find(|e| e.path == std::path::Path::new("envs/prod/gone.tf"))
        .unwrap();
    assert_eq!(gone.unresolved, ["../../modules/gone"]);
}
