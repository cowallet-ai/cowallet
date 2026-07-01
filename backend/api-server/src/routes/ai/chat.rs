use super::intent::{detect_threat, has_transfer_intent};
use super::tools::{tool_kind, tool_widget_type, wallet_tools, wallet_tools_meta, ToolKind, SYSTEM_PROMPT};
use crate::services::ai_executor::{ToolContext, ToolExecutionResult};
use crate::services::ai_provider::{
    ChatMessage, ChatRole, ToolDef,
    ToolCallInfo as ProviderToolCallInfo, StreamEvent,
};
use crate::services::chat_store::ChatStore;
use crate::state::AppState;
use axum::{
    Json,
    body::Body,
    extract::State,
    http::{StatusCode, header},
    response::Response,
};
use bytes::Bytes;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::sync::Arc;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub message: String,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(default)]
    pub wallet_address: Option<String>,
    #[serde(default)]
    pub supported_chains: Option<Vec<u64>>,
    #[serde(default)]
    pub portfolio: Option<serde_json::Value>,
    #[serde(default)]
    pub contacts: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub auth_method: Option<String>,
    #[serde(default)]
    pub lang: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallInfo {
    pub name: String,
    pub parameters: serde_json::Value,
    pub id: String,
}

// ---------------------------------------------------------------------------
// SSE streaming chat — POST /ai/chat
//
// SSE events:
//   event: session     data: {"session_id":"..."}
//   event: token       data: {"text":"..."}
//   event: tool_call   data: {"id":"...","name":"...","parameters":{}}
//   event: tool_result data: {"tool_id":"...","tool_name":"...","success":true,"result":{}}
//   event: done        data: {"needs_confirmation":["..."]}
//   event: error       data: {"message":"..."}
// ---------------------------------------------------------------------------

