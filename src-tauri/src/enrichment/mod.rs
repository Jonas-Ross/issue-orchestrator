use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use specta::Type;
use tokio::sync::mpsc;
use tokio::time;
use tracing::{info, warn};

use crate::config::{ConfigHandle, IssueProvider};
use crate::error::Result;
use crate::registry::{RegistryCmd, RegistryEvent, SessionId};

#[cfg(test)]
pub mod tests;

/// Rollup of all CI check statuses for a PR.
#[derive(Clone, Debug, PartialEq, Eq, Type, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChecksRollup {
    /// All checks passed.
    Pass,
    /// At least one check failed.
    Fail,
    /// Checks are queued or running.
    Pending,
    /// No checks have been reported.
    None,
}

/// Snapshot of the PR open for a worktree branch.
#[derive(Clone, Debug, PartialEq, Eq, Type, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrStatus {
    pub number: u32,
    pub url: String,
    pub checks: ChecksRollup,
}

/// Boundary trait for PR lookups. `GhPrInspector` is the production impl;
/// tests substitute a recording mock.
#[async_trait]
pub trait PrInspector: Send + Sync {
    async fn pr_for_branch(&self, repo_path: &str, branch: &str) -> Result<Option<PrStatus>>;
}

/// Production impl: shells out to `gh pr view`.
pub struct GhPrInspector;

#[async_trait]
impl PrInspector for GhPrInspector {
    async fn pr_for_branch(&self, repo_path: &str, branch: &str) -> Result<Option<PrStatus>> {
        let output = tokio::process::Command::new("gh")
            .arg("pr")
            .arg("view")
            .arg(branch)
            .args(["--json", "number,state,statusCheckRollup,url"])
            .current_dir(repo_path)
            .output()
            .await
            .map_err(|e| crate::error::Error::Issues(format!("gh pr view: {e}")))?;

        if !output.status.success() {
            // gh exits non-zero when no PR exists for the branch
            return Ok(None);
        }

        let val: serde_json::Value = serde_json::from_slice(&output.stdout)
            .map_err(|e| crate::error::Error::Issues(format!("gh pr view parse: {e}")))?;

        if val.get("state").and_then(|s| s.as_str()) != Some("OPEN") {
            return Ok(None);
        }

        let number = val["number"].as_u64().unwrap_or(0) as u32;
        let url = val["url"].as_str().unwrap_or("").to_owned();

        let checks = rollup_from_json(val.get("statusCheckRollup"));

        Ok(Some(PrStatus { number, url, checks }))
    }
}

fn rollup_from_json(val: Option<&serde_json::Value>) -> ChecksRollup {
    let Some(arr) = val.and_then(|v| v.as_array()) else {
        return ChecksRollup::None;
    };
    if arr.is_empty() {
        return ChecksRollup::None;
    }

    let mut any_fail = false;
    let mut any_pending = false;

    for check in arr {
        let conclusion = check
            .get("conclusion")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let status = check.get("status").and_then(|v| v.as_str()).unwrap_or("");

        match conclusion {
            "FAILURE" | "TIMED_OUT" | "CANCELLED" | "ACTION_REQUIRED" => any_fail = true,
            "SUCCESS" | "NEUTRAL" | "SKIPPED" => {}
            _ => {
                if matches!(status, "QUEUED" | "IN_PROGRESS" | "WAITING" | "PENDING") {
                    any_pending = true;
                }
            }
        }
    }

    if any_fail {
        ChecksRollup::Fail
    } else if any_pending {
        ChecksRollup::Pending
    } else {
        ChecksRollup::Pass
    }
}

/// Snapshot of a session that the actor needs for polling.
#[derive(Clone)]
pub struct SessionInfo {
    pub id: SessionId,
    pub branch: String,
    pub repo_path: String,
}

/// Commands sent to the enrichment actor's mailbox.
pub enum EnrichmentCmd {
    /// Notify the actor that a session was added/updated with these details.
    UpsertSession(SessionInfo),
    /// Notify the actor that a session was removed and can be dropped.
    RemoveSession(SessionId),
    /// Trigger an immediate one-shot refresh for a single session (e.g. on status change).
    RefreshOne(SessionId),
}

