#!/usr/bin/env python3
"""Generate the pixel-art Light Mode / Dark Mode thumbnails for Settings.

Each one is a monitor as a video game would draw it, showing the app in that
mode: bezel and stand, title bar, sidebar, editor, terminal, status bar, with
scanlines and a glass glare. The picture is painted at high resolution, then
pixelated the modern way: box-downsampled to a 66 x 42 grid, quantized to a
72-color palette without dither so blocks stay flat, and scaled back up with
nearest-neighbour so every block is crisp. Output is 264 x 168 (4 px blocks), drawn by the app at
132 x 84 points so a block is 2 points wide on a Retina display.

    python3 scripts/theme_thumbnails.py

Writes assets/theme-light.png and assets/theme-dark.png. Requires Pillow.
"""

from __future__ import annotations

import math
import random
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

GRID_W, GRID_H = 66, 42
SUPER = 16  # hi-res pixels per block
W, H = GRID_W * SUPER, GRID_H * SUPER
BLOCK = 4  # output pixels per block
OUT = Path(__file__).resolve().parent.parent / "assets"


def lerp(a, b, t):
    return tuple(int(round(a[i] + (b[i] - a[i]) * t)) for i in range(3))


def vgradient(img, box, top, bottom):
    x0, y0, x1, y1 = box
    draw = ImageDraw.Draw(img)
    for y in range(y0, y1):
        t = (y - y0) / max(1, (y1 - y0 - 1))
        draw.line([(x0, y), (x1, y)], fill=lerp(top, bottom, t))


def glow(img, center, radius, color, strength):
    """Additive radial glow composited over the image."""
    layer = Image.new("RGB", img.size, (0, 0, 0))
    draw = ImageDraw.Draw(layer)
    cx, cy = center
    steps = 24
    for i in range(steps, 0, -1):
        r = radius * i / steps
        a = strength * (1 - i / steps) ** 2
        draw.ellipse([cx - r, cy - r, cx + r, cy + r], fill=lerp((0, 0, 0), color, a))
    layer = layer.filter(ImageFilter.GaussianBlur(radius / 6))
    return _screen(img, layer)


def _screen(base, layer):
    b = base.load()
    l = layer.load()
    out = Image.new("RGB", base.size)
    o = out.load()
    for y in range(base.size[1]):
        for x in range(base.size[0]):
            br, bg, bb = b[x, y]
            lr, lg, lb = l[x, y]
            o[x, y] = (
                255 - (255 - br) * (255 - lr) // 255,
                255 - (255 - bg) * (255 - lg) // 255,
                255 - (255 - bb) * (255 - lb) // 255,
            )
    return out


def rounded(draw, box, radius, fill):
    draw.rounded_rectangle(box, radius=radius, fill=fill)


