pub mod events;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, State};
use tauri_specta::Event;
use tokio::sync::{mpsc, oneshot};
use tracing::warn;

use crate::config::{Config, IssueProvider, RepoEntry};
use crate::issues::secrets::ProviderSecretKind;
use crate::issues::{self, secrets};
use crate::registry::{RegistryCmd, RegistryEvent, SessionId, SessionSummary, SpawnSpec};
use crate::spawn::{self, Decision, GitRunner, Issue};

/// Tauri-managed state — actor mailbox plus paths/config and the git
/// runner. The issue client is constructed per-call by `issues::make_client`
/// from the target repo's `IssueProvider`, so different repos can use
/// different sources (GitHub / Jira / Linear) within one session.
pub struct AppState {
    pub registry: mpsc::Sender<RegistryCmd>,
    pub config_path: PathBuf,
    pub config: Mutex<Config>,
    pub git_runner: Arc<dyn GitRunner>,
    /// Shared HTTP client. `reqwest::Client::new()` allocates a fresh
    /// connection pool and rustls context, so we build it once at app
    /// start and clone the cheap handle into each Jira/Linear client.
    pub http: reqwest::Client,
}

/// Bridge: drains domain events from the actor and re-emits them as the
/// matching typed Tauri events. Spawned once during app setup.
pub fn spawn_event_bridge(app: AppHandle, mut rx: mpsc::UnboundedReceiver<RegistryEvent>) {
    tauri::async_runtime::spawn(async move {
        while let Some(evt) = rx.recv().await {
            if let Err(e) = forward(&app, evt) {
                warn!(?e, "failed to emit Tauri event");
            }
        }
    });
}

