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
    #[error("spawn: {0}")]
    Spawn(String),
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