def scene(night: bool) -> Image.Image:
    """A monitor as a game would draw it: bezel, glowing screen, the app on it."""
    img = Image.new("RGB", (W, H), (0, 0, 0))
    d = ImageDraw.Draw(img)
    # Card fill behind the monitor. Tinted to the mode so the bezel's edge
    # reads as a silhouette rather than a hard rectangle.
    vgradient(img, (0, 0, W, H), (16, 18, 26) if night else (226, 228, 232), (8, 9, 14) if night else (206, 209, 216))
    d = ImageDraw.Draw(img)

    # --- bezel ---------------------------------------------------------------
    bx0, by0, bx1, by1 = SUPER * 3, SUPER * 2, W - SUPER * 3, H - SUPER * 7
    bezel = (38, 40, 48) if night else (58, 60, 68)
    bezel_hi = (58, 60, 70) if night else (84, 86, 96)
    rounded(d, (bx0, by0, bx1, by1), SUPER * 2, bezel)
    rounded(d, (bx0, by0, bx1, by0 + SUPER * 2), SUPER * 2, bezel_hi)
    d.rectangle([bx0, by0 + SUPER, bx1, by0 + SUPER * 2], fill=bezel)
    # stand
    sx = W // 2
    d.rectangle([sx - SUPER * 3, by1, sx + SUPER * 3, by1 + SUPER * 3], fill=bezel)
    rounded(d, (sx - SUPER * 12, by1 + SUPER * 3, sx + SUPER * 12, by1 + SUPER * 5), SUPER, bezel_hi)
    # power light
    d.ellipse([bx1 - SUPER * 4, by1 - SUPER * 1.3, bx1 - SUPER * 3.2, by1 - SUPER * 0.5], fill=(120, 255, 160) if night else (60, 200, 110))

    # --- screen --------------------------------------------------------------
    sx0, sy0, sx1, sy1 = bx0 + SUPER * 2, by0 + SUPER * 2, bx1 - SUPER * 2, by1 - SUPER * 2
    ui = {
        "chrome": (36, 36, 36), "panel": (27, 27, 34), "editor": (25, 25, 32), "terminal": (20, 20, 26),
        "border": (52, 54, 62), "text": (171, 178, 191), "muted": (110, 116, 130), "accent": (106, 184, 255),
        "sel": (48, 52, 64), "prompt": (106, 255, 144),
        "code": [(198, 120, 221), (97, 175, 239), (152, 195, 121), (229, 192, 123), (224, 108, 117), (171, 178, 191)],
    } if night else {
        "chrome": (248, 248, 248), "panel": (255, 255, 255), "editor": (255, 255, 255), "terminal": (250, 250, 250),
        "border": (222, 224, 228), "text": (59, 59, 59), "muted": (150, 150, 156), "accent": (0, 95, 184),
        "sel": (229, 235, 241), "prompt": (19, 115, 79),
        "code": [(166, 38, 164), (64, 120, 242), (80, 161, 79), (193, 132, 1), (228, 86, 73), (56, 58, 66)],
    }
    d.rectangle([sx0, sy0, sx1, sy1], fill=ui["editor"])

    # title bar with traffic lights and two tabs
    tb = sy0 + SUPER * 2
    d.rectangle([sx0, sy0, sx1, tb], fill=ui["chrome"])
    d.line([(sx0, tb), (sx1, tb)], fill=ui["border"], width=2)
    for i, col in enumerate([(255, 95, 87), (255, 189, 46), (40, 201, 64)]):
        cx = sx0 + SUPER * (1 + i * 1.2)
        d.ellipse([cx, sy0 + SUPER * 0.6, cx + SUPER * 0.8, sy0 + SUPER * 1.4], fill=col)
    d.rectangle([sx0 + SUPER * 6, sy0 + SUPER * 0.4, sx0 + SUPER * 13, tb], fill=ui["editor"])
    d.rectangle([sx0 + SUPER * 7, sy0 + SUPER * 0.9, sx0 + SUPER * 12, sy0 + SUPER * 1.4], fill=ui["accent"])
    d.rectangle([sx0 + SUPER * 14.5, sy0 + SUPER * 0.9, sx0 + SUPER * 19, sy0 + SUPER * 1.4], fill=ui["muted"])

    # sidebar with file rows
    sb1 = sx0 + SUPER * 12
    d.rectangle([sx0, tb, sb1, sy1], fill=ui["panel"])
    d.line([(sb1, tb), (sb1, sy1)], fill=ui["border"], width=2)
    rnd = random.Random(5)
    y = tb + SUPER
    i = 0
    while y + SUPER * 0.8 < sy1 - SUPER * 2:
        indent = SUPER * (1 + (0 if i % 5 == 0 else 1.5))
        if i == 2:
            d.rectangle([sx0 + SUPER * 0.5, y - SUPER * 0.3, sb1 - SUPER * 0.5, y + SUPER * 1.1], fill=ui["sel"])
        col = ui["accent"] if i % 5 == 0 else ui["text"]
        d.rectangle([sx0 + indent, y, sx0 + indent + SUPER * rnd.uniform(3, 7), y + SUPER * 0.8], fill=col)
        y += SUPER * 1.6
        i += 1

    # editor: gutter numbers and code lines
    ex0 = sb1 + SUPER
    split = sy0 + int((sy1 - sy0) * 0.62)
    rnd = random.Random(21)
    y = tb + SUPER
    line = 0
    while y + SUPER * 0.8 < split - SUPER * 0.6:
        d.rectangle([ex0, y, ex0 + SUPER * 1.2, y + SUPER * 0.8], fill=ui["muted"])
        x = ex0 + SUPER * 2.6 + SUPER * rnd.choice([0, 0, 2, 4])
        if line == 3:
            d.rectangle([ex0 - SUPER * 0.6, y - SUPER * 0.3, sx1, y + SUPER * 1.1], fill=ui["sel"])
        for _ in range(rnd.randint(2, 5)):
            w = SUPER * rnd.uniform(1.6, 7)
            if x + w > sx1 - SUPER:
                break
            d.rectangle([x, y, x + w, y + SUPER * 0.8], fill=rnd.choice(ui["code"]))
            x += w + SUPER * 0.8
        y += SUPER * 1.6
        line += 1

    # terminal pane below the editor
    d.rectangle([sb1 + 2, split, sx1, sy1], fill=ui["terminal"])
    d.line([(sb1, split), (sx1, split)], fill=ui["border"], width=2)
    y = split + SUPER
    widths = [6, 11, 4, 14, 9, 5, 12, 7]
    i = 0
    while y + SUPER * 0.8 < sy1 - SUPER * 2:
        prompt = i % 3 == 0
        if prompt:
            d.rectangle([ex0, y, ex0 + SUPER * 0.9, y + SUPER * 0.8], fill=ui["prompt"])
        w = SUPER * widths[i % len(widths)]
        x = ex0 + (SUPER * 1.6 if prompt else 0)
        d.rectangle([x, y, min(x + w, sx1 - SUPER), y + SUPER * 0.8], fill=ui["text"] if prompt else ui["muted"])
        last = (x + w, y)
        y += SUPER * 1.6
        i += 1
    # block cursor after the last line
    d.rectangle([last[0] + SUPER * 0.6, last[1], last[0] + SUPER * 1.5, last[1] + SUPER * 0.8], fill=ui["accent"])

    # status bar
    d.rectangle([sx0, sy1 - SUPER * 1.6, sx1, sy1], fill=ui["chrome"])
    d.line([(sx0, sy1 - SUPER * 1.6), (sx1, sy1 - SUPER * 1.6)], fill=ui["border"], width=2)
    d.rectangle([sx0 + SUPER * 0.8, sy1 - SUPER * 1.15, sx0 + SUPER * 5, sy1 - SUPER * 0.45], fill=ui["accent"])
    d.rectangle([sx1 - SUPER * 6, sy1 - SUPER * 1.15, sx1 - SUPER * 0.8, sy1 - SUPER * 0.45], fill=ui["muted"])

    # --- the game-screen treatment: scanlines, glass glare, corner shade ---
    px_ = img.load()
    for yy in range(sy0, sy1):
        band = (yy - sy0) // (SUPER // 2)
        if band % 2 == 1:
            for xx in range(sx0, sx1):
                r, g, b = px_[xx, yy]
                px_[xx, yy] = (int(r * 0.93), int(g * 0.93), int(b * 0.94)) if night else (int(r * 0.965), int(g * 0.965), int(b * 0.97))
    glare = Image.new("RGB", img.size, (0, 0, 0))
    gd = ImageDraw.Draw(glare)
    gd.polygon([(sx0, sy0), (sx0 + SUPER * 16, sy0), (sx0, sy0 + SUPER * 9)], fill=(34, 38, 50) if night else (36, 36, 36))
    gd.polygon([(sx0 + SUPER * 19, sy0), (sx0 + SUPER * 23, sy0), (sx0, sy0 + SUPER * 13), (sx0, sy0 + SUPER * 11)], fill=(20, 22, 30) if night else (22, 22, 22))
    glare = glare.filter(ImageFilter.GaussianBlur(SUPER * 0.6))
    img = _screen(img, glare)
    if night:
        img = glow(img, ((sx0 + sx1) / 2, (sy0 + sy1) / 2), SUPER * 26, (60, 90, 140), 0.12)
    # keep the bezel crisp over the glow
    d = ImageDraw.Draw(img)
    d.rectangle([sx0 - SUPER, sy0 - SUPER, sx1 + SUPER, sy0], fill=bezel)
    d.rectangle([sx0 - SUPER, sy1, sx1 + SUPER, sy1 + SUPER], fill=bezel)
    d.rectangle([sx0 - SUPER, sy0, sx0, sy1], fill=bezel)
    d.rectangle([sx1, sy0, sx1 + SUPER, sy1], fill=bezel)
    return img


def pixelate(img: Image.Image, colors: int) -> Image.Image:
    small = img.resize((GRID_W, GRID_H), Image.BOX)
    quant = small.quantize(colors=colors, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE)
    return quant.convert("RGB").resize((GRID_W * BLOCK, GRID_H * BLOCK), Image.NEAREST)


def main():
    OUT.mkdir(exist_ok=True)
    for night, name in [(False, "theme-light.png"), (True, "theme-dark.png")]:
        pixelate(scene(night), 72).save(OUT / name, optimize=True)
        print("wrote", OUT / name)


if __name__ == "__main__":
    main()
