pub mod log;
pub mod script;

#[cfg(test)]
mod tests;

use std::path::PathBuf;

use serde_json::Value;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::error::{Error, Result};
use crate::registry::RegistryCmd;

use self::log::Logger;

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
    pub raw: Value,
}

impl HookEvent {
    pub fn from_value(v: Value) -> Option<Self> {
        let obj = v.as_object()?;
        let name = obj.get("hook_event_name")?.as_str()?.to_owned();
        Some(Self {
            hook_event_name: name,
            session_orch_id: obj
                .get("session_orch_id")
                .and_then(Value::as_str)
                .map(str::to_owned),
            claude_session_id: obj
                .get("session_id")
                .and_then(Value::as_str)
                .map(str::to_owned),
            cwd: obj.get("cwd").and_then(Value::as_str).map(str::to_owned),
            transcript_path: obj
                .get("transcript_path")
                .and_then(Value::as_str)
                .map(str::to_owned),
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
    let logger = Logger::open(&log_path)?;
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
    stream: tokio::net::UnixStream,
    registry: mpsc::Sender<RegistryCmd>,
    logger: Logger,
) {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => return,
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                match serde_json::from_str::<Value>(trimmed) {
                    Ok(value) => {
                        if let Err(e) = logger.append(&value) {
                            warn!(?e, "failed to append hook event to log");
                        }
                        if let Some(evt) = HookEvent::from_value(value) {
                            if let Err(e) = registry.send(RegistryCmd::HookEvent(evt)).await {
                                warn!(?e, "failed to forward hook event to registry");
                            }
                        } else {
                            warn!(line = %trimmed, "hook event missing hook_event_name");
                        }
                    }
                    Err(e) => warn!(?e, line = %trimmed, "invalid hook json"),
                }
            }
            Err(e) => {
                warn!(?e, "hook connection read failed");
                return;
            }
        }
    }
}
