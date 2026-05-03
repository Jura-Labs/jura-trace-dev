"""
Export OpenCLIP ViT-B/32 (laion2b_s34b_b79k) vision + text encoders to ONNX.

JTV-143 (2026-05-03): the production sidecar bundle excludes torch + open_clip
to keep the binary lean (~180 MB), but that costs the UnivFD v9 deepfake
probe (DiffusionDB recall 67.6% → 97.3% with vs without). Switching the
inference backend to ONNX runtime keeps the recall while pulling the
size delta from ~2 GB down to ~740 MB.

The probe was trained on FP32 CLIP embeddings produced by this exact
checkpoint. INT8 quantisation moves the LogReg decision boundary
(0.01–0.04 cosine drift; estimated 1.5–3.5 pp recall loss on flux_dev /
sdxl_turbo) so we ship FP32 ONNX. Re-export from source rather than
trusting community checkpoints because normalisation / projection layers
must match exactly.

Usage:
    python scripts/export_clip_onnx.py \
        --output-dir models/ \
        --opset 14

The companion script `scripts/validate_clip_onnx_drift.py` runs after
export to confirm cosine drift < 0.002 mean against torch reference
embeddings on a corpus sample. That gate is the binding decision for
whether to ship JTV-143 in Sprint 31.
"""

from __future__ import annotations

import argparse
import hashlib
import sys
from pathlib import Path

import torch
import open_clip


VISION_INPUT_NAME = "image"
VISION_OUTPUT_NAME = "image_features"
TEXT_INPUT_NAME = "text_tokens"
TEXT_OUTPUT_NAME = "text_features"


def _sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


class _VisionWrapper(torch.nn.Module):
    """Wraps OpenCLIP's encode_image for ONNX export.

    The bare model returns (image, text) features tuple-style depending on
    forward args; wrapping isolates the image branch and gives ONNX a clean
    Module to trace.
    """

    def __init__(self, clip_model: torch.nn.Module) -> None:
        super().__init__()
        self.clip_model = clip_model

    def forward(self, image: torch.Tensor) -> torch.Tensor:
        return self.clip_model.encode_image(image)


class _TextWrapper(torch.nn.Module):
    def __init__(self, clip_model: torch.nn.Module) -> None:
        super().__init__()
        self.clip_model = clip_model

    def forward(self, text: torch.Tensor) -> torch.Tensor:
        return self.clip_model.encode_text(text)


def export(output_dir: Path, opset: int = 14) -> dict[str, str]:
    """Run the export. Returns {filename: sha256} for the validation gate."""
    output_dir.mkdir(parents=True, exist_ok=True)

    print("Loading ViT-B-32 (laion2b_s34b_b79k)...")
    model, _, _ = open_clip.create_model_and_transforms(
        "ViT-B-32", pretrained="laion2b_s34b_b79k"
    )
    model.eval()
    tokenizer = open_clip.get_tokenizer("ViT-B-32")

    vision = _VisionWrapper(model).eval()
    text = _TextWrapper(model).eval()

    vision_path = output_dir / "clip-vit-b32-vision.onnx"
    text_path = output_dir / "clip-vit-b32-text.onnx"

    # ── Vision encoder ──────────────────────────────────────────────────
    # OpenCLIP's preprocess transforms produce a 3x224x224 float32 tensor.
    # Dynamic batch axis lets the sidecar score one image per call (batch=1)
    # without re-exporting if a batched path is added later.
    dummy_image = torch.randn(1, 3, 224, 224)
    print(f"Exporting vision encoder → {vision_path} (opset {opset})...")
    with torch.no_grad():
        torch.onnx.export(
            vision,
            dummy_image,
            str(vision_path),
            input_names=[VISION_INPUT_NAME],
            output_names=[VISION_OUTPUT_NAME],
            dynamic_axes={
                VISION_INPUT_NAME: {0: "batch"},
                VISION_OUTPUT_NAME: {0: "batch"},
            },
            opset_version=opset,
            do_constant_folding=True,
        )

    # ── Text encoder ────────────────────────────────────────────────────
    # SimpleTokenizer pads to context length 77. Dynamic batch axis lets the
    # zero-shot path tokenise the 5 prompts (batch=5) in one call.
    dummy_text = tokenizer(["a photo"])
    print(f"Exporting text encoder → {text_path} (opset {opset})...")
    with torch.no_grad():
        torch.onnx.export(
            text,
            dummy_text,
            str(text_path),
            input_names=[TEXT_INPUT_NAME],
            output_names=[TEXT_OUTPUT_NAME],
            dynamic_axes={
                TEXT_INPUT_NAME: {0: "batch"},
                TEXT_OUTPUT_NAME: {0: "batch"},
            },
            opset_version=opset,
            do_constant_folding=True,
        )

    sha = {
        vision_path.name: _sha256(vision_path),
        text_path.name: _sha256(text_path),
    }

    vision_mb = vision_path.stat().st_size / 1_048_576
    text_mb = text_path.stat().st_size / 1_048_576
    print(f"\nExport complete:")
    print(f"  {vision_path.name}: {vision_mb:.1f} MB")
    print(f"    sha256: {sha[vision_path.name]}")
    print(f"  {text_path.name}: {text_mb:.1f} MB")
    print(f"    sha256: {sha[text_path.name]}")

    return sha


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--output-dir", type=Path, default=Path("models"))
    ap.add_argument("--opset", type=int, default=14)
    args = ap.parse_args()

    export(args.output_dir, opset=args.opset)
    return 0


if __name__ == "__main__":
    sys.exit(main())
