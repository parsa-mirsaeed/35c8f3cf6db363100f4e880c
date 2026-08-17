#!/usr/bin/env bash
# Shared exact-head orchestration helpers for PR-15 Final Release Acceptance.
# The functions are intentionally fail-closed so command substitution cannot
# turn a failed upstream workflow into an accepted downstream run ID.

retry_gh() {
  local attempt max_attempts="${GH_API_RETRY_ATTEMPTS:-10}" rc delay
  local out err
  out="$(mktemp)"
  err="$(mktemp)"
  for attempt in $(seq 1 "$max_attempts"); do
    : >"$out"
    : >"$err"
    if "$@" >"$out" 2>"$err"; then
      cat "$out"
      rm -f "$out" "$err"
      return 0
    else
      rc=$?
    fi
    if ! grep -Eqi 'HTTP (408|409|425|429|5[0-9]{2})|Server Error|Bad Gateway|Service Unavailable|Gateway Timeout|rate limit|timed out|timeout|connection reset|unexpected EOF|EOF' "$err"; then
      cat "$out"
      cat "$err" >&2
      rm -f "$out" "$err"
      return "$rc"
    fi
    if (( attempt == max_attempts )); then
      cat "$err" >&2
      rm -f "$out" "$err"
      return "$rc"
    fi
    delay=$((attempt * 3))
    (( delay > 30 )) && delay=30
    sleep "$delay"
  done
}

find_run_id() {
  local workflow="$1" event="$2"
  retry_gh gh run list --repo "$REPOSITORY" --workflow "$workflow" --branch "$HEAD_REF" --event "$event" --limit 50 \
    --json databaseId,headSha \
    --jq "[.[] | select(.headSha == \"$HEAD_SHA\")] | if length == 0 then empty else max_by(.databaseId).databaseId end"
}

wait_for_run() {
  local run_id="$1" timeout_seconds="${2:-21000}" started now status conclusion tmp
  started="$(date +%s)"
  tmp="$(mktemp)"
  while true; do
    if ! retry_gh gh api "/repos/${REPOSITORY}/actions/runs/${run_id}" >"$tmp"; then
      rm -f "$tmp"
      return 1
    fi
    status="$(jq -r '.status' "$tmp")"
    conclusion="$(jq -r '.conclusion // ""' "$tmp")"
    echo "Run ${run_id}: ${status}${conclusion:+/${conclusion}}" >&2
    if [[ "$status" == completed ]]; then
      rm -f "$tmp"
      if [[ "$conclusion" != success ]]; then
        echo "Run ${run_id} completed with ${conclusion}; refusing it as release evidence." >&2
        return 1
      fi
      return 0
    fi
    now="$(date +%s)"
    if (( now - started >= timeout_seconds )); then
      rm -f "$tmp"
      echo "Timed out waiting for run ${run_id}" >&2
      return 1
    fi
    sleep 15
  done
}

write_run_evidence() {
  local run_id="$1" output="$2" run_file jobs_file
  run_file="$(mktemp)"
  jobs_file="$(mktemp)"
  if ! retry_gh gh api "/repos/${REPOSITORY}/actions/runs/${run_id}" >"$run_file"; then
    rm -f "$run_file" "$jobs_file"
    return 1
  fi
  if ! retry_gh gh api "/repos/${REPOSITORY}/actions/runs/${run_id}/jobs?per_page=100" >"$jobs_file"; then
    rm -f "$run_file" "$jobs_file"
    return 1
  fi
  if ! jq -n --slurpfile run "$run_file" --slurpfile jobs "$jobs_file" \
    '($run[0]) as $r | ($jobs[0]) as $j | {id:$r.id,headSha:$r.head_sha,event:$r.event,status:$r.status,conclusion:$r.conclusion,url:$r.html_url,jobs:[$j.jobs[]|{name:.name,status:.status,conclusion:.conclusion}]}' \
    >"$output"; then
    rm -f "$run_file" "$jobs_file"
    return 1
  fi
  rm -f "$run_file" "$jobs_file"
}

validate_run() {
  local path="$1" expected_event="$2"
  shift 2
  python3 - "$HEAD_SHA" "$expected_event" "$path" "$@" <<'PY'
import json
import sys

expected_sha, expected_event, path, *required = sys.argv[1:]
run = json.load(open(path, encoding="utf-8"))
assert run["headSha"] == expected_sha, (run["headSha"], expected_sha)
assert run["event"] == expected_event, (run["event"], expected_event)
assert run["status"] == "completed", run["status"]
assert run["conclusion"] == "success", run["conclusion"]
jobs = {job["name"]: job["conclusion"] for job in run["jobs"]}
for name in required:
    assert jobs.get(name) == "success", (name, jobs.get(name), jobs)
PY
}

