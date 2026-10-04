#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# The built `jura` against a real jura-trace-api, in two parts.
#
# 1. Exit codes from real responses (assertion A5 against the server rather
#    than a stub): a server with no sidecar, a real JPEG, an unsupported
#    file, a rejected key, and so on. Each case checks the exit status first
#    (BL-SILENT-001 Cause D) and the run fails unless every case ran.
#
# 2. The guard self-test (A4): the same server with a sidecar that answers
#    its health checks and refuses every analysis. scripts/assert_sidecar_ran.py
#    must FAIL on that result. If it passed, it could not tell a working
#    sidecar from a hollow one, and A3 would mean nothing.
#
# Usage: scripts/ci/cli-against-server.sh <jura-trace-api> <jura>
# Linux and macOS. Needs python3 and curl.

set -uo pipefail

API="${1:?path to jura-trace-api}"
JURA="${2:?path to jura}"
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
FIXTURE="$ROOT/docs/c2pa-conformance/test-vectors/unsigned.jpg"
W="$(mktemp -d)"
SERVERS=""
# Killing the server lets its supervisor stop the sidecar it started.
cleanup() { for p in $SERVERS; do kill "$p" 2>/dev/null; wait "$p" 2>/dev/null; done; rm -rf "$W"; }
trap cleanup EXIT

for f in "$API" "$JURA" "$FIXTURE"; do
  [ -e "$f" ] || { echo "missing: $f"; exit 1; }
done

free_port() { python3 -c 'import socket;s=socket.socket();s.bind(("127.0.0.1",0));print(s.getsockname()[1])'; }

# Start a server and wait until /api/v1/ready answers. Sets PORT. Not
# called in $(...): a subshell would lose the PID and leak the server.
start_server() {
  local db="$1"; shift
  local port; port=$(free_port)
  "$API" --port "$port" --db "$db" "$@" > "$W/server-$port.log" 2>&1 &
  SERVERS="$SERVERS $!"
  for _ in $(seq 1 100); do
    curl -sf "http://127.0.0.1:$port/api/v1/ready" > /dev/null && { PORT="$port"; return 0; }
    sleep 0.2
  done
  echo "server on $port did not answer; log:" >&2
  cat "$W/server-$port.log" >&2
  return 1
}

new_key() {
  "$API" keys add --name ci --db "$1" 2> /dev/null | grep -o 'jt_[0-9a-f]*' | head -1
}

PASS=0
FAIL=0
expect() {
  local want="$1" label="$2"; shift 2
  "$JURA" "$@" > "$W/out" 2> "$W/err"
  local got=$?
  if [ "$got" = "$want" ]; then
    PASS=$((PASS + 1)); echo "PASS  $label: exit $got"
  else
    FAIL=$((FAIL + 1)); echo "FAIL  $label: exit $got, wanted $want"
    sed 's/^/      stdout: /' "$W/out" | head -8
    sed 's/^/      stderr: /' "$W/err" | head -8
  fi
}

# ── Part 1: exit codes from a real server with no sidecar ─────────────────

DB1="$W/one.db"
KEY1=$(new_key "$DB1")
[ -n "$KEY1" ] || { echo "could not create a key"; exit 1; }
start_server "$DB1" --no-sidecar || exit 1
PORT1="$PORT"
export JURA_NO_KEYRING=1
export JURA_API_URL="http://127.0.0.1:$PORT1"
export JURA_API_KEY="$KEY1"

printf 'notes, not media\n' > "$W/notes.xyz"
: > "$W/empty.jpg"

expect 0 "a completed analysis, whatever the verdict" verify "$FIXTURE"
expect 0 "json output" verify "$FIXTURE" --format json --compact
if python3 -c 'import json,sys; b=json.load(open(sys.argv[1])); assert b["data"]["verdict"]["band"]=="inconclusive", b["data"]["verdict"]; assert b["degraded"] is True' "$W/out"; then
  PASS=$((PASS + 1)); echo "PASS  json body: verdict inconclusive, degraded, as a server without a sidecar must report"
else
  FAIL=$((FAIL + 1)); echo "FAIL  json body did not have the expected verdict"; head -c 600 "$W/out"; echo
fi
expect 8 "--fail-on with an inconclusive band" verify "$FIXTURE" --fail-on untrusted
expect 8 "--require-complete on a degraded result" verify "$FIXTURE" --require-complete
expect 7 "--wait-ready with no engine to wait for" verify "$FIXTURE" --wait-ready 5
expect 5 "unsupported content" verify "$W/notes.xyz"
expect 4 "an empty file" verify "$W/empty.jpg"
expect 4 "a missing file" verify "$W/missing.jpg"
expect 1 "an unknown mode" verify "$FIXTURE" --mode Deep
expect 1 "a URL on this machine" verify --url "http://[::1]:$PORT1/a.jpg"
expect 0 "auth status" auth status
expect 0 "version" version
JURA_API_KEY="jt_$(printf '0%.0s' $(seq 1 64))" expect 3 "a rejected key" verify "$FIXTURE"
JURA_API_KEY="" expect 3 "no key" verify "$FIXTURE"
JURA_API_URL="http://127.0.0.1:$(free_port)" expect 2 "nothing listening" verify "$FIXTURE"

# ── Part 2: the guard self-test (A4) ──────────────────────────────────────

DB2="$W/two.db"
KEY2=$(new_key "$DB2")
export STUB_SIDECAR_LOG="$W/stub.log"
: > "$STUB_SIDECAR_LOG"
chmod +x "$ROOT/scripts/ci/stub_sidecar.py"
start_server "$DB2" --sidecar-binary "$ROOT/scripts/ci/stub_sidecar.py" || exit 1
PORT2="$PORT"
JURA_API_URL="http://127.0.0.1:$PORT2" JURA_API_KEY="$KEY2" \
  expect 0 "verify through the hollow sidecar" verify "$FIXTURE" --mode deep --wait-ready 30 --format json
cp "$W/out" "$W/hollow.json"
FORENSIC_CALLS=$(grep -vc '/health' "$STUB_SIDECAR_LOG" || true)
if [ "${FORENSIC_CALLS:-0}" -lt 1 ]; then
  FAIL=$((FAIL + 1))
  echo "FAIL  the pipeline never called the stub sidecar's analysis routes, so this self-test proves nothing"
else
  PASS=$((PASS + 1)); echo "PASS  the pipeline called the hollow sidecar $FORENSIC_CALLS times"
fi
python3 "$ROOT/scripts/assert_sidecar_ran.py" "$W/hollow.json" 2> "$W/a3.err"
A3=$?
if [ "$A3" = 1 ]; then
  PASS=$((PASS + 1)); echo "PASS  A3 check failed on the hollow sidecar, as it must:"
  sed 's/^/      /' "$W/a3.err"
else
  FAIL=$((FAIL + 1)); echo "FAIL  A3 check exited $A3 on a sidecar that analysed nothing; it cannot tell"
  cat "$W/a3.err"
fi

# ── Verdict ───────────────────────────────────────────────────────────────

EXPECTED=19
echo
echo "$PASS passed, $FAIL failed, of $EXPECTED"
[ "$FAIL" = 0 ] && [ "$PASS" = "$EXPECTED" ] || exit 1
