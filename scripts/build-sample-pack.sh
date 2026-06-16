#!/usr/bin/env bash
# Build the Jura Trace onboarding sample pack (jura-trace-sample-images.zip).
#
# Produces four practice images plus a readme.txt that show what each kind of
# verification result looks like. The pack is attached to the GitHub release as
# an asset; it is not committed to the repo (the large binaries stay out of
# source control).
#
# Inputs (provide these two source images):
#   AUTHENTIC  a real camera photograph with intact EXIF (the "authentic" and,
#              once signed, the "signed" samples derive from this)
#   AI_IMAGE   an AI-generated image. The shipped pack uses an OpenAI image that
#              carries valid C2PA Content Credentials (digitalSourceType:
#              trainedAlgorithmicMedia), which is what makes it a rich teaching
#              sample. Confirm your AI image verifies as Low Trust before use.
#
# Requires:
#   - python3 with Pillow (for the re-saved variant + EXIF handling)
#   - exiftool                 (to scrub GPS / serials from the authentic photo)
#   - a running Jura Trace app with the local REST API on 127.0.0.1:8300 and an
#     API key, exported as JURA_API_KEY (the "jt_..." bearer value). Used to
#     sign the C2PA sample and to verify every file's verdict.
#
# Usage:
#   AUTHENTIC=path/to/photo.jpg AI_IMAGE=path/to/ai.png \
#   JURA_API_KEY=jt_xxxx bash scripts/build-sample-pack.sh
#
set -euo pipefail

: "${AUTHENTIC:?set AUTHENTIC to the source camera photo}"
: "${AI_IMAGE:?set AI_IMAGE to the AI-generated source image}"
: "${JURA_API_KEY:?set JURA_API_KEY to a jt_... bearer for the local REST API}"
API="${JURA_API:-http://127.0.0.1:8300}"

OUT="docs/sample-media/jura-trace-sample-images"
rm -rf "$OUT" "$OUT.zip"; mkdir -p "$OUT" build/sample-media
work=build/sample-media

verify() { # file -> prints overallTrust + deepfake verdict
  curl -s -m 180 -H "Authorization: Bearer ${JURA_API_KEY}" -F "file=@$1" "$API/api/v1/verify" \
    | python3 -c "import sys,json;r=json.load(sys.stdin).get('data',{});print('  trust',round(r.get('overallTrust',-1),3),'deepfake',(r.get('deepfakeResult') or {}).get('verdictLevel'),'c2paValid',r.get('c2paValid'))"
}

echo "1/4 authentic.jpg (scrub GPS + serials, keep camera identity)"
exiftool -o "$OUT/authentic.jpg" -GPS:all= -SerialNumber= -InternalSerialNumber= \
  -LensSerialNumber= -OwnerName= -XMP:all= "$AUTHENTIC" >/dev/null
verify "$OUT/authentic.jpg"          # expect ~0.84 Authentic

echo "2/4 ai-generated-openai.png (AI image, Content Credentials preserved)"
cp "$AI_IMAGE" "$OUT/ai-generated-openai.png"
verify "$OUT/ai-generated-openai.png"  # expect ~0.15 Low Trust, c2paValid true

echo "3/4 resaved-uncertain.jpg (downscale + re-save, metadata stripped)"
python3 - "$OUT/authentic.jpg" "$work/resaved.jpg" <<'PY'
import sys; from PIL import Image
im = Image.open(sys.argv[1]).convert("RGB"); w,h = im.size; s = 800/max(w,h)
im.resize((round(w*s), round(h*s))).save(sys.argv[2], "JPEG", quality=92)  # no EXIF
PY
cp "$work/resaved.jpg" "$OUT/resaved-uncertain.jpg"
verify "$OUT/resaved-uncertain.jpg"  # expect ~0.55 Uncertain

echo "4/4 jura-trace-signed.jpg (Local Signing via Protect API)"
curl -s -m 60 -H "Authorization: Bearer ${JURA_API_KEY}" \
  -F "file=@$OUT/authentic.jpg" -F "creator_name=Jura Labs" -F "license=CC0-1.0" \
  "$API/api/v1/protect/sign" -o "$OUT/jura-trace-signed.jpg"
verify "$OUT/jura-trace-signed.jpg"  # expect ~0.94, Jura Trace Content Credentials

# readme.txt is authored alongside this script; keep it in sync if verdicts move.
cp docs/sample-media/readme.txt "$OUT/readme.txt"

( cd docs/sample-media && zip -rq jura-trace-sample-images.zip jura-trace-sample-images )
echo "Built docs/sample-media/jura-trace-sample-images.zip"
echo "Attach it to the GitHub release; do not commit the zip or the images."
