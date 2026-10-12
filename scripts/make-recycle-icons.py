#!/usr/bin/env python3
"""Draw the WineXP Recycle Bin icons (empty, full, and the desktop shortcut).

The upstream shell32 icons are not redistributable, so these are drawn from
scratch: a translucent blue-green bin with the paper either absent or showing
through the top, which is what Windows XP's own Recycle Bin read as at 16px.

Run:  python3 scripts/make-recycle-icons.py
"""
import os

from PIL import Image, ImageDraw

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, os.pardir, "assets", "windowsIcons")

# Windows XP "silver-blue" glass, sampled from the Luna colour scheme.
BIN_EDGE = (58, 96, 122, 255)
BIN_EDGE_SOFT = (86, 132, 162, 255)
BIN_FILL_TOP = (198, 226, 240, 130)
BIN_FILL_BOTTOM = (120, 172, 200, 90)
PAPER = (250, 250, 244, 255)
PAPER_SHADE = (206, 216, 224, 255)


def bin_shape(size):
    """(outline, inner) polygons for a bin at `size`, in pixels."""
    w = size
    inset = max(1, round(size * 0.06))
    top = round(size * 0.30)
    bottom = round(size * 0.94)
    # Slightly tapered, like the Luna bin.
    top_left = inset + round(size * 0.03)
    top_right = w - inset - round(size * 0.03)
    bottom_left = inset + round(size * 0.10)
    bottom_right = w - inset - round(size * 0.10)
    outer = [
        (top_left, top),
        (top_right, top),
        (bottom_right, bottom),
        (bottom_left, bottom),
    ]
    lip = max(1, round(size * 0.035))
    inner = [
        (top_left + lip, top + lip),
        (top_right - lip, top + lip),
        (bottom_right - lip, bottom - lip),
        (bottom_left + lip, bottom - lip),
    ]
    return outer, inner


def draw_bin(size, full):
    scale = 4  # supersample, then downscale for clean edges
    s = size * scale
    image = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    d = ImageDraw.Draw(image)
    outer, inner = bin_shape(s)

    if full:
        # Paper spilling over the rim, the way XP draws a bin holding items.
        sheet = Image.new("RGBA", (s, s), (0, 0, 0, 0))
        sd = ImageDraw.Draw(sheet)
        w = s * 0.030
        for dx, dy, tilt in ((-0.10, -0.10, -14), (0.02, -0.15, 6), (0.12, -0.06, 18)):
            x0 = s * (0.5 + dx)
            y0 = s * (0.30 + dy)
            x1 = x0 + s * 0.16
            y1 = y0 + s * 0.30
            sd.polygon([(x0, y0), (x1, y0 - w), (x1 + w, y1), (x0 + w, y1)],
                       fill=PAPER, outline=PAPER_SHADE)
        sheet = sheet.rotate(0)
        image.alpha_composite(sheet)

    d.polygon(inner, fill=BIN_FILL_TOP)
    # Vertical fade toward the base makes the bin read as translucent.
    fade = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    fd = ImageDraw.Draw(fade)
    top = inner[0][1]
    bottom = inner[2][1]
    for y in range(top, bottom):
        t = (y - top) / max(1, bottom - top)
        alpha = round(130 + (90 - 130) * t)
        fd.line([(0, y), (s, y)], fill=(120, 172, 200, alpha))
    mask = Image.new("L", (s, s), 0)
    ImageDraw.Draw(mask).polygon(inner, fill=255)
    image.alpha_composite(Image.composite(fade, Image.new("RGBA", (s, s)), mask))

    d.polygon(outer, outline=BIN_EDGE)
    # Highlight down the left edge, shadow down the right: cheap gloss.
    d.line([outer[0], outer[3]], fill=BIN_EDGE_SOFT, width=max(1, s // 64))
    d.line([outer[1], outer[2]], fill=(40, 70, 92, 255), width=max(1, s // 96))

    # Rim.
    rim_y = outer[0][1]
    d.line([outer[0], outer[1]], fill=BIN_EDGE, width=max(1, s // 32))

    return image.resize((size, size), Image.LANCZOS)


def main():
    os.makedirs(OUT, exist_ok=True)
    for size in (16, 32, 48, 64, 128, 256):
        draw_bin(size, full=False).save(os.path.join(OUT, f"emptybin-{size}.png"))
        draw_bin(size, full=True).save(os.path.join(OUT, f"recyclebin-{size}.png"))
    print("wrote recycle icons to", os.path.normpath(OUT))


if __name__ == "__main__":
    main()