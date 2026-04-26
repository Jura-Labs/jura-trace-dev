#!/usr/bin/env python3
"""Generate Jura Trace social/OG card — 1280x640 PNG.

The eye-mark is the almond/lens form (matching the iconic eye-icon
reference) rendered in the mineral palette: lapis outline + obsidian
pupil + quartz catchlight.
"""
import math
from PIL import Image, ImageDraw, ImageFont

W, H = 1280, 640

OBSIDIAN = (30, 33, 40)        # #1E2128
QUARTZ = (237, 234, 228)       # #EDEAE4
LAPIS = (90, 133, 181)         # #5A85B5
LAPIS_LIGHT = (122, 160, 204)  # #7AA0CC
FLINT = (171, 168, 160)        # #ABA8A0
MALACHITE = (125, 167, 113)    # #7DA771
PUPIL_BLACK = (0, 0, 0)        # max contrast against malachite


def draw_almond_eye(img: Image.Image, cx: int, cy: int, scale: float = 1.0):
    """Almond/lens eye logo from two circular arcs (vesica piscis form).

    With circle radius R=125 and vertical offset d=150 the lens spans
    width 200 × height 100 in a 256-unit reference space. Scaled by
    `scale` for the requested render size.
    """
    R = 125 * scale
    stroke = max(8, int(16 * scale))

    # Circle B (centred BELOW eye centre) → its TOP arc = upper eyelid
    cx_B, cy_B = cx, cy + 75 * scale
    bbox_B = [cx_B - R, cy_B - R, cx_B + R, cy_B + R]
    a1 = math.degrees(math.atan2(-75, -100)) % 360  # left corner  ≈ 216.87°
    a2 = math.degrees(math.atan2(-75,  100)) % 360  # right corner ≈ 323.13°

    # Circle A (centred ABOVE eye centre) → its BOTTOM arc = lower eyelid
    cx_A, cy_A = cx, cy - 75 * scale
    bbox_A = [cx_A - R, cy_A - R, cx_A + R, cy_A + R]
    a3 = math.degrees(math.atan2(75,  100)) % 360   # right corner ≈ 36.87°
    a4 = math.degrees(math.atan2(75, -100)) % 360   # left corner  ≈ 143.13°

    draw = ImageDraw.Draw(img, "RGBA")
    draw.arc(bbox_B, a1, a2, fill=LAPIS, width=stroke)
    draw.arc(bbox_A, a3, a4, fill=LAPIS, width=stroke)

    # Iris (malachite fill + lapis ring)
    iris_r = 54.6 * scale
    iris_stroke = max(6, int(12 * scale))
    # Filled disc first
    draw.ellipse([cx - iris_r, cy - iris_r, cx + iris_r, cy + iris_r], fill=MALACHITE)
    # Then outline (PIL ellipse with both fill and outline doesn't honour width
    # cleanly across versions, so do them separately)
    draw.ellipse([cx - iris_r, cy - iris_r, cx + iris_r, cy + iris_r],
                  outline=LAPIS, width=iris_stroke)

    # Pupil — true black for max contrast against malachite
    pup_r = 26.9 * scale
    draw.ellipse([cx - pup_r, cy - pup_r, cx + pup_r, cy + pup_r], fill=PUPIL_BLACK)

    # Catchlight — small, top-right of pupil
    cl_r = 4.3 * scale
    cl_x, cl_y = cx + 8 * scale, cy - 16 * scale
    draw.ellipse([cl_x - cl_r, cl_y - cl_r, cl_x + cl_r, cl_y + cl_r], fill=QUARTZ)


def main():
    img = Image.new("RGB", (W, H), OBSIDIAN)
    draw = ImageDraw.Draw(img, "RGBA")

    # Subtle horizontal strata (geological feel)
    for i in range(8):
        y = int(H * (i / 7.5))
        draw.rectangle([0, y, W, y + max(2, H // 80)],
                        fill=(237, 234, 228, 8 if i % 2 == 0 else 4))

    # Earth-line gradient (lapis → malachite)
    grad = Image.new("RGBA", (W, 2), (0, 0, 0, 0))
    gd = ImageDraw.Draw(grad)
    for x in range(W):
        t = x / W
        r = int(LAPIS[0] * (1 - t) + MALACHITE[0] * t)
        g = int(LAPIS[1] * (1 - t) + MALACHITE[1] * t)
        b = int(LAPIS[2] * (1 - t) + MALACHITE[2] * t)
        gd.line([(x, 0), (x, 1)], fill=(r, g, b, 110))
    img.paste(grad, (0, 232), grad)

    draw_almond_eye(img, cx=176, cy=320, scale=1.10)

    f_brand = ImageFont.truetype("/System/Library/Fonts/Supplemental/Georgia Bold.ttf", 100)
    f_tag = ImageFont.truetype("/System/Library/Fonts/Supplemental/Georgia Italic.ttf", 56)
    f_sub = ImageFont.truetype("/System/Library/Fonts/Avenir Next.ttc", 28)
    f_meta = ImageFont.truetype("/System/Library/Fonts/Avenir Next.ttc", 22)

    draw.text((328, 198), "Jura Trace", font=f_brand, fill=QUARTZ)
    draw.text((332, 308), "Know What's Real.", font=f_tag, fill=LAPIS_LIGHT)
    sub = "Forensic media verification    ·    Local-first    ·    Free for not-for-profits"
    draw.text((334, 388), sub, font=f_sub, fill=FLINT)

    draw.text((W - 224, H - 64), "juralabs.org", font=f_meta, fill=FLINT)

    pill = "v0.9 release candidate"
    bbox = draw.textbbox((0, 0), pill, font=f_meta)
    pw = bbox[2] - bbox[0] + 28
    ph = bbox[3] - bbox[1] + 18
    px, py = 56, H - 80
    draw.rounded_rectangle([px, py, px + pw, py + ph], radius=ph // 2,
                            outline=(LAPIS_LIGHT[0], LAPIS_LIGHT[1], LAPIS_LIGHT[2], 200),
                            width=1,
                            fill=(LAPIS[0], LAPIS[1], LAPIS[2], 36))
    draw.text((px + 14, py + 6), pill, font=f_meta, fill=LAPIS_LIGHT)

    out = "/tmp/jura-trace-og.png"
    img.save(out, "PNG", optimize=True)
    print(f"wrote {out} {W}x{H}")


if __name__ == "__main__":
    main()
