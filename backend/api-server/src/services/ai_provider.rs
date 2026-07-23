use futures::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

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

impl std::str::FromStr for ProviderKind {
    type Err = ();

    /// Parse a provider name, case-insensitively. Only `bedrock` / `deepseek`
    /// are accepted; anything else is an error (callers fall back to default).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "bedrock" => Ok(Self::Bedrock),
            "deepseek" => Ok(Self::DeepSeek),
            _ => Err(()),
        }
    }
}

impl ProviderKind {
    /// Resolve the operator's preferred provider from the `AI_PROVIDER` env var.
    /// Values are case-insensitive (`bedrock` | `deepseek`). A missing var uses
    /// the default silently; a present-but-invalid value warns and falls back to
    /// the default so a typo can never take AI chat down.
    pub fn from_env() -> Self {
        match std::env::var("AI_PROVIDER") {
            Ok(raw) => match raw.parse::<ProviderKind>() {
                Ok(kind) => kind,
                Err(()) => {
                    tracing::warn!(
                        "Invalid AI_PROVIDER value {:?}; falling back to default ({:?})",
                        raw,
                        ProviderKind::default()
                    );
                    ProviderKind::default()
                }
            },
            Err(_) => ProviderKind::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str_is_case_insensitive() {
        assert_eq!("bedrock".parse(), Ok(ProviderKind::Bedrock));
        assert_eq!("Bedrock".parse(), Ok(ProviderKind::Bedrock));
        assert_eq!("  DEEPSEEK  ".parse(), Ok(ProviderKind::DeepSeek));
        assert_eq!("deepseek".parse(), Ok(ProviderKind::DeepSeek));
    }

    #[test]
    fn from_str_rejects_unknown() {
        assert_eq!("openai".parse::<ProviderKind>(), Err(()));
        assert_eq!("".parse::<ProviderKind>(), Err(()));
    }

    #[test]
    fn default_is_bedrock() {
        assert_eq!(ProviderKind::default(), ProviderKind::Bedrock);
    }
}
