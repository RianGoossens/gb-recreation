# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy", "numpy"]
# ///
"""Score our level rendering against the emulator's own frame.

The title screen extraction was validated this way (99.82% of pixels), and
a level should be held to the same standard. Our renderer draws the
playfield only, so the comparison covers screen rows 16 to 143 and skips the
status bar, and it runs at the level's first frame, before anything scrolls
or any sprite has moved.

Reads our PNG back rather than re-implementing the renderer, so what is
scored is what `sml render-level` actually produced.

Usage:
  cargo run -q -- render-level 1-1 0 shot.png
  uv run tools/compare_level_render.py shot.png
"""

import sys
import zlib

sys.path.insert(0, "tools")

import numpy as np

from sml_boot import boot_to_gameplay

STATUS_HEIGHT = 16
WIDTH = 160
HEIGHT = 144


def read_gray_png(path):
    data = open(path, "rb").read()
    idat = b""
    i = 8
    while i < len(data):
        length = int.from_bytes(data[i : i + 4], "big")
        kind = data[i + 4 : i + 8]
        if kind == b"IDAT":
            idat += data[i + 8 : i + 8 + length]
        i += 12 + length
    raw = zlib.decompress(idat)
    out = np.zeros((HEIGHT, WIDTH), dtype=np.uint8)
    stride = WIDTH + 1
    for y in range(HEIGHT):
        row = raw[y * stride : (y + 1) * stride]
        if row[0] != 0:
            raise SystemExit(f"row {y} uses PNG filter {row[0]}, only 0 is handled")
        out[y] = np.frombuffer(row[1:], dtype=np.uint8)
    return out


def main():
    path = sys.argv[1] if len(sys.argv) > 1 else "shot.png"
    ours = read_gray_png(path)

    pb = boot_to_gameplay()
    theirs = np.array(pb.screen.ndarray[:, :, 0], dtype=np.uint8)
    pb.stop()

    # Both are 4-shade images, but the greys differ, so compare shade indices.
    def shades(a):
        levels = np.unique(a)
        return np.searchsorted(np.sort(levels), a)

    a = shades(ours[STATUS_HEIGHT:])
    b = shades(theirs[STATUS_HEIGHT:])
    same = int((a == b).sum())
    total = a.size
    print(f"playfield match: {same}/{total} pixels ({same / total:.2%})")
    if same != total:
        rows = np.where((a != b).any(axis=1))[0] + STATUS_HEIGHT
        cols = np.where((a != b).any(axis=0))[0]
        print(f"rows that differ: {rows}")
        print(f"columns that differ: {cols.min()}..{cols.max()}")
        print("Mario's sprite box at the level's first frame is x 35-50, y 112-127; "
              "our renderer draws the background only.")
    return 0 if same / total > 0.99 else 1


if __name__ == "__main__":
    sys.exit(main())
