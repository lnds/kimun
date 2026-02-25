//! AI-powered code analysis module.
//!
//! Implements an agentic loop that calls an LLM provider (currently Claude)
//! with access to `km` tool definitions. The LLM decides which tools to
//! run, receives the JSON output, and produces a comprehensive analysis.
//!
//! The loop iterates up to `MAX_ITERATIONS` turns, executing tool calls
//! on each turn until the model produces a final text response (no more
//! tool_use blocks). This design lets the model explore the codebase
//! incrementally, running multiple tools across several turns.

/// HTTP client and API types for the Claude Messages API.
mod client;
/// Tool executor: maps tool calls to `km` subprocess invocations.
mod executor;
/// Tool definitions: JSON Schema descriptions of all 11 `km` subcommands.
pub(crate) mod schema;
/// Claude Code skill installer (`km ai skill claude`).
pub mod skill;

use client::{ApiRequest, ContentBlock, Message, MessageContent};
use std::fs;
use std::path::Path;

/// Default model used when `--model` is not specified.
const DEFAULT_MODEL: &str = "claude-sonnet-4-5-20250929";
/// Maximum tokens per API response (prevents runaway generation).
const MAX_TOKENS: u32 = 4096;
/// Safety limit on agentic loop iterations to prevent infinite loops.
const MAX_ITERATIONS: usize = 15;

const SYSTEM_PROMPT: &str = "\
You are a code analysis expert. You have access to the `km` code metrics tool \
which can analyze a software repository across multiple dimensions: lines of code, \
complexity, maintainability, duplication, hotspots, ownership, and temporal coupling.

Analyze the repository using the available tools. Run the tools you consider most \
relevant, then provide a comprehensive analysis including:

1. **Overview**: Project size and language breakdown
2. **Code Health**: Overall score and grade interpretation
3. **Complexity Hotspots**: Files with highest complexity or risk
4. **Maintainability**: Files that are hardest to maintain
5. **Key Findings**: Notable patterns, risks, or areas for improvement
6. **Recommendations**: Prioritized, actionable suggestions

Be specific — reference file names and metrics. Keep the analysis concise but thorough.";

pub fn run(
    provider: &str,
    path: &Path,
    model: Option<&str>,
    output: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    if provider != "claude" {
        return Err(format!("Unsupported provider: {provider}. Supported: claude").into());
    }

    let api_key = std::env::var("ANTHROPIC_API_KEY").map_err(|_| {
        "ANTHROPIC_API_KEY environment variable not set. \
         Get your key at https://console.anthropic.com/"
    })?;

    let canonical_path = path
        .canonicalize()
        .map_err(|e| format!("Cannot resolve path '{}': {e}", path.display()))?;

    let model = model.unwrap_or(DEFAULT_MODEL).to_string();
    let tool_defs = schema::tool_definitions();

    agentic_loop(&api_key, &model, &tool_defs, &canonical_path, output)
}

/// Extract all text blocks from a response into a single string.
fn collect_final_text(content: &[ContentBlock]) -> String {
    let mut text = String::new();
    for block in content {
        if let ContentBlock::Text { text: t } = block {
            text.push_str(t);
        }
    }
    text
}

/// Save analysis text to a file, if an output path was provided.
fn save_output(text: &str, output: Option<&Path>) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(path) = output {
        fs::write(path, text)?;
        eprintln!("Report saved to {}", path.display());
    }
    Ok(())
}

/// Execute every tool-use block in `content` and return the corresponding results.
fn execute_tool_uses(content: &[ContentBlock], project_path: &Path) -> Vec<ContentBlock> {
    content
        .iter()
        .filter_map(|block| {
            if let ContentBlock::ToolUse { id, name, input } = block {
                eprintln!("  Running tool: {name}");
                let result = executor::execute_tool(name, input, project_path);
                Some(ContentBlock::ToolResult {
                    tool_use_id: id.clone(),
                    content: result,
                })
            } else {
                None
            }
        })
        .collect()
}

fn agentic_loop(
    api_key: &str,
    model: &str,
    tool_defs: &[serde_json::Value],
    project_path: &Path,
    output: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let http_client = client::build_client()?;
    let path_str = project_path.to_string_lossy();

    let mut messages = vec![Message {
        role: "user".to_string(),
        content: MessageContent::Text(format!("Analyze the repository at: {path_str}")),
    }];

    for iteration in 0..MAX_ITERATIONS {
        let request = ApiRequest {
            model: model.to_string(),
            max_tokens: MAX_TOKENS,
            system: SYSTEM_PROMPT.to_string(),
            tools: tool_defs.to_vec(),
            messages: messages.clone(),
        };

        eprintln!("Calling Claude API (turn {})...", iteration + 1);
        let response = client::send_message(&http_client, api_key, &request)?;

        if response.stop_reason == "max_tokens" {
            eprintln!("warning: response truncated (max_tokens reached)");
            return Err("API response was truncated due to max_tokens limit. \
                        Try again or increase MAX_TOKENS."
                .into());
        }

        let has_tool_use = response
            .content
            .iter()
            .any(|b| matches!(b, ContentBlock::ToolUse { .. }));

        if !has_tool_use {
            let analysis = collect_final_text(&response.content);
            println!("{analysis}");
            save_output(&analysis, output)?;
            return Ok(());
        }

        messages.push(Message {
            role: "assistant".to_string(),
            content: MessageContent::Blocks(response.content.clone()),
        });

        let tool_results = execute_tool_uses(&response.content, project_path);
        messages.push(Message {
            role: "user".to_string(),
            content: MessageContent::Blocks(tool_results),
        });
    }

    Err(format!("Exceeded maximum iterations ({MAX_ITERATIONS})").into())
}
