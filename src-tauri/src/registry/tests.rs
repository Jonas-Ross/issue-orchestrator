use std::time::Duration;

use tokio::sync::{mpsc, oneshot};
use tokio::time::timeout;

use super::{
    RegistryCmd, RegistryEvent, SessionRegistryActor, SessionSummary, SpawnSpec, Status,
};

/// Test helper: drives a Spawn command and returns the resulting summary.
async fn spawn_bash(
    tx: &mpsc::Sender<RegistryCmd>,
    cols: u16,
    rows: u16,
) -> SessionSummary {
    let (reply_tx, reply_rx) = oneshot::channel();
    tx.send(RegistryCmd::Spawn {
        spec: SpawnSpec::Bash,
        cols,
        rows,
        reply: reply_tx,
    })
    .await
    .expect("send Spawn");
    reply_rx
        .await
        .expect("Spawn reply")
        .expect("Spawn succeeded")
}

async fn list(tx: &mpsc::Sender<RegistryCmd>) -> Vec<SessionSummary> {
    let (reply_tx, reply_rx) = oneshot::channel();
    tx.send(RegistryCmd::List { reply: reply_tx })
        .await
        .expect("send List");
    reply_rx.await.expect("List reply")
}

async fn kill(tx: &mpsc::Sender<RegistryCmd>, id: &str) {
    let (reply_tx, reply_rx) = oneshot::channel();
    tx.send(RegistryCmd::Kill {
        id: id.to_owned(),
        reply: reply_tx,
    })
    .await
    .expect("send Kill");
    reply_rx
        .await
        .expect("Kill reply")
        .expect("Kill succeeded");
}

/// Wait for the next event matching `pred`, draining anything else.
/// Times out after 5s to keep CI honest.
async fn wait_for<F>(rx: &mut mpsc::UnboundedReceiver<RegistryEvent>, pred: F) -> RegistryEvent
where
    F: Fn(&RegistryEvent) -> bool,
{
    timeout(Duration::from_secs(5), async {
        loop {
            let evt = rx.recv().await.expect("event channel closed");
            if pred(&evt) {
                return evt;
            }
        }
    })
    .await
    .expect("waiting for event timed out")
}

#[tokio::test]
async fn spawn_then_kill_round_trip() {
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();
    let cmd_tx = SessionRegistryActor::spawn(event_tx);

    let summary = spawn_bash(&cmd_tx, 80, 24).await;
    assert_eq!(summary.title, "bash");
    assert_eq!(summary.status, Status::Running);

    let added_id = match wait_for(&mut event_rx, |e| matches!(e, RegistryEvent::SessionAdded(_)))
        .await
    {
        RegistryEvent::SessionAdded(s) => s.id,
        _ => unreachable!(),
    };
    assert_eq!(added_id, summary.id);

    let listed = list(&cmd_tx).await;
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, summary.id);

    kill(&cmd_tx, &summary.id).await;

    let removed = match wait_for(&mut event_rx, |e| {
        matches!(e, RegistryEvent::SessionRemoved { .. })
    })
    .await
    {
        RegistryEvent::SessionRemoved { session_id } => session_id,
        _ => unreachable!(),
    };
    assert_eq!(removed, summary.id);

    let listed = list(&cmd_tx).await;
    assert!(listed.is_empty(), "session should be gone after Kill");
}

#[tokio::test]
async fn write_to_unknown_session_errors() {
    let (event_tx, _event_rx) = mpsc::unbounded_channel();
    let cmd_tx = SessionRegistryActor::spawn(event_tx);

    let (reply_tx, reply_rx) = oneshot::channel();
    cmd_tx
        .send(RegistryCmd::Write {
            id: "no-such-session".to_owned(),
            data: "ls\n".to_owned(),
            reply: reply_tx,
        })
        .await
        .expect("send Write");

    let result = reply_rx.await.expect("Write reply");
    assert!(result.is_err(), "expected SessionNotFound, got {:?}", result);
}

#[tokio::test]
async fn pty_data_events_flow_after_spawn() {
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();
    let cmd_tx = SessionRegistryActor::spawn(event_tx);

    let summary = spawn_bash(&cmd_tx, 80, 24).await;

    // Kick the shell so it definitely produces output we can observe.
    let (reply_tx, reply_rx) = oneshot::channel();
    cmd_tx
        .send(RegistryCmd::Write {
            id: summary.id.clone(),
            data: "echo hello-orch\n".to_owned(),
            reply: reply_tx,
        })
        .await
        .expect("send Write");
    reply_rx.await.expect("Write reply").expect("Write succeeded");

    let needle = "hello-orch";
    let evt = wait_for(&mut event_rx, |e| match e {
        RegistryEvent::PtyData {
            session_id,
            chunk,
        } => session_id == &summary.id && chunk.contains(needle),
        _ => false,
    })
    .await;

    match evt {
        RegistryEvent::PtyData { chunk, .. } => assert!(chunk.contains(needle)),
        _ => unreachable!(),
    }

    kill(&cmd_tx, &summary.id).await;
}
