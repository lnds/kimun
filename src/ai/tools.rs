use serde_json::{Value, json};
use std::path::Path;
use std::process::Command;

/// JSON schema property for the `path` parameter shared by all tools.
fn path_prop() -> Value {
    json!({"type": "string", "description": "Directory to analyze (default: project root)"})
}

/// JSON schema property for the `top` parameter (limit number of results).
fn top_prop() -> Value {
    json!({"type": "integer", "description": "Show only the top N files (default: 20)"})
}

/// JSON schema property for the `since` parameter (time range filter).
fn since_prop() -> Value {
    json!({"type": "string", "description": "Only consider commits since this time (e.g. 6m, 1y, 30d)"})
}

/// Build a tool definition JSON object with the standard `path` property
/// plus any extra properties specific to that tool.
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

/// Return the list of all 11 `cm` tool definitions for the AI provider,
/// each with name, description, and input JSON schema.
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

/// Execute a `cm` subcommand by name, passing `--json` and any extra arguments
/// extracted from the AI tool input. Returns the JSON output or an error message.
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

/// Resolve the `path` field from the AI input to a safe, canonical path
/// within the project root. Falls back to the project root if the path
/// is missing, invalid, or outside the project.
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

/// Build the CLI argument list from the AI tool input. Converts named
/// parameters to `--flag value` pairs and appends the resolved path.
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
#[path = "tools_test.rs"]
mod tests;
