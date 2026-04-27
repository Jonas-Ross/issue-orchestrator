use serde::{Deserialize, Serialize};
use specta::Type;
use tauri_specta::Event;

use crate::registry::{SessionId, SessionSummary, Status};

/// One chunk of UTF-8 stdout/stderr from a session's PTY.
#[derive(Clone, Debug, Type, Serialize, Deserialize, Event)]
#[serde(rename_all = "camelCase")]
pub struct PtyData {
    pub session_id: SessionId,
    pub chunk: String,
}

/// A session's status changed. Driven by M3 hook events; the frontend
/// uses this to repaint the per-tab status pill.
#[derive(Clone, Debug, Type, Serialize, Deserialize, Event)]
#[serde(rename_all = "camelCase")]
pub struct StatusChange {
    pub session_id: SessionId,
    pub status: Status,
}

/// A new session is now in the registry.
#[derive(Clone, Debug, Type, Serialize, Deserialize, Event)]
pub struct SessionAdded(pub SessionSummary);

/// A session was removed (kill flow or child exit).
#[derive(Clone, Debug, Type, Serialize, Deserialize, Event)]
#[serde(rename_all = "camelCase")]
pub struct SessionRemoved {
    pub session_id: SessionId,
}