wait_for_pr_run() {
  local workflow="$1" run_id attempt
  for attempt in $(seq 1 180); do
    if ! run_id="$(find_run_id "$workflow" pull_request)"; then
      echo "Unable to query exact-head pull_request run for ${workflow}." >&2
      return 1
    fi
    if [[ -n "$run_id" ]]; then
      if ! wait_for_run "$run_id"; then
        echo "Exact-head ${workflow} pull_request run ${run_id} did not succeed." >&2
        return 1
      fi
      printf '%s\n' "$run_id"
      return 0
    fi
    sleep 5
  done
  echo "No exact-head pull_request run found for ${workflow}" >&2
  return 1
}

reuse_or_dispatch() {
  local workflow="$1" workflow_file="$2" evidence="$3"
  shift 3
  local existing previous run_id attempt wait_status

  if ! existing="$(find_run_id "$workflow" workflow_dispatch)"; then
    echo "Unable to query existing exact-head ${workflow} dispatch." >&2
    return 1
  fi
  if [[ -n "$existing" ]]; then
    set +e
    wait_for_run "$existing"
    wait_status=$?
    set -e
    if [[ "$wait_status" -eq 0 ]]; then
      if ! write_run_evidence "$existing" "$evidence"; then
        echo "Unable to write evidence for existing exact-head ${workflow} run ${existing}." >&2
        return 1
      fi
      if validate_run "$evidence" workflow_dispatch "$@"; then
        echo "Reusing complete exact-head ${workflow} run ${existing}" >&2
        printf '%s\n' "$existing"
        return 0
      fi
    fi
    echo "Existing exact-head ${workflow} dispatch is not reusable; dispatching once." >&2
  fi

  previous="${existing:-0}"
  if ! retry_gh gh workflow run "$workflow_file" --repo "$REPOSITORY" --ref "$HEAD_REF" >/dev/null; then
    echo "Failed to dispatch exact-head ${workflow}." >&2
    return 1
  fi
  for attempt in $(seq 1 120); do
    if ! run_id="$(find_run_id "$workflow" workflow_dispatch)"; then
      echo "Unable to resolve new exact-head ${workflow} dispatch." >&2
      return 1
    fi
    if [[ -n "$run_id" ]] && (( run_id > previous )); then
      if ! wait_for_run "$run_id"; then
        echo "New exact-head ${workflow} run ${run_id} failed; refusing downstream release evidence." >&2
        return 1
      fi
      if ! write_run_evidence "$run_id" "$evidence"; then
        echo "Unable to write evidence for new exact-head ${workflow} run ${run_id}." >&2
        return 1
      fi
      if ! validate_run "$evidence" workflow_dispatch "$@"; then
        echo "New exact-head ${workflow} run ${run_id} failed required-job validation." >&2
        return 1
      fi
      printf '%s\n' "$run_id"
      return 0
    fi
    sleep 5
  done
  echo "Unable to resolve new exact-head ${workflow} dispatch" >&2
  return 1
}

check_release_entry() {
  local state_file="$1" manual_file="$2"

  if ! retry_gh gh api graphql \
    -f owner="${REPOSITORY%%/*}" \
    -f repo="${REPOSITORY#*/}" \
    -F number="$PR_NUMBER" \
    -f query='query($owner:String!,$repo:String!,$number:Int!){repository(owner:$owner,name:$repo){issues(first:100,states:OPEN){totalCount pageInfo{hasNextPage} nodes{number title labels(first:30){nodes{name}}}} pullRequest(number:$number){reviewThreads(first:100){pageInfo{hasNextPage} nodes{isResolved}}}}}' \
    >"$state_file"; then
    return 1
  fi

  python3 - "$state_file" <<'PY'
import json
import re
import sys

data = json.load(open(sys.argv[1], encoding="utf-8"))["data"]["repository"]
issues = data["issues"]
assert not issues["pageInfo"]["hasNextPage"], "more than 100 open issues; release review is incomplete"
blocker = re.compile(r"(^|[:/_ -])(p0|release[- _]?blocker)($|[:/_ -])", re.I)
blocked = []
for issue in issues["nodes"]:
    labels = [node["name"] for node in issue["labels"]["nodes"]]
    if any(blocker.search(label) for label in labels):
        blocked.append((issue["number"], issue["title"], labels))
assert not blocked, f"open P0/release blockers: {blocked}"
threads = data["pullRequest"]["reviewThreads"]
assert not threads["pageInfo"]["hasNextPage"], "more than 100 review threads; release review is incomplete"
unresolved = [node for node in threads["nodes"] if not node["isResolved"]]
assert not unresolved, f"unresolved review threads: {len(unresolved)}"
PY

  if ! retry_gh gh api "/repos/${REPOSITORY}/pulls/16" >"$manual_file"; then
    return 1
  fi
  python3 - "$manual_file" <<'PY'
import json
import sys

pr = json.load(open(sys.argv[1], encoding="utf-8"))
assert pr["number"] == 16
assert "Manual/external production acceptance" in pr["title"]
print(f"Manual/external acceptance PR #16 state={pr['state']} merged_at={pr['merged_at']}")
PY
}
