#!/usr/bin/env python3
"""Generate the CAI Member Lockup SVGs for Jura Labs.

Composes the official Content Authenticity Initiative lockup with the
Jura Labs identity (eye-mark + "Jura Labs" wordmark), per the CAI Brand
Guidelines p. 16 Member Lockup spec:

    CAI lockup  |  1.5X  |  divider  |  1.5X  |  Member logo

where X = ½ the symbol height (CAI guideline p. 12). Member logo is sized
to be optically equal to the CAI lockup, aligned to its centre.

Produces:
    cai/member-lockup-black.svg  — use on LIGHT backgrounds
    cai/member-lockup-white.svg  — use on DARK backgrounds

Sources:
    cai/CAI_Lockup_RGB_Black.svg  } official CAI assets (do not modify)
    cai/CAI_Lockup_RGB_White.svg  }
    logo-eye-mark.svg             — Jura Labs eye-mark
"""
from __future__ import annotations

import re
from pathlib import Path

from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.ttLib import TTFont

HERE = Path(__file__).parent
CAI_DIR = HERE / "cai"

CAI_BLACK_SRC = CAI_DIR / "CAI_Lockup_RGB_Black.svg"
CAI_WHITE_SRC = CAI_DIR / "CAI_Lockup_RGB_White.svg"
EYE_MARK_SRC = HERE / "logo-eye-mark.svg"
WORDMARK_FONT = HERE / "fonts" / "SourceSerif4-Bold.ttf"

OBSIDIAN = "#1E2128"
QUARTZ = "#EDEAE4"


def render_wordmark_paths(
    text: str, font_size: float, fill: str, x: float, baseline_y: float
) -> tuple[str, float]:
    """Outline ``text`` into SVG <path> data and return (svg_fragment, advance_width).

    Outlining the wordmark removes any runtime font dependency: the lockup
    renders identically wherever it is placed, regardless of which serif faces
    are installed at the destination. Source Serif 4 is OFL-licensed.
    """
    font = TTFont(WORDMARK_FONT)
    cmap = font.getBestCmap()
    glyph_set = font.getGlyphSet()
    units_per_em = font["head"].unitsPerEm
    scale = font_size / units_per_em

    fragments: list[str] = []
    cursor_units = 0.0
    for ch in text:
        glyph_name = cmap.get(ord(ch))
        if glyph_name is None:
            glyph_name = ".notdef"
        glyph = glyph_set[glyph_name]
        pen = SVGPathPen(glyph_set)
        glyph.draw(pen)
        d = pen.getCommands()
        if d:
            tx = x + cursor_units * scale
            # Source Serif glyphs are drawn with y increasing upwards; flip to
            # SVG's y-down coordinate space and translate to the visual baseline.
            fragments.append(
                f'<path d="{d}" fill="{fill}" '
                f'transform="translate({tx:.4f},{baseline_y:.4f}) scale({scale:.6f},-{scale:.6f})"/>'
            )
        cursor_units += glyph.width

    return "\n    ".join(fragments), cursor_units * scale


def read_svg_inner(path: Path) -> tuple[str, tuple[float, float]]:
    """Return (inner_xml, (viewBox_w, viewBox_h)) for an SVG file."""
    text = path.read_text()
    vb = re.search(r'viewBox="([^"]+)"', text)
    if not vb:
        raise SystemExit(f"{path}: no viewBox")
    parts = vb.group(1).split()
    w, h = float(parts[2]), float(parts[3])
    # Strip outer <svg ...> wrapper and any <?xml…?> / generator comment.
    inner = re.sub(r'^.*?<svg[^>]*>', '', text, count=1, flags=re.S)
    inner = re.sub(r'</svg>\s*$', '', inner)
    return inner, (w, h)


