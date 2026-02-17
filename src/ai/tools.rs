use serde_json::{Value, json};
use std::path::Path;
use std::process::Command;

fn path_prop() -> Value {
    json!({"type": "string", "description": "Directory to analyze (default: project root)"})
}

fn top_prop() -> Value {
    json!({"type": "integer", "description": "Show only the top N files (default: 20)"})
}

fn since_prop() -> Value {
    json!({"type": "string", "description": "Only consider commits since this time (e.g. 6m, 1y, 30d)"})
}

fn tool(name: &str, desc: &str, extra_props: &[(&str, Value)]) -> Value {
    let mut props = serde_json::Map::new();
    props.insert("path".into(), path_prop());
    for (k, v) in extra_props {
        props.insert((*k).into(), v.clone());
    }
    json!({
        "name": name,
        "description": desc,
        "input_schema": {
            "type": "object",
            "properties": Value::Object(props),
            "required": []
        }
    })
}

pub fn tool_definitions() -> Vec<Value> {
    vec![
        tool(
            "cm_loc",
            "Count lines of code (blank, comment, code) by language. Returns per-language breakdown with totals.",
            &[],
        ),
        tool(
            "cm_score",
            "Compute an overall code health score (A++ to F--) across 6 dimensions: maintainability, complexity, duplication, indentation, Halstead effort, and file size.",
            &[],
        ),
        tool(
            "cm_hal",
            "Analyze Halstead complexity metrics per file: volume, difficulty, effort, estimated bugs, and development time.",
            &[("top", top_prop())],
        ),
        tool(
            "cm_cycom",
            "Analyze cyclomatic complexity per file: total, max, and average complexity with per-function breakdown.",
            &[("top", top_prop())],
        ),
        tool(
            "cm_indent",
            "Analyze indentation complexity per file: standard deviation and max depth of indentation.",
            &[],
        ),
        tool(
            "cm_mi",
            "Compute Maintainability Index per file (Visual Studio variant, 0-100 scale). Green (20-100), Yellow (10-19), Red (0-9).",
            &[("top", top_prop())],
        ),
        tool(
            "cm_miv",
            "Compute Maintainability Index per file (verifysoft variant, with comment weight). Good (85+), Moderate (65-84), Difficult (<65).",
            &[("top", top_prop())],
        ),
        tool(
            "cm_dups",
            "Detect duplicate code blocks across files. Shows duplicate percentage and group details.",
            &[(
                "min_lines",
                json!({"type": "integer", "description": "Minimum lines for a duplicate block (default: 6)"}),
            )],
        ),
        tool(
            "cm_hotspots",
            "Find hotspots: files that change frequently AND have high complexity. Score = commits x complexity. Requires git repository.",
            &[("top", top_prop()), ("since", since_prop())],
        ),
        tool(
            "cm_knowledge",
            "Analyze code ownership patterns via git blame (knowledge maps). Shows primary owner, concentration, and knowledge loss risk per file.",
            &[
                ("top", top_prop()),
                (
                    "since",
                    json!({"type": "string", "description": "Only consider recent activity since this time for knowledge loss detection (e.g. 6m, 1y, 30d)"}),
                ),
            ],
        ),
        tool(
            "cm_tc",
            "Analyze temporal coupling: files that change together in commits. Shows coupling strength between file pairs. Requires git repository.",
            &[
                (
                    "top",
                    json!({"type": "integer", "description": "Show only the top N file pairs (default: 20)"}),
                ),
                ("since", since_prop()),
            ],
        ),
    ]
}

pub fn execute_tool(tool_name: &str, input: &Value, project_path: &Path) -> String {
    let cm_binary = std::env::current_exe().unwrap_or_else(|_| "cm".into());

    let (subcmd, args) = match tool_name {
        "cm_loc" => ("loc", build_args(input, &[], project_path)),
        "cm_score" => ("score", build_args(input, &[], project_path)),
        "cm_hal" => ("hal", build_args(input, &["top"], project_path)),
        "cm_cycom" => ("cycom", build_args(input, &["top"], project_path)),
        "cm_indent" => ("indent", build_args(input, &[], project_path)),
        "cm_mi" => ("mi", build_args(input, &["top"], project_path)),
        "cm_miv" => ("miv", build_args(input, &["top"], project_path)),
        "cm_dups" => ("dups", build_args(input, &["min_lines"], project_path)),
        "cm_hotspots" => (
            "hotspots",
            build_args(input, &["top", "since"], project_path),
        ),
        "cm_knowledge" => (
            "knowledge",
            build_args(input, &["top", "since"], project_path),
        ),
        "cm_tc" => ("tc", build_args(input, &["top", "since"], project_path)),
        _ => return format!("Unknown tool: {tool_name}"),
    };

    let mut cmd = Command::new(&cm_binary);
    cmd.arg(subcmd).arg("--json");
    for arg in &args {
        cmd.arg(arg);
    }

    match cmd.output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            if output.status.success() {
                stdout.into_owned()
            } else {
                format!("Error running cm {subcmd}: {stderr}")
            }
        }
        Err(e) => format!("Failed to execute cm {subcmd}: {e}"),
    }
}

fn resolve_path(input: &Value, project_path: &Path) -> String {
    let raw = input.get("path").and_then(|v| v.as_str()).unwrap_or("");

    if raw.is_empty() {
        return project_path.to_string_lossy().into_owned();
    }

    let candidate = if Path::new(raw).is_absolute() {
        Path::new(raw).to_path_buf()
    } else {
        project_path.join(raw)
    };

    // Canonicalize to resolve symlinks and ../ components
    let resolved = match candidate.canonicalize() {
        Ok(p) => p,
        Err(_) => return project_path.to_string_lossy().into_owned(),
    };

    // Verify the resolved path is inside the project root
    if !resolved.starts_with(project_path) {
        eprintln!(
            "  Warning: path '{}' is outside project root, using project root instead",
            raw
        );
        return project_path.to_string_lossy().into_owned();
    }

    resolved.to_string_lossy().into_owned()
}

fn build_args(input: &Value, named: &[&str], project_path: &Path) -> Vec<String> {
    let mut args = Vec::new();

    for name in named {
        if let Some(val) = input.get(name) {
            let flag = format!("--{}", name.replace('_', "-"));
            match val {
                Value::Number(n) => {
                    args.push(flag);
                    args.push(n.to_string());
                }
                Value::String(s) => {
                    args.push(flag);
                    args.push(s.clone());
                }
                _ => {
                    eprintln!("  Warning: ignoring invalid value for --{name}: {val}");
                }
            }
        }
    }

    args.push(resolve_path(input, project_path));
    args
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let result = execute_tool("cm_unknown", &input, &project);
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
}
