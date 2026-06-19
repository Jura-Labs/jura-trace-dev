#!/usr/bin/env bash
# Snapshot GitHub release asset download counts into a CSV time series.
#
# GitHub exposes a per-asset `download_count` that captures EVERY download
# from the release page, including direct-to-GitHub traffic that umami on
# juralabs.org cannot see. This script appends a timestamped row per asset
# so the running totals become deltas you can reconcile against umami.
#
# Usage:   scripts/snapshot-download-counts.sh [tag]      (default: v1.0.0)
# Output:  $JURA_STATS_DIR/download-counts.csv            (default: ~/jura-trace-analytics)
# Cron:    */30 * * * *  cd <repo> && scripts/snapshot-download-counts.sh >> /tmp/jura-dlsnap.log 2>&1
#
# The CSV lives OUTSIDE the repo by default (analytics data, not source).
set -euo pipefail

TAG="${1:-v1.0.0}"
REPO="${JURA_RELEASE_REPO:-Jura-Labs/jura-trace}"
OUT_DIR="${JURA_STATS_DIR:-$HOME/jura-trace-analytics}"
OUT="$OUT_DIR/download-counts.csv"

mkdir -p "$OUT_DIR"
[[ -f "$OUT" ]] || echo "timestamp_utc,tag,asset,download_count" > "$OUT"

TS="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

# @tsv keeps asset names with spaces intact on a single field.
gh api "repos/${REPO}/releases/tags/${TAG}" \
  --jq '.assets[] | [.name, .download_count] | @tsv' \
| while IFS=$'\t' read -r name count; do
    printf '%s,%s,%s,%s\n' "$TS" "$TAG" "$name" "$count" >> "$OUT"
  done

echo "Snapshot appended to $OUT at $TS (tag $TAG, repo $REPO)"
echo "Latest rows:"
tail -5 "$OUT"
