use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::{Error, Result};

/// Per-repo issue source. The `kind` discriminator is what the factory in
/// `crate::issues::make_client` matches on. Tokens for `Jira` / `Linear`
/// live in the macOS Keychain — never in this struct, never in
/// `config.json`.
#[derive(Clone, Debug, Type, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum IssueProvider {
    Github,
    Jira {
        base_url: String,
        email: String,
        project_key: String,
    },
    Linear {
        team_key: String,
    },
}

impl Default for IssueProvider {
    fn default() -> Self {
        Self::Github
    }
}

impl IssueProvider {
    /// Stable identifier used as the Keychain account suffix and in the
    /// `provider_secret_*` IPC commands. Matches the camelCased serde
    /// tag so the frontend can use the same string.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Github => "github",
            Self::Jira { .. } => "jira",
            Self::Linear { .. } => "linear",
        }
    }
}

#[derive(Clone, Debug, Type, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoEntry {
    pub name: String,
    pub path: String,
    /// Issue source for this repo. Missing in legacy v1 configs — defaults
    /// to `Github` so existing users see no behavior change.
    #[serde(default)]
    pub provider: IssueProvider,
}

#[derive(Clone, Debug, Type, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub version: u32,
    pub worktree_root: String,
    pub repos: Vec<RepoEntry>,
    /// User-configured spawn prompt template. `None` means "use the
    /// built-in `DEFAULT_SPAWN_PROMPT`". Marking the field
    /// `default`-on-deserialize means existing `config.json` files
    /// without it round-trip cleanly to `None`.
    #[serde(default)]
    pub spawn_prompt_template: Option<String>,
    pub setup_done: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: 2,
            worktree_root: "~/dev/worktrees".into(),
            repos: Vec::new(),
            spawn_prompt_template: None,
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
            provider: IssueProvider::default(),
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

    /// Replace the issue provider for a named repo. Returns the updated
    /// entry. Errors if no repo by that name exists. The IPC layer is
    /// responsible for refusing this when the repo has live sessions —
    /// see `ipc::repos::update_repo_provider`.
    pub fn update_repo_provider(
        &mut self,
        name: &str,
        provider: IssueProvider,
    ) -> Result<RepoEntry> {
        let entry = self
            .repos
            .iter_mut()
            .find(|r| r.name == name)
            .ok_or_else(|| Error::Config(format!("no repo named {name}")))?;
        entry.provider = provider;
        Ok(entry.clone())
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

    #[test]
    fn deserializes_legacy_config_without_spawn_prompt_template() {
        // A config.json saved before this field existed must still load
        // and produce `None`, not a parse error.
        let legacy = r#"{
            "version": 1,
            "worktreeRoot": "~/dev/worktrees",
            "repos": [],
            "setupDone": true
        }"#;
        let parsed: Config = serde_json::from_str(legacy).unwrap();
        assert_eq!(parsed.spawn_prompt_template, None);
        assert!(parsed.setup_done);
    }

    #[test]
    fn round_trips_spawn_prompt_template() {
        let mut original = Config::default();
        original.spawn_prompt_template = Some("Custom #{issue_number}".into());
        let json = serde_json::to_string(&original).unwrap();
        let parsed: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(
            parsed.spawn_prompt_template.as_deref(),
            Some("Custom #{issue_number}")
        );
    }

    #[test]
    fn deserializes_legacy_config_repo_without_provider_defaults_to_github() {
        // v1 configs predate the provider field; they must keep loading
        // and each repo must default to GitHub so behavior is unchanged
        // for existing users.
        let legacy = r#"{
            "version": 1,
            "worktreeRoot": "~/dev/worktrees",
            "repos": [
                { "name": "alpha", "path": "/tmp/alpha" }
            ],
            "setupDone": true
        }"#;
        let parsed: Config = serde_json::from_str(legacy).unwrap();
        assert_eq!(parsed.repos.len(), 1);
        assert_eq!(parsed.repos[0].provider, IssueProvider::Github);
    }

    #[test]
    fn round_trips_jira_repo_entry() {
        let provider = IssueProvider::Jira {
            base_url: "https://acme.atlassian.net".into(),
            email: "ada@example.com".into(),
            project_key: "PROJ".into(),
        };
        let entry = RepoEntry {
            name: "alpha".into(),
            path: "/tmp/alpha".into(),
            provider: provider.clone(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        // Sanity: tag discriminator is camelCased to match the IPC contract.
        assert!(json.contains(r#""kind":"jira""#));
        assert!(json.contains(r#""baseUrl":"https://acme.atlassian.net""#));
        let parsed: RepoEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.provider, provider);
    }

    #[test]
    fn round_trips_linear_repo_entry() {
        let provider = IssueProvider::Linear {
            team_key: "ENG".into(),
        };
        let entry = RepoEntry {
            name: "alpha".into(),
            path: "/tmp/alpha".into(),
            provider: provider.clone(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains(r#""kind":"linear""#));
        assert!(json.contains(r#""teamKey":"ENG""#));
        let parsed: RepoEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.provider, provider);
    }

    #[test]
    fn update_repo_provider_swaps_in_place() {
        let dir = make_git_dir();
        let mut config = empty_config();
        let entry = config.add_repo(dir.path()).unwrap();
        let updated = config
            .update_repo_provider(
                &entry.name,
                IssueProvider::Linear {
                    team_key: "ENG".into(),
                },
            )
            .unwrap();
        assert_eq!(
            updated.provider,
            IssueProvider::Linear {
                team_key: "ENG".into()
            }
        );
        assert_eq!(config.repos[0].provider, updated.provider);
    }

    #[test]
    fn update_repo_provider_unknown_name_errors() {
        let mut config = empty_config();
        let err = config
            .update_repo_provider("nope", IssueProvider::Github)
            .unwrap_err();
        assert!(matches!(err, Error::Config(_)));
    }

    #[test]
    fn add_repo_defaults_provider_to_github() {
        let dir = make_git_dir();
        let mut config = empty_config();
        let entry = config.add_repo(dir.path()).unwrap();
        assert_eq!(entry.provider, IssueProvider::Github);
    }
}
