use super::*;
use crate::kind::Selection;
use crate::walk::ExcludeFilter;

fn walk_smells(dir: &std::path::Path, fail_on: &[Smell]) -> Result<(), Box<dyn Error>> {
    let filter = ExcludeFilter::default().with_kinds(Selection::new(
        &[Kind::Infra],
        &crate::config::KindsConfig::default(),
    ));
    let cfg = WalkConfig::new(dir, false, &filter);
    let options = SmellsOptions {
        top: None,
        since_ref: None,
        fail_on,
    };
    run(&cfg, OutputMode::Json, &options)
}

#[test]
fn reads_infrastructure_and_fails_on_what_is_asked() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("main.tf"), "password = \"hunter2\"\n").unwrap();
    std::fs::write(dir.path().join("Dockerfile"), "FROM alpine\n").unwrap();
    std::fs::write(dir.path().join("app.py"), "password = 'not infra'\n").unwrap();
    assert!(walk_smells(dir.path(), &[]).is_ok());
    assert!(walk_smells(dir.path(), &[Smell::EmptyPassword]).is_ok());
    let err = walk_smells(dir.path(), &[Smell::HardCodedSecret]).unwrap_err();
    assert!(err.to_string().contains("Hard-coded secret"));
}
