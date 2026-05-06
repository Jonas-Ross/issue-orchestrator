use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use specta::Type;
use tokio::process::Command;

use crate::config::RepoEntry;
use crate::error::{Error, Result};
use crate::issues::{Issue, IssueClient};

/// Output of the headless "Decide next task" agent. Returned to the
/// frontend so the picker can highlight the recommendation and surface
/// the model's one-line reasoning. `id` is the same provider-agnostic
/// string identifier carried by [`Issue`] (e.g. `"123"`, `"PROJ-7"`).
#[derive(Clone, Debug, Type, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Decision {
    pub id: String,
    pub reasoning: String,
}

/// Run a one-shot `claude -p` over the open issues and ask the model to
/// pick the best one to tackle next. The prompt instructs the model to
/// respond with a single JSON object so we can parse it deterministically.
pub async fn decide_next_issue(
    repo: &RepoEntry,
    issue_client: Arc<dyn IssueClient>,
) -> Result<Decision> {
    let repo_path = PathBuf::from(&repo.path);
    let issues = issue_client.list(&repo_path).await?;
    if issues.is_empty() {
        return Err(Error::ClaudeCli("no open issues to choose from".into()));
    }

    let prompt = build_decide_prompt(&issues);
    let stdout = run_claude_p(&repo_path, &prompt).await?;
    let decision = parse_decision(&stdout)?;

    if !issues.iter().any(|i| i.id == decision.id) {
        return Err(Error::ClaudeCli(format!(
            "model picked #{} but it is not in the open issue list",
            decision.id
        )));
    }
    Ok(decision)
}

fn build_decide_prompt(issues: &[Issue]) -> String {
    let mut lines = String::new();
    for i in issues {
        let labels = if i.labels.is_empty() {
            String::new()
        } else {
            format!(" (labels: {})", i.labels.join(", "))
        };
        lines.push_str(&format!("- #{} — {}{}\n", i.id, i.title, labels));
    }
    format!(
        "You are picking the best issue to work on next from the list below.\n\
         Output ONLY a single JSON object — no preamble, no fenced code block, no commentary.\n\
         Schema: {{\"id\": \"<issue id exactly as listed>\", \"reasoning\": \"<one short sentence>\"}}\n\n\
         Issues:\n{lines}"
    )
}

/// Run a one-shot `claude -p` inside the chosen repo so the model can
/// inspect the skills, plugins and MCPs available to a session there,
/// then ask it to rewrite `current_prompt` to take better advantage of
/// them. Returns the rewritten template (placeholders preserved).
pub async fn optimize_spawn_prompt(
    repo: &RepoEntry,
    current_prompt: &str,
) -> Result<String> {
    let repo_path = PathBuf::from(&repo.path);
    let prompt = build_optimize_prompt(current_prompt);
    let stdout = run_claude_p(&repo_path, &prompt).await?;
    parse_optimized_prompt(&stdout)
}

fn build_optimize_prompt(current_prompt: &str) -> String {
    format!(
        "You are improving a prompt template that will spawn a Claude Code session to implement \
         a tracked issue (GitHub, Jira, or Linear). The session runs inside this repository, so \
         you have access to its skills, plugins, MCPs, hooks and CLAUDE.md.\n\n\
         Inspect what's available in this session and rewrite the template to make better use of \
         the most relevant skills (e.g. issue-team, feature-dev), tools, or MCPs you find. \
         Preserve the literal placeholders `{{issue_id}}` and `{{issue_title}}` somewhere in \
         the output so they can be interpolated at spawn time. Keep the result a single \
         template string — do not split it into multiple instructions or steps.\n\n\
         Output ONLY a single JSON object — no preamble, no fenced code block, no commentary.\n\
         Schema: {{\"prompt\": \"<rewritten template>\"}}\n\n\
         Current template:\n{current_prompt}"
    )
}