pub(super) async fn chat_stream(
    State(state): State<AppState>,
    Json(req): Json<ChatRequest>,
) -> Response {
    let user_message = req.message.clone();

    let user_uuid = req.user_id.as_deref()
        .and_then(|id| Uuid::parse_str(id).ok())
        .unwrap_or_else(Uuid::nil);

    let session_id = req.session_id.as_deref()
        .and_then(|s| Uuid::parse_str(s).ok());

    // Resolve session
    let db_session_id = if let Some(db) = &state.db {
        if let Some(sid) = session_id {
            sid
        } else {
            ChatStore::get_or_create_session(db, user_uuid).await
                .map(|s| s.id)
                .unwrap_or_else(|_| Uuid::new_v4())
        }
    } else {
        Uuid::new_v4()
    };

    // Persist user message
    if let Some(db) = &state.db {
        let _ = ChatStore::save_message(db, db_session_id, "user", Some(&user_message), None, None).await;
    }

    // Threat detection — block before calling AI
    let threat_warning = detect_threat(&user_message);

    // Build the SSE response as a stream
    let stream = async_stream::stream! {
        // Send session_id first
        yield sse_event("session", &serde_json::json!({"session_id": db_session_id.to_string()}));

        // If threat detected, respond with warning and skip AI
        if let Some(warning) = threat_warning {
            yield sse_event("token", &serde_json::json!({"text": warning}));
            if let Some(db) = &state.db {
                let _ = ChatStore::save_message(db, db_session_id, "assistant", Some(warning), None, None).await;
            }
            yield sse_event("done", &serde_json::json!({"needs_confirmation": []}));
            return;
        }

        let _ = req.model;
        let ai = match state.select_ai_provider() {
            Some(c) => c,
            None => {
                yield sse_event("error", &serde_json::json!({"message": "AI 服务未配置"}));
                yield sse_event("done", &serde_json::json!({"needs_confirmation": []}));
                return;
            }
        };

        // Build context messages with language directive
        let system_content = if req.lang.as_deref() == Some("en") {
            format!("{}\n\n## Language\nIMPORTANT: You MUST respond in English. All your replies, clarify options, and descriptions must be in English.", SYSTEM_PROMPT)
        } else {
            SYSTEM_PROMPT.to_string()
        };
        let mut messages: Vec<ChatMessage> = vec![
            ChatMessage { role: ChatRole::System, content: Some(system_content), tool_calls: None, tool_call_id: None },
        ];

        // Load history from DB
        if let Some(db) = &state.db {
            if let Ok(rows) = ChatStore::load_messages(db, db_session_id, 20).await {
                for row in rows {
                    if row.role == "tool_result" {
                        continue;
                    }
                    if row.role == "user" && row.content.as_deref() == Some(user_message.as_str()) {
                        continue;
                    }
                    let role = match row.role.as_str() {
                        "system" => ChatRole::System,
                        "assistant" => ChatRole::Assistant,
                        "tool" => ChatRole::Tool,
                        _ => ChatRole::User,
                    };
                    messages.push(ChatMessage {
                        role,
                        content: row.content,
                        tool_calls: None,
                        tool_call_id: row.tool_call_id,
                    });
                }
            }
        }

        // Inject portfolio and contacts context into user message if provided.
        // SECURITY: this data is client-supplied and untrusted. It is sanitized
        // (control chars stripped, length-capped) and wrapped in an explicit
        // untrusted-data boundary so the model does not treat it as instructions
        // (indirect prompt-injection defense).
        let mut user_content = user_message.clone();
        if let Some(portfolio) = &req.portfolio {
            let portfolio_str = serde_json::to_string(portfolio).unwrap_or_default();
            let clean = sanitize_untrusted(&portfolio_str, 4000);
            user_content = format!(
                "{}\n\n<untrusted_data source=\"portfolio\">\n{}\n</untrusted_data>",
                user_content, clean
            );
        }
        if let Some(contacts) = &req.contacts {
            if !contacts.is_empty() {
                let contacts_str = serde_json::to_string(contacts).unwrap_or_default();
                let clean = sanitize_untrusted(&contacts_str, 4000);
                user_content = format!(
                    "{}\n\n<untrusted_data source=\"contacts\">\n{}\n</untrusted_data>",
                    user_content, clean
                );
            }
        }

        messages.push(ChatMessage {
            role: ChatRole::User,
            content: Some(user_content),
            tool_calls: None,
            tool_call_id: None,
        });

        let tools = wallet_tools();

        // Stream first response
        let mut full_content = String::new();
        let mut tool_calls_result: Vec<ProviderToolCallInfo> = Vec::new();

        let stream_resp = ai.stream_chat(&messages, &tools, None).await;
        let mut event_stream = match stream_resp {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("AI stream failed: {}", e);
                yield sse_event("error", &serde_json::json!({"message": "AI 服务暂时不可用，请稍后重试"}));
                yield sse_event("done", &serde_json::json!({"needs_confirmation": []}));
                return;
            }
        };

        while let Some(event) = event_stream.next().await {
            match event {
                StreamEvent::Token(text) => {
                    full_content.push_str(&text);
                    yield sse_event("token", &serde_json::json!({"text": text}));
                }
                StreamEvent::ToolCall(tc) => {
                    tool_calls_result.push(tc);
                }
                StreamEvent::Done => break,
            }
        }

        // If no tool calls, check if the user message had transaction intent
        // that the AI failed to handle with a tool_call (safety net)
        if tool_calls_result.is_empty() {
            if has_transfer_intent(&user_message) {
                // AI missed a transfer intent — clear the misleading text and retry
                yield sse_event("replace", &serde_json::json!({"text": ""}));

                let retry_msg = format!(
                    "你必须使用 send_transaction 或 clarify 工具来处理这个请求。用户说的是：「{}」\n\n请重新处理，调用正确的工具。如果缺少参数（地址/金额/链），使用 clarify 工具询问用户。绝对不能用文本回复转账请求。",
                    user_message
                );
                messages.push(ChatMessage {
                    role: ChatRole::Assistant,
                    content: Some(full_content.clone()),
                    tool_calls: None,
                    tool_call_id: None,
                });
                messages.push(ChatMessage {
                    role: ChatRole::User,
                    content: Some(retry_msg),
                    tool_calls: None,
                    tool_call_id: None,
                });

                full_content.clear();
                tool_calls_result.clear();

                if let Ok(mut retry_stream) = ai.stream_chat(&messages, &tools, None).await {
                    while let Some(event) = retry_stream.next().await {
                        match event {
                            StreamEvent::Token(text) => {
                                full_content.push_str(&text);
                                yield sse_event("token", &serde_json::json!({"text": text}));
                            }
                            StreamEvent::ToolCall(tc) => {
                                tool_calls_result.push(tc);
                            }
                            StreamEvent::Done => break,
                        }
                    }
                }

                if tool_calls_result.is_empty() {
                    let fallback = "抱歉，我无法处理这个转账请求。请用更明确的格式描述，例如：「转0.1 POL到0x1234...」";
                    yield sse_event("replace", &serde_json::json!({"text": fallback}));
                    if let Some(db) = &state.db {
                        let _ = ChatStore::save_message(db, db_session_id, "assistant", Some(fallback), None, None).await;
                    }
                    yield sse_event("done", &serde_json::json!({"needs_confirmation": []}));
                    return;
                }
                // Otherwise fall through to tool_call processing below
            } else {
                if let Some(db) = &state.db {
                    let _ = ChatStore::save_message(db, db_session_id, "assistant", Some(&full_content), None, None).await;
                }
                yield sse_event("done", &serde_json::json!({"needs_confirmation": []}));
                return;
            }
        }

        // Parse and emit tool calls with kind/widget metadata
        let mut parsed_tool_calls: Vec<ToolCallInfo> = Vec::new();
        for tc in &tool_calls_result {
            let params: serde_json::Value = serde_json::from_str(&tc.arguments).unwrap_or(serde_json::json!({}));
            let kind = tool_kind(&tc.name);
            let widget = tool_widget_type(&tc.name);
            parsed_tool_calls.push(ToolCallInfo {
                id: tc.id.clone(),
                name: tc.name.clone(),
                parameters: params.clone(),
            });
            yield sse_event("tool_call", &serde_json::json!({
                "id": tc.id,
                "name": tc.name,
                "parameters": params,
                "kind": kind,
                "widget_type": widget,
            }));
            // Persist write-tool widgets at tool_call phase (params contain the display data)
            if kind == ToolKind::Write {
                if let (Some(db), Some(w)) = (&state.db, &widget) {
                    let _ = ChatStore::save_widget_message(db, db_session_id, w, &params, Some(&tc.id)).await;
                }
            }
        }

        // Execute tools based on kind
        let tool_ctx = ToolContext {
            app_state: state.clone(),
            user_id: req.user_id.clone(),
            wallet_address: req.wallet_address.clone(),
            auth_method: req.auth_method.clone(),
            // F-013: pass the user's ORIGINAL typed message (not the context-injected
            // variant) so tools can cross-validate LLM-chosen recipient addresses.
            user_message: Some(user_message.clone()),
        };

        let mut tool_results: Vec<ToolExecutionResult> = Vec::new();
        let mut needs_confirmation: Vec<String> = Vec::new();
        let mut has_meta_tool = false;

        for tc in &parsed_tool_calls {
            let kind = tool_kind(&tc.name);
            let widget = tool_widget_type(&tc.name);

            // Meta tools (clarify) are handled directly without execution
            if kind == ToolKind::Meta {
                has_meta_tool = true;
                yield sse_event("tool_result", &serde_json::json!({
                    "tool_id": tc.id,
                    "tool_name": tc.name,
                    "kind": kind,
                    "widget_type": widget,
                    "success": true,
                    "result": tc.parameters,
                    "error": null,
                }));
                if let (Some(db), Some(w)) = (&state.db, &widget) {
                    let _ = ChatStore::save_widget_message(db, db_session_id, w, &tc.parameters, Some(&tc.id)).await;
                }
                continue;
            }

            // Write tools: execute to get estimates (gas, quotes), but still require confirmation
            if kind == ToolKind::Write {
                needs_confirmation.push(tc.id.clone());
                // Execute the tool to get gas estimates and preparation data
                let exec_result = tool_ctx.execute_tool(&tc.name, &tc.id, tc.parameters.clone()).await;
                let prepared = if exec_result.success {
                    // Merge pending_confirmation status with the execution result
                    let mut result_map = exec_result.result.clone();
                    if let Some(obj) = result_map.as_object_mut() {
                        obj.insert("status".into(), serde_json::json!("pending_confirmation"));
                    }
                    result_map
                } else {
                    serde_json::json!({
                        "status": "pending_confirmation",
                        "parameters": tc.parameters,
                    })
                };
                yield sse_event("tool_result", &serde_json::json!({
                    "tool_id": tc.id,
                    "tool_name": tc.name,
                    "kind": kind,
                    "widget_type": widget,
                    "success": true,
                    "result": prepared,
                    "error": null,
                }));
                tool_results.push(ToolExecutionResult {
                    tool_id: tc.id.clone(),
                    tool_name: tc.name.clone(),
                    success: true,
                    result: prepared,
                    error: None,
                });
                continue;
            }

            // Read tools: execute immediately
            let result = tool_ctx.execute_tool(&tc.name, &tc.id, tc.parameters.clone()).await;
            yield sse_event("tool_result", &serde_json::json!({
                "tool_id": result.tool_id,
                "tool_name": result.tool_name,
                "kind": kind,
                "widget_type": widget,
                "success": result.success,
                "result": result.result,
                "error": result.error,
            }));
            if result.success {
                if let (Some(db), Some(w)) = (&state.db, &widget) {
                    let _ = ChatStore::save_widget_message(db, db_session_id, w, &result.result, Some(&result.tool_id)).await;
                }
            }
            tool_results.push(result);
        }

        // If only meta tools were called, skip the second AI round
        if has_meta_tool && tool_results.is_empty() {
            if let Some(db) = &state.db {
                let tc_json = serde_json::to_value(&parsed_tool_calls).ok();
                let _ = ChatStore::save_message(db, db_session_id, "assistant", Some(&full_content), tc_json.as_ref(), None).await;
            }
            yield sse_event("done", &serde_json::json!({"needs_confirmation": needs_confirmation}));
            return;
        }

        // Build second round messages with tool results
        // Add assistant message with tool_calls
        messages.push(ChatMessage {
            role: ChatRole::Assistant,
            content: if full_content.is_empty() { None } else { Some(full_content.clone()) },
            tool_calls: Some(tool_calls_result.clone()),
            tool_call_id: None,
        });

        for result in &tool_results {
            let content = if result.success {
                serde_json::to_string(&result.result).unwrap_or_else(|_| "{}".into())
            } else {
                format!("Error: {}", result.error.as_deref().unwrap_or("unknown"))
            };
            messages.push(ChatMessage {
                role: ChatRole::Tool,
                content: Some(content),
                tool_calls: None,
                tool_call_id: Some(result.tool_id.clone()),
            });
        }

        // Determine which tools to provide in the second round.
        // If the first round only used Read/Meta tools (no Write tools) and the user
        // has transfer/swap intent, provide all tools so the AI can call send_transaction
        // after checking balance. Otherwise only provide clarify for follow-up suggestions.
        let first_round_had_write = parsed_tool_calls.iter().any(|tc| tool_kind(&tc.name) == ToolKind::Write);
        let second_round_tools: Vec<ToolDef> = if !first_round_had_write && has_transfer_intent(&user_message) {
            wallet_tools()
        } else {
            wallet_tools_meta()
                .into_iter()
                .filter(|m| m.definition.name == "clarify")
                .map(|m| m.definition)
                .collect()
        };
        let stream_resp2 = ai.stream_chat(&messages, &second_round_tools, None).await;
        match stream_resp2 {
            Ok(mut resp2) => {
                let mut final_content = String::new();
                let mut tool_calls_result2: Vec<ProviderToolCallInfo> = Vec::new();

                while let Some(event) = resp2.next().await {
                    match event {
                        StreamEvent::Token(text) => {
                            final_content.push_str(&text);
                            yield sse_event("token", &serde_json::json!({"text": text}));
                        }
                        StreamEvent::ToolCall(tc) => {
                            tool_calls_result2.push(tc);
                        }
                        StreamEvent::Done => break,
                    }
                }

                // Safety net for second round: if AI still refused to call tools
                // despite having transfer intent, replace its text with fallback
                if tool_calls_result2.is_empty() && !first_round_had_write && has_transfer_intent(&user_message) {
                    let fallback = "抱歉，我无法处理这个转账请求。请用更明确的格式描述，例如：「转0.1 USDT到0x1234...（Polygon链）」";
                    yield sse_event("replace", &serde_json::json!({"text": fallback}));
                    if let Some(db) = &state.db {
                        let _ = ChatStore::save_message(db, db_session_id, "assistant", Some(fallback), None, None).await;
                    }
                    yield sse_event("done", &serde_json::json!({"needs_confirmation": needs_confirmation}));
                    return;
                }

                // Process tool calls from second round (clarify, send_transaction, etc.)
                for tc in &tool_calls_result2 {
                    let params: serde_json::Value = serde_json::from_str(&tc.arguments).unwrap_or(serde_json::json!({}));
                    let kind = tool_kind(&tc.name);
                    let widget = tool_widget_type(&tc.name);

                    // Emit tool_call event so client renders appropriate UI
                    yield sse_event("tool_call", &serde_json::json!({
                        "id": tc.id,
                        "name": tc.name,
                        "parameters": params,
                        "kind": kind,
                        "widget_type": widget,
                    }));

                    // Persist write-tool widgets at tool_call phase (params have display data)
                    if kind == ToolKind::Write {
                        if let (Some(db), Some(w)) = (&state.db, &widget) {
                            let _ = ChatStore::save_widget_message(db, db_session_id, w, &params, Some(&tc.id)).await;
                        }
                    }

                    if kind == ToolKind::Meta {
                        yield sse_event("tool_result", &serde_json::json!({
                            "tool_id": tc.id,
                            "tool_name": tc.name,
                            "kind": kind,
                            "widget_type": widget,
                            "success": true,
                            "result": params,
                            "error": null,
                        }));
                        if let (Some(db), Some(w)) = (&state.db, &widget) {
                            let _ = ChatStore::save_widget_message(db, db_session_id, w, &params, Some(&tc.id)).await;
                        }
                    } else if kind == ToolKind::Write {
                        needs_confirmation.push(tc.id.clone());
                        let exec_result = tool_ctx.execute_tool(&tc.name, &tc.id, params.clone()).await;
                        let prepared = if exec_result.success {
                            let mut result_map = exec_result.result.clone();
                            if let Some(obj) = result_map.as_object_mut() {
                                obj.insert("status".into(), serde_json::json!("pending_confirmation"));
                            }
                            result_map
                        } else {
                            serde_json::json!({
                                "status": "pending_confirmation",
                                "parameters": params,
                            })
                        };
                        yield sse_event("tool_result", &serde_json::json!({
                            "tool_id": tc.id,
                            "tool_name": tc.name,
                            "kind": kind,
                            "widget_type": widget,
                            "success": true,
                            "result": prepared,
                            "error": null,
                        }));
                    } else {
                        // Read tools in second round
                        let result = tool_ctx.execute_tool(&tc.name, &tc.id, params.clone()).await;
                        yield sse_event("tool_result", &serde_json::json!({
                            "tool_id": result.tool_id,
                            "tool_name": result.tool_name,
                            "kind": kind,
                            "widget_type": widget,
                            "success": result.success,
                            "result": result.result,
                            "error": result.error,
                        }));
                        if result.success {
                            if let (Some(db), Some(w)) = (&state.db, &widget) {
                                let _ = ChatStore::save_widget_message(db, db_session_id, w, &result.result, Some(&result.tool_id)).await;
                            }
                        }
                    }
                }

                // Persist final assistant response
                if let Some(db) = &state.db {
                    let tc_json = serde_json::to_value(&parsed_tool_calls).ok();
                    let _ = ChatStore::save_message(db, db_session_id, "assistant", Some(&final_content), tc_json.as_ref(), None).await;
                }
            }
            Err(e) => {
                tracing::error!("AI second stream failed: {}", e);
                yield sse_event("error", &serde_json::json!({"message": "工具结果处理失败"}));
            }
        }

        yield sse_event("done", &serde_json::json!({"needs_confirmation": needs_confirmation}));
    };

    let body = Body::from_stream(stream);

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/event-stream")
        .header(header::CACHE_CONTROL, "no-cache")
        .header("X-Accel-Buffering", "no")
        .body(body)
        .unwrap()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn sse_event(event: &str, data: &serde_json::Value) -> Result<Bytes, Infallible> {
    Ok(Bytes::from(format!("event: {}\ndata: {}\n\n", event, data)))
}

