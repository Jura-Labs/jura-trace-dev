#!/bin/bash
#
# Weekly corpus mirror: /Volumes/MAC SSD/Training Data → /Volumes/Samsung USB/Training Data
#
# Purpose: offsite copy of the training corpus. The daily backup stores the
# corpus alongside snapshots on the same Thunderbolt SSD, so a drive failure
# would lose both canonical and backup copies. This script keeps the Samsung
# USB as an independent offsite mirror.
#
# Scheduled via ~/Library/LaunchAgents/org.juralabs.corpus-mirror.plist
# (runs Sundays at 02:00, after the daily backup completes at 01:00).
#
# If either drive is unmounted, exits cleanly without error so missed runs
# don't page the user.

set -uo pipefail
IFS=$'\n\t'

SOURCE="/Volumes/MAC SSD/Training Data"
TARGET="/Volumes/Samsung USB/Training Data"
LOG_DIR="/Volumes/MAC SSD/Backups/jura-labs/logs"
TODAY="$(date +%Y-%m-%d)"
LOG_FILE="$LOG_DIR/corpus-mirror-$TODAY.log"

log() {
  local msg="[$(date +%H:%M:%S)] $*"
  echo "$msg"
  [[ -d "$LOG_DIR" ]] && echo "$msg" >> "$LOG_FILE"
}

# ── Preflight ──────────────────────────────────────────────────────

if [[ ! -d "$SOURCE" ]]; then
  echo "[$(date +%Y-%m-%d\ %H:%M:%S)] Source not mounted: $SOURCE — skipping." >&2
  exit 0
fi

if [[ ! -d "/Volumes/Samsung USB" ]]; then
  echo "[$(date +%Y-%m-%d\ %H:%M:%S)] Samsung USB not mounted — skipping corpus mirror." >&2
  exit 0
fi

mkdir -p "$LOG_DIR" "$TARGET"

log "═══════════════════════════════════════════════════════════════"
log "Corpus mirror starting"
log "Source: $SOURCE"
log "Target: $TARGET"

# ── Rsync ──────────────────────────────────────────────────────────

# Samsung USB is likely FAT32 / exFAT / HFS+ depending on how it was formatted.
# Avoid APFS-specific flags (--sparse is fine on any FS).
# --delete keeps target in lockstep with source.
# We DO NOT use --link-dest here — this is a flat mirror, not a snapshot
# series.

RSYNC_OPTS=(
  --archive
  --sparse
  --delete
  --delete-excluded
  --stats
  --human-readable
  --no-perms           # Samsung USB may be non-POSIX; don't fail on perms
  --no-owner
  --no-group
)

log "Running rsync — this will take a while on the first mirror (USB 3.0 bottleneck)."

START=$(date +%s)
if rsync "${RSYNC_OPTS[@]}" "$SOURCE/" "$TARGET/" >> "$LOG_FILE" 2>&1; then
  log "OK — mirror complete"
  RC=0
else
  rc=$?
  if [[ $rc -eq 23 || $rc -eq 24 ]]; then
    log "WARN — rsync exit $rc (partial transfer, typically safe to ignore)"
    RC=0
  else
    log "FAIL — rsync exit $rc"
    RC=$rc
  fi
fi

ELAPSED=$(( $(date +%s) - START ))
MINS=$(( ELAPSED / 60 ))
SECS=$(( ELAPSED % 60 ))

SIZE=$(du -sh "$TARGET" 2>/dev/null | awk '{print $1}')
log "Mirror size on Samsung USB: ${SIZE:-?}"
log "Duration: ${MINS}m ${SECS}s"

# Write a marker file on the Samsung USB so user can see when it last synced
echo "Last mirrored from /Volumes/MAC SSD/Training Data on $(date)" > "$TARGET/.last-mirror"

log "Corpus mirror complete — exit $RC"

# Prune old corpus-mirror logs (keep 90 days)
find "$LOG_DIR" -maxdepth 1 -type f -name 'corpus-mirror-????-??-??.log' -mtime +90 -delete 2>/dev/null || true

exit $RC
