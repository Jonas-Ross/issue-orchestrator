#!/bin/bash
# Forwards Claude Code hook payloads to the issue-orchestrator app over
# a Unix socket. If the socket isn't there (app not running) the script
# silently exits, so Claude sessions started outside the orchestrator
# are unaffected.

set -u

sock="${ISSUE_ORCH_SOCK:-$HOME/Library/Application Support/app.issue-orchestrator.desktop/hooks.sock}"

if [ ! -S "$sock" ]; then
  exit 0
fi

payload=$(cat)

# Always emit compact one-line JSON. Claude Code pretty-prints the hook
# payload, so without jq -c the listener sees ~10 separate lines per
# event and can't parse any of them. jq does both compaction and the
# optional session_orch_id annotation in a single pass.
if command -v jq >/dev/null 2>&1; then
  printf '%s' "$payload" \
    | jq -c --arg orch_id "${ISSUE_ORCH_SESSION_ID:-}" \
        'if $orch_id == "" then . else . + {session_orch_id: $orch_id} end' \
    | nc -U "$sock" -w 1 || true
else
  # No jq → degraded mode. Listener still tolerates multi-line JSON
  # (it reads the whole connection and lets serde_json stream values),
  # but the orch_id annotation is skipped so non-orch hooks won't
  # correlate.
  printf '%s' "$payload" | nc -U "$sock" -w 1 || true
fi
