use crate::services::ai_provider::*;
use futures::StreamExt;
use reqwest::{header, Client};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// DeepSeek provider (OpenAI-compatible API).
#[derive(Clone)]
pub struct AiClient {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
}

// -- Internal OpenAI-format types --

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionDefinition,
}

#[derive(Debug, Clone, Serialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest<'a> {
    model: &'a str,
    messages: &'a [Message],
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<&'a [ToolDefinition]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ChatCompletionResponse {
    pub choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
pub struct Choice {
    pub message: ChoiceMessage,
}

#[derive(Debug, Deserialize)]
pub struct ChoiceMessage {
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub tool_calls: Option<Vec<ToolCall>>,
}

// -- Conversion helpers --

impl ChatMessage {
    pub fn to_openai(&self) -> Message {
        Message {
            role: match self.role {
                ChatRole::System => "system".into(),
                ChatRole::User => "user".into(),
                ChatRole::Assistant => "assistant".into(),
                ChatRole::Tool => "tool".into(),
            },
            content: self.content.clone(),
            reasoning_content: None,
            tool_calls: self.tool_calls.as_ref().map(|tcs| {
                tcs.iter()
                    .map(|tc| ToolCall {
                        id: tc.id.clone(),
                        call_type: "function".into(),
                        function: FunctionCall {
                            name: tc.name.clone(),
                            arguments: tc.arguments.clone(),
                        },
                    })
                    .collect()
            }),
            tool_call_id: self.tool_call_id.clone(),
        }
    }
}

fn tool_def_to_openai(t: &ToolDef) -> ToolDefinition {
    ToolDefinition {
        tool_type: "function".into(),
        function: FunctionDefinition {
            name: t.name.clone(),
            description: t.description.clone(),
            parameters: t.parameters.clone(),
        },
    }
}

impl AiClient {
    pub fn new(api_key: String, base_url: Option<String>, model: Option<String>) -> AiResult<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .connect_timeout(Duration::from_secs(10))
            .build()?;
        Ok(Self {
            client,
            api_key,
            base_url: base_url.unwrap_or_else(|| "https://api.deepseek.com".into()),
            model: model.unwrap_or_else(|| "deepseek-v4-flash".into()),
        })
    }

    pub fn from_env() -> AiResult<Self> {
        let api_key = std::env::var("DEEPSEEK_API_KEY")
            .ok()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| -> Box<dyn std::error::Error + Send + Sync> {
                "DEEPSEEK_API_KEY not set".into()
            })?;
        let base_url = std::env::var("DEEPSEEK_BASE_URL").ok();
        let model = std::env::var("DEEPSEEK_MODEL").ok();
        tracing::info!(
            "AI provider: DeepSeek (base={}, model={})",
            base_url.as_deref().unwrap_or("https://api.deepseek.com"),
            model.as_deref().unwrap_or("deepseek-v4-flash"),
        );
        Self::new(api_key, base_url, model)
    }

    fn convert_messages(messages: &[ChatMessage]) -> Vec<Message> {
        messages.iter().map(|m| m.to_openai()).collect()
    }

    fn convert_tools(tools: &[ToolDef]) -> Vec<ToolDefinition> {
        tools.iter().map(tool_def_to_openai).collect()
    }
}

