#!/bin/zsh
# SPDX-License-Identifier: AGPL-3.0-or-later
# Build the drift probe environments under $JURA_DRIFT_WORK/envs/ from the
# pinned requirements in envs/. With no arguments it builds every file in
# envs/; name environments to build only those, e.g. `build_envs.sh baseline
# candidate`. A single-variable attribution run needs two: baseline and one
# file that differs from it in exactly one pin.
set -euo pipefail
D=${JURA_DRIFT_WORK:-$HOME/.cache/jura-drift}
HERE=${0:A:h}
PY=${JURA_DRIFT_PYTHON:-3.13.5}
names=("$@")
(( ${#names} )) || names=(${HERE}/envs/*.txt(:t:r))
mkdir -p "$D/envs" "$D/logs"
for n in $names; do
  req="$HERE/envs/$n.txt"
  [[ -f $req ]] || { echo "no $req" >&2; exit 1; }
  echo "== $n start $(date)" | tee "$D/logs/build_$n.log"
  uv venv --python "$PY" --clear "$D/envs/$n" >> "$D/logs/build_$n.log" 2>&1
  uv pip install --python "$D/envs/$n/bin/python" -r "$req" >> "$D/logs/build_$n.log" 2>&1
  echo "== $n install rc=$? $(date)" | tee -a "$D/logs/build_$n.log"
done
