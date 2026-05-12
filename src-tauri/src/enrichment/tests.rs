use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::{mpsc, oneshot};
use tokio::time::timeout;

use crate::enrichment::{ChecksRollup, EnrichmentActor, EnrichmentCmd, PrInspector, PrStatus, SessionInfo};
use crate::error::Result;
use crate::registry::{RegistryCmd, RegistryEvent, SessionRegistryActor, SpawnSpec};

struct MockInspector {
    result: Option<PrStatus>,
}

#[async_trait]
impl PrInspector for MockInspector {
    async fn pr_for_branch(&self, _repo_path: &str, _branch: &str) -> Result<Option<PrStatus>> {
        Ok(self.result.clone())
    }
}

async fn spawn_bash(registry_tx: &mpsc::Sender<RegistryCmd>) -> String {
    let (reply_tx, reply_rx) = oneshot::channel();
    registry_tx
        .send(RegistryCmd::Spawn {
            spec: SpawnSpec::Bash,
            cols: 80,
            rows: 24,
            reply: reply_tx,
        })
        .await
        .unwrap();
    reply_rx.await.unwrap().unwrap().id
}

async fn wait_for_pr_change(
    rx: &mut mpsc::UnboundedReceiver<RegistryEvent>,
) -> (String, Option<PrStatus>) {
    timeout(Duration::from_secs(5), async {
        loop {
            match rx.recv().await.expect("channel closed") {
                RegistryEvent::PrStatusChange { session_id, pr_status } => {
                    return (session_id, pr_status);
                }
                _ => continue,
            }
        }
    })
    .await
    .expect("timed out waiting for PrStatusChange")
}

#[tokio::test]
async fn refresh_now_emits_pr_status_change_with_open_pr() {
    let pr = PrStatus {
        number: 42,
        url: "https://github.com/foo/bar/pull/42".into(),
        checks: ChecksRollup::Pass,
    };

    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<RegistryEvent>();
    let registry_tx = SessionRegistryActor::spawn(event_tx);
    let session_id = spawn_bash(&registry_tx).await;

    let enrich_tx = EnrichmentActor::spawn(
        registry_tx,
        Arc::new(MockInspector { result: Some(pr.clone()) }),
    );

    enrich_tx
        .send(EnrichmentCmd::UpsertSession(SessionInfo {
            id: session_id.clone(),
            branch: "feature/my-branch".into(),
            repo_path: "/fake/repo".into(),
        }))
        .await
        .unwrap();

    enrich_tx.send(EnrichmentCmd::RefreshNow).await.unwrap();

    let (sid, ps) = wait_for_pr_change(&mut event_rx).await;
    assert_eq!(sid, session_id);
    assert_eq!(ps, Some(pr));
}

#[tokio::test]
async fn refresh_now_emits_pr_status_change_when_pr_disappears() {
    // Simulate a PR that existed (Some) and then disappeared (None).
    // The registry deduplicates: None→None is a no-op, but Some→None fires.
    let pr = PrStatus {
        number: 3,
        url: "https://github.com/foo/bar/pull/3".into(),
        checks: ChecksRollup::Pass,
    };

    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<RegistryEvent>();
    let registry_tx = SessionRegistryActor::spawn(event_tx);
    let session_id = spawn_bash(&registry_tx).await;

    // First: inspector returns a PR.
    let enrich_tx = EnrichmentActor::spawn(
        registry_tx.clone(),
        Arc::new(MockInspector { result: Some(pr.clone()) }),
    );
    enrich_tx
        .send(EnrichmentCmd::UpsertSession(SessionInfo {
            id: session_id.clone(),
            branch: "feature/disappearing".into(),
            repo_path: "/fake/repo".into(),
        }))
        .await
        .unwrap();
    enrich_tx.send(EnrichmentCmd::RefreshNow).await.unwrap();
    let (_, ps) = wait_for_pr_change(&mut event_rx).await;
    assert_eq!(ps, Some(pr));

    // Now push a None directly via the registry to simulate the PR closing.
    registry_tx
        .send(RegistryCmd::UpdatePrStatus {
            id: session_id.clone(),
            pr_status: None,
        })
        .await
        .unwrap();

    let (sid, ps) = wait_for_pr_change(&mut event_rx).await;
    assert_eq!(sid, session_id);
    assert!(ps.is_none(), "PR closing should emit PrStatusChange with None");
}

