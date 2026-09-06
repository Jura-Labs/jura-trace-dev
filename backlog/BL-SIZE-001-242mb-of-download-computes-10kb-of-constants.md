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

## Precompute, not "make it optional"

A parallel review on the same day reached the same file and proposed making
the text encoder an optional in-app download, or dropping it, at one and a
half to two days. Worth being clear why precomputing is the better answer.

That review is right that the text encoder is not load-bearing for the
calibrated probe: UnivFD scores from vision embeddings only, and the text
side feeds the auxiliary five-bar zero-shot readout in Expert View.

But dropping the encoder removes that readout, and making it an optional
download adds a downloader, progress UI, resume handling and offline
messaging, which is the expensive part. **Precomputing keeps the readout
exactly as it is, removes the same 242 MB, and adds no runtime machinery
at all**, because the call site already caches. It should be well under a
day, and the only real work is the cross-platform equality check above.

## Related, and worth measuring at the same time

**pyarrow, 114 MB.** The same review found pyarrow inside the PyInstaller
bundle as a transitive dependency that does not appear in
`sidecar/requirements.txt` at all, and judged it almost certainly
excludable, at about half a day. If both land, roughly 356 MB comes out of
a 1.15 GB uncompressed payload.

**The sidecar bundle as a whole is 529 MB** uncompressed
(`sidecar/dist/jura-sidecar`), nearly as large as the models: cv2 at 88 MB
and onnxruntime at 63 MB are the next largest known components. It has
never been audited properly. That is probably a bigger prize than this
item and a much longer investigation. This item and pyarrow are the parts
that are provably cheap.

## Do not confuse this with quantisation

Quantising the vision encoder is a different proposition and is correctly
blocked. The FP32 ONNX port was gated at mean cosine similarity 1.000000
with maximum drift 0.000163
(`docs/calibration/univfd-v9-onnx-validation.md`). int8 or fp16 means a
full recalibration, not a size optimisation.

## What not to do

Do not quantise the vision encoder to save space. That changes inference
numerically, and the detector thresholds were calibrated against the
current weights. It is a recalibration, not a size optimisation.
