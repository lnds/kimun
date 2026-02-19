//! Tool executor for AI-powered analysis.
//!
//! Maps tool call names (e.g. `"cm_loc"`) to `cm` subprocess invocations.
//! Each tool runs `cm <subcmd> --json <path>` and returns the stdout output.
//! Named parameters from the AI input (like `top`, `since`, `min_lines`)
//! are converted to `--flag value` CLI arguments.

use serde_json::Value;
use std::path::Path;
use std::process::Command;

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
