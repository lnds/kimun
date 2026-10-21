use std::fs;
use std::path::Path;

use tempfile::tempdir;

use super::*;
use crate::config::KindsConfig;
use crate::loc::language::detect;

fn kind_by_name(path: &str) -> Kind {
    let path = Path::new(path);
    Kind::by_name(path, detect(path).expect("a known language"))
}

#[test]
fn languages_give_most_kinds() {
    assert_eq!(kind_by_name("src/lib.rs"), Kind::Code);
    assert_eq!(kind_by_name("Makefile"), Kind::Code);
    assert_eq!(kind_by_name("README.md"), Kind::Docs);
    assert_eq!(kind_by_name("docs/index.rst"), Kind::Docs);
    assert_eq!(kind_by_name("infra/main.tf"), Kind::Infra);
    assert_eq!(kind_by_name("Dockerfile"), Kind::Infra);
    assert_eq!(kind_by_name("package.json"), Kind::Data);
    assert_eq!(kind_by_name("Cargo.toml"), Kind::Data);
}

#[test]
fn yaml_is_infrastructure_by_where_it_sits() {
    assert_eq!(kind_by_name(".github/workflows/ci.yml"), Kind::Infra);
    assert_eq!(kind_by_name(".gitlab-ci.yml"), Kind::Infra);
    assert_eq!(kind_by_name("docker-compose.prod.yml"), Kind::Infra);
    assert_eq!(kind_by_name("compose.yaml"), Kind::Infra);
    assert_eq!(kind_by_name("charts/api/Chart.yaml"), Kind::Infra);
    assert_eq!(kind_by_name(".circleci/config.yml"), Kind::Infra);
    assert_eq!(kind_by_name("roles/web/tasks/main.yml"), Kind::Infra);
    assert_eq!(kind_by_name("config/settings.yml"), Kind::Data);
    assert_eq!(kind_by_name(".github/dependabot.yml"), Kind::Data);
}

#[test]
fn yaml_is_infrastructure_by_what_it_holds() {
    let lines = |text: &str| text.lines().map(String::from).collect::<Vec<_>>();
    assert!(yaml_holds_infra(&lines(
        "---\napiVersion: apps/v1\nkind: Deployment\n"
    )));
    assert!(yaml_holds_infra(&lines(
        "AWSTemplateFormatVersion: '2010-09-09'\n"
    )));
    assert!(yaml_holds_infra(&lines("- hosts: web\n  tasks: []\n")));
    assert!(yaml_holds_infra(&lines("- name: setup\n  hosts: all\n")));
    assert!(!yaml_holds_infra(&lines("kind: fruit\nname: apple\n")));
    assert!(!yaml_holds_infra(&lines("servers:\n  hosts: [a, b]\n")));
}

#[test]
fn a_file_on_disk_is_read_when_its_path_does_not_tell() {
    let dir = tempdir().unwrap();
    let manifest = dir.path().join("svc.yaml");
    let settings = dir.path().join("settings.yaml");
    fs::write(&manifest, "apiVersion: v1\nkind: Service\n").unwrap();
    fs::write(&settings, "a: 1\n").unwrap();
    let yaml = detect(&manifest).unwrap();
    assert_eq!(Kind::of(&manifest, yaml), Kind::Infra);
    assert_eq!(Kind::of(&settings, yaml), Kind::Data);
}

#[test]
fn declared_kinds_come_first() {
    let config = KindsConfig {
        docs: vec!["notes/**".into()],
        infra: vec!["deploy/**/*.yml".into()],
        ..Default::default()
    };
    let selection = Selection::new(&[], &config);
    let root = Path::new("/repo");
    let kind = |path: &str| {
        let path = root.join(path);
        selection.kind(&path, detect(&path).unwrap(), root)
    };
    assert_eq!(kind("notes/todo.txt"), Kind::Docs);
    assert_eq!(kind("notes/script.py"), Kind::Docs);
    assert_eq!(kind("deploy/prod/app.yml"), Kind::Infra);
    assert_eq!(kind("src/app.yml"), Kind::Data);
}

#[test]
fn without_a_choice_code_and_infrastructure_in_a_language_of_code_are_measured() {
    let selection = Selection::new(&[], &KindsConfig::default());
    let tf = detect(Path::new("main.tf")).unwrap();
    let yaml = detect(Path::new("ci.yml")).unwrap();
    let md = detect(Path::new("README.md")).unwrap();
    let rs = detect(Path::new("lib.rs")).unwrap();
    assert!(selection.measures(Kind::Code, rs));
    assert!(selection.measures(Kind::Infra, tf));
    assert!(!selection.measures(Kind::Infra, yaml));
    assert!(!selection.measures(Kind::Docs, md));
    assert!(!selection.measures(Kind::Data, yaml));
    assert!(selection.reads(Kind::Docs));
}

#[test]
fn separate_leaves_infrastructure_out_of_code() {
    let config = KindsConfig {
        separate: true,
        ..Default::default()
    };
    let selection = Selection::new(&[], &config);
    let tf = detect(Path::new("main.tf")).unwrap();
    assert!(!selection.measures(Kind::Infra, tf));
    assert!(selection.reads(Kind::Infra));
}

#[test]
fn a_choice_reads_only_its_kinds() {
    let selection = Selection::new(&[Kind::Code, Kind::Docs], &KindsConfig::default());
    let tf = detect(Path::new("main.tf")).unwrap();
    let md = detect(Path::new("README.md")).unwrap();
    assert!(!selection.measures(Kind::Infra, tf));
    assert!(selection.measures(Kind::Docs, md));
    assert!(!selection.reads(Kind::Data));
    assert!(selection.measures_path(Path::new("assets/logo.png"), Path::new("")));
    let docs = Selection::new(&[Kind::Docs], &KindsConfig::default());
    assert!(!docs.measures_path(Path::new("assets/logo.png"), Path::new("")));
}

#[test]
fn paths_of_history_have_a_kind_and_across_skips_data() {
    let config = KindsConfig {
        docs: vec!["notes/**".into()],
        ..Default::default()
    };
    let selection = Selection::new(&[], &config);
    let kind = |path: &str| selection.kind_of_path(Path::new(path), Path::new(""));
    assert_eq!(kind("src/lib.rs"), Some(Kind::Code));
    assert_eq!(kind("notes/run.py"), Some(Kind::Docs));
    assert_eq!(kind("infra/main.tf"), Some(Kind::Infra));
    assert_eq!(kind("assets/logo.png"), None);
    assert!(selection.across(Kind::Infra) && !selection.across(Kind::Data));
    let chosen = Selection::new(&[Kind::Code, Kind::Data], &KindsConfig::default());
    assert!(chosen.across(Kind::Data) && !chosen.across(Kind::Docs));
}