/// The enrichment actor. Ticks on the given interval and on `RefreshOne`,
/// querying `PrInspector` for each tracked session and forwarding changes
/// into the registry via `RegistryCmd::UpdatePrStatus`.
pub struct EnrichmentActor {
    rx: mpsc::Receiver<EnrichmentCmd>,
    registry: mpsc::Sender<RegistryCmd>,
    inspector: Arc<dyn PrInspector>,
    sessions: HashMap<SessionId, SessionInfo>,
    interval: Duration,
}

impl EnrichmentActor {
    pub fn spawn(
        registry: mpsc::Sender<RegistryCmd>,
        inspector: Arc<dyn PrInspector>,
        interval: Duration,
    ) -> mpsc::Sender<EnrichmentCmd> {
        let (tx, rx) = mpsc::channel(64);
        let actor = Self {
            rx,
            registry,
            inspector,
            sessions: HashMap::new(),
            interval,
        };
        tauri::async_runtime::spawn(actor.run());
        tx
    }

    async fn run(mut self) {
        info!("enrichment actor started");
        let mut interval = time::interval(self.interval);
        interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                Some(cmd) = self.rx.recv() => {
                    match cmd {
                        EnrichmentCmd::UpsertSession(info) => {
                            self.sessions.insert(info.id.clone(), info);
                        }
                        EnrichmentCmd::RemoveSession(id) => {
                            self.sessions.remove(&id);
                        }
                        EnrichmentCmd::RefreshOne(id) => {
                            self.poll_one(&id).await;
                        }
                    }
                }
                _ = interval.tick() => {
                    self.poll_all().await;
                }
                else => break,
            }
        }
        info!("enrichment actor stopped");
    }

    async fn poll_all(&self) {
        for info in self.sessions.values() {
            self.poll_session(info).await;
        }
    }

    async fn poll_one(&self, id: &str) {
        if let Some(info) = self.sessions.get(id) {
            self.poll_session(info).await;
        }
    }

    async fn poll_session(&self, info: &SessionInfo) {
        match self
            .inspector
            .pr_for_branch(&info.repo_path, &info.branch)
            .await
        {
            Ok(pr_status) => {
                let cmd = RegistryCmd::UpdatePrStatus {
                    id: info.id.clone(),
                    pr_status,
                };
                if let Err(e) = self.registry.send(cmd).await {
                    warn!(?e, "enrichment: registry channel closed");
                }
            }
            Err(e) => {
                warn!(session_id = %info.id, ?e, "enrichment: pr_for_branch failed");
            }
        }
    }
}

/// Helper: subscribe to `RegistryEvent` and keep the enrichment actor in sync.
/// Only forwards sessions that have both a branch and a repo path (i.e. GitHub
/// worktree sessions). Ad-hoc/bash sessions never have a branch.
pub fn spawn_enrichment_bridge(
    mut event_rx: mpsc::UnboundedReceiver<RegistryEvent>,
    enrichment_tx: mpsc::Sender<EnrichmentCmd>,
    config: ConfigHandle,
) {
    tauri::async_runtime::spawn(async move {
        while let Some(evt) = event_rx.recv().await {
            match evt {
                RegistryEvent::SessionAdded(summary) => {
                    if let (Some(branch), Some(repo_name)) = (&summary.branch, &summary.repo_name) {
                        if let Ok(repo) = config.lookup_repo(repo_name).await {
                            if matches!(repo.provider, IssueProvider::Github) {
                                let info = SessionInfo {
                                    id: summary.id.clone(),
                                    branch: branch.clone(),
                                    repo_path: repo.path.clone(),
                                };
                                let _ = enrichment_tx.send(EnrichmentCmd::UpsertSession(info)).await;
                            }
                        }
                    }
                }
                RegistryEvent::SessionRemoved { session_id } => {
                    let _ = enrichment_tx
                        .send(EnrichmentCmd::RemoveSession(session_id))
                        .await;
                }
                RegistryEvent::StatusChange { session_id, .. } => {
                    let _ = enrichment_tx.send(EnrichmentCmd::RefreshOne(session_id)).await;
                }
                _ => {}
            }
        }
    });
}
