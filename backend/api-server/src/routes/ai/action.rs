use super::intent::detect_threat;
use crate::services::ai_provider::{ChatMessage, ChatRole};
use crate::state::AppState;
use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ActionRequest {
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(tag = "action")]
#[serde(rename_all = "snake_case")]
pub enum ActionResponse {
    Transfer {
        params: TransferParams,
        confidence: f32,
        confirm_text: String,
    },
    Balance {
        confidence: f32,
    },
    Chat {
        message: String,
    },
}

#[derive(Debug, Serialize)]
pub struct TransferParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

// ---------------------------------------------------------------------------
// Structured AI action endpoint — POST /ai/action
//
// Returns either a structured action (transfer, balance) or falls back to chat
// ---------------------------------------------------------------------------

pub(super) async fn ai_action(
    State(state): State<AppState>,
    Json(req): Json<ActionRequest>,
) -> Result<Json<ActionResponse>, (StatusCode, Json<serde_json::Value>)> {
    use ai_bridge::intent::{classify, EntityKind, IntentKind};

    // First, check for threats
    if let Some(warning) = detect_threat(&req.message) {
        return Ok(Json(ActionResponse::Chat {
            message: warning.to_string(),
        }));
    }

    // Classify intent using local regex classifier
    let intent = classify(&req.message);

    // If high confidence and sufficient entities, return structured action
    if intent.confidence >= 0.7 {
        match intent.kind {
            IntentKind::CheckBalance => {
                return Ok(Json(ActionResponse::Balance {
                    confidence: intent.confidence,
                }));
            }
            IntentKind::Transfer => {
                // Extract entities
                let amount = intent
                    .entities
                    .iter()
                    .find(|e| e.kind == EntityKind::Amount)
                    .map(|e| e.value.clone());

                let token = intent
                    .entities
                    .iter()
                    .find(|e| e.kind == EntityKind::Token)
                    .map(|e| e.value.clone());

                let to = intent
                    .entities
                    .iter()
                    .find(|e| e.kind == EntityKind::Address)
                    .map(|e| e.value.clone())
                    .or_else(|| {
                        intent
                            .entities
                            .iter()
                            .find(|e| e.kind == EntityKind::Contact)
                            .map(|e| e.value.clone())
                    });

                // Check if we have sufficient info for execution
                let has_sufficient_info = amount.is_some() && (to.is_some() || token.is_some());

                if has_sufficient_info {
                    let confirm_text = format!(
                        "Send {} {} to {}?",
                        amount.as_deref().unwrap_or("?"),
                        token.as_deref().unwrap_or("ETH"),
                        to.as_deref().unwrap_or("?")
                    );

                    return Ok(Json(ActionResponse::Transfer {
                        params: TransferParams {
                            to,
                            amount,
                            token: token.or_else(|| Some("ETH".to_string())),
                        },
                        confidence: intent.confidence,
                        confirm_text,
                    }));
                }
            }
            _ => {
                // Other intent types don't have structured actions yet
            }
        }
    }

    // Fall back to AI chat if confidence is low or entities insufficient.
    // Providers are tried in preference order and fail over on request error.
    let providers = state.ai_providers_ordered();
    if providers.is_empty() {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "AI service not configured"})),
        ));
    }

    let messages = vec![
        ChatMessage {
            role: ChatRole::System,
            content: Some("You are CoWallet, an AI crypto wallet assistant. Answer the user's question concisely.".into()),
            tool_calls: None,
            tool_call_id: None,
        },
        ChatMessage {
            role: ChatRole::User,
            content: Some(req.message.clone()),
            tool_calls: None,
            tool_call_id: None,
        },
    ];

    // Use non-streaming chat for simple response, failing over across providers.
    let mut response = None;
    let mut last_err = None;
    for (kind, provider) in &providers {
        match provider.chat(&messages, &[], None).await {
            Ok(resp) => {
                response = Some(resp);
                break;
            }
            Err(e) => {
                tracing::warn!("AI provider {:?} chat failed: {} — trying next", kind, e);
                last_err = Some(e);
            }
        }
    }
    let response = response.ok_or_else(|| {
        let detail = last_err
            .map(|e| e.to_string())
            .unwrap_or_else(|| "no provider available".to_string());
        tracing::error!("All AI providers failed: {}", detail);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("AI request failed: {}", detail)})),
        )
    })?;

    let text = response.content.unwrap_or_default();
    let message = if text.is_empty() {
        "Sorry, I couldn't process that request.".to_string()
    } else {
        text
    };

    Ok(Json(ActionResponse::Chat { message }))
}
