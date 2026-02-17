/// HTTP client for the Anthropic Messages API.
///
/// Handles request serialization, authentication headers, timeout, and
/// response deserialization for the AI-assisted analysis feature.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

/// Anthropic Messages API endpoint.
const API_URL: &str = "https://api.anthropic.com/v1/messages";
/// API version header required by the Anthropic API.
const ANTHROPIC_VERSION: &str = "2023-06-01";
/// Maximum time to wait for a model response (5 minutes).
const REQUEST_TIMEOUT: Duration = Duration::from_secs(300);

/// Request payload for the Anthropic Messages API.
#[derive(Serialize)]
pub struct ApiRequest {
    pub model: String,
    pub max_tokens: u32,
    pub system: String,
    pub tools: Vec<Value>,
    pub messages: Vec<Message>,
}

/// A single message in the conversation (role + content).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Message {
    pub role: String,
    pub content: MessageContent,
}

/// Message content: either a plain text string or a list of content blocks
/// (text, tool_use, tool_result) for multi-turn tool interactions.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

/// A typed content block within a message: text, tool invocation, or tool result.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
    },
}

/// Response from the Anthropic Messages API.
#[derive(Deserialize, Debug)]
pub struct ApiResponse {
    pub content: Vec<ContentBlock>,
    #[allow(dead_code)]
    pub stop_reason: String,
}

/// Send a request to the Anthropic Messages API and return the parsed response.
/// Fails with a descriptive error on HTTP errors or deserialization failures.
pub fn send_message(
    api_key: &str,
    request: &ApiRequest,
) -> Result<ApiResponse, Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()?;
    let resp = client
        .post(API_URL)
        .header("x-api-key", api_key)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .header("content-type", "application/json")
        .json(request)
        .send()?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().unwrap_or_default();
        return Err(format!("API error ({status}): {body}").into());
    }

    let response: ApiResponse = resp.json()?;
    Ok(response)
}