/// Shared `claude -p` driver: 60s timeout, surfaces non-zero exit + stderr.
async fn run_claude_p(repo_path: &std::path::Path, prompt: &str) -> Result<String> {
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(60),
        Command::new("claude")
            .arg("-p")
            .arg(prompt)
            .current_dir(repo_path)
            .output(),
    )
    .await
    .map_err(|_| Error::ClaudeCli("claude -p timed out after 60s".into()))?
    .map_err(|e| Error::ClaudeCli(format!("claude: {e}")))?;

    if !output.status.success() {
        return Err(Error::ClaudeCli(format!(
            "claude -p failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub fn parse_optimized_prompt(raw: &str) -> Result<String> {
    #[derive(Deserialize)]
    struct OptimizedPrompt {
        prompt: String,
    }
    let parsed: OptimizedPrompt = parse_first_json(raw, "optimized prompt")?;
    if parsed.prompt.trim().is_empty() {
        return Err(Error::ClaudeCli("model returned empty prompt".into()));
    }
    Ok(parsed.prompt)
}

pub fn parse_decision(raw: &str) -> Result<Decision> {
    parse_first_json(raw, "decision")
}

/// Pull the first balanced JSON object out of `claude -p` stdout. The
/// model is told to emit raw JSON; in practice it sometimes adds
/// preamble or wraps the payload in ``` fences. Strip the fence, jump
/// to the first `{`, then let serde_json's streaming `Deserializer`
/// own the brace/string/escape tracking — we only have to slice off
/// the chatter.
fn parse_first_json<T: serde::de::DeserializeOwned>(raw: &str, ctx: &str) -> Result<T> {
    let cleaned = strip_fence(raw.trim());
    let start = cleaned.find('{').ok_or_else(|| {
        Error::ClaudeCli(format!("no JSON object in claude output ({ctx}): {raw}"))
    })?;
    serde_json::Deserializer::from_str(&cleaned[start..])
        .into_iter::<T>()
        .next()
        .ok_or_else(|| {
            Error::ClaudeCli(format!("no JSON object in claude output ({ctx}): {raw}"))
        })?
        .map_err(|e| Error::ClaudeCli(format!("parse {ctx} json: {e} (input: {raw})")))
}

fn strip_fence(s: &str) -> &str {
    let s = s.trim();
    if let Some(rest) = s.strip_prefix("```json") {
        return rest.trim_start_matches('\n').trim_end_matches("```").trim();
    }
    if let Some(rest) = s.strip_prefix("```") {
        return rest.trim_start_matches('\n').trim_end_matches("```").trim();
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_raw_json_numeric_id() {
        let d = parse_decision(r#"{"id": "12", "reasoning": "small + isolated"}"#).unwrap();
        assert_eq!(d.id, "12");
        assert_eq!(d.reasoning, "small + isolated");
    }

    #[test]
    fn parses_jira_key_id() {
        let d = parse_decision(r#"{"id": "PROJ-7", "reasoning": "unblocks the auth release"}"#)
            .unwrap();
        assert_eq!(d.id, "PROJ-7");
    }

    #[test]
    fn parses_json_inside_code_fence() {
        let raw = "```json\n{\"id\": \"5\", \"reasoning\": \"oldest open\"}\n```";
        let d = parse_decision(raw).unwrap();
        assert_eq!(d.id, "5");
    }

    #[test]
    fn parses_json_with_chatter() {
        let raw =
            "Sure! Here's my pick:\n{\"id\": \"42\", \"reasoning\": \"unblocks others\"}\nLet me know.";
        let d = parse_decision(raw).unwrap();
        assert_eq!(d.id, "42");
    }

    #[test]
    fn errors_with_no_object() {
        assert!(parse_decision("nothing here").is_err());
    }

    #[test]
    fn brace_inside_string_value_does_not_break_parsing() {
        let raw = r#"{"id": "12", "reasoning": "use {brace} carefully"}"#;
        let d = parse_decision(raw).unwrap();
        assert_eq!(d.id, "12");
        assert_eq!(d.reasoning, "use {brace} carefully");
    }

    #[test]
    fn escaped_quote_inside_string_does_not_break_parsing() {
        let raw = r#"{"id": "5", "reasoning": "they said \"go\" today"}"#;
        let d = parse_decision(raw).unwrap();
        assert_eq!(d.id, "5");
        assert_eq!(d.reasoning, r#"they said "go" today"#);
    }
}
