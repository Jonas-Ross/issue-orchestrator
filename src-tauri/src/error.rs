use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("pty: {0}")]
    Pty(String),
    #[error("registry: {0}")]
    Registry(String),
    #[error("session not found: {0}")]
    SessionNotFound(String),
    #[error("hooks: {0}")]
    Hooks(String),
    /// Reserved for the spawn-flow itself (worktree setup, registry
    /// channel sends). Subsystem-specific failures use the typed
    /// variants below so the user gets meaningful diagnostics.
    #[error("spawn: {0}")]
    Spawn(String),
    #[error("git: {0}")]
    Git(String),
    /// Issue-tracker shellouts (`gh`) and provider construction.
    #[error("issues: {0}")]
    Issues(String),
    /// HTTP failures from Jira/Linear clients (transport, status, JSON).
    #[error("http: {0}")]
    Http(String),
    /// Headless `claude -p` invocations (timeouts, non-zero exits, JSON
    /// parsing of model output).
    #[error("claude: {0}")]
    ClaudeCli(String),
    #[error("config: {0}")]
    Config(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<Error> for String {
    fn from(value: Error) -> Self {
        value.to_string()
    }
}
