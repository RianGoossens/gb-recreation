# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Draw each object kind's sprite, so the kind bytes can be given names.

The object list says where and what, and "what" is a number. Turning those
numbers into enemies needs a look at them, and the game will show them: an
object in a slot has a screen position, and the sprites drawn at that position
are its own.

So this walks World 1, and whenever a slot holds a kind not seen before and
sits on screen, it collects the OAM entries near the slot's position, pulls
their tile data out of video RAM, and lays them out in the arrangement the
game drew them in. The result is one PNG per run with every kind on it.

Mario is excluded by position: he is pinned at the fly height, and the sprites
that make him up are the ones nearest his own screen X.

Usage: uv run tools/capture_object_sprites.py [out.png]
"""

import struct
import sys
import zlib

sys.path.insert(0, "tools")

from dump_object_slots import EMPTY, SLOTS, SLOT_BASE, SLOT_SIZE, slot
from run_through_levels import (
    BONUS_FRAMES,
    Capture,
    FLY_Y,
    LEVEL_GAP_FRAMES,
    LIVES,
    MARIO_Y,
    PHASE,
    RELEASE_FRAMES,
    SAMPLE_EVERY,
    SCREEN_X,
    SPAWN_X,
    read_column,
)
from sml_boot import boot_to_gameplay
from sml_level import VISIBLE_COLUMNS, known_screens

OAM = 0xFE00
SPRITES = 40
SPRITE_TILES = 0x8000
LCDC = 0xFF40
SHADES = [0xFF, 0xAA, 0x55, 0x00]

NEAR = 9
FRAMES = 16000
CELL = 8


def sprite_tile(pb, index):
    base = SPRITE_TILES + index * 16
    rows = []
    for y in range(8):
        lo = pb.memory[base + y * 2]
        hi = pb.memory[base + y * 2 + 1]
        rows.append([((hi >> (7 - x)) & 1) * 2 | ((lo >> (7 - x)) & 1) for x in range(8)])
    return rows


def sprites_near(pb, x, y, tall):
    """OAM entries whose position sits within NEAR pixels of (x, y)."""
    out = []
    for i in range(SPRITES):
        sy = pb.memory[OAM + i * 4]
        sx = pb.memory[OAM + i * 4 + 1]
        tile = pb.memory[OAM + i * 4 + 2]
        if sy == 0 or sx == 0:
            continue
        if abs(sx - x) <= NEAR and abs(sy - y) <= NEAR:
            out.append((sx, sy, tile & 0xFE if tall else tile))
    return out


def render(pb, entries, tall):
    """Lay the sprites out the way the game placed them, as a pixel grid."""
    xs = sorted({e[0] for e in entries})
    ys = sorted({e[1] for e in entries})
    w = len(xs) * CELL
    h = len(ys) * CELL * (2 if tall else 1)
    pixels = [[0] * w for _ in range(h)]
    for sx, sy, tile in entries:
        cx = xs.index(sx) * CELL
        cy = ys.index(sy) * CELL * (2 if tall else 1)
        for part in range(2 if tall else 1):
            rows = sprite_tile(pb, tile + part)
            for r, row in enumerate(rows):
                for c, v in enumerate(row):
                    ry = cy + part * CELL + r
                    if ry < h and cx + c < w:
                        pixels[ry][cx + c] = v
    return pixels


def write_png(path, rows):
    h = len(rows)
    w = len(rows[0]) if h else 0
    raw = b"".join(b"\x00" + bytes(SHADES[v] for v in row) for row in rows)

    def chunk(kind, data):
        body = kind + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body))

    header = struct.pack(">IIBBBBB", w, h, 8, 0, 0, 0, 0)
    with open(path, "wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n")
        f.write(chunk(b"IHDR", header))
        f.write(chunk(b"IDAT", zlib.compress(raw)))
        f.write(chunk(b"IEND", b""))


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else "assets/extracted/object_kinds.png"
    screens = known_screens()
    pb = boot_to_gameplay()
    capture = Capture(pb, 0)
    playing = True
    quiet = 0
    blank = 0
    seen = {}
    best = {}

    pb.button_press("right")
    for frame in range(FRAMES):
        if playing:
            pb.memory[MARIO_Y] = FLY_Y
            if pb.memory[SCREEN_X] > SPAWN_X:
                pb.memory[PHASE] = 0
        pb.tick()
        if pb.memory[LIVES] == 0:
            break

        if playing:
            quiet = 0 if capture.step(pb, frame) else quiet + 1
            tall = pb.memory[LCDC] & 0x04 != 0
            for s in range(SLOTS):
                state = slot(pb, s)
                kind = state[0]
                if kind == EMPTY:
                    continue
                sx, sy = state[3], state[2]
                # Well inside the screen on both axes. An object caught at the
                # edge is drawn clipped, and half a sprite names nothing.
                if not (40 <= sx <= 140 and 24 <= sy <= 140):
                    continue
                # Two objects standing on top of each other would each collect
                # the other's sprites, and both would be labelled wrong.
                if any(
                    o != s
                    and slot(pb, o)[0] != EMPTY
                    and abs(slot(pb, o)[3] - sx) < 2 * NEAR
                    and abs(slot(pb, o)[2] - sy) < 2 * NEAR
                    for o in range(SLOTS)
                ):
                    continue
                entries = sprites_near(pb, sx, sy, tall)
                # Keep the fullest view of each kind rather than the first.
                if len(entries) <= best.get(kind, 0):
                    continue
                best[kind] = len(entries)
                seen[kind] = render(pb, entries, tall)
                print(f"frame {frame:5d} slot {s} kind 0x{kind:02X} "
                      f"at ({sx}, {sy}): {len(entries)} sprites, "
                      f"tiles {sorted({e[2] for e in entries})}")
            if capture.columns and quiet > LEVEL_GAP_FRAMES:
                playing = False
                blank = 0
            continue

        if frame % 40 == 0:
            pb.button_press("a")
        elif frame % 40 == 12:
            pb.button_release("a")
        if frame % SAMPLE_EVERY:
            continue
        if not screens.get(repr([read_column(pb, r) for r in range(VISIBLE_COLUMNS)])):
            blank += SAMPLE_EVERY
            continue
        if blank < BONUS_FRAMES:
            blank = 0
            continue
        pb.button_release("right")
        for _ in range(RELEASE_FRAMES):
            pb.tick()
        pb.button_press("right")
        capture = Capture(pb, frame)
        playing = True
        quiet = 0
    pb.button_release("right")
    pb.stop()

    if not seen:
        print("no object sprites captured")
        return 1
    pad = 4
    width = max(len(rows[0]) for rows in seen.values()) + pad
    height = max(len(rows) for rows in seen.values()) + pad
    sheet = [[0] * (width * len(seen)) for _ in range(height)]
    for i, kind in enumerate(sorted(seen)):
        for r, row in enumerate(seen[kind]):
            for c, v in enumerate(row):
                sheet[r][i * width + c] = v
    write_png(out, sheet)
    print(f"\nwrote {out}: kinds " + " ".join(f"0x{k:02X}" for k in sorted(seen)))
    return 0


if __name__ == "__main__":
    sys.exit(main())
