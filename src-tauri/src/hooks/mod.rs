pub mod log;

#[cfg(test)]
mod tests;

use std::path::PathBuf;

use serde_json::Value;
use tokio::io::AsyncReadExt;
use tokio::net::UnixListener;
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::error::{Error, Result};
use crate::registry::RegistryCmd;

use self::log::Logger;

/// On `Notification` events, Claude Code distinguishes between a real
/// permission prompt (blocking on the user) and the 60s inactivity
/// reminder. They get different status mappings so calm sessions don't
/// pulse mint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationKind {
    PermissionPrompt,
    IdlePrompt,
    Other,
}

impl NotificationKind {
    fn parse(s: &str) -> Self {
        match s {
            "permission_prompt" => Self::PermissionPrompt,
            "idle_prompt" => Self::IdlePrompt,
            _ => Self::Other,
        }
    }
}

/// One hook event from Claude Code. We keep both a normalized view (for
/// the actor's status logic) and the raw payload (for the audit log,
/// which preserves any fields we don't yet care about).
#[derive(Debug, Clone)]
pub struct HookEvent {
    pub hook_event_name: String,
    pub session_orch_id: Option<String>,
    pub claude_session_id: Option<String>,
    pub cwd: Option<String>,
    pub transcript_path: Option<String>,
    pub notification_kind: Option<NotificationKind>,
    pub raw: Value,
}

impl HookEvent {
    pub fn from_value(v: Value) -> Option<Self> {
        let obj = v.as_object()?;
        let s = |k: &str| obj.get(k).and_then(Value::as_str).map(str::to_owned);
        Some(Self {
            hook_event_name: s("hook_event_name")?,
            session_orch_id: s("session_orch_id"),
            claude_session_id: s("session_id"),
            cwd: s("cwd"),
            transcript_path: s("transcript_path"),
            notification_kind: obj
                .get("notification_type")
                .and_then(Value::as_str)
                .map(NotificationKind::parse),
            raw: v,
        })
    }
}

/// Run the hook UDS server until the socket fails to accept. Each
/// connection reads newline-delimited JSON and dispatches each line as
/// a `RegistryCmd::HookEvent` plus an audit log append.
pub async fn run_listener(
    sock_path: PathBuf,
    log_path: PathBuf,
    registry: mpsc::Sender<RegistryCmd>,
) -> Result<()> {
    if sock_path.exists() {
        // Stale socket from a previous run — remove or bind() will fail.
        std::fs::remove_file(&sock_path)?;
    }
    let listener = UnixListener::bind(&sock_path)
        .map_err(|e| Error::Hooks(format!("bind {}: {e}", sock_path.display())))?;
    let logger = Logger::open(&log_path).await?;
    info!(socket = %sock_path.display(), log = %log_path.display(), "hook listener started");

    loop {
        match listener.accept().await {
            Ok((stream, _addr)) => {
                let registry = registry.clone();
                let logger = logger.clone();
                tokio::spawn(async move {
                    handle_connection(stream, registry, logger).await;
                });
            }
            Err(e) => {
                warn!(?e, "hook listener accept failed");
            }
        }
    }
}

async fn handle_connection(
    mut stream: tokio::net::UnixStream,
    registry: mpsc::Sender<RegistryCmd>,
    logger: Logger,
) {
    let mut bytes = Vec::with_capacity(4096);
    if let Err(e) = stream.read_to_end(&mut bytes).await {
        warn!(?e, "hook connection read failed");
        return;
    }
    if bytes.iter().all(u8::is_ascii_whitespace) {
        return;
    }

    // serde_json's streaming Deserializer handles both compact one-line
    // payloads (the hook.sh happy path) and pretty-printed multi-line
    // ones (Claude Code's raw payload, which lands here when jq isn't
    // installed). It also tolerates multiple JSON values per connection,
    // which keeps the protocol forward-compatible.
    let mut stream = serde_json::Deserializer::from_slice(&bytes).into_iter::<Value>();
    while let Some(result) = stream.next() {
        match result {
            Ok(value) => {
                if let Err(e) = logger.append(&value).await {
                    warn!(?e, "failed to append hook event to log");
                }
                match HookEvent::from_value(value) {
                    Some(evt) => {
                        if let Err(e) = registry.send(RegistryCmd::HookEvent(evt)).await {
                            warn!(?e, "failed to forward hook event to registry");
                        }
                    }
                    None => warn!("hook event missing hook_event_name"),
                }
            }
            Err(e) => {
                let preview = String::from_utf8_lossy(&bytes);
                let truncated = if preview.len() > 200 {
                    format!("{}…", &preview[..200])
                } else {
                    preview.into_owned()
                };
                warn!(?e, payload = %truncated, "invalid hook json");
                break;
            }
        }
    }
}
