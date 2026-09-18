#!/bin/bash
set -euo pipefail

input=$(cat)

trace_dir="$CLAUDE_PROJECT_DIR/.claude"
trace_file="$trace_dir/indra-traces.jsonl"
mkdir -p "$trace_dir"

timestamp=$(date -u +%Y-%m-%dT%H:%M:%SZ)

session_id=$(echo "$input" | jq -r '.session_id // empty')
hook_event=$(echo "$input" | jq -r '.hook_event_name // empty')
reason=$(echo "$input" | jq -r '.reason // empty')
cwd=$(echo "$input" | jq -r '.cwd // empty')

jq -n \
  --arg ts "$timestamp" \
  --arg session_id "$session_id" \
  --arg event "$hook_event" \
  --arg reason "$reason" \
  --arg cwd "$cwd" \
  '{timestamp: $ts, session_id: $session_id, event: $event, reason: $reason, cwd: $cwd}' \
  >> "$trace_file"

exit 0
