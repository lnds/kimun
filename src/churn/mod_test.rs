use crate::walk::{ExcludeFilter, WalkConfig};

#[test]
fn run_on_current_repo() {
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(std::path::Path::new("."), false, &filter);
    super::run(&cfg, false, 20, "commits", None).unwrap();
}

#[test]
fn run_json_on_current_repo() {
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(std::path::Path::new("."), false, &filter);
    super::run(&cfg, true, 20, "commits", None).unwrap();
}

#[test]
fn run_sort_by_rate() {
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(std::path::Path::new("."), false, &filter);
    super::run(&cfg, false, 20, "rate", None).unwrap();
}

#[test]
fn run_sort_by_file() {
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(std::path::Path::new("."), false, &filter);
    super::run(&cfg, false, 20, "file", None).unwrap();
}

#[test]
fn run_with_since() {
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(std::path::Path::new("."), false, &filter);
    super::run(&cfg, false, 20, "commits", Some("1y")).unwrap();
}
