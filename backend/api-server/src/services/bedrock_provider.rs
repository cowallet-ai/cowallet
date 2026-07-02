use crate::services::ai_provider::*;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Claude provider via the right.codes Anthropic-compatible relay.
///
/// This speaks the standard Anthropic Messages API (`/v1/messages`) with
/// `x-api-key` auth and standard SSE streaming — NOT the AWS Bedrock binary
/// event-stream format. The struct name is kept as `BedrockProvider` so the
/// rest of the app (state wiring, `select_ai_provider`) is unchanged.
#[derive(Clone)]
pub struct BedrockProvider {
    client: Client,
    api_key: String,
    base_url: String,
    model_id: String,
}

impl BedrockProvider {
    pub async fn from_env() -> AiResult<Self> {
        // Accept either RIGHTCODES_API_KEY or the legacy BEDROCK_API_KEY.
        let api_key = std::env::var("RIGHTCODES_API_KEY")
            .or_else(|_| std::env::var("BEDROCK_API_KEY"))
            .map_err(|_| "RIGHTCODES_API_KEY (or BEDROCK_API_KEY) not set".to_string())?;
        let base_url = std::env::var("RIGHTCODES_BASE_URL")
            .unwrap_or_else(|_| "https://www.right.codes/deepseek/anthropic".into());
        let model_id = std::env::var("RIGHTCODES_MODEL")
            .or_else(|_| std::env::var("BEDROCK_MODEL_ID"))
            .unwrap_or_else(|_| "deepseek-v4-flash".into());

        let client = Client::new();
        tracing::info!("AI provider: right.codes DeepSeek (model={})", model_id);
        Ok(Self { client, api_key, base_url, model_id })
    }

    fn messages_url(&self) -> String {
        format!("{}/v1/messages", self.base_url.trim_end_matches('/'))
    }
}

// -- Request types (standard Anthropic Messages format) --

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<ApiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<ApiTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    stream: bool,
}

#[derive(Serialize, Clone)]
struct ApiMessage {
    role: String,
    content: Vec<ContentPart>,
}

#[derive(Serialize, Clone)]
#[serde(tag = "type")]
enum ContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse { id: String, name: String, input: serde_json::Value },
    #[serde(rename = "tool_result")]
    ToolResult { tool_use_id: String, content: String },
}

#[derive(Serialize)]
struct ApiTool {
    name: String,
    description: String,
    input_schema: serde_json::Value,
}

#[derive(Deserialize)]
struct ApiResponse {
    content: Vec<ResponseBlock>,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum ResponseBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse { id: String, name: String, input: serde_json::Value },
    // Ignore any other block types (e.g. thinking) gracefully.
    #[serde(other)]
    Other,
}

// -- Conversion helpers --

fn convert_messages(messages: &[ChatMessage]) -> (Option<String>, Vec<ApiMessage>) {
    let mut system_text: Option<String> = None;
    let mut api_msgs: Vec<ApiMessage> = Vec::new();

    for msg in messages {
        match msg.role {
            ChatRole::System => {
                system_text = msg.content.clone();
            }
            ChatRole::User => {
                let text = msg.content.as_deref().unwrap_or("");
                api_msgs.push(ApiMessage {
                    role: "user".into(),
                    content: vec![ContentPart::Text { text: text.into() }],
                });
            }
            ChatRole::Assistant => {
                let mut parts = Vec::new();
                if let Some(text) = &msg.content {
                    if !text.is_empty() {
                        parts.push(ContentPart::Text { text: text.clone() });
                    }
                }
                if let Some(tool_calls) = &msg.tool_calls {
                    for tc in tool_calls {
                        let input: serde_json::Value =
                            serde_json::from_str(&tc.arguments)
                                .unwrap_or(serde_json::Value::Object(Default::default()));
                        parts.push(ContentPart::ToolUse {
                            id: tc.id.clone(),
                            name: tc.name.clone(),
                            input,
                        });
                    }
                }
                if !parts.is_empty() {
                    api_msgs.push(ApiMessage { role: "assistant".into(), content: parts });
                }
            }
            ChatRole::Tool => {
                let content = msg.content.as_deref().unwrap_or("{}").to_string();
                let tool_id = msg.tool_call_id.as_deref().unwrap_or("unknown").to_string();
                api_msgs.push(ApiMessage {
                    role: "user".into(),
                    content: vec![ContentPart::ToolResult { tool_use_id: tool_id, content }],
                });
            }
        }
    }
    (system_text, api_msgs)
}

fn convert_tools(tools: &[ToolDef]) -> Option<Vec<ApiTool>> {
    if tools.is_empty() { return None; }
    Some(tools.iter().map(|t| ApiTool {
        name: t.name.clone(),
        description: t.description.clone(),
        input_schema: t.parameters.clone(),
    }).collect())
}

const MAX_TOKENS: u32 = 4096;

// -- AiProvider implementation --

