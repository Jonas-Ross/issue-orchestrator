use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::{Error, Result};

#[derive(Clone, Debug, Type, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoEntry {
    pub name: String,
    pub path: String,
}

#[derive(Clone, Debug, Type, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub version: u32,
    pub worktree_root: String,
    pub repos: Vec<RepoEntry>,
    pub setup_done: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: 1,
            worktree_root: "~/dev/worktrees".into(),
            repos: Vec::new(),
            setup_done: false,
        }
    }
}

impl Config {
    pub fn load_or_default(path: &Path) -> Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(s) => serde_json::from_str(&s)
                .map_err(|e| Error::Config(format!("parse {}: {e}", path.display()))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(Error::Io(e)),
        }
    }

    /// Atomic save: write to `<path>.tmp`, then rename. Avoids leaving a
    /// truncated file on disk if the process dies mid-write.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("json.tmp");
        let body = serde_json::to_string_pretty(self).map_err(|e| Error::Config(e.to_string()))?;
        std::fs::write(&tmp, body)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    pub fn worktree_root_expanded(&self) -> PathBuf {
        PathBuf::from(shellexpand::tilde(&self.worktree_root).into_owned())
    }
}
