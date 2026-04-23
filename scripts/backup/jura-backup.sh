#!/bin/bash
#
# Daily Jura Labs disaster-recovery backup.
#
# Mirrors development environment, Claude context, dotfiles, and training
# corpus to an external SSD using rsync with --link-dest so unchanged files
# are hardlinked (each daily snapshot is a full browseable tree but only
# consumes space for files that changed).
#
# Retention: last 7 daily snapshots + 4 weekly (Sunday) snapshots.
# Scheduled via ~/Library/LaunchAgents/org.juralabs.backup.plist.

set -uo pipefail
IFS=$'\n\t'

# ── Configuration ──────────────────────────────────────────────────

BACKUP_ROOT="/Volumes/MAC SSD/Backups/jura-labs"
SNAPSHOT_DIR="$BACKUP_ROOT/snapshots"
LOG_DIR="$BACKUP_ROOT/logs"
EXCLUDE_FILE="$(cd "$(dirname "$0")" && pwd)/exclude.txt"

TODAY="$(date +%Y-%m-%d)"
NOW="$(date +%Y-%m-%d_%H%M%S)"
CURRENT_SNAPSHOT="$SNAPSHOT_DIR/$TODAY"
LOG_FILE="$LOG_DIR/$TODAY.log"

# ── Logging ─────────────────────────────────────────────────────────

log() {
  local msg="[$(date +%H:%M:%S)] $*"
  echo "$msg"
  [[ -d "$LOG_DIR" ]] && echo "$msg" >> "$LOG_FILE"
}

fail() {
  log "ERROR: $*"
  exit 1
}

# ── Preflight ──────────────────────────────────────────────────────

if [[ ! -d "/Volumes/MAC SSD" ]]; then
  # SSD not mounted. Log to stderr (launchd captures it) and exit cleanly
  # so we don't spam failure notifications when the drive is disconnected.
  echo "[$(date +%Y-%m-%d\ %H:%M:%S)] MAC SSD not mounted — skipping backup run." >&2
  exit 0
fi

mkdir -p "$SNAPSHOT_DIR" "$LOG_DIR" || fail "cannot create backup dirs"

if [[ ! -f "$EXCLUDE_FILE" ]]; then
  fail "exclude file missing: $EXCLUDE_FILE"
fi

log "═══════════════════════════════════════════════════════════════"
log "Jura Labs backup starting — $NOW"
log "Target: $CURRENT_SNAPSHOT"

# ── Find the most recent previous snapshot for --link-dest ─────────

# Pick the newest snapshot that isn't today's in-progress one.
PREV_SNAPSHOT=""
if [[ -d "$SNAPSHOT_DIR" ]]; then
  # shellcheck disable=SC2012  # ls -t is fine for date-sortable names
  while IFS= read -r candidate; do
    [[ -z "$candidate" ]] && continue
    if [[ "$candidate" != "$TODAY" && -d "$SNAPSHOT_DIR/$candidate" ]]; then
      PREV_SNAPSHOT="$SNAPSHOT_DIR/$candidate"
      break
    fi
  done < <(ls -t "$SNAPSHOT_DIR" 2>/dev/null | grep -E '^[0-9]{4}-[0-9]{2}-[0-9]{2}$' || true)
fi

if [[ -n "$PREV_SNAPSHOT" ]]; then
  log "Hardlinking unchanged files from: $PREV_SNAPSHOT"
  LINK_DEST_ARG=(--link-dest="$PREV_SNAPSHOT")
else
  log "No previous snapshot found — this will be a full initial copy."
  LINK_DEST_ARG=()
fi

# If today's snapshot already exists (re-run same day), write into an
# .incomplete suffix, rename atomically on success.
STAGING_DIR="$CURRENT_SNAPSHOT.incomplete"
rm -rf "$STAGING_DIR"
mkdir -p "$STAGING_DIR"

# ── rsync runner ───────────────────────────────────────────────────

RSYNC_OPTS=(
  --archive           # -a: recursive, preserve perms/times/links/group/owner
  --hard-links        # preserve hardlinks within source tree
  --sparse            # preserve sparse files (ChromaDB HNSW index etc.)
                      # Required: link_lists.bin is 157 GB logical / 2.4 GB physical.
  --delete            # mirror — drop files removed from source
  --delete-excluded   # drop previously-included files now excluded
  --exclude-from="$EXCLUDE_FILE"
  --stats             # end-of-transfer summary
  --human-readable
  --itemize-changes   # one line per changed file (kept short in log)
)

