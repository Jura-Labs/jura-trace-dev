# BL-SIZE-001: 242 MB of every download exists to compute 10 KB of constants

**Status**: Open. Found 3 September 2026, assessing adoption barriers.
**Raised**: 3 September 2026
**Severity**: High for adoption, low for risk. This is the cheapest large
reduction available in the download, and download size is the barrier
persona review named as a hard gate for field users on metered or slow
connections.

## What is wrong

`src-tauri/models/` ships both halves of CLIP ViT-B/32:

| File | Size |
|---|---|
| `clip-vit-b32-vision.onnx.data` | 335 MB |
| `clip-vit-b32-text.onnx.data` | 242 MB |
| plus the two small `.onnx` graph files | ~2.3 MB |

The vision encoder earns its place: every image is passed through it, both
for zero-shot scoring and for the UnivFD linear probe.

The text encoder does one thing. `sidecar/app/services/clip_detector.py:66-72`:

```python
_TEXT_PROMPTS = [
    "a photograph taken by a camera",
    "a real photograph of a real scene",
    "an AI-generated image",
    "a synthetic image created by artificial intelligence",
    "a digitally manipulated photograph",
]
```

Five hardcoded strings. `_encode_text_prompts()` at line 312 runs the text
encoder over them once and caches the result, and its own docstring calls
them "the fixed `_TEXT_PROMPTS`". The output is a `(5, 512)` float32 array,
which is **10,240 bytes**.

So 242 MB ships in every installer, on every platform, to compute ten
kilobytes of constants that cannot change, because the inputs are literals
in the source.

## Why it matters

The installers are 690 to 815 MB. This is roughly 30 per cent of that, for
one array.

Persona review on 3 September named the 800 MB download as a hard gate for
the field-investigator persona, not a grumble. Every megabyte here is paid
by someone on a metered or slow connection, on every install, and again on
every update once the updater works. It also feeds the release pipeline
problems: the Linux build had to drop RPM in commit `aa6f562` because xz
compression hung on the ONNX payload.

## What would fix it

1. **Precompute the five embeddings at build time** and ship the resulting
   `(5, 512)` float32 array as a small `.npy` file next to the probe.
2. **Load that array instead of running the text encoder.** The call site
   already caches, so the change is to populate the cache from a file
   rather than from an inference run. `_encode_text_prompts()` becomes a
   file read.
3. **Drop `clip-vit-b32-text.onnx` and its `.data` from
   `src-tauri/models/`**, and from the resources list once BL-REL-001's
   explicit enumeration replaces the `models/*` glob.
4. **Keep the text encoder in the canonical `models/` directory** and in
   the build tooling, so the embeddings can be regenerated if the prompts
   ever change. Record in the model card that they were generated at build
   time and from which model file.

## The one thing to verify before doing it

The thresholds are calibrated, so nothing may change the numbers.
Precomputed embeddings should be bit-identical to runtime-generated ones,
since it is the same model, the same tokeniser and deterministic inference,
but that has to be demonstrated rather than assumed: onnxruntime versions
differ across platforms, and the host runs a different version from CI per
SR-21.

The test is cheap. Generate the array, then assert it equals the runtime
result on each platform, and re-run the calibration corpus to confirm the
verdicts are unchanged. If any platform diverges, ship the encoder for that
platform only, or fall back to shipping it everywhere and note the finding.

## Related, and worth measuring at the same time

The PyInstaller sidecar bundle is 529 MB uncompressed
(`sidecar/dist/jura-sidecar`), which is nearly as large as the models. It
has not been audited for what it is actually carrying. Whether numpy,
opencv, scipy and onnxruntime need to be present at those sizes is a
separate question and probably a larger prize than this one, but it is also
a much longer investigation. This item is the part that is provably free.

## What not to do

Do not quantise the vision encoder to save space. That changes inference
numerically, and the detector thresholds were calibrated against the
current weights. It is a recalibration, not a size optimisation.
