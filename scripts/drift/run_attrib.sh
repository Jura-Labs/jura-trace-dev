#!/bin/zsh
# SPDX-License-Identifier: AGPL-3.0-or-later
D=${JURA_DRIFT_WORK:-$HOME/.cache/jura-drift}
cd "${0:A:h}"
unset JURA_SIDECAR_KEY
run() { env=$1; shift; echo "== $env $* $(date +%T)"; "$D/envs/$env/bin/python" "$@" 2>&1 | grep -v "^$" | tail -5; echo "== rc=${pipestatus[1]}"; }
for e in numpy_only scipy_only opencv_only sklearn_only; do run $e probe2_gbm.py; done
run sklearn_only probe3_clf.py
run numpy_only probe3_clf.py
for e in opencv_only numpy_only scipy_only; do run $e probe5_cv.py; done
run numpy_only probe4_univfd.py
echo ATTRIB_DONE $(date +%T)