#[async_trait::async_trait]
impl AiProvider for BedrockProvider {
    async fn chat(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDef],
        temperature: Option<f32>,
    ) -> AiResult<ChatResponse> {
        let (system, api_messages) = convert_messages(messages);
        let body = AnthropicRequest {
            model: self.model_id.clone(),
            max_tokens: MAX_TOKENS,
            system,
            messages: api_messages,
            tools: convert_tools(tools),
            temperature,
            stream: false,
        };

        let resp = self.client.post(self.messages_url())
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("right.codes API error {}: {}", status, text).into());
        }

        let api_resp: ApiResponse = resp.json().await?;
        let mut content = String::new();
        let mut tool_calls = Vec::new();

        for block in api_resp.content {
            match block {
                ResponseBlock::Text { text } => content.push_str(&text),
                ResponseBlock::ToolUse { id, name, input } => {
                    tool_calls.push(ToolCallInfo {
                        id, name,
                        arguments: serde_json::to_string(&input)
                            .unwrap_or_default(),
                    });
                }
                ResponseBlock::Other => {}
            }
        }

        Ok(ChatResponse {
            content: if content.is_empty() { None } else { Some(content) },
            tool_calls,
        })
    }

    async fn stream_chat(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDef],
        temperature: Option<f32>,
    ) -> AiResult<BoxStream<StreamEvent>> {
        let (system, api_messages) = convert_messages(messages);
        let body = AnthropicRequest {
            model: self.model_id.clone(),
            max_tokens: MAX_TOKENS,
            system,
            messages: api_messages,
            tools: convert_tools(tools),
            temperature,
            stream: true,
        };

        let resp = self.client.post(self.messages_url())
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("right.codes stream error {}: {}", status, text).into());
        }

        let stream = parse_anthropic_sse(resp);
        Ok(Box::pin(stream))
    }
}

// -- Standard Anthropic SSE parser --
// The relay returns text/event-stream: lines of `event: <type>` and
// `data: <json>`, separated by blank lines. We only need the `data:` JSON
// (it carries the `type` field too), so we parse each `data:` payload.

fn parse_anthropic_sse(resp: reqwest::Response) -> impl futures::Stream<Item = StreamEvent> {
    use futures::stream;

    let byte_stream = resp.bytes_stream();

    stream::unfold(
        (byte_stream, String::new(), StreamState::default(), false),
        |(mut bytes, mut buffer, mut state, mut done)| async move {
            if done {
                return None;
            }
            loop {
                // Emit a complete SSE data event if one is buffered.
                if let Some((evt, finished)) = next_sse_event(&mut buffer, &mut state) {
                    if finished {
                        done = true;
                    }
                    if let Some(evt) = evt {
                        return Some((evt, (bytes, buffer, state, done)));
                    }
                    if done {
                        return None;
                    }
                    continue;
                }
                match bytes.next().await {
                    Some(Ok(data)) => {
                        buffer.push_str(&String::from_utf8_lossy(&data));
                    }
                    Some(Err(e)) => {
                        tracing::error!("right.codes stream read error: {}", e);
                        return None;
                    }
                    None => return None,
                }
            }
        },
    )
}

/// Pull the next complete SSE line (terminated by `\n`) out of the buffer and,
/// if it's a `data:` line, parse it. Returns:
/// - `Some((Some(evt), finished))` when a stream event is produced,
/// - `Some((None, finished))` when a line was consumed but produced no event,
/// - `None` when no complete line is buffered yet (caller must read more).
fn next_sse_event(
    buffer: &mut String,
    state: &mut StreamState,
) -> Option<(Option<StreamEvent>, bool)> {
    let newline = buffer.find('\n')?;
    let line: String = buffer.drain(..=newline).collect();
    let line = line.trim_end_matches(['\r', '\n']);

    let data = match line.strip_prefix("data:") {
        Some(rest) => rest.trim(),
        None => return Some((None, false)), // event:/blank/comment line — ignore
    };

    if data.is_empty() {
        return Some((None, false));
    }

    match parse_event_json(data, state) {
        Some(StreamEvent::Done) => Some((Some(StreamEvent::Done), true)),
        Some(evt) => Some((Some(evt), false)),
        None => Some((None, false)),
    }
}

// -- Event JSON types (standard Anthropic streaming events) --

#[derive(Default)]
struct StreamState {
    current_tool_id: Option<String>,
    current_tool_name: Option<String>,
    current_tool_input: String,
}

#[derive(Deserialize)]
struct EventData {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    delta: Option<DeltaBlock>,
    #[serde(default)]
    content_block: Option<ContentBlockStart>,
}

#[derive(Deserialize)]
struct DeltaBlock {
    #[serde(rename = "type")]
    #[serde(default)]
    delta_type: Option<String>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    partial_json: Option<String>,
}

#[derive(Deserialize)]
struct ContentBlockStart {
    #[serde(rename = "type")]
    #[serde(default)]
    block_type: Option<String>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
}

fn parse_event_json(json: &str, state: &mut StreamState) -> Option<StreamEvent> {
    let parsed: EventData = serde_json::from_str(json).ok()?;

    match parsed.event_type.as_str() {
        "content_block_start" => {
            if let Some(block) = &parsed.content_block {
                if block.block_type.as_deref() == Some("tool_use") {
                    state.current_tool_id = block.id.clone();
                    state.current_tool_name = block.name.clone();
                    state.current_tool_input.clear();
                }
            }
            None
        }
        "content_block_delta" => {
            if let Some(delta) = &parsed.delta {
                match delta.delta_type.as_deref() {
                    Some("text_delta") => {
                        delta.text.as_ref().map(|t| StreamEvent::Token(t.clone()))
                    }
                    Some("input_json_delta") => {
                        if let Some(json) = &delta.partial_json {
                            state.current_tool_input.push_str(json);
                        }
                        None
                    }
                    _ => None,
                }
            } else {
                None
            }
        }
        "content_block_stop" => {
            if state.current_tool_id.is_some() {
                let evt = StreamEvent::ToolCall(ToolCallInfo {
                    id: state.current_tool_id.take().unwrap_or_default(),
                    name: state.current_tool_name.take().unwrap_or_default(),
                    arguments: std::mem::take(&mut state.current_tool_input),
                });
                Some(evt)
            } else {
                None
            }
        }
        "message_stop" => Some(StreamEvent::Done),
        _ => None,
    }
}
