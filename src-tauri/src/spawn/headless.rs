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
        return Err(Error::Spawn("no open issues to choose from".into()));
    }

    let prompt = build_decide_prompt(&issues);
    let stdout = run_claude_p(&repo_path, &prompt).await?;
    let decision = parse_decision(&stdout)?;

    if !issues.iter().any(|i| i.id == decision.id) {
        return Err(Error::Spawn(format!(
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
    .map_err(|_| Error::Spawn("claude -p timed out after 60s".into()))?
    .map_err(|e| Error::Spawn(format!("claude: {e}")))?;

    if !output.status.success() {
        return Err(Error::Spawn(format!(
            "claude -p failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Tolerantly extract a rewritten prompt from `claude -p` stdout. Same
/// fence/chatter tolerance as `parse_decision`.
pub fn parse_optimized_prompt(raw: &str) -> Result<String> {
    #[derive(Deserialize)]
    struct OptimizedPrompt {
        prompt: String,
    }
    let trimmed = raw.trim();
    let cleaned = strip_fence(trimmed);
    let json_str = extract_first_object(cleaned)
        .ok_or_else(|| Error::Spawn(format!("no JSON object in claude output: {raw}")))?;
    let parsed: OptimizedPrompt = serde_json::from_str(json_str)
        .map_err(|e| Error::Spawn(format!("parse optimized prompt json: {e} (input: {json_str})")))?;
    if parsed.prompt.trim().is_empty() {
        return Err(Error::Spawn("model returned empty prompt".into()));
    }
    Ok(parsed.prompt)
}

/// Tolerantly extract a `Decision` from `claude -p` stdout. The model is
/// asked to emit raw JSON, but we strip ``` fences and locate the first
/// `{...}` block defensively in case it adds chatter.
pub fn parse_decision(raw: &str) -> Result<Decision> {
    let trimmed = raw.trim();
    let cleaned = strip_fence(trimmed);
    let json_str = extract_first_object(cleaned)
        .ok_or_else(|| Error::Spawn(format!("no JSON object in claude output: {raw}")))?;
    serde_json::from_str::<Decision>(json_str)
        .map_err(|e| Error::Spawn(format!("parse decision json: {e} (input: {json_str})")))
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

fn extract_first_object(s: &str) -> Option<&str> {
    let start = s.find('{')?;
    let mut depth = 0i32;
    for (i, c) in s[start..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    let end = start + i + c.len_utf8();
                    return Some(&s[start..end]);
                }
            }
            _ => {}
        }
    }
    None
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
}
