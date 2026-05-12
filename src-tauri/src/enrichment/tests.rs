use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::{mpsc, oneshot};
use tokio::time::timeout;

use crate::config::{Config, ConfigActor, IssueProvider, RepoEntry};
use crate::enrichment::{
    ChecksRollup, EnrichmentActor, EnrichmentCmd, PrInspector, PrStatus, SessionInfo,
    spawn_enrichment_bridge,
};
use crate::error::Result;
use crate::registry::{RegistryCmd, RegistryEvent, SessionRegistryActor, SessionSummary, SpawnSpec};

struct MockInspector {
    result: Option<PrStatus>,
}

#[async_trait]
impl PrInspector for MockInspector {
    async fn pr_for_branch(&self, _repo_path: &str, _branch: &str) -> Result<Option<PrStatus>> {
        Ok(self.result.clone())
    }
}

/// An inspector that records every (repo_path, branch) pair it is called with.
struct RecordingInspector {
    calls: Arc<Mutex<Vec<(String, String)>>>,
    result: Option<PrStatus>,
}

impl RecordingInspector {
    fn new(result: Option<PrStatus>) -> (Self, Arc<Mutex<Vec<(String, String)>>>) {
        let calls = Arc::new(Mutex::new(Vec::new()));
        (Self { calls: calls.clone(), result }, calls)
    }
}

#[async_trait]
impl PrInspector for RecordingInspector {
    async fn pr_for_branch(&self, repo_path: &str, branch: &str) -> Result<Option<PrStatus>> {
        self.calls
            .lock()
            .unwrap()
            .push((repo_path.to_owned(), branch.to_owned()));
        Ok(self.result.clone())
    }
}

fn make_config_handle(repos: Vec<RepoEntry>) -> crate::config::ConfigHandle {
    let config = Config { repos, ..Config::default() };
    ConfigActor::spawn(config, PathBuf::from("/dev/null"))
}

fn make_session_summary(
    id: &str,
    branch: Option<&str>,
    repo_name: Option<&str>,
) -> SessionSummary {
    SessionSummary {
        id: id.to_owned(),
        title: id.to_owned(),
        status: crate::registry::Status::Running,
        worktree_path: None,
        issue_url: None,
        branch: branch.map(|s| s.to_owned()),
        repo_name: repo_name.map(|s| s.to_owned()),
        pr_status: None,
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
        Duration::from_secs(3600),
    );

    enrich_tx
        .send(EnrichmentCmd::UpsertSession(SessionInfo {
            id: session_id.clone(),
            branch: "feature/my-branch".into(),
            repo_path: "/fake/repo".into(),
        }))
        .await
        .unwrap();

    enrich_tx.send(EnrichmentCmd::RefreshOne(session_id.clone())).await.unwrap();

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
        Duration::from_secs(3600),
    );
    enrich_tx
        .send(EnrichmentCmd::UpsertSession(SessionInfo {
            id: session_id.clone(),
            branch: "feature/disappearing".into(),
            repo_path: "/fake/repo".into(),
        }))
        .await
        .unwrap();
    enrich_tx.send(EnrichmentCmd::RefreshOne(session_id.clone())).await.unwrap();
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
        Duration::from_secs(3600),
    );
    enrich_tx
        .send(EnrichmentCmd::UpsertSession(SessionInfo {
            id: session_id.clone(),
            branch: "feature/no-pr".into(),
            repo_path: "/fake/repo".into(),
        }))
        .await
        .unwrap();
    enrich_tx.send(EnrichmentCmd::RefreshOne(session_id.clone())).await.unwrap();

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
        Duration::from_secs(3600),
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

    enrich_tx.send(EnrichmentCmd::RefreshOne(session_id.clone())).await.unwrap();

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
        Duration::from_secs(3600),
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
    enrich_tx.send(EnrichmentCmd::RefreshOne(session_id.clone())).await.unwrap();
    let (sid, ps) = wait_for_pr_change(&mut event_rx).await;
    assert_eq!(sid, session_id);
    assert_eq!(ps, Some(pr.clone()));

    // Second refresh with same result — registry actor deduplicates, no new event.
    enrich_tx.send(EnrichmentCmd::RefreshOne(session_id.clone())).await.unwrap();

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

// ── AC #2: non-GitHub repos are skipped ─────────────────────────────────────

