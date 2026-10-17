#!/bin/zsh
# SPDX-License-Identifier: AGPL-3.0-or-later
D=${JURA_DRIFT_WORK:-$HOME/.cache/jura-drift}
cd "${0:A:h}"
unset JURA_SIDECAR_KEY
run() { env=$1; shift; echo "== $env $* $(date +%T)"; "$D/envs/$env/bin/python" "$@" 2>&1 | grep -v "^$" | tail -20; echo "== rc=${pipestatus[1]}"; }
run baseline probe1_decode.py
run candidate probe1_decode.py
run baseline probe2_gbm.py
run candidate probe2_gbm.py
run baseline probe3_clf.py
run candidate probe3_clf.py
run baseline probe4_univfd.py
run candidate probe4_univfd.py
run baseline probe5_cv.py
run candidate probe5_cv.py
run baseline probe5_cv.py run2
echo ALLDONE $(date +%T)
