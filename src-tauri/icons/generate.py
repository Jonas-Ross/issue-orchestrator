"""Render src-tauri/icons/icon.png — the 1024x1024 source for `tauri icon`.

Design: dark chrome rounded square (matches sidebar bg) with three stacked
sidebar-row silhouettes; the middle row carries the mint needs-input
treatment (rail + dot + tinted bg). Mirrors the calm-sidebar's "one of
these wants you" idea at icon scale.

Usage:
    pip install Pillow
    python3 src-tauri/icons/generate.py
    npm run tauri -- icon src-tauri/icons/icon.png
"""

from pathlib import Path
from PIL import Image, ImageDraw

SIZE = 1024
RADIUS = 224  # macOS squircle approximation for 1024x1024

# Tokens (mirror src/lib/tokens.css)
BG_CHROME = (19, 20, 24, 255)        # #131418
NEEDS_MINT = (134, 239, 172, 255)    # #86efac
NEEDS_BG = (134, 239, 172, 38)       # ~15% mint over chrome
DOT_RUNNING = (63, 185, 80, 255)     # #3fb950
DOT_IDLE = (93, 142, 255, 255)       # #5d8eff
TITLE_DIM = (138, 143, 153, 200)     # text-2-ish
TITLE_BRIGHT = (230, 232, 238, 255)  # text-1
TITLE_FAINT = (90, 94, 104, 200)     # text-3-ish

ROW_H = 130
ROW_GAP = 60
PAD_X = 170
ROW_RADIUS = 28
DOT_R = 18
DOT_OFFSET_X = 70
TITLE_X_OFFSET = 36
TITLE_BAR_H = 22
RAIL_W = 14


def draw_row(d, y, dot_color, title_w, title_color, *, bg=None, rail=False):
    x0, x1 = PAD_X, SIZE - PAD_X
    if bg is not None:
        d.rounded_rectangle((x0, y, x1, y + ROW_H), radius=ROW_RADIUS, fill=bg)
    if rail:
        d.rectangle((x0, y + 8, x0 + RAIL_W, y + ROW_H - 8), fill=NEEDS_MINT)
    cy = y + ROW_H // 2
    cx = x0 + DOT_OFFSET_X
    d.ellipse((cx - DOT_R, cy - DOT_R, cx + DOT_R, cy + DOT_R), fill=dot_color)
    bar_x = cx + TITLE_X_OFFSET
    bar_y = cy - TITLE_BAR_H // 2
    d.rounded_rectangle(
        (bar_x, bar_y, bar_x + title_w, bar_y + TITLE_BAR_H),
        radius=TITLE_BAR_H // 2,
        fill=title_color,
    )


def main():
    img = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    d.rounded_rectangle((0, 0, SIZE - 1, SIZE - 1), radius=RADIUS, fill=BG_CHROME)

    total_h = 3 * ROW_H + 2 * ROW_GAP
    y0 = (SIZE - total_h) // 2

    draw_row(d, y0, DOT_RUNNING, 460, TITLE_DIM)
    draw_row(d, y0 + ROW_H + ROW_GAP, NEEDS_MINT, 540, TITLE_BRIGHT,
             bg=NEEDS_BG, rail=True)
    draw_row(d, y0 + 2 * (ROW_H + ROW_GAP), DOT_IDLE, 380, TITLE_FAINT)

    out = Path(__file__).resolve().parent / "icon.png"
    img.save(out)
    print(f"wrote {out}")


if __name__ == "__main__":
    main()
