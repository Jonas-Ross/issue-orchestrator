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

    /// Register a repo by absolute path. Canonicalizes the path so symlinks
    /// and `..` segments don't produce duplicates, derives `name` from the
    /// folder basename, and suffixes `-2`, `-3`, … on name collisions with
    /// a *different* path. Returns the resulting entry.
    ///
    /// If the same canonical path is added twice the existing entry wins
    /// and is returned unchanged.
    pub fn add_repo(&mut self, path: &Path) -> Result<RepoEntry> {
        let canonical = std::fs::canonicalize(path)
            .map_err(|e| Error::Config(format!("canonicalize {}: {e}", path.display())))?;
        let canonical_str = canonical.display().to_string();

        if let Some(existing) = self.repos.iter().find(|r| r.path == canonical_str) {
            return Ok(existing.clone());
        }

        let base = canonical
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| Error::Config(format!("no folder name in {}", canonical.display())))?
            .to_owned();

        let name = unique_repo_name(&self.repos, &base);
        let entry = RepoEntry {
            name,
            path: canonical_str,
        };
        self.repos.push(entry.clone());
        Ok(entry)
    }

    pub fn remove_repo(&mut self, name: &str) -> Result<()> {
        let before = self.repos.len();
        self.repos.retain(|r| r.name != name);
        if self.repos.len() == before {
            return Err(Error::Config(format!("no repo named {name}")));
        }
        Ok(())
    }
}

fn unique_repo_name(existing: &[RepoEntry], base: &str) -> String {
    if !existing.iter().any(|r| r.name == base) {
        return base.to_owned();
    }
    for n in 2.. {
        let candidate = format!("{base}-{n}");
        if !existing.iter().any(|r| r.name == candidate) {
            return candidate;
        }
    }
    unreachable!("u64 of names exhausted");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_config() -> Config {
        Config::default()
    }

    fn make_git_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        dir
    }

    #[test]
    fn add_repo_succeeds_on_canonical_path() {
        let dir = make_git_dir();
        let mut config = empty_config();
        let entry = config.add_repo(dir.path()).unwrap();
        let canonical = std::fs::canonicalize(dir.path()).unwrap();
        assert_eq!(entry.path, canonical.display().to_string());
        assert_eq!(config.repos.len(), 1);
    }

    #[test]
    fn add_repo_dedupes_by_canonical_path() {
        let dir = make_git_dir();
        let mut config = empty_config();
        config.add_repo(dir.path()).unwrap();
        let nested = dir.path().join(".").join(".");
        let entry2 = config.add_repo(&nested).unwrap();
        assert_eq!(config.repos.len(), 1);
        assert_eq!(entry2.path, config.repos[0].path);
    }

    #[test]
    fn add_repo_collision_appends_counter() {
        let parent_a = tempfile::tempdir().unwrap();
        let parent_b = tempfile::tempdir().unwrap();
        let a = parent_a.path().join("repo");
        let b = parent_b.path().join("repo");
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();

        let mut config = empty_config();
        let e1 = config.add_repo(&a).unwrap();
        let e2 = config.add_repo(&b).unwrap();
        assert_eq!(e1.name, "repo");
        assert_eq!(e2.name, "repo-2");
    }

    #[test]
    fn remove_repo_drops_entry() {
        let dir = make_git_dir();
        let mut config = empty_config();
        let entry = config.add_repo(dir.path()).unwrap();
        config.remove_repo(&entry.name).unwrap();
        assert!(config.repos.is_empty());
    }

    #[test]
    fn remove_repo_unknown_name_errors() {
        let mut config = empty_config();
        let err = config.remove_repo("nope").unwrap_err();
        assert!(matches!(err, Error::Config(_)));
    }
}
