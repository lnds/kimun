mod client;
mod tools;

use client::{ApiRequest, ContentBlock, Message, MessageContent};
use std::fs;
use std::path::Path;

const DEFAULT_MODEL: &str = "claude-sonnet-4-5-20250929";
const MAX_TOKENS: u32 = 4096;
const MAX_ITERATIONS: usize = 15;

const SYSTEM_PROMPT: &str = "\
You are a code analysis expert. You have access to the `cm` code metrics tool \
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
    let tool_defs = tools::tool_definitions();

    agentic_loop(&api_key, &model, &tool_defs, &canonical_path, output)
}

fn agentic_loop(
    api_key: &str,
    model: &str,
    tool_defs: &[serde_json::Value],
    project_path: &Path,
    output: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
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
        let response = client::send_message(api_key, &request)?;

        // Collect tool use blocks
        let has_tool_use = response
            .content
            .iter()
            .any(|b| matches!(b, ContentBlock::ToolUse { .. }));

        if !has_tool_use {
            // No tools requested — collect final text
            let mut analysis = String::new();
            for block in &response.content {
                if let ContentBlock::Text { text } = block {
                    analysis.push_str(text);
                }
            }
            println!("{analysis}");
            if let Some(out_path) = output {
                fs::write(out_path, &analysis)?;
                eprintln!("Report saved to {}", out_path.display());
            }
            return Ok(());
        }

        // Append assistant message
        messages.push(Message {
            role: "assistant".to_string(),
            content: MessageContent::Blocks(response.content.clone()),
        });

        // Execute tools and collect results
        let mut tool_results = Vec::new();
        for block in &response.content {
            if let ContentBlock::ToolUse { id, name, input } = block {
                eprintln!("  Running tool: {name}");
                let result = tools::execute_tool(name, input, project_path);
                tool_results.push(ContentBlock::ToolResult {
                    tool_use_id: id.clone(),
                    content: result,
                });
            }
        }

        messages.push(Message {
            role: "user".to_string(),
            content: MessageContent::Blocks(tool_results),
        });
    }

    Err(format!("Exceeded maximum iterations ({MAX_ITERATIONS})").into())
}
