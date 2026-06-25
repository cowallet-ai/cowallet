use crate::state::AppState;
use axum::{
    Router,
    routing::{get, post},
};

mod action;
mod chat;
mod intent;
mod sessions;
mod tools;

use action::ai_action;
use chat::chat_stream;
use sessions::{create_session, delete_session, get_session_messages, list_sessions};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/chat", post(chat_stream))
        .route("/action", post(ai_action))
        .route("/sessions", get(list_sessions).post(create_session))
        .route("/sessions/{session_id}/messages", get(get_session_messages))
        .route("/sessions/{session_id}", axum::routing::delete(delete_session))
}
