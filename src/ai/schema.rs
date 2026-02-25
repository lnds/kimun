//! Tool schema definitions for the AI analysis module.
//!
//! Defines the 11 `km` subcommands as Claude tool-use schemas in JSON Schema
//! format. Each tool has a name, description, and an `input_schema` object
//! with the shared `path` property plus any tool-specific parameters.
//! Shared property builders (`path_prop`, `top_prop`, `since_prop`) avoid
//! repeating the same JSON property definitions across tools.

use serde_json::{Value, json};

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

/// Return the list of all 11 `km` tool definitions for the AI provider,
/// each with name, description, and input JSON schema.
pub fn tool_definitions() -> Vec<Value> {
    vec![
        tool(
            "km_loc",
            "Count lines of code (blank, comment, code) by language. Returns per-language breakdown with totals.",
            &[],
        ),
        tool(
            "km_score",
            "Compute an overall code health score (A++ to F--) across 6 dimensions: maintainability, complexity, duplication, indentation, Halstead effort, and file size.",
            &[],
        ),
        tool(
            "km_hal",
            "Analyze Halstead complexity metrics per file: volume, difficulty, effort, estimated bugs, and development time.",
            &[("top", top_prop())],
        ),
        tool(
            "km_cycom",
            "Analyze cyclomatic complexity per file: total, max, and average complexity with per-function breakdown.",
            &[("top", top_prop())],
        ),
        tool(
            "km_indent",
            "Analyze indentation complexity per file: standard deviation and max depth of indentation.",
            &[],
        ),
        tool(
            "km_mi",
            "Compute Maintainability Index per file (Visual Studio variant, 0-100 scale). Green (20-100), Yellow (10-19), Red (0-9).",
            &[("top", top_prop())],
        ),
        tool(
            "km_miv",
            "Compute Maintainability Index per file (verifysoft variant, with comment weight). Good (85+), Moderate (65-84), Difficult (<65).",
            &[("top", top_prop())],
        ),
        tool(
            "km_dups",
            "Detect duplicate code blocks across files. Shows duplicate percentage and group details.",
            &[(
                "min_lines",
                json!({"type": "integer", "description": "Minimum lines for a duplicate block (default: 6)"}),
            )],
        ),
        tool(
            "km_hotspots",
            "Find hotspots: files that change frequently AND have high complexity. Score = commits x complexity. Requires git repository.",
            &[("top", top_prop()), ("since", since_prop())],
        ),
        tool(
            "km_knowledge",
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
            "km_tc",
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
