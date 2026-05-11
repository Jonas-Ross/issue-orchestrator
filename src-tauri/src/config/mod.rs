use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::{Error, Result};

mod actor;
pub use actor::{ConfigActor, ConfigHandle};

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

    /// Find the `RepoEntry` whose stored path is the longest path-boundary
    /// prefix of `path`. Canonicalizes `path` so macOS `/var` →
    /// `/private/var` symlink quirks don't miss; on canonicalize failure
    /// (path no longer exists, etc.) falls back to the raw input rather
    /// than dropping the lookup. Returns `None` if no repo contains the
    /// path. `strip_prefix` matches at component boundaries, so
    /// `/a/foo-bar` correctly does not match a repo at `/a/foo`.
    pub fn repo_containing_path(&self, path: &str) -> Option<RepoEntry> {
        let canonical = std::fs::canonicalize(path)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| path.to_owned());
        let target = Path::new(&canonical);
        self.repos
            .iter()
            .filter_map(|r| {
                let repo_path = Path::new(&r.path);
                target
                    .strip_prefix(repo_path)
                    .ok()
                    .map(|_| (repo_path.components().count(), r.clone()))
            })
            .max_by_key(|(depth, _)| *depth)
            .map(|(_, r)| r)
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

    fn config_with_repo(name: &str, path: &str) -> Config {
        let mut c = empty_config();
        c.repos.push(RepoEntry {
            name: name.into(),
            path: path.into(),
            provider: IssueProvider::default(),
        });
        c
    }

    #[test]
    fn repo_containing_path_exact_match() {
        let dir = tempfile::tempdir().unwrap();
        let canonical = std::fs::canonicalize(dir.path()).unwrap();
        let config = config_with_repo("alpha", &canonical.display().to_string());
        let m = config
            .repo_containing_path(&canonical.display().to_string())
            .expect("expected match");
        assert_eq!(m.name, "alpha");
    }

    #[test]
    fn repo_containing_path_nested_dir_matches() {
        let dir = tempfile::tempdir().unwrap();
        let canonical = std::fs::canonicalize(dir.path()).unwrap();
        let nested = canonical.join("src").join("lib");
        std::fs::create_dir_all(&nested).unwrap();
        let config = config_with_repo("alpha", &canonical.display().to_string());
        let m = config
            .repo_containing_path(&nested.display().to_string())
            .expect("expected nested match");
        assert_eq!(m.name, "alpha");
    }

    #[test]
    fn repo_containing_path_no_match_when_outside() {
        let dir = tempfile::tempdir().unwrap();
        let canonical = std::fs::canonicalize(dir.path()).unwrap();
        let config = config_with_repo("alpha", &canonical.display().to_string());
        // Sibling that isn't inside the repo.
        let other = tempfile::tempdir().unwrap();
        let other_canonical = std::fs::canonicalize(other.path()).unwrap();
        assert!(config
            .repo_containing_path(&other_canonical.display().to_string())
            .is_none());
    }

    #[test]
    fn repo_containing_path_sibling_prefix_does_not_match() {
        // /a/foo must not match a repo at /a/foo-bar. Regression guard
        // for the path-boundary rule.
        let parent = tempfile::tempdir().unwrap();
        let parent_canonical = std::fs::canonicalize(parent.path()).unwrap();
        let foo = parent_canonical.join("foo");
        let foo_bar = parent_canonical.join("foo-bar");
        std::fs::create_dir_all(&foo).unwrap();
        std::fs::create_dir_all(&foo_bar).unwrap();
        let config = config_with_repo("foo", &foo.display().to_string());
        // Query against foo-bar; must not match the repo at foo.
        let result = config.repo_containing_path(&foo_bar.display().to_string());
        assert!(
            result.is_none(),
            "expected None — /a/foo-bar must not match repo /a/foo, got {result:?}"
        );
    }

    #[test]
    fn repo_containing_path_longest_prefix_wins() {
        // If two repos are registered and one is a subdirectory of the
        // other (legal under add_repo since canonical paths differ),
        // the nested repo takes precedence for paths inside it.
        let parent = tempfile::tempdir().unwrap();
        let parent_canonical = std::fs::canonicalize(parent.path()).unwrap();
        let outer = parent_canonical.join("outer");
        let inner = outer.join("inner");
        let leaf = inner.join("deeper");
        std::fs::create_dir_all(&leaf).unwrap();
        let mut config = empty_config();
        config.repos.push(RepoEntry {
            name: "outer".into(),
            path: outer.display().to_string(),
            provider: IssueProvider::default(),
        });
        config.repos.push(RepoEntry {
            name: "inner".into(),
            path: inner.display().to_string(),
            provider: IssueProvider::default(),
        });
        let m = config
            .repo_containing_path(&leaf.display().to_string())
            .expect("expected match");
        assert_eq!(m.name, "inner");
    }

    #[test]
    fn repo_containing_path_canonicalizes_symlinks() {
        // Stored repo path is canonical. Querying via a symlinked
        // alias (created in a sibling tempdir, pointing at the real
        // repo) must still match because we canonicalize the input.
        let real = tempfile::tempdir().unwrap();
        let real_canonical = std::fs::canonicalize(real.path()).unwrap();
        let link_parent = tempfile::tempdir().unwrap();
        let link = link_parent.path().join("alias");
        std::os::unix::fs::symlink(&real_canonical, &link).unwrap();
        let config = config_with_repo("alpha", &real_canonical.display().to_string());
        let m = config
            .repo_containing_path(&link.display().to_string())
            .expect("expected match via symlink");
        assert_eq!(m.name, "alpha");
    }
}
