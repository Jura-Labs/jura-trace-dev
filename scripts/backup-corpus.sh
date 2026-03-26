#!/usr/bin/env bash
# backup-corpus.sh — Create a timestamped archive of the training corpus.
#
# Usage:
#   ./scripts/backup-corpus.sh [output-dir]
#
# Arguments:
#   output-dir  Directory to write the archive to. Defaults to the current
#               working directory. The directory must already exist.
#
# The corpus/ directory (under scripts/) is gitignored because it contains
# training images (~1 GB). This script creates a tar.gz archive suitable
# for upload to a cloud object store (Backblaze B2, AWS S3, etc.).
#
# After creating the archive, upload it with rclone:
#   rclone copy "${archive}" remote:jura-trace-corpus/
#
# One-time rclone setup:
#   brew install rclone
#   rclone config  # follow the interactive prompts for your cloud provider
#
# See docs/development-workflow.md §4 for the full backup strategy.

set -euo pipefail

# ── Resolve paths ──────────────────────────────────────────────────────────────

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
OUTPUT_DIR="${1:-${PROJECT_ROOT}}"
CORPUS_DIR="${SCRIPT_DIR}/corpus"
EXPANDED_DIR="${SCRIPT_DIR}/expanded_corpus"

# ── Validate ───────────────────────────────────────────────────────────────────

if [ ! -d "${CORPUS_DIR}" ] && [ ! -d "${EXPANDED_DIR}" ]; then
    echo "Error: Neither scripts/corpus/ nor scripts/expanded_corpus/ found."
    echo "Run from the project root, or run scripts/build_corpus.py first."
    exit 1
fi

if [ ! -d "${OUTPUT_DIR}" ]; then
    echo "Error: Output directory '${OUTPUT_DIR}' does not exist."
    echo "Create it first: mkdir -p '${OUTPUT_DIR}'"
    exit 1
fi

# ── Build the archive ──────────────────────────────────────────────────────────

TIMESTAMP=$(date +%Y%m%d_%H%M%S)
ARCHIVE="${OUTPUT_DIR}/corpus_backup_${TIMESTAMP}.tar.gz"

echo "Jura Trace — corpus backup"
echo "  Timestamp : ${TIMESTAMP}"
echo "  Output    : ${ARCHIVE}"
echo ""

# Determine what to include: corpus/ and/or expanded_corpus/, whichever exist.
DIRS_TO_ARCHIVE=()
if [ -d "${CORPUS_DIR}" ]; then
    DIRS_TO_ARCHIVE+=("scripts/corpus")
    COUNT_CORPUS=$(find "${CORPUS_DIR}" -type f | wc -l | tr -d ' ')
    echo "  corpus/          : ${COUNT_CORPUS} files"
fi
if [ -d "${EXPANDED_DIR}" ]; then
    DIRS_TO_ARCHIVE+=("scripts/expanded_corpus")
    COUNT_EXPANDED=$(find "${EXPANDED_DIR}" -type f | wc -l | tr -d ' ')
    echo "  expanded_corpus/ : ${COUNT_EXPANDED} files"
fi

echo ""
echo "Compressing..."

# Run tar from the project root so archive paths are relative to the project.
(cd "${PROJECT_ROOT}" && tar -czf "${ARCHIVE}" "${DIRS_TO_ARCHIVE[@]}")

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
echo "  rclone copy ${ARCHIVE} remote:jura-trace-corpus/"
echo ""
echo "Store the archive in a private bucket. Approximate cost:"
echo "  Backblaze B2 : \$0.006/GB/month  (~\$0.006/month for a 1 GB corpus)"
echo "  AWS S3       : \$0.023/GB/month  (~\$0.023/month for a 1 GB corpus)"