sync_one() {
  local label="$1" source="$2" subdir="$3"
  local extra_raw="${4:-}"

  if [[ ! -e "$source" ]]; then
    log "SKIP [$label]: source missing ($source)"
    return 0
  fi

  local dest="$STAGING_DIR/$subdir"
  mkdir -p "$dest"

  # Build full rsync argv, conditionally appending link-dest + extras.
  # Bash 3.2 (macOS default) chokes on `"${arr[@]}"` when arr is empty
  # under `set -u`, so we concatenate into one array to avoid that trap.
  local -a argv=("${RSYNC_OPTS[@]}")
  if [[ -n "$PREV_SNAPSHOT" && -d "$PREV_SNAPSHOT/$subdir" ]]; then
    argv+=(--link-dest="$PREV_SNAPSHOT/$subdir")
  fi
  if [[ -n "$extra_raw" ]]; then
    # Split on spaces — callers should avoid whitespace in extras
    local IFS=' '
    local -a extras
    read -r -a extras <<< "$extra_raw"
    argv+=("${extras[@]}")
  fi
  argv+=("$source/" "$dest/")

  log "SYNC [$label]: $source → $subdir/"
  if rsync "${argv[@]}" >> "$LOG_FILE" 2>&1; then
    log "OK   [$label]"
  else
    local rc=$?
    # rsync exit codes 23/24 are "partial transfer" (e.g. files vanished
    # mid-run) — non-fatal for a live system. Anything else is a real error.
    if [[ $rc -eq 23 || $rc -eq 24 ]]; then
      log "WARN [$label]: rsync exit $rc (partial transfer — typically files changed mid-run; safe to ignore)"
    else
      log "FAIL [$label]: rsync exit $rc"
      return $rc
    fi
  fi
}

# ── Sources ────────────────────────────────────────────────────────

OVERALL_RC=0

sync_one "workspace"  "$HOME/Downloads/ecoadvisor"               "workspace"  || OVERALL_RC=$?
sync_one "claude"     "$HOME/.claude"                            "claude"     || OVERALL_RC=$?

# Dotfiles — individual files rather than all of $HOME
DOTFILES_STAGE="$STAGING_DIR/dotfiles"
mkdir -p "$DOTFILES_STAGE"
for f in .zshrc .zshenv .zprofile .bashrc .bash_profile .gitconfig .gitignore_global .npmrc .cargo/config.toml; do
  src="$HOME/$f"
  [[ -f "$src" ]] || continue
  dest="$DOTFILES_STAGE/$f"
  mkdir -p "$(dirname "$dest")"
  cp -a "$src" "$dest" 2>>"$LOG_FILE" && log "OK   [dotfile]: $f" || log "WARN [dotfile]: $f copy failed"
done

# SSH config AND keys (user opted in 2026-04-22; flag is that external SSD is
# not FileVault-encrypted). If you later enable FileVault on the SSD this is
# fine; if you remove the drive, treat the keys as compromised. rsync -a
# preserves the 600/700 perms already set on the source files.
if [[ -d "$HOME/.ssh" ]]; then
  sync_one "ssh"     "$HOME/.ssh"                                "dotfiles/.ssh" || OVERALL_RC=$?
fi

# Training corpus (on separate external drive — skip if not mounted)
sync_one "corpus"    "/Volumes/MAC SSD/Training Data"        "corpus"     || OVERALL_RC=$?

# Record manifest of sources for recovery reference
cat > "$STAGING_DIR/MANIFEST.txt" <<EOF
Jura Labs disaster-recovery snapshot
Created: $NOW
Host:    $(hostname)
User:    $USER

Contents:
  workspace/  ← ~/Downloads/ecoadvisor (all repos, excluding build artefacts)
  claude/     ← ~/.claude (memory, settings, projects — excluding transient caches)
  dotfiles/   ← .zshrc, .gitconfig, .ssh/, .cargo/config.toml, etc.
  corpus/     ← /Volumes/MAC SSD/Training Data (training data and models)

