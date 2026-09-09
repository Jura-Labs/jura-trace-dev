#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# stage.sh: the macOS staging leg of the release gate, as a command.
#
# Proves that a pristine, publicly downloaded Jura Trace v1.0.0 receives the
# local v1.1.0 build through the REAL updater path, without touching
# production. The plan calls this the staging leg (docs/release/v1.1.0-plan.md,
# "Manual, in order", item 1). Nobody had run it on any platform before
# 9 September 2026.
#
# How it works, and why it works. tauri-plugin-updater carries its own HTTP
# stack (reqwest with rustls-platform-verifier) and validates TLS against the
# SYSTEM trust store. So: a throwaway CA trusted in the keychain, a leaf
# certificate for juralabs.org, a line in /etc/hosts sending juralabs.org to
# 127.0.0.1, and a local HTTPS server on 443 handing out a staging
# latest.json and the local build's .app.tar.gz. The installed v1.0.0 asks
# "juralabs.org" for its manifest and gets ours. The live endpoint never sees
# any of it. This is the same trick .github/workflows/updater-e2e.yml uses on
# Windows in CI; it is the Mac version, run by hand, and it goes one step
# further because a person can then click Check for Updates and watch the
# install and relaunch, which the CI leg explicitly cannot observe.
#
# Subcommands, in the order you run them:
#
#   prepare   download the public v1.0.0 DMG, install it to a THROWAWAY
#             location on the SSD, generate the CA and leaf, write the
#             staging manifest from the local build's .sig.  No sudo.
#   trust     add the throwaway CA to your login keychain as a trusted root.
#             macOS will ask for your password once.  No sudo.
#   serve     add the hosts entry and serve on 443 until Ctrl-C, logging every
#             request. NEEDS sudo (hosts file and port 443). Restores
#             /etc/hosts on exit, even on Ctrl-C.
#   untrust   remove the throwaway CA from the keychain.
#   status    show what is in place right now.
#
# Every artefact lives under $STAGE_ROOT (default: the SSD). Nothing is
# written to /Applications, and the only two things touched outside that
# directory are the hosts entry and the keychain trust, both of which have
# an explicit undo and are reported by `status`.
#
# Do the throwaway install under a location the real app does not use. The
# updater replaces the RUNNING app bundle in place, wherever it lives.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

HOST="juralabs.org"
PORT="${STAGE_PORT:-443}"
HOSTS_FILE="${HOSTS_FILE:-/etc/hosts}"
MARK="# jura-trace updater staging leg (scripts/updater-staging/stage.sh)"
TARGET="aarch64-apple-darwin"
# Where things are. `prepare` records the stage root it used in a marker
# file beside this script, and every other subcommand reads it back when the
# environment does not say. That is what makes `sudo stage.sh serve` work:
# sudo resets the environment, so CARGO_TARGET_DIR is gone and, without the
# marker, serve looked under src-tauri/target and said "Not prepared" to a
# fully prepared SSD (9 September 2026). The marker is gitignored.
MARKER="$REPO_ROOT/scripts/updater-staging/.stage-root"
if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then BASE="$CARGO_TARGET_DIR"; else BASE="$REPO_ROOT/src-tauri/target"; fi
if [[ -z "${STAGE_ROOT:-}" && -z "${CARGO_TARGET_DIR:-}" && -s "$MARKER" ]]; then
  STAGE_ROOT="$(cat "$MARKER")"
  BASE="$(dirname "$STAGE_ROOT")"
fi
BUNDLE_MACOS="$BASE/$TARGET/release/bundle/macos"
STAGE_ROOT="${STAGE_ROOT:-$BASE/updater-staging}"
TLS_DIR="$STAGE_ROOT/tls"
WWW_DIR="$STAGE_ROOT/www"
OLD_DIR="$STAGE_ROOT/v1.0.0"
PUBLIC_REPO="Jura-Labs/jura-trace"
PUBLIC_TAG="v1.0.0"

log()  { printf "\n\033[1;34m==>\033[0m \033[1m%s\033[0m\n" "$*"; }
ok()   { printf "\033[1;32m✓\033[0m %s\n" "$*"; }
warn() { printf "\033[1;33m!\033[0m %s\n" "$*"; }
die()  { printf "\033[1;31m✗\033[0m %s\n" "$*" >&2; exit 1; }

need_macos() { [[ "$(uname -s)" == "Darwin" ]] || die "macOS only."; }