#[tokio::test]
async fn refresh_now_produces_no_event_when_no_pr_exists_from_start() {
    // When a session has never had a PR and the inspector returns None,
    // the registry deduplicates (None→None) and no PrStatusChange is emitted.
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<RegistryEvent>();
    let registry_tx = SessionRegistryActor::spawn(event_tx);
    let session_id = spawn_bash(&registry_tx).await;

    let enrich_tx = EnrichmentActor::spawn(
        registry_tx,
        Arc::new(MockInspector { result: None }),
    );
    enrich_tx
        .send(EnrichmentCmd::UpsertSession(SessionInfo {
            id: session_id.clone(),
            branch: "feature/no-pr".into(),
            repo_path: "/fake/repo".into(),
        }))
        .await
        .unwrap();
    enrich_tx.send(EnrichmentCmd::RefreshNow).await.unwrap();

    // No PrStatusChange should arrive (None→None is deduped away).
    let no_event = timeout(Duration::from_millis(300), async {
        loop {
            match event_rx.recv().await.unwrap() {
                RegistryEvent::PrStatusChange { session_id: id, .. } if id == session_id => {
                    return false;
                }
                _ => continue,
            }
        }
    })
    .await;
    assert!(
        no_event.is_err(),
        "expected no PrStatusChange when inspector returns None and session starts with None"
    );
}

#[tokio::test]
async fn remove_session_stops_polling() {
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<RegistryEvent>();
    let registry_tx = SessionRegistryActor::spawn(event_tx);
    let session_id = spawn_bash(&registry_tx).await;

    let enrich_tx = EnrichmentActor::spawn(
        registry_tx,
        Arc::new(MockInspector {
            result: Some(PrStatus {
                number: 1,
                url: "https://github.com/foo/bar/pull/1".into(),
                checks: ChecksRollup::Pending,
            }),
        }),
    );

    enrich_tx
        .send(EnrichmentCmd::UpsertSession(SessionInfo {
            id: session_id.clone(),
            branch: "feature/will-be-removed".into(),
            repo_path: "/fake/repo".into(),
        }))
        .await
        .unwrap();

    // Remove session before refreshing.
    enrich_tx
        .send(EnrichmentCmd::RemoveSession(session_id.clone()))
        .await
        .unwrap();

    enrich_tx.send(EnrichmentCmd::RefreshNow).await.unwrap();

    // No PrStatusChange should arrive after removal.
    let no_event = timeout(Duration::from_millis(300), async {
        loop {
            match event_rx.recv().await.unwrap() {
                RegistryEvent::PrStatusChange { .. } => return false,
                _ => continue,
            }
        }
    })
    .await;
    assert!(
        no_event.is_err(),
        "expected no PrStatusChange after session was removed"
    );
}

#[tokio::test]
async fn dedup_suppresses_unchanged_pr_status() {
    let pr = PrStatus {
        number: 5,
        url: "https://github.com/foo/bar/pull/5".into(),
        checks: ChecksRollup::Pass,
    };

    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<RegistryEvent>();
    let registry_tx = SessionRegistryActor::spawn(event_tx);
    let session_id = spawn_bash(&registry_tx).await;

    let enrich_tx = EnrichmentActor::spawn(
        registry_tx,
        Arc::new(MockInspector { result: Some(pr.clone()) }),
    );

    enrich_tx
        .send(EnrichmentCmd::UpsertSession(SessionInfo {
            id: session_id.clone(),
            branch: "feature/stable".into(),
            repo_path: "/fake/repo".into(),
        }))
        .await
        .unwrap();

    // First refresh — should emit.
    enrich_tx.send(EnrichmentCmd::RefreshNow).await.unwrap();
    let (sid, ps) = wait_for_pr_change(&mut event_rx).await;
    assert_eq!(sid, session_id);
    assert_eq!(ps, Some(pr.clone()));

    // Second refresh with same result — registry actor deduplicates, no new event.
    enrich_tx.send(EnrichmentCmd::RefreshNow).await.unwrap();

    let no_change = timeout(Duration::from_millis(300), async {
        loop {
            match event_rx.recv().await.unwrap() {
                RegistryEvent::PrStatusChange { .. } => return false,
                _ => continue,
            }
        }
    })
    .await;
    assert!(no_change.is_err(), "second identical poll must not re-emit PrStatusChange");
}
