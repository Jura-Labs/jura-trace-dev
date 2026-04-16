# C2PA Validator Conformance — Video Walkthrough Script

**Purpose**: Demonstrate to C2PA Conformance Programme staff how Jura Trace validates C2PA manifests, step-by-step, covering trust chain interrogation, hash verification, and failure detection.

**Duration target**: 8–12 minutes
**Recording**: Screen share with narration (OBS or QuickTime)
**Resolution**: 1920×1080 minimum — no compression that reduces text readability

---

## Pre-recording checklist

- [ ] Jura Trace dev build running (`make dev-tauri`)
- [ ] Python sidecar running (`make dev-sidecar`)
- [ ] Terminal with `c2patool` available for cross-reference
- [ ] Test files prepared (see §Test files below)
- [ ] Browser DevTools open on Network tab (to show raw API responses if needed)
- [ ] Screen zoom set so all text is clearly legible

## Test files needed

| # | File | Purpose | Expected outcome |
|---|------|---------|-----------------|
| 1 | `jura_signed.jpg` | Jura-signed JPEG (Bedrock mode, self-signed CA) | Valid (signingCredential.untrusted accepted by policy) |
| 2 | `jura_signed.png` | Jura-signed PNG | Valid — demonstrates format support |
| 3 | `PXL_*.jpg` (Pixel Camera) | Externally-signed, short-lived cert + timestamp | Valid at signing (new tri-state) |
| 4 | `c2pa-rs_exp-test1.png` | c2pa-rs reference fixture | Demonstrates inbound interop |
| 5 | Tampered file | Copy a signed JPEG → hex-edit one byte of image data | Invalid — `assertion.dataHash.mismatch` |
| 6 | Stripped manifest | Copy a signed JPEG → remove JUMBF box | No manifest detected |
| 7 | Unsigned photo | Any camera photo without C2PA | No manifest detected |

### Creating the tampered test file

```bash
# Copy a signed file
cp jura_signed.jpg tampered.jpg
# Flip one byte near the end of image data (past JUMBF, in pixel data)
python3 -c "
d = bytearray(open('tampered.jpg','rb').read())
d[-1000] ^= 0xFF  # flip one byte
open('tampered.jpg','wb').write(d)
"
```

### Creating the stripped manifest file

```bash
# c2patool can strip manifests
c2patool jura_signed.jpg --strip --output stripped.jpg
# Or simply re-save without manifest via Pillow
python3 -c "from PIL import Image; Image.open('jura_signed.jpg').save('stripped.jpg')"
```

---

## Video script

### Scene 1 — Introduction (30s)

> "This is a product walkthrough of Jura Trace's C2PA Validator implementation. Jura Trace is a local-first desktop application for content verification, built on c2pa-rs 0.79. I'll demonstrate how it validates manifests across seven test scenarios: valid signatures in two formats, an externally-signed Pixel Camera photo, inbound interop with the c2pa-rs reference fixture, a tampered file, a stripped manifest, and an unsigned photo."

Show: Jura Trace main window, version number visible in Settings or title bar.

### Scene 2 — Valid Jura-signed JPEG (90s)

1. Drag `jura_signed.jpg` onto the Verify tab
2. **Pause on the C2PA Credentials panel** — point out:
   - "Valid" badge (green)
   - Claim Generator: "Jura Trace/0.9.0"
   - Signed At timestamp
   - Three assertions listed (c2pa.actions.v2, c2pa.rights, stds.iptc)
3. **Open the assertion detail** — expand `c2pa.actions.v2`, show the `c2pa.created` action and `digitalSourceType`
4. **Provenance chain timeline** — show the single-node timeline with "(valid)" label
5. **Cross-reference in terminal**: run `c2patool jura_signed.jpg` — show matching claim generator, assertions, and `validation_state: Valid` (noting signingCredential.untrusted is expected for self-signed)

> "Jura Trace correctly identifies the manifest, verifies the COSE signature against the embedded certificate chain, validates all assertion hashes and the data hash binding, and reports the signing timestamp. The signingCredential.untrusted status is expected — our Bedrock Signing mode uses a per-install local CA that isn't on any external trust list. Our validation policy accepts this as structurally valid."