#[tokio::test]
async fn bridge_skips_jira_repo_session() {
    // Register a Jira-provider repo and add a session that belongs to it.
    // The enrichment bridge must NOT forward the session to the actor, so
    // the inspector mock should never be called.
    let jira_repo = RepoEntry {
        name: "jira-project".to_owned(),
        path: "/fake/jira-repo".to_owned(),
        provider: IssueProvider::Jira {
            base_url: "https://example.atlassian.net".to_owned(),
            email: "dev@example.com".to_owned(),
            project_key: "PROJ".to_owned(),
        },
    };
    let config = make_config_handle(vec![jira_repo]);

    let (inspector, calls) = RecordingInspector::new(None);
    let (event_tx, _event_rx) = mpsc::unbounded_channel::<RegistryEvent>();
    let (enrich_event_tx, enrich_event_rx) = mpsc::unbounded_channel::<RegistryEvent>();
    let registry_tx = SessionRegistryActor::spawn(event_tx);
    let enrich_tx = EnrichmentActor::spawn(registry_tx, Arc::new(inspector), Duration::from_secs(3600));

    spawn_enrichment_bridge(enrich_event_rx, enrich_tx.clone(), config);

    // Emit SessionAdded for a session belonging to the Jira repo.
    let summary = make_session_summary("jira-s1", Some("feature/jira-branch"), Some("jira-project"));
    enrich_event_tx.send(RegistryEvent::SessionAdded(summary)).unwrap();

    // Give the bridge time to process, then trigger a refresh for any id
    // (actor has no sessions since bridge filtered the Jira one out).
    tokio::time::sleep(Duration::from_millis(50)).await;
    enrich_tx.send(EnrichmentCmd::RefreshOne("probe".into())).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    assert_eq!(
        calls.lock().unwrap().len(),
        0,
        "inspector must not be called for Jira-provider repo sessions"
    );
}

#[tokio::test]
async fn bridge_skips_linear_repo_session() {
    let linear_repo = RepoEntry {
        name: "linear-project".to_owned(),
        path: "/fake/linear-repo".to_owned(),
        provider: IssueProvider::Linear { team_key: "ENG".to_owned() },
    };
    let config = make_config_handle(vec![linear_repo]);

    let (inspector, calls) = RecordingInspector::new(None);
    let (enrich_event_tx, enrich_event_rx) = mpsc::unbounded_channel::<RegistryEvent>();
    let (registry_event_tx, _registry_event_rx) = mpsc::unbounded_channel::<RegistryEvent>();
    let registry_tx = SessionRegistryActor::spawn(registry_event_tx);
    let enrich_tx = EnrichmentActor::spawn(registry_tx, Arc::new(inspector), Duration::from_secs(3600));

    spawn_enrichment_bridge(enrich_event_rx, enrich_tx.clone(), config);

    let summary =
        make_session_summary("linear-s1", Some("feature/linear-branch"), Some("linear-project"));
    enrich_event_tx.send(RegistryEvent::SessionAdded(summary)).unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;
    enrich_tx.send(EnrichmentCmd::RefreshOne("probe".into())).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    assert_eq!(
        calls.lock().unwrap().len(),
        0,
        "inspector must not be called for Linear-provider repo sessions"
    );
}

// ── AC #3: Bash sessions (no repo_name / no branch) are skipped ─────────────

#[tokio::test]
async fn bridge_skips_bash_session_with_no_repo_name() {
    let github_repo = RepoEntry {
        name: "my-repo".to_owned(),
        path: "/fake/my-repo".to_owned(),
        provider: IssueProvider::Github,
    };
    let config = make_config_handle(vec![github_repo]);

    let (inspector, calls) = RecordingInspector::new(None);
    let (registry_event_tx, _registry_event_rx) = mpsc::unbounded_channel::<RegistryEvent>();
    let registry_tx = SessionRegistryActor::spawn(registry_event_tx);
    let enrich_tx = EnrichmentActor::spawn(registry_tx, Arc::new(inspector), Duration::from_secs(3600));
    let (enrich_event_tx, enrich_event_rx) = mpsc::unbounded_channel::<RegistryEvent>();

    spawn_enrichment_bridge(enrich_event_rx, enrich_tx.clone(), config);

    // Bash session: no repo_name and no branch.
    let summary = make_session_summary("bash-s1", None, None);
    enrich_event_tx.send(RegistryEvent::SessionAdded(summary)).unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;
    enrich_tx.send(EnrichmentCmd::RefreshOne("probe".into())).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    assert_eq!(
        calls.lock().unwrap().len(),
        0,
        "inspector must not be called for Bash sessions (no repo_name)"
    );
}

