#!/usr/bin/env bash
# refresh-trust-lists.sh — Re-download the four C2PA trust list PEM bundles.
#
# Usage:
#   ./scripts/refresh-trust-lists.sh
#
# Refreshes:
#   src-tauri/trust-list/C2PA-TRUST-LIST.pem       (signing CAs — live list)
#   src-tauri/trust-list/C2PA-TSA-TRUST-LIST.pem   (timestamp authorities — live list)
#   src-tauri/trust-list/ITL-anchors.pem           (Interim Trust List — frozen Jan 2026)
#   src-tauri/trust-list/ITL-allowed.pem           (ITL end-entity allowlist — frozen Jan 2026)
#
# The first two are live — new CAs are added when vendors complete the C2PA
# Conformance Program.  Refresh on every minor release, or out-of-band if a
# Validator conformance round flags a missing CA.
#
# The ITL bundles are frozen as of January 2026 and should not change, but we
# still ship them because third-party assets signed before Jan 2026 continue
# to chain to them.  They are re-fetched for completeness.
#
# After running this script, run the following to verify:
#   cd src-tauri && cargo test --lib c2pa::tests::concatenated_trust_bundle_has_all_expected_certs
#
# If the cert count changed, update the expected count in that test AND in
# `src-tauri/trust-list/README.md`.

set -euo pipefail

# ── Resolve paths ──────────────────────────────────────────────────────────────

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
TRUST_DIR="${PROJECT_ROOT}/src-tauri/trust-list"

if [[ ! -d "${TRUST_DIR}" ]]; then
  echo "error: trust-list directory not found at ${TRUST_DIR}" >&2
  exit 1
fi

# ── Sources ────────────────────────────────────────────────────────────────────

CONFORMANCE_BASE="https://raw.githubusercontent.com/c2pa-org/conformance-public/main/trust-list"
VERIFY_SITE_BASE="https://raw.githubusercontent.com/contentauth/verify-site/main/static/trust"

declare -a SOURCES=(
  "C2PA-TRUST-LIST.pem|${CONFORMANCE_BASE}/C2PA-TRUST-LIST.pem"
  "C2PA-TSA-TRUST-LIST.pem|${CONFORMANCE_BASE}/C2PA-TSA-TRUST-LIST.pem"
  "ITL-anchors.pem|${VERIFY_SITE_BASE}/anchors.pem"
  "ITL-allowed.pem|${VERIFY_SITE_BASE}/allowed.pem"
)

# ── Fetch ──────────────────────────────────────────────────────────────────────

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TMP_DIR}"' EXIT

echo "Fetching C2PA trust lists..."
for entry in "${SOURCES[@]}"; do
  name="${entry%%|*}"
  url="${entry##*|}"
  echo "  ${name}  <-  ${url}"
  if ! curl -sSfL -o "${TMP_DIR}/${name}" "${url}"; then
    echo "error: failed to fetch ${url}" >&2
    exit 1
  fi
done

# ── Validate ───────────────────────────────────────────────────────────────────

echo ""
echo "Validating..."
total=0
for entry in "${SOURCES[@]}"; do
  name="${entry%%|*}"
  file="${TMP_DIR}/${name}"
  size="$(wc -c < "${file}" | tr -d ' ')"
  count="$(grep -c 'BEGIN CERTIFICATE' "${file}" || echo 0)"
  if [[ "${count}" -lt 1 ]]; then
    echo "error: ${name} contains no certificates — refusing to overwrite" >&2
    exit 1
  fi
  echo "  ${name}: ${count} certs (${size} bytes)"
  total=$((total + count))
done
echo "  total: ${total} certs across all four bundles"

# ── Diff summary ───────────────────────────────────────────────────────────────

echo ""
echo "Change summary (vs. currently vendored):"
changed=0
for entry in "${SOURCES[@]}"; do
  name="${entry%%|*}"
  old="${TRUST_DIR}/${name}"
  new="${TMP_DIR}/${name}"
  if [[ ! -f "${old}" ]]; then
    echo "  ${name}: NEW"
    changed=$((changed + 1))
    continue
  fi
  old_count="$(grep -c 'BEGIN CERTIFICATE' "${old}" || echo 0)"
  new_count="$(grep -c 'BEGIN CERTIFICATE' "${new}" || echo 0)"
  if cmp -s "${old}" "${new}"; then
    echo "  ${name}: unchanged (${new_count} certs)"
  else
    delta=$((new_count - old_count))
    sign="+"
    if [[ "${delta}" -lt 0 ]]; then sign=""; fi
    echo "  ${name}: CHANGED (${old_count} -> ${new_count}, ${sign}${delta} certs)"
    changed=$((changed + 1))
  fi
done

if [[ "${changed}" -eq 0 ]]; then
  echo ""
  echo "No changes — trust lists already up to date."
  exit 0
fi

# ── Install ────────────────────────────────────────────────────────────────────

echo ""
echo "Installing updated PEMs to ${TRUST_DIR}..."
for entry in "${SOURCES[@]}"; do
  name="${entry%%|*}"
  cp "${TMP_DIR}/${name}" "${TRUST_DIR}/${name}"
done

# ── Bump README "Last fetched" date ────────────────────────────────────────────

README="${TRUST_DIR}/README.md"
today="$(date +%Y-%m-%d)"
if [[ -f "${README}" ]]; then
  # macOS sed requires '' after -i; GNU sed does not.  Use a portable form.
  tmp_readme="$(mktemp)"
  sed "s/^\*\*Last fetched\*\*:.*/**Last fetched**: ${today}/" "${README}" > "${tmp_readme}"
  mv "${tmp_readme}" "${README}"
  echo "Updated ${README} (Last fetched: ${today})"
fi

# ── Next steps ─────────────────────────────────────────────────────────────────

echo ""
echo "Done.  Next steps:"
echo "  1. Run the trust-bundle test to confirm the expected cert count:"
echo "       cd src-tauri && cargo test --lib c2pa::tests::concatenated_trust_bundle_has_all_expected_certs"
echo "  2. If the total cert count changed from 175, update:"
echo "       - the assert_eq!(cert_count, 175, ...) line in src-tauri/src/c2pa.rs"
echo "       - the counts section in ${README}"
echo "  3. Run the Google Pixel end-to-end conformance test:"
echo "       cd src-tauri && cargo test --lib c2pa::tests::google_pixel_asset_chains_to_trusted_ca"
echo "  4. Commit: 'chore(trust): refresh C2PA + ITL trust lists (${today})'"
