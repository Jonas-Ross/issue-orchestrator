import { useState } from "preact/hooks";
import { commands } from "../../lib/bindings";
import type { IssueProvider, RepoEntry } from "../../lib/bindings";
import { loadRepos } from "../../state/repos";
import { ProviderTokenControls } from "./ProviderTokenControls";

type ProviderKind = IssueProvider["kind"];

interface Props {
  repo: RepoEntry;
}

interface DraftState {
  kind: ProviderKind;
  baseUrl: string;
  email: string;
  projectKey: string;
  teamKey: string;
}

function draftFromProvider(p: IssueProvider | undefined): DraftState {
  if (!p || p.kind === "github") {
    return { kind: "github", baseUrl: "", email: "", projectKey: "", teamKey: "" };
  }
  if (p.kind === "jira") {
    return {
      kind: "jira",
      baseUrl: p.baseUrl,
      email: p.email,
      projectKey: p.projectKey,
      teamKey: "",
    };
  }
  return {
    kind: "linear",
    baseUrl: "",
    email: "",
    projectKey: "",
    teamKey: p.teamKey,
  };
}

function buildProvider(d: DraftState): IssueProvider {
  if (d.kind === "github") return { kind: "github" };
  if (d.kind === "jira") {
    return { kind: "jira", baseUrl: d.baseUrl, email: d.email, projectKey: d.projectKey };
  }
  return { kind: "linear", teamKey: d.teamKey };
}

function summary(p: IssueProvider | undefined): string {
  if (!p || p.kind === "github") return "GitHub (gh CLI)";
  if (p.kind === "jira") return `Jira · ${p.projectKey} @ ${p.baseUrl}`;
  return `Linear · ${p.teamKey}`;
}

export function RepoProviderSettings({ repo }: Props) {
  const [editing, setEditing] = useState(false);
  if (!editing) {
    return (
      <div class="repo-provider-row">
        <div class="repo-provider-summary">
          <strong class="repo-provider-name">{repo.name}</strong>
          <span class="repo-provider-current">{summary(repo.provider)}</span>
        </div>
        <button type="button" class="prompt-btn" onClick={() => setEditing(true)}>
          Edit
        </button>
      </div>
    );
  }
  return <RepoProviderEditor repo={repo} onClose={() => setEditing(false)} />;
}

function RepoProviderEditor({ repo, onClose }: { repo: RepoEntry; onClose: () => void }) {
  const [draft, setDraft] = useState<DraftState>(() => draftFromProvider(repo.provider));
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const provider = buildProvider(draft);

  const onSave = async () => {
    if (saving) return;
    setSaving(true);
    setError(null);
    const result = await commands.updateRepoProvider(repo.name, provider);
    setSaving(false);
    if (result.status === "error") {
      setError(result.error);
      return;
    }
    await loadRepos();
    onClose();
  };

  return (
    <div class="repo-provider-edit">
      <div class="repo-provider-edit-head">
        <strong class="repo-provider-name">{repo.name}</strong>
        <select
          class="prompt-repo-select"
          value={draft.kind}
          onChange={(e) =>
            setDraft({ ...draft, kind: (e.target as HTMLSelectElement).value as ProviderKind })
          }
        >
          <option value="github">GitHub</option>
          <option value="jira">Jira</option>
          <option value="linear">Linear</option>
        </select>
      </div>
      <ProviderFields draft={draft} setDraft={setDraft} />
      {draft.kind !== "github" && <ProviderTokenControls repoName={repo.name} kind={draft.kind} />}
      <div class="prompt-toolbar">
        <button
          type="button"
          class="prompt-btn primary"
          disabled={saving}
          onClick={() => void onSave()}
        >
          {saving ? "Saving…" : "Save"}
        </button>
        <button type="button" class="prompt-btn" onClick={onClose}>
          Cancel
        </button>
      </div>
      {error && <p class="prompt-error">{error}</p>}
    </div>
  );
}

function ProviderFields({
  draft,
  setDraft,
}: {
  draft: DraftState;
  setDraft: (d: DraftState) => void;
}) {
  if (draft.kind === "github") {
    return (
      <p class="settings-row-desc">
        Uses the local <code>gh</code> CLI; no extra configuration required.
      </p>
    );
  }
  if (draft.kind === "jira") {
    return (
      <div class="repo-provider-fields">
        <Field
          label="Base URL"
          placeholder="https://acme.atlassian.net"
          value={draft.baseUrl}
          onChange={(v) => setDraft({ ...draft, baseUrl: v })}
        />
        <Field
          label="Email"
          placeholder="you@company.com"
          value={draft.email}
          onChange={(v) => setDraft({ ...draft, email: v })}
        />
        <Field
          label="Project key"
          placeholder="PROJ"
          value={draft.projectKey}
          onChange={(v) => setDraft({ ...draft, projectKey: v })}
        />
      </div>
    );
  }
  return (
    <div class="repo-provider-fields">
      <Field
        label="Team key"
        placeholder="ENG"
        value={draft.teamKey}
        onChange={(v) => setDraft({ ...draft, teamKey: v })}
      />
    </div>
  );
}

function Field({
  label,
  placeholder,
  value,
  onChange,
}: {
  label: string;
  placeholder?: string;
  value: string;
  onChange: (v: string) => void;
}) {
  return (
    <label class="repo-provider-field">
      <span class="repo-provider-field-label">{label}</span>
      <input
        type="text"
        class="repo-provider-input"
        placeholder={placeholder}
        value={value}
        onInput={(e) => onChange((e.target as HTMLInputElement).value)}
      />
    </label>
  );
}