# ── prepare ───────────────────────────────────────────────────────────
cmd_prepare() {
  need_macos
  mkdir -p "$TLS_DIR" "$WWW_DIR/api/updates" "$WWW_DIR/staging" "$OLD_DIR"
  printf '%s\n' "$STAGE_ROOT" > "$MARKER"

  log "Local v1.1.0 build artefacts"
  local tgz="$BUNDLE_MACOS/Jura Trace.app.tar.gz" sig="$BUNDLE_MACOS/Jura Trace.app.tar.gz.sig"
  [[ -s "$tgz" ]] || die "No updater archive at $tgz. Run scripts/build-local-mac.sh first."
  [[ -s "$sig" ]] || die "No signature at $sig. The build ran without the updater key."
  local ver; ver=$(grep -m1 '"version"' src-tauri/tauri.conf.json | sed -E 's/.*"version": *"([^"]+)".*/\1/')
  ok "archive $(du -h "$tgz" | cut -f1), signature $(wc -c < "$sig" | tr -d ' ') bytes, declared version $ver"

  log "Pristine public $PUBLIC_TAG"
  if [[ ! -d "$OLD_DIR/Jura Trace.app" ]]; then
    local dmg; dmg=$(ls "$OLD_DIR"/*.dmg 2>/dev/null | head -1 || true)
    if [[ -z "$dmg" ]]; then
      gh release download "$PUBLIC_TAG" --repo "$PUBLIC_REPO" --pattern '*.dmg' --dir "$OLD_DIR" \
        || die "Could not download the $PUBLIC_TAG DMG from $PUBLIC_REPO."
      dmg=$(ls "$OLD_DIR"/*.dmg | head -1)
    fi
    ok "DMG: $(basename "$dmg") ($(du -h "$dmg" | cut -f1))"
    local mp; mp=$(hdiutil attach -readonly -nobrowse -noverify "$dmg" | awk -F'\t' '/\/Volumes\//{print $NF}' | tail -1)
    [[ -d "$mp/Jura Trace.app" ]] || { hdiutil detach "$mp" -quiet || true; die "No Jura Trace.app inside the DMG."; }
    ditto "$mp/Jura Trace.app" "$OLD_DIR/Jura Trace.app"
    hdiutil detach "$mp" -quiet
    # This copy came from a disk image, not a browser download, so it carries
    # no quarantine attribute and Gatekeeper will not prompt on launch. That
    # matches an installed app, which is what we are simulating.
  fi
  local oldver; oldver=$(/usr/libexec/PlistBuddy -c "Print CFBundleShortVersionString" "$OLD_DIR/Jura Trace.app/Contents/Info.plist")
  [[ "$oldver" == "1.0.0" ]] || die "Throwaway install reports $oldver, expected 1.0.0."
  ok "throwaway install: $OLD_DIR/Jura Trace.app (version $oldver, pubkey in its tauri.conf must match the .sig's key)"

  log "Throwaway CA and leaf certificate for $HOST (valid one day)"
  if [[ ! -s "$TLS_DIR/ca.pem" ]]; then
    openssl req -x509 -newkey rsa:2048 -nodes -days 1 -sha256 \
      -subj "/CN=Jura Trace updater staging CA (throwaway)" \
      -addext "basicConstraints=critical,CA:TRUE" -addext "keyUsage=critical,keyCertSign,cRLSign" \
      -keyout "$TLS_DIR/ca.key" -out "$TLS_DIR/ca.pem" 2>/dev/null
    openssl req -newkey rsa:2048 -nodes -subj "/CN=$HOST" \
      -keyout "$TLS_DIR/leaf.key" -out "$TLS_DIR/leaf.csr" 2>/dev/null
    printf "subjectAltName=DNS:%s\nextendedKeyUsage=serverAuth\nbasicConstraints=CA:FALSE\n" "$HOST" > "$TLS_DIR/leaf.ext"
    openssl x509 -req -in "$TLS_DIR/leaf.csr" -CA "$TLS_DIR/ca.pem" -CAkey "$TLS_DIR/ca.key" -CAcreateserial \
      -days 1 -sha256 -extfile "$TLS_DIR/leaf.ext" -out "$TLS_DIR/leaf.pem" 2>/dev/null
    chmod 600 "$TLS_DIR"/*.key
  fi
  ok "CA $(openssl x509 -in "$TLS_DIR/ca.pem" -noout -fingerprint -sha256 | cut -d= -f2 | cut -c1-23)..."

  log "Staging manifest"
  cp "$tgz" "$WWW_DIR/staging/Jura.Trace.app.tar.gz"
  python3 - "$sig" "$ver" "$WWW_DIR/api/updates/latest.json" "$HOST" <<'PY'
import json, sys, datetime
sig, ver, out, host = sys.argv[1:5]
manifest = {
  "version": ver,
  "notes": "Updater STAGING leg. Not a release. Served from 127.0.0.1 via a hosts redirect.",
  "pub_date": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
  "platforms": {
    "darwin-aarch64": {
      "signature": open(sig).read().strip(),
      "url": f"https://{host}/staging/Jura.Trace.app.tar.gz",
    }
  },
}
json.dump(manifest, open(out, "w"), indent=2)
print(f"  {out}: version {ver}, darwin-aarch64, signature {len(manifest['platforms']['darwin-aarch64']['signature'])} chars")
PY
  ok "prepared. Next: $0 trust, then: sudo $0 serve"
}

# ── trust / untrust ───────────────────────────────────────────────────
cmd_trust() {
  need_macos
  [[ -s "$TLS_DIR/ca.pem" ]] || die "No CA yet; run prepare first."
  log "Trusting the throwaway CA in your login keychain (macOS will ask for your password)"
  security add-trusted-cert -r trustRoot -p ssl -k "$HOME/Library/Keychains/login.keychain-db" "$TLS_DIR/ca.pem"
  ok "trusted. Undo with: $0 untrust"
}
cmd_untrust() {
  need_macos
  [[ -s "$TLS_DIR/ca.pem" ]] || { warn "No CA file; nothing to untrust."; return 0; }
  log "Removing the throwaway CA from the login keychain"
  security remove-trusted-cert "$TLS_DIR/ca.pem" 2>/dev/null || warn "trust setting already gone"
  local sha; sha=$(openssl x509 -in "$TLS_DIR/ca.pem" -noout -fingerprint -sha1 | cut -d= -f2 | tr -d ':')
  security delete-certificate -Z "$sha" "$HOME/Library/Keychains/login.keychain-db" 2>/dev/null || warn "certificate already gone"
  ok "untrusted."
}

# ── serve ─────────────────────────────────────────────────────────────
SERVER_PID=""
restore_hosts() {
  # Runs on EXIT, INT and TERM. Stop the server child first, then put the
  # hosts file back. Tested by sending INT to this shell alone, not the
  # process group, which is the case a terminal Ctrl-C does not exercise.
  if [[ -n "$SERVER_PID" ]] && kill -0 "$SERVER_PID" 2>/dev/null; then
    kill -TERM "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
  if [[ -n "${HOSTS_BACKUP:-}" && -s "$HOSTS_BACKUP" ]]; then
    cp "$HOSTS_BACKUP" "$HOSTS_FILE" && ok "hosts file restored from $HOSTS_BACKUP"
  else
    sed -i.bak "/$MARK/d" "$HOSTS_FILE" 2>/dev/null || true
  fi
  dscacheutil -flushcache 2>/dev/null || true
  trap - EXIT INT TERM
}
serve_and_wait() {
  # The Python server runs as a child so that a signal delivered to THIS
  # shell reaches the trap while the child is still running; the trap stops
  # the child and restores the hosts file. serve_python execs python3, so $!
  # is the server itself and not a subshell wrapping it: the first version
  # killed the subshell, orphaned the server, and left the port busy for
  # the next run (9 September 2026).
  serve_python &
  SERVER_PID=$!
  local rc=0
  wait "$SERVER_PID" || rc=$?
  SERVER_PID=""
  if [[ $rc -ne 0 && $rc -ne 143 && $rc -ne 130 ]]; then
    warn "the server exited on its own (status $rc); see the messages above"
  fi
}
cmd_serve() {
  need_macos
  [[ -s "$WWW_DIR/api/updates/latest.json" && -s "$TLS_DIR/leaf.pem" ]] || die "Not prepared; run prepare first."
  if [[ "$PORT" -lt 1024 && "$(id -u)" -ne 0 ]]; then die "Port $PORT and $HOSTS_FILE need root. Run: sudo $0 serve"; fi
  grep -q "$MARK" "$HOSTS_FILE" && die "A staging hosts entry is already present in $HOSTS_FILE. Remove it before serving again."
  if lsof -nP -iTCP:"$PORT" -sTCP:LISTEN >/dev/null 2>&1; then
    die "Port $PORT is already in use: $(lsof -nP -iTCP:"$PORT" -sTCP:LISTEN 2>/dev/null | awk 'NR==2{print $1, "pid", $2}'). Stop it first."
  fi

  log "Redirecting $HOST to 127.0.0.1 in $HOSTS_FILE (backed up; restored on exit)"
  HOSTS_BACKUP="$STAGE_ROOT/hosts.backup.$(date +%s)"; cp "$HOSTS_FILE" "$HOSTS_BACKUP"
  printf "\n127.0.0.1 %s %s\n" "$HOST" "$MARK" >> "$HOSTS_FILE"
  dscacheutil -flushcache 2>/dev/null || true
  trap restore_hosts EXIT INT TERM
  ok "$(grep "$MARK" "$HOSTS_FILE" | head -1)"

  log "Serving https://$HOST:$PORT/ from $WWW_DIR (Ctrl-C to stop; hosts restored automatically)"
  echo "   Now, in the throwaway v1.0.0 at $OLD_DIR: launch it, wait for the startup check,"
  echo "   then Settings -> Check for Updates. Watch below for /api/updates/latest.json and /staging/...tar.gz."
  serve_and_wait
}
serve_python() {
  exec python3 - "$WWW_DIR" "$TLS_DIR/leaf.pem" "$TLS_DIR/leaf.key" "$PORT" <<'PY'
import http.server, ssl, sys, os, datetime
www, cert, key, port = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4])
os.chdir(www)
class H(http.server.SimpleHTTPRequestHandler):
    def log_message(self, fmt, *args):
        ts = datetime.datetime.now().strftime("%H:%M:%S")
        sys.stdout.write(f"   {ts}  {self.client_address[0]}  {fmt % args}\n"); sys.stdout.flush()
    def end_headers(self):
        self.send_header("Cache-Control", "no-store"); super().end_headers()
ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER); ctx.load_cert_chain(cert, key)
srv = http.server.ThreadingHTTPServer(("127.0.0.1", port), H)
srv.socket = ctx.wrap_socket(srv.socket, server_side=True)
print(f"   listening on 127.0.0.1:{port}"); sys.stdout.flush()
try: srv.serve_forever()
except KeyboardInterrupt: pass
PY
}

# ── status ────────────────────────────────────────────────────────────
cmd_status() {
  need_macos
  log "Staging leg status"
  printf "  stage root      %s\n" "$STAGE_ROOT"
  [[ -d "$OLD_DIR/Jura Trace.app" ]] && printf "  throwaway app   %s (version %s)\n" "$OLD_DIR/Jura Trace.app" "$(/usr/libexec/PlistBuddy -c 'Print CFBundleShortVersionString' "$OLD_DIR/Jura Trace.app/Contents/Info.plist" 2>/dev/null)" || printf "  throwaway app   not installed\n"
  [[ -s "$WWW_DIR/api/updates/latest.json" ]] && printf "  manifest        version %s\n" "$(python3 -c "import json;print(json.load(open('$WWW_DIR/api/updates/latest.json'))['version'])")" || printf "  manifest        none\n"
  if [[ -s "$TLS_DIR/ca.pem" ]]; then
    if security find-certificate -c "Jura Trace updater staging CA" "$HOME/Library/Keychains/login.keychain-db" >/dev/null 2>&1; then printf "  CA in keychain  TRUSTED (undo: %s untrust)\n" "$0"; else printf "  CA in keychain  not present\n"; fi
  else printf "  CA              not generated\n"; fi
  if grep -q "$MARK" "$HOSTS_FILE" 2>/dev/null; then printf "  hosts entry     PRESENT: %s\n" "$(grep "$MARK" "$HOSTS_FILE" | head -1)"; else printf "  hosts entry     absent\n"; fi
  if lsof -nP -iTCP:"$PORT" -sTCP:LISTEN >/dev/null 2>&1; then printf "  port %s        in use\n" "$PORT"; else printf "  port %s        free\n" "$PORT"; fi
}

case "${1:-}" in
  prepare) cmd_prepare ;;
  trust)   cmd_trust ;;
  untrust) cmd_untrust ;;
  serve)   cmd_serve ;;
  status)  cmd_status ;;
  *) sed -n '2,45p' "$0" | sed 's/^# \{0,1\}//'; exit 2 ;;
esac
