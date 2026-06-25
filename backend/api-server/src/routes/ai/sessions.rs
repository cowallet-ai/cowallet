use crate::services::chat_store::ChatStore;
use crate::state::AppState;
use axum::{
    Json,
    extract::State,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct SessionQuery {
    pub user_id: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub user_id: String,
    pub title: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SessionInfo {
    pub id: String,
    pub title: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// ---------------------------------------------------------------------------
// Session management
// ---------------------------------------------------------------------------

pub(super) async fn create_session(
    State(state): State<AppState>,
    Json(req): Json<CreateSessionRequest>,
) -> Result<Json<SessionInfo>, (StatusCode, Json<serde_json::Value>)> {
    let db = state.db.as_ref().ok_or_else(|| {
        (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"error": "database unavailable"})))
    })?;

    let user_uuid = Uuid::parse_str(&req.user_id).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "invalid user_id"})))
    })?;

    let session = ChatStore::create_session(db, user_uuid, req.title.as_deref()).await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()})))
    })?;

    Ok(Json(SessionInfo {
        id: session.id.to_string(),
        title: session.title,
        created_at: session.created_at.to_rfc3339(),
        updated_at: session.updated_at.to_rfc3339(),
    }))
}

pub(super) async fn list_sessions(
    State(state): State<AppState>,
    axum::extract::Query(query): axum::extract::Query<SessionQuery>,
) -> Result<Json<Vec<SessionInfo>>, (StatusCode, Json<serde_json::Value>)> {
    let db = state.db.as_ref().ok_or_else(|| {
        (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"error": "database unavailable"})))
    })?;

    let user_uuid = Uuid::parse_str(&query.user_id).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "invalid user_id"})))
    })?;

    let sessions = ChatStore::list_sessions(db, user_uuid, 50).await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()})))
    })?;

    let result: Vec<SessionInfo> = sessions.into_iter().map(|s| SessionInfo {
        id: s.id.to_string(),
        title: s.title,
        created_at: s.created_at.to_rfc3339(),
        updated_at: s.updated_at.to_rfc3339(),
    }).collect();

    Ok(Json(result))
}

pub(super) async fn get_session_messages(
    State(state): State<AppState>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Result<Json<Vec<serde_json::Value>>, (StatusCode, Json<serde_json::Value>)> {
    let db = state.db.as_ref().ok_or_else(|| {
        (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"error": "database unavailable"})))
    })?;

    let session_uuid = Uuid::parse_str(&session_id).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "invalid session_id"})))
    })?;

    let messages = ChatStore::load_messages(db, session_uuid, 100).await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()})))
    })?;

    let result: Vec<serde_json::Value> = messages.into_iter().map(|m| {
        serde_json::json!({
            "id": m.id.to_string(),
            "role": m.role,
            "content": m.content,
            "tool_calls": m.tool_calls,
            "widget_type": m.widget_type,
            "widget_data": m.widget_data,
            "created_at": m.created_at.to_rfc3339(),
        })
    }).collect();

    Ok(Json(result))
}

pub(super) async fn delete_session(
    State(state): State<AppState>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
    axum::extract::Query(query): axum::extract::Query<SessionQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let db = state.db.as_ref().ok_or_else(|| {
        (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"error": "database unavailable"})))
    })?;

    let session_uuid = Uuid::parse_str(&session_id).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "invalid session_id"})))
    })?;

    let user_uuid = Uuid::parse_str(&query.user_id).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "invalid user_id"})))
    })?;

    let deleted = ChatStore::delete_session(db, session_uuid, user_uuid).await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()})))
    })?;

    Ok(Json(serde_json::json!({"deleted": deleted})))
}
