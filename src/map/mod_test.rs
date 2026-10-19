use std::fs;

use super::*;
use crate::walk::ExcludeFilter;

fn project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("core")).unwrap();
    fs::write(
        dir.path().join("main.py"),
        "import core.a\n\ndef run():\n    return core.a.f()\n",
    )
    .unwrap();
    fs::write(dir.path().join("core/a.py"), "def f():\n    return 1\n").unwrap();
    dir
}

fn options(html: Option<&Path>, color_by: ColorBy) -> Options<'_> {
    Options {
        html,
        color_by,
        since: None,
    }
}

#[test]
fn the_page_is_written_with_the_layout_inside() {
    let dir = project();
    let out = tempfile::tempdir().unwrap();
    let page = out.path().join("map.html");
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(
        &cfg,
        OutputMode::Table,
        &options(Some(&page), ColorBy::Score),
    )
    .unwrap();
    let html = fs::read_to_string(page).unwrap();
    assert!(html.contains("<title>Code map · "));
    assert!(html.contains(r#""metric":"health score""#));
    assert!(html.contains(r#""name":"a.py""#));
}

#[test]
fn json_gives_the_laid_out_tree_and_cycles_need_no_history() {
    let dir = project();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Json, &options(None, ColorBy::Cycles)).unwrap();
}

#[test]
fn a_table_needs_a_page_to_write() {
    let dir = project();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let error = run(&cfg, OutputMode::Table, &options(None, ColorBy::Score)).unwrap_err();
    assert!(error.to_string().contains("--html FILE"));
    assert!(run(&cfg, OutputMode::Short, &options(None, ColorBy::Score)).is_err());
}

#[test]
fn git_metrics_outside_a_repository_fail() {
    let dir = project();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let error = run(&cfg, OutputMode::Json, &options(None, ColorBy::Churn)).unwrap_err();
    assert!(error.to_string().contains("git repository"));
}
