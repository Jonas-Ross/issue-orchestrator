//! Actor wrapping the on-disk `Config`. Every read or mutation goes
//! through the typed mailbox, so by construction no caller can hold the
//! config across an `.await` and deadlock-shaped bugs are designed away.

use std::path::{Path, PathBuf};

use tokio::sync::{mpsc, oneshot};
use tracing::info;

use crate::error::{Error, Result};

use super::{Config, IssueProvider, RepoEntry};

enum ConfigCmd {
    Snapshot {
        reply: oneshot::Sender<Config>,
    },
    LookupRepo {
        name: String,
        reply: oneshot::Sender<Result<RepoEntry>>,
    },
    ListRepos {
        reply: oneshot::Sender<Vec<RepoEntry>>,
    },
    GetSetupState {
        reply: oneshot::Sender<bool>,
    },
    AddRepo {
        path: PathBuf,
        reply: oneshot::Sender<Result<RepoEntry>>,
    },
    RemoveRepo {
        name: String,
        reply: oneshot::Sender<Result<()>>,
    },
    UpdateRepoProvider {
        name: String,
        provider: IssueProvider,
        reply: oneshot::Sender<Result<RepoEntry>>,
    },
    UpdateSpawnPrompt {
        template: Option<String>,
        reply: oneshot::Sender<Result<()>>,
    },
    MarkSetupDone {
        reply: oneshot::Sender<Result<()>>,
    },
}

pub struct ConfigActor {
    config: Config,
    path: PathBuf,
    rx: mpsc::Receiver<ConfigCmd>,
}

impl ConfigActor {
    /// Boot the actor on the current tokio runtime and return a cheap
    /// `ConfigHandle` cloneable into `AppState`.
    pub fn spawn(config: Config, path: PathBuf) -> ConfigHandle {
        let (tx, rx) = mpsc::channel(32);
        let actor = Self { config, path, rx };
        tauri::async_runtime::spawn(actor.run());
        ConfigHandle { tx }
    }

    async fn run(mut self) {
        info!("config actor started");
        while let Some(cmd) = self.rx.recv().await {
            match cmd {
                ConfigCmd::Snapshot { reply } => {
                    let _ = reply.send(self.config.clone());
                }
                ConfigCmd::LookupRepo { name, reply } => {
                    let _ = reply.send(self.lookup(&name));
                }
                ConfigCmd::ListRepos { reply } => {
                    let _ = reply.send(self.config.repos.clone());
                }
                ConfigCmd::GetSetupState { reply } => {
                    let _ = reply.send(self.config.setup_done);
                }
                ConfigCmd::AddRepo { path, reply } => {
                    let _ = reply.send(self.add(&path));
                }
                ConfigCmd::RemoveRepo { name, reply } => {
                    let _ = reply.send(self.remove(&name));
                }
                ConfigCmd::UpdateRepoProvider { name, provider, reply } => {
                    let _ = reply.send(self.set_provider(&name, provider));
                }
                ConfigCmd::UpdateSpawnPrompt { template, reply } => {
                    let _ = reply.send(self.set_spawn_prompt(template));
                }
                ConfigCmd::MarkSetupDone { reply } => {
                    let _ = reply.send(self.mark_setup_done());
                }
            }
        }
        info!("config actor stopped");
    }

    fn lookup(&self, name: &str) -> Result<RepoEntry> {
        self.config
            .repos
            .iter()
            .find(|r| r.name == name)
            .cloned()
            .ok_or_else(|| Error::Config(format!("unknown repo: {name}")))
    }

    fn add(&mut self, path: &Path) -> Result<RepoEntry> {
        let entry = self.config.add_repo(path)?;
        self.persist()?;
        Ok(entry)
    }

    fn remove(&mut self, name: &str) -> Result<()> {
        self.config.remove_repo(name)?;
        self.persist()
    }

    fn set_provider(&mut self, name: &str, provider: IssueProvider) -> Result<RepoEntry> {
        let entry = self.config.update_repo_provider(name, provider)?;
        self.persist()?;
        Ok(entry)
    }

    fn set_spawn_prompt(&mut self, template: Option<String>) -> Result<()> {
        self.config.spawn_prompt_template = template.filter(|t| !t.trim().is_empty());
        self.persist()
    }

    fn mark_setup_done(&mut self) -> Result<()> {
        if self.config.setup_done {
            return Ok(());
        }
        self.config.setup_done = true;
        self.persist()
    }

    fn persist(&self) -> Result<()> {
        self.config.save(&self.path)
    }
}

/// Cheap handle stored in `AppState`. Cloning forwards to the same actor
/// mailbox.
#[derive(Clone)]
pub struct ConfigHandle {
    tx: mpsc::Sender<ConfigCmd>,
}

impl ConfigHandle {
    pub async fn snapshot(&self) -> Result<Config> {
        self.ask(|reply| ConfigCmd::Snapshot { reply }).await
    }

    pub async fn lookup_repo(&self, name: &str) -> Result<RepoEntry> {
        self.ask_fallible(|reply| ConfigCmd::LookupRepo {
            name: name.to_owned(),
            reply,
        })
        .await
    }

    pub async fn list_repos(&self) -> Result<Vec<RepoEntry>> {
        self.ask(|reply| ConfigCmd::ListRepos { reply }).await
    }

    pub async fn get_setup_state(&self) -> Result<bool> {
        self.ask(|reply| ConfigCmd::GetSetupState { reply }).await
    }

    pub async fn add_repo(&self, path: PathBuf) -> Result<RepoEntry> {
        self.ask_fallible(|reply| ConfigCmd::AddRepo { path, reply })
            .await
    }

    pub async fn remove_repo(&self, name: String) -> Result<()> {
        self.ask_fallible(|reply| ConfigCmd::RemoveRepo { name, reply })
            .await
    }

    pub async fn update_repo_provider(
        &self,
        name: String,
        provider: IssueProvider,
    ) -> Result<RepoEntry> {
        self.ask_fallible(|reply| ConfigCmd::UpdateRepoProvider {
            name,
            provider,
            reply,
        })
        .await
    }

    pub async fn update_spawn_prompt(&self, template: Option<String>) -> Result<()> {
        self.ask_fallible(|reply| ConfigCmd::UpdateSpawnPrompt { template, reply })
            .await
    }

    pub async fn mark_setup_done(&self) -> Result<()> {
        self.ask_fallible(|reply| ConfigCmd::MarkSetupDone { reply })
            .await
    }

    /// Round-trip a command whose reply is the value itself (infallible
    /// reads).
    async fn ask<R>(&self, build: impl FnOnce(oneshot::Sender<R>) -> ConfigCmd) -> Result<R> {
        let (tx, rx) = oneshot::channel();
        self.send(build(tx)).await?;
        rx.await.map_err(channel_err)
    }

    /// Round-trip a command whose reply is itself a `Result` (mutators
    /// that can fail to persist or to find a target).
    async fn ask_fallible<R>(
        &self,
        build: impl FnOnce(oneshot::Sender<Result<R>>) -> ConfigCmd,
    ) -> Result<R> {
        self.ask(build).await?
    }

    async fn send(&self, cmd: ConfigCmd) -> Result<()> {
        self.tx
            .send(cmd)
            .await
            .map_err(|e| Error::Config(format!("config actor: {e}")))
    }
}

fn channel_err(e: oneshot::error::RecvError) -> Error {
    Error::Config(format!("config actor reply dropped: {e}"))
}
