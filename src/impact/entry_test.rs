use super::*;

fn entry_points(patterns: &[&str]) -> EntryPoints {
    let patterns: Vec<String> = patterns.iter().map(|p| p.to_string()).collect();
    EntryPoints::new(&patterns).unwrap()
}

#[test]
fn command_line_tasks_are_entry_points_by_convention() {
    let entry = entry_points(&[]);
    assert!(entry.conventional(Path::new("shop/lib/mix/tasks/seed.ex")));
    assert!(entry.conventional(Path::new("shop/management/commands/seed.py")));
    assert!(entry.conventional(Path::new("scripts/seed.ts")));
    assert!(entry.conventional(Path::new("tools/bin/seed.js")));
}

#[test]
fn what_a_rust_package_runs_is_an_entry_point() {
    let entry = entry_points(&[]);
    for run in [
        "cli/src/main.rs",
        "src/main.rs",
        "shop/src/bin/seed.rs",
        "shop/examples/demo.rs",
        "shop/benches/totals.rs",
        "shop/build.rs",
    ] {
        assert!(entry.conventional(Path::new(run)), "{run}");
    }
    for used in [
        "shop/src/lib.rs",
        "shop/src/main_menu.rs",
        "shop/src/rebuild.rs",
    ] {
        assert!(!entry.conventional(Path::new(used)), "{used}");
    }
}

#[test]
fn what_python_runs_is_an_entry_point() {
    let entry = entry_points(&[]);
    for run in ["shop/__main__.py", "setup.py", "site/manage.py"] {
        assert!(entry.conventional(Path::new(run)), "{run}");
    }
    for used in ["shop/__init__.py", "shop/main.py", "shop/setup_tools.py"] {
        assert!(!entry.conventional(Path::new(used)), "{used}");
    }
}

#[test]
fn a_directory_that_only_shares_part_of_the_name_is_not_one() {
    let entry = entry_points(&[]);
    assert!(!entry.conventional(Path::new("shop/lib/shop/tasks/seed.ex")));
    assert!(!entry.conventional(Path::new("shop/lib/mix/seed.ex")));
    assert!(!entry.conventional(Path::new("shop/lib/shop/cart.ex")));
}

#[test]
fn a_file_named_like_the_directory_is_not_one() {
    assert!(!entry_points(&[]).conventional(Path::new("shop/lib/scripts")));
}

#[test]
fn configured_globs_are_told_from_the_convention() {
    let entry = entry_points(&["**/endpoint.ex"]);
    assert!(entry.configured(Path::new("shop/lib/shop_web/endpoint.ex")));
    assert!(!entry.conventional(Path::new("shop/lib/shop_web/endpoint.ex")));
    assert!(!entry.configured(Path::new("scripts/seed.ts")));
    assert!(!entry.configured(Path::new("shop/lib/shop_web/router.ex")));
}

#[test]
fn an_invalid_glob_names_the_key() {
    let error = EntryPoints::new(&["a{".to_string()]).err().unwrap();
    assert!(error.to_string().contains("[impact] entry_points"));
}