/// Sanitize client-supplied, untrusted context (portfolio / contacts) before
/// injecting it into the AI prompt. Strips control characters (which can be used
/// to spoof role boundaries or smuggle hidden instructions), neutralizes literal
/// untrusted-data boundary tags so the payload can't close its own wrapper, and
/// caps length to bound the injection surface.
fn sanitize_untrusted(input: &str, max_len: usize) -> String {
    let mut out = String::with_capacity(input.len().min(max_len));
    for c in input.chars() {
        // Drop C0/C1 control chars except plain space; keep normal whitespace as space.
        if c == '\t' || c == '\n' || c == '\r' {
            out.push(' ');
        } else if c.is_control() {
            continue;
        } else {
            out.push(c);
        }
        if out.len() >= max_len {
            out.push_str(" …(truncated)");
            break;
        }
    }
    // Prevent the payload from closing the wrapper element or forging a new one.
    out.replace("<untrusted_data", "<_untrusted_data")
        .replace("</untrusted_data", "</_untrusted_data")
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_sse_event_format() {
        let data = serde_json::json!({"text": "hello"});
        let result = super::sse_event("token", &data).unwrap();
        let s = std::str::from_utf8(&result).unwrap();
        assert!(s.starts_with("event: token\n"));
        assert!(s.contains("\"text\":\"hello\""));
        assert!(s.ends_with("\n\n"));
    }
}