fn forward(app: &AppHandle, evt: RegistryEvent) -> tauri::Result<()> {
    match evt {
        RegistryEvent::PtyData { session_id, chunk } => {
            events::PtyData { session_id, chunk }.emit(app)
        }
        RegistryEvent::SessionAdded(summary) => events::SessionAdded(summary).emit(app),
        RegistryEvent::SessionRemoved { session_id } => {
            events::SessionRemoved { session_id }.emit(app)
        }
        RegistryEvent::StatusChange { session_id, status } => {
            events::StatusChange { session_id, status }.emit(app)
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn pty_spawn(
    state: State<'_, AppState>,
    cols: u16,
    rows: u16,
) -> Result<SessionSummary, String> {
    let (tx, rx) = oneshot::channel();
    state
        .registry
        .send(RegistryCmd::Spawn {
            spec: SpawnSpec::Bash,
            cols,
            rows,
            reply: tx,
        })
        .await
        .map_err(|e| e.to_string())?;
    rx.await.map_err(|e| e.to_string())?.map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub async fn pty_write(
    state: State<'_, AppState>,
    id: SessionId,
    data: String,
) -> Result<(), String> {
    let (tx, rx) = oneshot::channel();
    state
        .registry
        .send(RegistryCmd::Write {
            id,
            data,
            reply: tx,
        })
        .await
        .map_err(|e| e.to_string())?;
    rx.await.map_err(|e| e.to_string())?.map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub async fn pty_resize(
    state: State<'_, AppState>,
    id: SessionId,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let (tx, rx) = oneshot::channel();
    state
        .registry
        .send(RegistryCmd::Resize {
            id,
            cols,
            rows,
            reply: tx,
        })
        .await
        .map_err(|e| e.to_string())?;
    rx.await.map_err(|e| e.to_string())?.map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub async fn pty_kill(state: State<'_, AppState>, id: SessionId) -> Result<(), String> {
    let (tx, rx) = oneshot::channel();
    state
        .registry
        .send(RegistryCmd::Kill { id, reply: tx })
        .await
        .map_err(|e| e.to_string())?;
    rx.await.map_err(|e| e.to_string())?.map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub async fn list_sessions(
    state: State<'_, AppState>,
) -> Result<Vec<SessionSummary>, String> {
    let (tx, rx) = oneshot::channel();
    state
        .registry
        .send(RegistryCmd::List { reply: tx })
        .await
        .map_err(|e| e.to_string())?;
    rx.await.map_err(|e| e.to_string())
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SetupState {
    pub setup_done: bool,
}

#[tauri::command]
#[specta::specta]
pub fn get_setup_state(state: State<'_, AppState>) -> Result<SetupState, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    Ok(SetupState {
        setup_done: config.setup_done,
    })
}

#[tauri::command]
#[specta::specta]
pub fn mark_setup_done(state: State<'_, AppState>) -> Result<(), String> {
    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    if config.setup_done {
        return Ok(());
    }
    config.setup_done = true;
    config.save(&state.config_path).map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub fn get_config(state: State<'_, AppState>) -> Result<Config, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    Ok(config.clone())
}

#[tauri::command]
#[specta::specta]
pub fn list_repos(state: State<'_, AppState>) -> Result<Vec<RepoEntry>, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    Ok(config.repos.clone())
}

#[tauri::command]
#[specta::specta]
pub fn add_repo(state: State<'_, AppState>, path: String) -> Result<RepoEntry, String> {
    let path = PathBuf::from(&path);
    spawn::validate_git_repo(&path).map_err(|e| e.to_string())?;
    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    let entry = config.add_repo(&path).map_err(|e| e.to_string())?;
    config.save(&state.config_path).map_err(|e| e.to_string())?;
    Ok(entry)
}

#[tauri::command]
#[specta::specta]
pub async fn remove_repo(state: State<'_, AppState>, name: String) -> Result<(), String> {
    let (tx, rx) = oneshot::channel();
    state
        .registry
        .send(RegistryCmd::List { reply: tx })
        .await
        .map_err(|e| e.to_string())?;
    let sessions = rx.await.map_err(|e| e.to_string())?;
    let live: Vec<String> = sessions
        .iter()
        .filter(|s| s.repo_name.as_deref() == Some(&name))
        .map(|s| s.id.clone())
        .collect();
    if !live.is_empty() {
        return Err(format!(
            "Kill this repo's sessions first: {}",
            live.join(", ")
        ));
    }
    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    config.remove_repo(&name).map_err(|e| e.to_string())?;
    config.save(&state.config_path).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn list_issues(
    state: State<'_, AppState>,
    repo_name: String,
) -> Result<Vec<Issue>, String> {
    let repo = lookup_repo(&state, &repo_name)?;
    let client = issues::make_client(&repo, &state.http).map_err(|e| e.to_string())?;
    let path = PathBuf::from(&repo.path);
    client.list(&path).await.map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub async fn get_issue_body(
    state: State<'_, AppState>,
    repo_name: String,
    issue_id: String,
) -> Result<String, String> {
    let repo = lookup_repo(&state, &repo_name)?;
    let client = issues::make_client(&repo, &state.http).map_err(|e| e.to_string())?;
    let path = PathBuf::from(&repo.path);
    client.body(&path, &issue_id).await.map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub async fn decide_next_issue(
    state: State<'_, AppState>,
    repo_name: String,
) -> Result<Decision, String> {
    let repo = lookup_repo(&state, &repo_name)?;
    let client = issues::make_client(&repo, &state.http).map_err(|e| e.to_string())?;
    spawn::decide_next_issue(&repo, client)
        .await
        .map_err(Into::into)
}

/// Find a repo by name in the locked config, cloning it out so callers
/// don't hold the mutex across `.await`. Returns a string error matching
/// the existing IPC convention.
fn lookup_repo(state: &State<'_, AppState>, repo_name: &str) -> Result<RepoEntry, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    config
        .repos
        .iter()
        .find(|r| r.name == repo_name)
        .cloned()
        .ok_or_else(|| format!("unknown repo: {repo_name}"))
}

/// Persist (or clear, when `template = None`) the user-configured spawn
/// prompt template. Atomic-saves the config file via the existing
/// `Config::save` path.
#[tauri::command]
#[specta::specta]
pub async fn update_spawn_prompt(
    state: State<'_, AppState>,
    template: Option<String>,
) -> Result<(), String> {
    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    config.spawn_prompt_template = template.filter(|t| !t.trim().is_empty());
    config.save(&state.config_path).map_err(|e| e.to_string())?;
    Ok(())
}

/// Ask `claude -p` (running in the chosen repo's cwd) to rewrite the
/// supplied template using whatever skills/MCPs/plugins are visible to a
/// session there. Returns the rewritten template; the caller is
/// responsible for calling `update_spawn_prompt` to actually persist it.
#[tauri::command]
#[specta::specta]
pub async fn optimize_spawn_prompt(
    state: State<'_, AppState>,
    repo_name: String,
    current_prompt: String,
) -> Result<String, String> {
    let repo = lookup_repo(&state, &repo_name)?;
    spawn::optimize_spawn_prompt(&repo, &current_prompt)
        .await
        .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub async fn spawn_issue_session(
    state: State<'_, AppState>,
    repo_name: String,
    issue_id: String,
    cols: u16,
    rows: u16,
    prompt_override: Option<String>,
) -> Result<SessionSummary, String> {
    let repo = lookup_repo(&state, &repo_name)?;
    let config = state.config.lock().map_err(|e| e.to_string())?.clone();
    let client = issues::make_client(&repo, &state.http).map_err(|e| e.to_string())?;

    spawn::spawn_issue_session(
        &repo,
        issue_id,
        &config,
        prompt_override,
        client,
        state.git_runner.clone(),
        state.registry.clone(),
        cols,
        rows,
    )
    .await
    .map_err(Into::into)
}

// ── Per-repo provider configuration & secrets ──────────────────────────

/// Replace a repo's issue provider in-memory and on disk. Refuses while
/// the repo has live sessions — same guard as `remove_repo` — so an open
/// session can't end up with a stale `issue_url`/`branch` shape.
#[tauri::command]
#[specta::specta]
pub async fn update_repo_provider(
    state: State<'_, AppState>,
    repo_name: String,
    provider: IssueProvider,
) -> Result<RepoEntry, String> {
    let (tx, rx) = oneshot::channel();
    state
        .registry
        .send(RegistryCmd::List { reply: tx })
        .await
        .map_err(|e| e.to_string())?;
    let sessions = rx.await.map_err(|e| e.to_string())?;
    let live: Vec<String> = sessions
        .iter()
        .filter(|s| s.repo_name.as_deref() == Some(&repo_name))
        .map(|s| s.id.clone())
        .collect();
    if !live.is_empty() {
        return Err(format!(
            "Kill this repo's sessions first: {}",
            live.join(", ")
        ));
    }

    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    let entry = config
        .update_repo_provider(&repo_name, provider)
        .map_err(|e| e.to_string())?;
    config.save(&state.config_path).map_err(|e| e.to_string())?;
    Ok(entry)
}

/// Write a provider token (Jira/Linear API key) into the macOS Keychain.
/// Tokens are NEVER returned by any other IPC; this is the only path
/// from the renderer to the credential store.
#[tauri::command]
#[specta::specta]
pub fn set_provider_secret(
    repo_name: String,
    kind: ProviderSecretKind,
    token: String,
) -> Result<(), String> {
    secrets::set_token(kind.as_str(), &repo_name, &token).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn delete_provider_secret(
    repo_name: String,
    kind: ProviderSecretKind,
) -> Result<(), String> {
    secrets::delete_token(kind.as_str(), &repo_name).map_err(|e| e.to_string())
}

/// Read-only check the settings UI uses to render "✓ Token saved" vs
/// "Set token…". Never returns the token itself.
#[tauri::command]
#[specta::specta]
pub fn provider_secret_exists(
    repo_name: String,
    kind: ProviderSecretKind,
) -> Result<bool, String> {
    secrets::token_exists(kind.as_str(), &repo_name).map_err(|e| e.to_string())
}
