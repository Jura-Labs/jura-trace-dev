#!/usr/bin/env bash
# backup-models.sh — Archive the trained classifier model and training metadata.
#
# Usage:
#   ./scripts/backup-models.sh [output-dir]
#
# Arguments:
#   output-dir  Directory to write the archive to. Defaults to the current
#               working directory. The directory must already exist.
#
# The models/ directory contains:
#   deepfake_classifier.joblib  — trained GBM classifier (~few MB, in git)
#   classifier_metadata.json    — training run metadata, if present
#   calibration_results.json    — calibration output, if present
#
# The .joblib file is already tracked in git, so it is backed up as part of
# normal git push. This script is for an explicit out-of-band backup copy
# alongside the corpus archive, so that model + data can be restored together
# without needing to re-run the full training pipeline.
#
# See docs/development-workflow.md §4 for the full backup strategy.

set -euo pipefail

# ── Resolve paths ──────────────────────────────────────────────────────────────

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
MODELS_DIR="${PROJECT_ROOT}/models"
OUTPUT_DIR="${1:-${PROJECT_ROOT}}"

# ── Validate ───────────────────────────────────────────────────────────────────

if [ ! -d "${MODELS_DIR}" ]; then
    echo "Error: models/ directory not found at ${MODELS_DIR}."
    echo "Run from the project root, or check the models/ directory exists."
    exit 1
fi

if [ ! -d "${OUTPUT_DIR}" ]; then
    echo "Error: Output directory '${OUTPUT_DIR}' does not exist."
    echo "Create it first: mkdir -p '${OUTPUT_DIR}'"
    exit 1
fi

# ── Check there is something to archive ───────────────────────────────────────

MODEL_COUNT=$(find "${MODELS_DIR}" -type f | wc -l | tr -d ' ')
if [ "${MODEL_COUNT}" -eq 0 ]; then
    echo "Warning: models/ directory is empty. Nothing to archive."
    exit 0
fi

# ── Build the archive ──────────────────────────────────────────────────────────

TIMESTAMP=$(date +%Y%m%d_%H%M%S)
ARCHIVE="${OUTPUT_DIR}/models_backup_${TIMESTAMP}.tar.gz"

echo "Jura Trace — model backup"
echo "  Timestamp : ${TIMESTAMP}"
echo "  Output    : ${ARCHIVE}"
echo ""

# List what is being archived.
echo "Files to archive:"
find "${MODELS_DIR}" -type f | while read -r f; do
    REL="${f#${PROJECT_ROOT}/}"
    SIZE=$(du -sh "${f}" | cut -f1)
    echo "  ${REL}  (${SIZE})"
done
echo ""
echo "Compressing..."

# Run tar from the project root so archive paths are relative to the project.
(cd "${PROJECT_ROOT}" && tar -czf "${ARCHIVE}" models/)

SIZE=$(du -sh "${ARCHIVE}" | cut -f1)
echo "Done."
echo ""
echo "Archive : ${ARCHIVE}"
echo "Size    : ${SIZE}"
echo ""
echo "To restore:"
echo "  tar -xzf $(basename "${ARCHIVE}")"
echo ""
echo "To upload to cloud storage (requires rclone configured):"
echo "  rclone copy ${ARCHIVE} remote:jura-trace-models/"
echo ""
echo "Note: The .joblib file is also tracked in git and backed up on"
echo "every git push. This archive is for paired corpus+model recovery."