### Scene 3 — Valid Jura-signed PNG (60s)

1. Drag `jura_signed.png` onto Verify
2. Brief tour: same C2PA panel structure, "Valid" badge, format shows `image/png`
3. Note: identical assertion set, same signing certificate

> "PNG validation exercises a different container format — c2pa-rs handles the JUMBF embedding differently for PNG versus JPEG. Same validation checks, same result."

### Scene 4 — Pixel Camera (expired cert + trusted timestamp) (90s)

1. Drag `PXL_*.jpg` onto Verify
2. **Key moment**: show the **amber "Valid at signing"** badge
3. Hover over the badge — read the tooltip explaining short-lived certs
4. Show the provenance timeline: "(valid at signing — certificate has since expired)"
5. Show the assertion detail: `c2pa.actions.v2` with `c2pa.created` + `digitalSourceType: computationalCapture`
6. **Cross-reference in terminal**: run `c2patool PXL_*.jpg` — show `validation_state: Invalid`, `signingCredential.expired` + `signingCredential.untrusted` in failures, but `timeStamp.validated` + `claimSignature.validated` in successes

> "This is a real-world Google Pixel Camera photo. Google uses short-lived signing certificates — by the time you validate, the cert has expired. But the manifest includes a trusted timestamp from Google's Time Stamping Authority, and the claim signature itself is cryptographically valid. Jura Trace's validation distinguishes this from a genuinely invalid manifest: the signature was valid at signing time, proved by the timestamp. We display this as 'Valid at signing' in amber — it's not the same as a current green-valid signature, and it's not the red-invalid of a tampered file."

### Scene 5 — c2pa-rs reference fixture (60s)

1. Drag `c2pa-rs_exp-test1.png` onto Verify
2. Show C2PA panel — note different claim generator (not Jura Trace)
3. Show validation result

> "This is the reference test fixture from the c2pa-rs repository — signed by the CAI SDK, not by Jura Trace. Our validator successfully parses the manifest, enumerates the assertions, and reports the validation status. This demonstrates inbound interoperability — we're not just validating our own signatures."

### Scene 6 — Tampered file (90s)

**This is the critical scene for the C2PA staff.**

1. Drag `tampered.jpg` onto Verify
2. **Key moment**: show **red "Invalid"** badge
3. Show that the manifest IS parsed (claim generator, assertions are readable)
4. But the data hash check fails — the signature covers the original bytes, and we modified one byte

> "I took the same signed JPEG from Scene 2 and flipped a single byte in the image data. The manifest is still present and parseable — Jura Trace extracts the claim generator, assertions, and signing timestamp. But the data hash verification fails: the SHA-256 of the actual image bytes no longer matches the hash stored in the `c2pa.hash.data` assertion. The COSE signature over the claim is technically still valid — the claim itself hasn't been modified — but the claim's hash binding to the asset data is broken. This is the core integrity check: any modification to the image data, no matter how small, produces a hash mismatch and an Invalid verdict."

### Scene 7 — Stripped manifest + unsigned (30s)

1. Drag `stripped.jpg` — show: no C2PA Credentials section appears
2. Drag an unsigned photo — show: same result, no manifest detected

> "When the JUMBF manifest store is absent — either stripped or never embedded — Jura Trace correctly reports no C2PA manifest. There's no false positive: we don't claim a manifest exists when it doesn't."

### Scene 8 — Closing (30s)

> "To summarise: Jura Trace's validator, built on c2pa-rs 0.79, performs signature verification, certificate chain evaluation, assertion hash matching, data hash binding verification, and timestamp validation. It correctly handles valid signatures, expired-but-timestamped credentials, tampered assets, and absent manifests. We currently claim conformance for image/jpeg and image/png. Thank you — happy to do a live interactive session if that would be helpful."

---

## Post-recording

- [ ] Review video for readability — all text must be legible at 1080p
- [ ] Upload to a private link (unlisted YouTube or direct file share)
- [ ] Send link to C2PA contact with updated evidence bundle (JPEG + PNG only)
