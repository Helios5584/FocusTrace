#!/usr/bin/env python3
"""
Generate FocusTrace app icon (.iconset PNGs) using only the Python stdlib.

Design:
  - Rounded-square blue gradient background
  - Two offset rounded "window" rectangles: back outline + front filled
    (represents focus changes between windows)

Output:
  macos/AppIcon.iconset/icon_*.png  (10 PNGs sized for `iconutil`)

Run via scripts/bundle.sh (or directly: python3 scripts/make_icon.py).
"""
import math
import os
import struct
import sys
import zlib

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ICONSET = os.path.join(ROOT, "macos", "AppIcon.iconset")

SIZES = [
    ("icon_16x16.png", 16),
    ("icon_16x16@2x.png", 32),
    ("icon_32x32.png", 32),
    ("icon_32x32@2x.png", 64),
    ("icon_128x128.png", 128),
    ("icon_128x128@2x.png", 256),
    ("icon_256x256.png", 256),
    ("icon_256x256@2x.png", 512),
    ("icon_512x512.png", 512),
    ("icon_512x512@2x.png", 1024),
]

# Colors (RGB)
BG_TOP = (54, 124, 214)
BG_BOT = (28, 78, 158)
FG = (255, 255, 255)


def write_png(path, w, h, pixels):
    """pixels: bytes of length w*h*4 (RGBA, row-major)."""
    sig = b"\x89PNG\r\n\x1a\n"

    def chunk(t, d):
        crc = zlib.crc32(t + d) & 0xFFFFFFFF
        return struct.pack(">I", len(d)) + t + d + struct.pack(">I", crc)

    ihdr = struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0)
    raw = bytearray()
    stride = w * 4
    for y in range(h):
        raw.append(0)  # filter: None
        raw.extend(pixels[y * stride : (y + 1) * stride])
    idat = zlib.compress(bytes(raw), 9)
    with open(path, "wb") as f:
        f.write(sig + chunk(b"IHDR", ihdr) + chunk(b"IDAT", idat) + chunk(b"IEND", b""))


def smoothstep(edge0, edge1, x):
    t = max(0.0, min(1.0, (x - edge0) / (edge1 - edge0))) if edge1 != edge0 else 1.0
    return t * t * (3.0 - 2.0 * t)


def aa_mask(dist, edge):
    # 1.0 inside, 0.0 outside, smooth across one pixel.
    return 1.0 - smoothstep(edge - 0.75, edge + 0.75, dist)


def rrect_sdf(x, y, cx, cy, hw, hh, r):
    """Signed distance from point (x,y) to rounded-rect edge (negative inside)."""
    qx = abs(x - cx) - hw + r
    qy = abs(y - cy) - hh + r
    qx_c = max(qx, 0.0)
    qy_c = max(qy, 0.0)
    outside = math.sqrt(qx_c * qx_c + qy_c * qy_c)
    inside = min(max(qx, qy), 0.0)
    return outside + inside - r


def render_app(size):
    """Return RGBA bytes for a square `size`-px app icon."""
    s = size
    cx = (s - 1) / 2.0
    cy = (s - 1) / 2.0
    bg_corner = s * 0.225

    # Window geometry (two offset rounded rects).
    win_hw = s * 0.235
    win_hh = s * 0.215
    win_r = s * 0.05
    stroke = max(1.0, s * 0.035)
    title_h = s * 0.05  # darker top strip on front window

    # Back window: top-left, outlined.
    back_cx = cx - s * 0.105
    back_cy = cy - s * 0.115
    # Front window: bottom-right, filled.
    front_cx = cx + s * 0.105
    front_cy = cy + s * 0.115

    # Tint for front-window title bar (slight blue cast over white).
    TITLE = (210, 222, 240)

    out = bytearray(s * s * 4)
    for y in range(s):
        for x in range(s):
            # Background rounded-square.
            bg_sdf = rrect_sdf(x, y, cx, cy, s / 2, s / 2, bg_corner)
            bg_a = aa_mask(bg_sdf, 0.0)
            i = (y * s + x) * 4
            if bg_a <= 0.0:
                out[i + 0] = 0
                out[i + 1] = 0
                out[i + 2] = 0
                out[i + 3] = 0
                continue
            # Vertical gradient base color.
            t = y / max(1, s - 1)
            r = int(BG_TOP[0] * (1 - t) + BG_BOT[0] * t)
            g = int(BG_TOP[1] * (1 - t) + BG_BOT[1] * t)
            b = int(BG_TOP[2] * (1 - t) + BG_BOT[2] * t)

            # --- Back window: outline only.
            d_back = rrect_sdf(x, y, back_cx, back_cy, win_hw, win_hh, win_r)
            # Annulus mask: inside outer edge AND outside (edge - stroke).
            back_cov = aa_mask(d_back, 0.0) * aa_mask(-d_back - stroke, 0.0)

            # --- Front window: filled white with darker title strip.
            d_front = rrect_sdf(x, y, front_cx, front_cy, win_hw, win_hh, win_r)
            front_cov = aa_mask(d_front, 0.0)
            # Title bar: top strip of front window.
            front_top = front_cy - win_hh
            in_title = (y >= front_top) and (y <= front_top + title_h)

            # Front window punches a hole in back outline (so it sits on top).
            if front_cov > 0.0:
                back_cov *= 1.0 - front_cov

            # Compose: gradient -> back outline (white) -> front fill.
            if back_cov > 0.0:
                r = int(r * (1 - back_cov) + FG[0] * back_cov)
                g = int(g * (1 - back_cov) + FG[1] * back_cov)
                b = int(b * (1 - back_cov) + FG[2] * back_cov)
            if front_cov > 0.0:
                fill = TITLE if in_title else FG
                r = int(r * (1 - front_cov) + fill[0] * front_cov)
                g = int(g * (1 - front_cov) + fill[1] * front_cov)
                b = int(b * (1 - front_cov) + fill[2] * front_cov)

            a = int(round(bg_a * 255))
            out[i + 0] = r
            out[i + 1] = g
            out[i + 2] = b
            out[i + 3] = a
    return bytes(out)


def main():
    os.makedirs(ICONSET, exist_ok=True)
    for name, size in SIZES:
        path = os.path.join(ICONSET, name)
        print(f"  rendering {name} ({size}x{size})", file=sys.stderr)
        px = render_app(size)
        write_png(path, size, size, px)
    print(f"==> wrote {ICONSET}", file=sys.stderr)


if __name__ == "__main__":
    main()
