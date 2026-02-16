use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

const API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Serialize)]
pub struct ApiRequest {
    pub model: String,
    pub max_tokens: u32,
    pub system: String,
    pub tools: Vec<Value>,
    pub messages: Vec<Message>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Message {
    pub role: String,
    pub content: MessageContent,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

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

#[derive(Deserialize, Debug)]
pub struct ApiResponse {
    pub content: Vec<ContentBlock>,
    #[allow(dead_code)]
    pub stop_reason: String,
}

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