// ── AC #5 (backend): non-OPEN PR state surfaces as None ─────────────────────

#[tokio::test]
async fn inspector_returning_none_for_closed_pr_causes_chip_to_disappear() {
    // First tick: inspector returns a PR (session has an open PR).
    // Second tick: inspector returns None (simulates gh filtering a closed/merged PR).
    // The registry must emit PrStatusChange(None) on the second tick.
    let open_pr = PrStatus {
        number: 99,
        url: "https://github.com/foo/bar/pull/99".into(),
        checks: ChecksRollup::Pass,
    };

    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<RegistryEvent>();
    let registry_tx = SessionRegistryActor::spawn(event_tx);
    let session_id = spawn_bash(&registry_tx).await;

    // Phase 1: inspector returns Some — chip should appear.
    let enrich_tx = EnrichmentActor::spawn(
        registry_tx.clone(),
        Arc::new(MockInspector { result: Some(open_pr.clone()) }),
        Duration::from_secs(3600),
    );
    enrich_tx
        .send(EnrichmentCmd::UpsertSession(SessionInfo {
            id: session_id.clone(),
            branch: "feature/pr-closed".into(),
            repo_path: "/fake/repo".into(),
        }))
        .await
        .unwrap();
    enrich_tx.send(EnrichmentCmd::RefreshOne(session_id.clone())).await.unwrap();
    let (_, ps) = wait_for_pr_change(&mut event_rx).await;
    assert_eq!(ps, Some(open_pr), "first tick must set pr_status");

    // Phase 2: inspector now returns None (closed PR filtered at boundary) — chip must vanish.
    // We push None through the registry directly (same path as when GhPrInspector
    // returns Ok(None) for a non-OPEN state).
    registry_tx
        .send(RegistryCmd::UpdatePrStatus {
            id: session_id.clone(),
            pr_status: None,
        })
        .await
        .unwrap();

    let (sid, ps) = wait_for_pr_change(&mut event_rx).await;
    assert_eq!(sid, session_id);
    assert!(
        ps.is_none(),
        "when inspector returns None (closed/merged PR), chip must be removed"
    );
}

// ── AC #8: interval is a constructor parameter ───────────────────────────────

#[tokio::test]
async fn enrichment_actor_polls_on_timer_interval() {
    // The actor accepts an interval: Duration parameter so tests can drive
    // ticks with sub-second intervals without relying on RefreshNow.
    let pr = PrStatus {
        number: 7,
        url: "https://github.com/foo/bar/pull/7".into(),
        checks: ChecksRollup::Pending,
    };

    let (inspector, calls) = RecordingInspector::new(Some(pr.clone()));
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<RegistryEvent>();
    let registry_tx = SessionRegistryActor::spawn(event_tx);
    let session_id = spawn_bash(&registry_tx).await;

    // Construct actor with a 80ms tick (production uses 30s).
    let enrich_tx = EnrichmentActor::spawn(
        registry_tx,
        Arc::new(inspector),
        Duration::from_millis(80),
    );

    enrich_tx
        .send(EnrichmentCmd::UpsertSession(SessionInfo {
            id: session_id.clone(),
            branch: "feature/timer-driven".into(),
            repo_path: "/fake/repo".into(),
        }))
        .await
        .unwrap();

    // Wait 350ms — enough for at least 2 timer ticks at 80ms interval.
    // The first tick fires immediately on interval creation (MissedTickBehavior::Skip
    // means the very first tick is instant); we expect ≥2 inspector calls from
    // the timer alone (no RefreshNow sent).
    tokio::time::sleep(Duration::from_millis(350)).await;

    // Drain events so we don't block — we care about call count, not events.
    while let Ok(evt) = event_rx.try_recv() {
        let _ = evt;
    }

    let call_count = calls.lock().unwrap().len();
    assert!(
        call_count >= 2,
        "inspector should be called at least twice by the timer within 350ms at 80ms interval, got {call_count}"
    );
}