#[async_trait::async_trait]
impl AiProvider for AiClient {
    async fn chat(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDef],
        temperature: Option<f32>,
    ) -> AiResult<ChatResponse> {
        let oai_msgs = Self::convert_messages(messages);
        let oai_tools = Self::convert_tools(tools);
        let url = format!("{}/v1/chat/completions", self.base_url);

        let request = ChatCompletionRequest {
            model: &self.model,
            messages: &oai_msgs,
            tools: if oai_tools.is_empty() {
                None
            } else {
                Some(&oai_tools)
            },
            tool_choice: if oai_tools.is_empty() {
                None
            } else {
                Some("auto")
            },
            temperature: Some(temperature.unwrap_or(0.7)),
            max_tokens: Some(4096),
            stream: None,
        };

        let resp = self
            .client
            .post(&url)
            .header(header::AUTHORIZATION, format!("Bearer {}", self.api_key))
            .header(header::CONTENT_TYPE, "application/json")
            .json(&request)
            .send()
            .await?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            return Err(format!("DeepSeek API error: {}", err).into());
        }

        let result: ChatCompletionResponse = resp.json().await?;
        let choice = result.choices.into_iter().next();
        Ok(ChatResponse {
            content: choice.as_ref().and_then(|c| c.message.content.clone()),
            tool_calls: choice
                .and_then(|c| c.message.tool_calls)
                .unwrap_or_default()
                .into_iter()
                .map(|tc| ToolCallInfo {
                    id: tc.id,
                    name: tc.function.name,
                    arguments: tc.function.arguments,
                })
                .collect(),
        })
    }

    async fn stream_chat(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDef],
        temperature: Option<f32>,
    ) -> AiResult<BoxStream<StreamEvent>> {
        let oai_msgs = Self::convert_messages(messages);
        let oai_tools = Self::convert_tools(tools);
        let url = format!("{}/v1/chat/completions", self.base_url);

        let request = ChatCompletionRequest {
            model: &self.model,
            messages: &oai_msgs,
            tools: if oai_tools.is_empty() {
                None
            } else {
                Some(&oai_tools)
            },
            tool_choice: if oai_tools.is_empty() {
                None
            } else {
                Some("auto")
            },
            temperature: Some(temperature.unwrap_or(0.7)),
            max_tokens: Some(4096),
            stream: Some(true),
        };

        let resp = self
            .client
            .post(&url)
            .header(header::AUTHORIZATION, format!("Bearer {}", self.api_key))
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::ACCEPT, "text/event-stream")
            .json(&request)
            .send()
            .await?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            return Err(format!("DeepSeek stream error: {}", err).into());
        }

        Ok(Box::pin(parse_openai_sse(resp)))
    }
}

// -- SSE stream parser --

#[derive(Default)]
struct AccToolCall {
    id: String,
    name: String,
    arguments: String,
}

fn parse_openai_sse(resp: reqwest::Response) -> impl futures::Stream<Item = StreamEvent> {
    async_stream::stream! {
        let mut byte_stream = resp.bytes_stream();
        let mut buffer = String::new();
        let mut tool_calls_acc: Vec<AccToolCall> = Vec::new();

        while let Some(chunk) = byte_stream.next().await {
            let chunk = match chunk {
                Ok(c) => c,
                Err(_) => break,
            };
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(pos) = buffer.find("\n\n") {
                let event_block = buffer[..pos].to_string();
                buffer = buffer[pos + 2..].to_string();

                for line in event_block.lines() {
                    if !line.starts_with("data: ") { continue; }
                    let data = &line[6..];
                    if data == "[DONE]" { continue; }

                    let Ok(chunk) = serde_json::from_str::<serde_json::Value>(data) else {
                        continue;
                    };
                    let Some(choices) = chunk.get("choices").and_then(|c| c.as_array()) else {
                        continue;
                    };

                    for choice in choices {
                        let Some(delta) = choice.get("delta") else { continue };

                        if let Some(text) = delta.get("content").and_then(|t| t.as_str()) {
                            if !text.is_empty() {
                                yield StreamEvent::Token(text.to_string());
                            }
                        }

                        if let Some(tcs) = delta.get("tool_calls").and_then(|t| t.as_array()) {
                            for tc in tcs {
                                let idx = tc.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as usize;
                                while tool_calls_acc.len() <= idx {
                                    tool_calls_acc.push(AccToolCall::default());
                                }
                                if let Some(id) = tc.get("id").and_then(|s| s.as_str()) {
                                    tool_calls_acc[idx].id = id.to_string();
                                }
                                if let Some(f) = tc.get("function") {
                                    if let Some(n) = f.get("name").and_then(|s| s.as_str()) {
                                        tool_calls_acc[idx].name = n.to_string();
                                    }
                                    if let Some(a) = f.get("arguments").and_then(|s| s.as_str()) {
                                        tool_calls_acc[idx].arguments.push_str(a);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Emit accumulated tool calls
        for tc in tool_calls_acc {
            if !tc.name.is_empty() {
                yield StreamEvent::ToolCall(ToolCallInfo {
                    id: tc.id,
                    name: tc.name,
                    arguments: tc.arguments,
                });
            }
        }

        yield StreamEvent::Done;
    }
}