def make_lockup(variant: str) -> str:
    if variant == "black":
        cai_inner, (cai_w, cai_h) = read_svg_inner(CAI_BLACK_SRC)
        ink = OBSIDIAN
        bg_descriptor = "for use on LIGHT backgrounds"
    elif variant == "white":
        cai_inner, (cai_w, cai_h) = read_svg_inner(CAI_WHITE_SRC)
        ink = QUARTZ
        bg_descriptor = "for use on DARK backgrounds"
    else:
        raise ValueError(variant)

    eye_inner, (eye_w, eye_h) = read_svg_inner(EYE_MARK_SRC)
    # Sanity-check eye-mark is the expected 256-unit square.
    assert eye_w == eye_h, "eye-mark must be square"

    # CAI Brand Guidelines p. 12: lockup symbol height = Y; in the official
    # Black/White lockup files Y ≈ viewBox height (the symbol is the tallest
    # element). X = ½ Y per the member-lockup spec annotations on p. 16.
    symbol_height = cai_h
    X = symbol_height / 2.0
    spacer = 3 * X  # 1.5X + 1.5X
    divider_x_offset = 1.5 * X  # divider sits at the midpoint of the spacer

    # Member side: eye-mark (square, matching the CAI symbol height) plus a
    # small gap and the "Jura Labs" wordmark, baseline-centred on the lockup.
    eye_target_h = symbol_height
    eye_scale = eye_target_h / eye_w
    eye_target_w = eye_target_h  # square

    # Inter-mark gap on the member side (eye-mark to wordmark). The CAI
    # lockup's own internal gap between symbol and wordmark is small —
    # mirror that visual rhythm.
    inter_member_gap = symbol_height * 0.18

    wordmark_text = "Jura Labs"
    # Source Serif Bold at ~0.62× symbol height matches the proportion used
    # in make-wordmark.py for the Canva wordmark exports.
    wordmark_size = symbol_height * 0.62
    # Small trailing right-margin so the viewBox is never glyph-tight.
    right_margin = symbol_height * 0.10

    # Layout x positions
    cai_x = 0.0
    divider_x = cai_w + divider_x_offset
    eye_x = cai_w + spacer
    wordmark_x = eye_x + eye_target_w + inter_member_gap

    height = symbol_height
    centre_y = height / 2.0
    # Outline the wordmark to path data so the SVG carries no runtime font
    # dependency. The visual baseline sits at ~0.74 of font-size below the
    # cap-line, which puts the wordmark optically centred against the
    # CAI "Content Authenticity Initiative" wordmark.
    wordmark_baseline_y = centre_y + wordmark_size * 0.28
    wordmark_paths, wordmark_advance = render_wordmark_paths(
        wordmark_text, wordmark_size, ink, wordmark_x, wordmark_baseline_y
    )
    total_w = wordmark_x + wordmark_advance + right_margin

    # Divider: thin vertical rule, height equal to the CAI symbol cap-height.
    # Per the p. 16 layout the divider visually matches the CAI symbol's
    # visible extent. Use ~80% of full height, vertically centred.
    divider_h = symbol_height * 0.78
    divider_y = (height - divider_h) / 2.0
    divider_w = max(symbol_height * 0.005, 2.0)  # ~0.5 % of height

    svg = f"""<?xml version="1.0" encoding="UTF-8"?>
<!--
  Jura Labs × Content Authenticity Initiative Member Lockup ({variant})

  Composed per CAI Brand Guidelines p. 16 (Member Lockup).
  {bg_descriptor}

  Generated by docs/branding/make-cai-member-lockup.py — regenerate after
  any change to the source CAI lockup or Jura Labs eye-mark.
-->
<svg xmlns="http://www.w3.org/2000/svg"
     viewBox="0 0 {total_w:.2f} {height:.2f}"
     role="img"
     aria-label="Jura Labs, member of the Content Authenticity Initiative">

  <title>Jura Labs — Member of the Content Authenticity Initiative</title>

  <!-- CAI lockup (official asset, unmodified) -->
  <g transform="translate({cai_x:.2f},0)">
    {cai_inner.strip()}
  </g>

  <!-- Divider rule, 1.5X from each side of the spacer -->
  <rect x="{divider_x - divider_w/2:.2f}" y="{divider_y:.2f}"
        width="{divider_w:.2f}" height="{divider_h:.2f}"
        fill="{ink}"/>

  <!-- Jura Labs eye-mark (brand colours; unchanged across light/dark) -->
  <g transform="translate({eye_x:.2f},0) scale({eye_scale:.6f})">
    {eye_inner.strip()}
  </g>

  <!-- Jura Labs wordmark (Source Serif 4 Bold, outlined to paths so the
       lockup has no runtime font dependency). Source Serif is OFL-licensed. -->
  <g>
    {wordmark_paths}
  </g>

</svg>
"""
    return svg


def main() -> None:
    if not CAI_BLACK_SRC.exists() or not CAI_WHITE_SRC.exists():
        raise SystemExit(
            f"Missing CAI source SVGs in {CAI_DIR}. "
            "Drop CAI_Lockup_RGB_Black.svg and CAI_Lockup_RGB_White.svg in first."
        )
    if not EYE_MARK_SRC.exists():
        raise SystemExit(f"Missing {EYE_MARK_SRC}")

    for variant in ("black", "white"):
        out = CAI_DIR / f"member-lockup-{variant}.svg"
        out.write_text(make_lockup(variant))
        print(f"wrote {out}  ({out.stat().st_size:,} bytes)")


if __name__ == "__main__":
    main()
