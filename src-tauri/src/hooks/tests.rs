use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::json;
use tempfile::tempdir;
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{sleep, timeout};

use crate::registry::{
    RegistryCmd, RegistryEvent, SessionRegistryActor, SessionSummary, SpawnSpec, Status,
};

use super::{run_listener, RepoLookup};

/// `RepoLookup` fake used by existing status-mapping tests that don't
/// care about cwd inference. Always returns `None`, matching the
/// "no repos registered" behavior of a fresh config.
struct NoRepos;

#[async_trait]
impl RepoLookup for NoRepos {
    async fn repo_for_path(&self, _path: &str) -> Option<String> {
        None
    }
}

fn no_repos() -> Arc<dyn RepoLookup> {
    Arc::new(NoRepos)
}

/// `RepoLookup` fake that returns a fixed repo name for any path with a
/// given prefix. Lets us verify the listener actually wires cwd through
/// to the inference path without a real config actor.
struct PrefixRepoLookup {
    prefix: String,
    repo: String,
}

#[async_trait]
impl RepoLookup for PrefixRepoLookup {
    async fn repo_for_path(&self, path: &str) -> Option<String> {
        path.starts_with(&self.prefix).then(|| self.repo.clone())
    }
}

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
    send_hook_bytes(sock, serde_json::to_vec(&payload).expect("serialize payload")).await
}

async fn send_hook_bytes(sock: &std::path::Path, bytes: Vec<u8>) {
    // The listener may not be ready immediately after run_listener returns;
    // retry connect briefly.
    let mut last_err = None;
    for _ in 0..20 {
        match UnixStream::connect(sock).await {
            Ok(mut stream) => {
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
            let _ = run_listener(sock, log, cmd_tx, no_repos()).await;
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
async fn notification_idle_prompt_maps_to_idle_not_needs_input() {
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
            let _ = run_listener(sock, log, cmd_tx, no_repos()).await;
        });
    }

    let summary = spawn_bash(&cmd_tx).await;

    // The 60s "Claude is waiting for your input" idle reminder should
    // surface as Idle, not NeedsInput — otherwise calm sessions pulse mint.
    send_hook(
        &sock,
        json!({
            "hook_event_name": "Notification",
            "session_id": "claude-1",
            "session_orch_id": summary.id,
            "notification_type": "idle_prompt",
            "message": "Claude is waiting for your input",
        }),
    )
    .await;
    let status = wait_for_status_change(&mut event_rx, &summary.id).await;
    assert_eq!(status, Status::Idle);

    // permission_prompt should still pulse mint.
    send_hook(
        &sock,
        json!({
            "hook_event_name": "Notification",
            "session_id": "claude-1",
            "session_orch_id": summary.id,
            "notification_type": "permission_prompt",
            "message": "Claude Code needs your attention",
        }),
    )
    .await;
    let status = wait_for_status_change(&mut event_rx, &summary.id).await;
    assert_eq!(status, Status::NeedsInput);
}

#[tokio::test]
async fn pretty_printed_hook_payload_parses() {
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
            let _ = run_listener(sock, log, cmd_tx, no_repos()).await;
        });
    }

    let summary = spawn_bash(&cmd_tx).await;

    // Mimic Claude Code's pretty-printed payload (newlines between fields)
    // — what the listener sees when jq isn't on the hook script's PATH.
    let pretty = format!(
        "{{\n  \"hook_event_name\": \"Notification\",\n  \"session_id\": \"claude-1\",\n  \"session_orch_id\": \"{}\",\n  \"cwd\": \"/tmp\"\n}}\n",
        summary.id
    );
    send_hook_bytes(&sock, pretty.into_bytes()).await;

    let status = wait_for_status_change(&mut event_rx, &summary.id).await;
    assert_eq!(status, Status::NeedsInput);
}

#[tokio::test]
async fn listener_populates_inferred_repo_name_from_cwd() {
    let dir = tempdir().expect("tempdir");
    let sock = dir.path().join("hooks.sock");
    let log = dir.path().join("events.jsonl");

    let (event_tx, mut event_rx) = mpsc::unbounded_channel();
    let cmd_tx = SessionRegistryActor::spawn(event_tx);

    let repos: Arc<dyn RepoLookup> = Arc::new(PrefixRepoLookup {
        prefix: "/work/alpha".into(),
        repo: "alpha".into(),
    });

    {
        let sock = sock.clone();
        let log = log.clone();
        let cmd_tx = cmd_tx.clone();
        let repos = Arc::clone(&repos);
        tokio::spawn(async move {
            let _ = run_listener(sock, log, cmd_tx, repos).await;
        });
    }

    let summary = spawn_bash(&cmd_tx).await;
    assert!(summary.repo_name.is_none());

    send_hook(
        &sock,
        json!({
            "hook_event_name": "SessionStart",
            "session_id": "claude-1",
            "session_orch_id": summary.id,
            "cwd": "/work/alpha/sub/dir",
        }),
    )
    .await;

    // The listener enriches with inferred_repo_name; the actor then
    // rebuckets the session and emits SessionUpdated.
    let updated = timeout(Duration::from_secs(5), async {
        loop {
            match event_rx.recv().await.expect("event channel closed") {
                RegistryEvent::SessionUpdated(s) if s.id == summary.id => return s,
                _ => continue,
            }
        }
    })
    .await
    .expect("SessionUpdated timed out");

    assert_eq!(updated.repo_name.as_deref(), Some("alpha"));
    assert_eq!(updated.title, "Claude · alpha");
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
            let _ = run_listener(sock, log, cmd_tx, no_repos()).await;
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
