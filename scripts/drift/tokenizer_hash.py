# SPDX-License-Identifier: AGPL-3.0-or-later

"""Hash the CLIP BPE tokeniser's output over a broad text set.

The probe for a `regex` bump: clip_tokenizer.py compiles the BPE pattern
with `regex`, so a matching change would change token ids. Run it from the
repository root with each environment's Python, in the same checkout (the
text set includes the lines of docs/**/*.md), and compare the hashes:

    $JURA_DRIFT_WORK/envs/<env>/bin/python scripts/drift/tokenizer_hash.py

Then run `scripts/clip_text_embeddings.py check --models-dir models` in the
candidate, which re-encodes the five prompts and must match bit for bit.
"""
import hashlib, sys, glob, json
sys.path.insert(0, "sidecar")
import regex
from app.services.clip_tokenizer import tokenize
from app.services.clip_detector import _TEXT_PROMPTS
texts = list(_TEXT_PROMPTS)
for p in sorted(glob.glob("docs/**/*.md", recursive=True))[:400]:
    texts += [l for l in open(p, encoding="utf-8", errors="replace").read().split("\n") if l.strip()]
texts += ["日本語のテキスト", "Ελληνικά κείμενο", "русский текст", "عربي نص", "emoji 🙂👍🏽 mix", "tab\tand  spaces", "don't won't it's", "1234567890 3.14159", "ÆØÅ æøå ß ﬁ", "a" * 500]
h = hashlib.sha256(); n = 0
for t in texts:
    h.update(tokenize([t]).tobytes()); n += 1
print(json.dumps({"regex": regex.__version__, "texts": n, "sha256": h.hexdigest()}))
