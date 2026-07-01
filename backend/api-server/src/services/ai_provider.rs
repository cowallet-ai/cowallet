use std::pin::Pin;
use futures::Stream;
use serde::{Deserialize, Serialize};

pub type BoxStream<T> = Pin<Box<dyn Stream<Item = T> + Send>>;
pub type AiResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Unified message format used by route handlers.
/// Providers convert to/from their native format internally.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallInfo {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

/// Unified tool definition.
#[derive(Debug, Clone, Serialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Events emitted by a streaming provider response.
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// A chunk of text content.
    Token(String),
    /// A complete tool call (accumulated by the provider).
    ToolCall(ToolCallInfo),
    /// Stream finished normally.
    Done,
}

/// Chat completion response (non-streaming).
#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub content: Option<String>,
    pub tool_calls: Vec<ToolCallInfo>,
}

/// Provider-agnostic AI chat interface.
#[async_trait::async_trait]
pub trait AiProvider: Send + Sync {
    /// Non-streaming chat completion.
    async fn chat(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDef],
        temperature: Option<f32>,
    ) -> AiResult<ChatResponse>;

    /// Streaming chat completion — returns a stream of events.
    async fn stream_chat(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDef],
        temperature: Option<f32>,
    ) -> AiResult<BoxStream<StreamEvent>>;
}

/// Which AI provider to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    DeepSeek,
    Bedrock,
}

impl Default for ProviderKind {
    fn default() -> Self {
        Self::Bedrock
    }
}
