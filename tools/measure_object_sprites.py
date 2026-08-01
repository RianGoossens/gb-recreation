# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Measure which atlas tiles each object kind is drawn from, and where.

`capture_object_sprites.py` drew a picture per kind, which was enough to see
that an enemy is built the same way Mario is (tiles `n`, `n+1`, `n+16`,
`n+17`, a 2x2 block of a sheet 16 tiles wide). It could not say how a kind's
block sits against the position the slot holds, because it collected OAM
entries within 9 pixels of the slot and anything further out was dropped: the
Chibibo came back as one tile and the Nokobon as two.

So this widens the window and reports offsets instead of pictures. For each
kind it prints every sprite the game drew near the slot as `(dx, dy, tile,
attributes)` relative to the slot's own x and y, which gives the block, the
anchor, and the palette the hardware was told to use. Objects with a
neighbour inside the window are skipped, since two overlapping sets would
each be labelled with the other's tiles.

Usage: uv run tools/measure_object_sprites.py [frames]
"""

import sys

sys.path.insert(0, "tools")

from dump_object_slots import EMPTY, SLOTS, slot
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
LCDC = 0xFF40
OBP0 = 0xFF48
OBP1 = 0xFF49

# Wide enough for a 2x2 block however it is anchored, and still under the
# spacing the skip below enforces between two objects.
NEAR = 20
# Mario is 16 px of sprite around his own x, so anything this close to him is
# skipped rather than trusted.
MARIO_CLEAR = 28
FRAMES = 16000


def entries_near(pb, x, y):
    """Every OAM entry within NEAR of (x, y), as offsets from it."""
    out = []
    for i in range(SPRITES):
        sy, sx, tile, attr = (pb.memory[OAM + i * 4 + f] for f in range(4))
        if sy == 0 or sx == 0:
            continue
        if abs(sx - x) <= NEAR and abs(sy - y) <= NEAR:
            out.append((sx - x, sy - y, tile, attr))
    return sorted(out, key=lambda e: (e[1], e[0]))


def main():
    frames = int(sys.argv[1]) if len(sys.argv) > 1 else FRAMES
    screens = known_screens()
    pb = boot_to_gameplay()
    capture = Capture(pb, 0)
    playing = True
    quiet = 0
    blank = 0
    best = {}

    pb.button_press("right")
    for frame in range(frames):
        if playing:
            pb.memory[MARIO_Y] = FLY_Y
            if pb.memory[SCREEN_X] > SPAWN_X:
                pb.memory[PHASE] = 0
        pb.tick()
        if pb.memory[LIVES] == 0:
            break

        if playing:
            quiet = 0 if capture.step(pb, frame) else quiet + 1
            for s in range(SLOTS):
                state = slot(pb, s)
                kind = state[0]
                if kind == EMPTY:
                    continue
                sx, sy = state[3], state[2]
                # A ground walker sits at y 132, so the lower bound has to
                # clear the floor or the two walkers never get measured.
                if not (40 <= sx <= 130 and 30 <= sy <= 140):
                    continue
                # Mario is drawn from the same atlas and would be collected as
                # if he were the object's own tiles, which is what put big
                # Mario's still frame under kind 0x23 the first time.
                if abs(pb.memory[SCREEN_X] - sx) < MARIO_CLEAR:
                    continue
                if any(
                    o != s
                    and slot(pb, o)[0] != EMPTY
                    and abs(slot(pb, o)[3] - sx) < 2 * NEAR
                    and abs(slot(pb, o)[2] - sy) < 2 * NEAR
                    for o in range(SLOTS)
                ):
                    continue
                found = entries_near(pb, sx, sy)
                if len(found) <= len(best.get(kind, ())):
                    continue
                best[kind] = found
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

    tall = pb.memory[LCDC] & 0x04 != 0
    palettes = (pb.memory[OBP0], pb.memory[OBP1])
    pb.button_release("right")
    pb.stop()

    print(f"\n8x16 sprite mode: {tall}")
    print(f"OBP0 0x{palettes[0]:02X}  OBP1 0x{palettes[1]:02X}\n")
    for kind in sorted(best):
        found = best[kind]
        print(f"kind 0x{kind:02X}: {len(found)} sprites")
        for dx, dy, tile, attr in found:
            print(f"    dx {dx:+3d} dy {dy:+3d}  tile 0x{tile:02X}  attr 0x{attr:02X}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
