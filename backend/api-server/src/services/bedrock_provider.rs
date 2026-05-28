use crate::services::ai_provider::*;
use base64::Engine;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Claude provider via AWS Bedrock (API Key + Bearer token auth).
#[derive(Clone)]
pub struct BedrockProvider {
    client: Client,
    api_key: String,
    region: String,
    model_id: String,
}

impl BedrockProvider {
    pub async fn from_env() -> AiResult<Self> {
        let api_key = std::env::var("BEDROCK_API_KEY")
            .map_err(|_| "BEDROCK_API_KEY not set".to_string())?;
        let region = std::env::var("BEDROCK_REGION")
            .unwrap_or_else(|_| "us-east-1".into());
        let model_id = std::env::var("BEDROCK_MODEL_ID")
            .unwrap_or_else(|_| "anthropic.claude-sonnet-4-20250514-v1:0".into());

        let client = Client::new();
        tracing::info!("AI provider: Bedrock Claude (region={}, model={})", region, model_id);
        Ok(Self { client, api_key, region, model_id })
    }

    fn base_url(&self) -> String {
        format!("https://bedrock-runtime.{}.amazonaws.com", self.region)
    }
}

// -- Request types (Bedrock Claude Messages format) --

#[derive(Serialize)]
struct BedrockRequest {
    anthropic_version: String,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<ApiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<ApiTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
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
        let body = BedrockRequest {
            anthropic_version: "bedrock-2023-05-31".into(),
            max_tokens: 4096,
            system,
            messages: api_messages,
            tools: convert_tools(tools),
            temperature,
        };

        let url = format!(
            "{}/model/{}/invoke",
            self.base_url(),
            self.model_id
        );

        let resp = self.client.post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Bedrock API error {}: {}", status, text).into());
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
        let body = BedrockRequest {
            anthropic_version: "bedrock-2023-05-31".into(),
            max_tokens: 4096,
            system,
            messages: api_messages,
            tools: convert_tools(tools),
            temperature,
        };

        let url = format!(
            "{}/model/{}/invoke-with-response-stream",
            self.base_url(),
            self.model_id
        );

        let resp = self.client.post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Bedrock stream error {}: {}", status, text).into());
        }

        let stream = parse_bedrock_sse(resp);
        Ok(Box::pin(stream))
    }
}

// -- AWS Event Stream parser --
// Bedrock returns binary event stream frames, each containing:
// - Binary headers (event-type, content-type, message-type)
// - JSON body: {"bytes":"<base64-encoded-event-json>","p":"..."}

fn parse_bedrock_sse(resp: reqwest::Response) -> impl futures::Stream<Item = StreamEvent> {
    use futures::stream;

    let byte_stream = resp.bytes_stream();

    stream::unfold(
        (byte_stream, Vec::<u8>::new(), StreamState::default()),
        |(mut bytes, mut buffer, mut state)| async move {
            use futures::StreamExt;
            loop {
                let events = extract_events_from_buffer(&mut buffer, &mut state);
                if let Some(evt) = events {
                    return Some((evt, (bytes, buffer, state)));
                }
                match bytes.next().await {
                    Some(Ok(data)) => {
                        buffer.extend_from_slice(&data);
                    }
                    Some(Err(e)) => {
                        tracing::error!("Bedrock stream read error: {}", e);
                        return None;
                    }
                    None => return None,
                }
            }
        },
    )
}

fn extract_events_from_buffer(
    buffer: &mut Vec<u8>,
    state: &mut StreamState,
) -> Option<StreamEvent> {
    let text = String::from_utf8_lossy(buffer);
    let json_start = text.find("{\"bytes\":\"")?;
    let after_key = json_start + 10; // skip {"bytes":"
    let b64_end = text[after_key..].find('"')?;
    let b64_str = &text[after_key..after_key + b64_end];

    // Find the closing } of the JSON object after the base64 string
    let after_b64 = after_key + b64_end + 1; // skip closing quote
    let frame_end = text[after_b64..].find('}')? + after_b64 + 1;

    let decoded = base64::engine::general_purpose::STANDARD
        .decode(b64_str)
        .ok()?;
    let event_json = String::from_utf8(decoded).ok()?;

    // Consume everything up to and including this frame
    *buffer = buffer[frame_end..].to_vec();

    parse_event_json(&event_json, state)
}

// -- Event JSON types --

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
