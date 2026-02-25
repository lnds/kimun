use super::*;
use crate::ai::schema::tool_definitions;
use serde_json::json;
use std::path::PathBuf;

#[test]
fn build_args_no_named_uses_project_path() {
    let project = PathBuf::from("/tmp/project");
    let input = json!({});
    let args = build_args(&input, &[], &project);
    assert_eq!(args, vec!["/tmp/project"]);
}

#[test]
fn build_args_with_named_integer() {
    let project = PathBuf::from("/tmp/project");
    let input = json!({"top": 10});
    let args = build_args(&input, &["top"], &project);
    assert_eq!(args, vec!["--top", "10", "/tmp/project"]);
}

#[test]
fn build_args_with_named_string() {
    let project = PathBuf::from("/tmp/project");
    let input = json!({"since": "6m"});
    let args = build_args(&input, &["since"], &project);
    assert_eq!(args, vec!["--since", "6m", "/tmp/project"]);
}

#[test]
fn build_args_ignores_missing_named() {
    let project = PathBuf::from("/tmp/project");
    let input = json!({});
    let args = build_args(&input, &["top", "since"], &project);
    assert_eq!(args, vec!["/tmp/project"]);
}

#[test]
fn resolve_path_empty_returns_project() {
    let project = PathBuf::from("/tmp/project");
    let input = json!({});
    assert_eq!(resolve_path(&input, &project), "/tmp/project");
}

#[test]
fn resolve_path_rejects_absolute_outside_project() {
    let project = std::env::current_dir().unwrap();
    let input = json!({"path": "/etc"});
    let result = resolve_path(&input, &project);
    assert_eq!(result, project.to_string_lossy());
}

#[test]
fn resolve_path_rejects_traversal() {
    let project = std::env::current_dir().unwrap();
    let input = json!({"path": "../../../../etc"});
    let result = resolve_path(&input, &project);
    assert_eq!(result, project.to_string_lossy());
}

#[test]
fn resolve_path_accepts_subdirectory() {
    let project = std::env::current_dir().unwrap();
    let input = json!({"path": "src"});
    let result = resolve_path(&input, &project);
    let expected = project.join("src").canonicalize().unwrap();
    assert_eq!(result, expected.to_string_lossy());
}

#[test]
fn resolve_path_nonexistent_falls_back() {
    let project = std::env::current_dir().unwrap();
    let input = json!({"path": "nonexistent_dir_xyz_12345"});
    let result = resolve_path(&input, &project);
    assert_eq!(result, project.to_string_lossy());
}

#[test]
fn execute_tool_unknown_returns_error() {
    let project = PathBuf::from("/tmp/project");
    let input = json!({});
    let result = execute_tool("km_unknown", &input, &project);
    assert!(result.starts_with("Unknown tool:"));
}

#[test]
fn tool_definitions_has_11_tools() {
    let defs = tool_definitions();
    assert_eq!(defs.len(), 11);
}

#[test]
fn tool_definitions_all_have_required_fields() {
    for def in tool_definitions() {
        assert!(def.get("name").is_some(), "missing name");
        assert!(def.get("description").is_some(), "missing description");
        assert!(def.get("input_schema").is_some(), "missing input_schema");
    }
}
