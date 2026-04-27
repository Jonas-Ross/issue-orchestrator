use std::time::Duration;

use serde_json::json;
use tempfile::tempdir;
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{sleep, timeout};

use crate::registry::{
    RegistryCmd, RegistryEvent, SessionRegistryActor, SessionSummary, SpawnSpec, Status,
};

use super::run_listener;

async fn spawn_bash(tx: &mpsc::Sender<RegistryCmd>) -> SessionSummary {
    let (reply_tx, reply_rx) = oneshot::channel();
    tx.send(RegistryCmd::Spawn {
        spec: SpawnSpec::Bash,
        cols: 80,
        rows: 24,
        reply: reply_tx,
    })
    .await
    .expect("send Spawn");
    reply_rx
        .await
        .expect("Spawn reply")
        .expect("Spawn succeeded")
}

async fn wait_for_status_change(
    rx: &mut mpsc::UnboundedReceiver<RegistryEvent>,
    target_id: &str,
) -> Status {
    timeout(Duration::from_secs(5), async {
        loop {
            match rx.recv().await.expect("event channel closed") {
                RegistryEvent::StatusChange { session_id, status } if session_id == target_id => {
                    return status;
                }
                _ => continue,
            }
        }
    })
    .await
    .expect("waiting for StatusChange timed out")
}

async fn send_hook(sock: &std::path::Path, payload: serde_json::Value) {
    // The listener may not be ready immediately after run_listener returns;
    // retry connect briefly.
    let mut last_err = None;
    for _ in 0..20 {
        match UnixStream::connect(sock).await {
            Ok(mut stream) => {
                let mut bytes = serde_json::to_vec(&payload).expect("serialize payload");
                bytes.push(b'\n');
                stream.write_all(&bytes).await.expect("write payload");
                stream.shutdown().await.ok();
                return;
            }
            Err(e) => {
                last_err = Some(e);
                sleep(Duration::from_millis(25)).await;
            }
        }
    }
    panic!("connect to {} failed: {last_err:?}", sock.display());
}

#[tokio::test]
async fn hook_routes_status_change_to_session() {
    let dir = tempdir().expect("tempdir");
    let sock = dir.path().join("hooks.sock");
    let log = dir.path().join("events.jsonl");

    let (event_tx, mut event_rx) = mpsc::unbounded_channel();
    let cmd_tx = SessionRegistryActor::spawn(event_tx);

    // Spawn the listener in the background.
    {
        let sock = sock.clone();
        let log = log.clone();
        let cmd_tx = cmd_tx.clone();
        tokio::spawn(async move {
            let _ = run_listener(sock, log, cmd_tx).await;
        });
    }

    let summary = spawn_bash(&cmd_tx).await;

    // Notification → NeedsInput
    send_hook(
        &sock,
        json!({
            "hook_event_name": "Notification",
            "session_id": "claude-session-uuid-1",
            "session_orch_id": summary.id,
            "cwd": "/tmp",
        }),
    )
    .await;
    let status = wait_for_status_change(&mut event_rx, &summary.id).await;
    assert_eq!(status, Status::NeedsInput);

    // Stop → Idle
    send_hook(
        &sock,
        json!({
            "hook_event_name": "Stop",
            "session_id": "claude-session-uuid-1",
            "session_orch_id": summary.id,
            "cwd": "/tmp",
        }),
    )
    .await;
    let status = wait_for_status_change(&mut event_rx, &summary.id).await;
    assert_eq!(status, Status::Idle);

    // The audit log should contain at least the two payloads we sent.
    let log_contents = std::fs::read_to_string(&log).expect("read log");
    let lines: Vec<_> = log_contents.lines().collect();
    assert!(
        lines.len() >= 2,
        "expected at least 2 audit lines, got {}",
        lines.len()
    );
}

#[tokio::test]
async fn hook_for_unknown_session_is_ignored() {
    let dir = tempdir().expect("tempdir");
    let sock = dir.path().join("hooks.sock");
    let log = dir.path().join("events.jsonl");

    let (event_tx, mut event_rx) = mpsc::unbounded_channel();
    let cmd_tx = SessionRegistryActor::spawn(event_tx);

    {
        let sock = sock.clone();
        let log = log.clone();
        let cmd_tx = cmd_tx.clone();
        tokio::spawn(async move {
            let _ = run_listener(sock, log, cmd_tx).await;
        });
    }

    send_hook(
        &sock,
        json!({
            "hook_event_name": "Notification",
            "session_id": "claude-1",
            "session_orch_id": "no-such-orch-id",
        }),
    )
    .await;

    // No StatusChange should arrive for our nonexistent session.
    let res = timeout(Duration::from_millis(300), event_rx.recv()).await;
    match res {
        Ok(Some(RegistryEvent::StatusChange { .. })) => {
            panic!("unexpected StatusChange for unknown orch_id")
        }
        _ => {} // either timeout or some unrelated event — both fine
    }
}