Exclusions applied from: scripts/backup/exclude.txt
Previous snapshot linked: ${PREV_SNAPSHOT:-none (initial copy)}

Recovery:
  1. Copy workspace/ back to ~/Downloads/ecoadvisor/
  2. Copy claude/ back to ~/.claude/
  3. Copy dotfiles/ back to \$HOME (preserving ~/.ssh perms: chmod 700 ~/.ssh, 600 ~/.ssh/id_*)
  4. Rebuild: cd juralabs/ui && npm install; cd ../src-tauri && cargo build; cd ../sidecar && pip install -r requirements.txt
EOF

# ── Finalise ───────────────────────────────────────────────────────

if [[ $OVERALL_RC -eq 0 ]]; then
  # Atomic promotion: remove any stale today's snapshot, rename staging in.
  rm -rf "$CURRENT_SNAPSHOT"
  mv "$STAGING_DIR" "$CURRENT_SNAPSHOT"
  log "Snapshot finalised: $CURRENT_SNAPSHOT"

  # Update 'current' symlink for easy access
  ln -sfn "$TODAY" "$SNAPSHOT_DIR/current"
else
  log "Snapshot left at $STAGING_DIR for inspection (one or more sources failed)"
fi

# ── Retention: keep 7 daily + 4 weekly (Sundays) ───────────────────

log "Applying retention policy (7 daily + 4 weekly Sundays)…"

# Collect all date-stamped snapshots, newest first. macOS ships bash 3.2 which
# has no `mapfile` builtin — use a while-read loop instead.
ALL_SNAPS=()
while IFS= read -r line; do
  [[ -z "$line" ]] && continue
  ALL_SNAPS+=("$line")
done < <(
  find "$SNAPSHOT_DIR" -maxdepth 1 -type d -name '????-??-??' -exec basename {} \; \
    2>/dev/null | sort -r
)

KEEP=()
WEEKLY_KEPT=0
DAILY_KEPT=0
# Guard against empty array under `set -u` in bash 3.2.
if [[ ${#ALL_SNAPS[@]} -eq 0 ]]; then
  log "No snapshots to prune."
  ALL_SNAPS=()  # keep the for-loop below a no-op
fi

for snap in "${ALL_SNAPS[@]+"${ALL_SNAPS[@]}"}"; do
  # Convert YYYY-MM-DD → day-of-week (0=Sun on macOS `date -j`)
  dow=$(date -j -f "%Y-%m-%d" "$snap" +%w 2>/dev/null || echo "-1")

  if [[ $DAILY_KEPT -lt 7 ]]; then
    KEEP+=("$snap")
    DAILY_KEPT=$((DAILY_KEPT + 1))
  elif [[ "$dow" == "0" && $WEEKLY_KEPT -lt 4 ]]; then
    KEEP+=("$snap")
    WEEKLY_KEPT=$((WEEKLY_KEPT + 1))
  fi
done

for snap in "${ALL_SNAPS[@]+"${ALL_SNAPS[@]}"}"; do
  local_match=0
  for k in "${KEEP[@]+"${KEEP[@]}"}"; do
    [[ "$k" == "$snap" ]] && { local_match=1; break; }
  done
  if [[ $local_match -eq 0 ]]; then
    log "RETAIN prune: $snap"
    rm -rf "$SNAPSHOT_DIR/$snap"
  fi
done
log "Kept ${#KEEP[@]} snapshot(s): ${KEEP[*]+${KEEP[*]}}"

# Also prune old logs — keep 30 days
find "$LOG_DIR" -maxdepth 1 -type f -name '????-??-??.log' -mtime +30 -delete 2>/dev/null || true

# ── Summary ────────────────────────────────────────────────────────

USED_SIZE=$(du -sh "$CURRENT_SNAPSHOT" 2>/dev/null | awk '{print $1}')
TOTAL_SIZE=$(du -sh "$SNAPSHOT_DIR" 2>/dev/null | awk '{print $1}')
AVAIL=$(df -h "/Volumes/MAC SSD" | awk 'NR==2 {print $4}')

log "Snapshot size: ${USED_SIZE:-?} (today) / ${TOTAL_SIZE:-?} (all retained)"
log "SSD available: $AVAIL"
log "Backup complete — exit $OVERALL_RC"
log ""

exit $OVERALL_RC
