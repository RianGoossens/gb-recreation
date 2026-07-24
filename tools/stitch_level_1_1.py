# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Stitch World 1-1's tilemap past the initially-loaded opening screen.

The background tilemap at 0x9800 is a 32-column ring buffer. At spawn it
already holds real data a few columns past the visible 20x18 screen (the
game preloads ahead of Mario's position), and it streams more in as he
walks right, reusing each buffer column roughly every 32 tiles of world
distance. There is no reliable way to compute which world column a given
buffer slot currently holds from a position estimate alone (tried and
discarded: dead-reckoning the estimate and picking the closest or the
next wraparound both mis-happened near lap boundaries, since preload
timing does not line up neatly with any fixed margin).

So this script does not guess. Every frame, it directly watches whether
each of the 32 buffer columns' contents changed since the previous frame.
The buffer only ever streams forward (never rewritten with older data),
so a slot's world-column identity starts at its raw buffer index (true at
spawn, confirmed against the static opening-screen tilemap) and increases
by exactly 32 every time that slot's value changes. This needs no scroll
register and no position estimate at all for correctness; dead reckoning
is used only to report a human-readable progress figure and to detect the
death/respawn below.

A naive "hold Right forever" script dies to a hazard around world column
48 (see docs/reference/level-1-1.md) and respawns at the level start,
which would otherwise reset the buffer back to its spawn contents and
corrupt the slot-tracking above (the tracker would see the reload as more
forward streaming and keep incrementing). This script watches for the
respawn directly (Mario's screen X snapping back near its spawn value
after the camera lock has engaged) and stops capturing there.

To get past that hazard at all, this walks in a "hold, then briefly let
go" rhythm (WALK_FRAMES held, then RELEASE_FRAMES released, repeating)
instead of holding Right the whole time. A jump-timing sweep against a
save state right before the hazard found that no jump height or timing
cleared it while approaching at full running speed, but simply slowing
down first (releasing Right for at least ~50 frames, no jump needed at
all) did survive it. That points to the hazard being a moving enemy
Mario needs to arrive past a different moment for, not an obstacle that
needs clearing, so periodically breaking stride is a reasonable general
survival heuristic pending a script that reacts to hazards properly.

Run: uv run tools/stitch_level_1_1.py
"""

import struct
import sys
from pathlib import Path

from sml_boot import boot_to_gameplay

OUT = Path("assets/extracted")
MAP_BASE = 0x9800
# Rows 0-1 are the status bar (score/coins/time), redrawn every frame
# regardless of scroll; they are not level geometry, so they are excluded.
HUD_ROWS = 2
ROWS = 18
COLS = 32
WALK_FRAMES = 40
RELEASE_FRAMES = 100


def main():
    pb = boot_to_gameplay()

    x_spawn = pb.memory[0xC202]

    def read_row(row):
        return [pb.memory[MAP_BASE + row * 32 + bx] for bx in range(COLS)]

    # Trusted starting point: at spawn (before any movement) buffer index
    # bx holds world column bx exactly, already confirmed against the
    # static opening-screen tilemap.
    slot_wc = [[bx for bx in range(COLS)] for _ in range(ROWS)]
    slot_val = [read_row(row) for row in range(ROWS)]
    combined = {}
    for row in range(HUD_ROWS, ROWS):
        for bx in range(COLS):
            combined[(slot_wc[row][bx], row)] = slot_val[row][bx]

    pb.button_press("right")
    right_held = True
    cycle_frame = 0
    locked_at = None
    stopped_at_frame = None
    max_frames = 20000

    for f in range(1, max_frames + 1):
        cycle_frame += 1
        if right_held and cycle_frame >= WALK_FRAMES:
            pb.button_release("right")
            right_held = False
            cycle_frame = 0
        elif not right_held and cycle_frame >= RELEASE_FRAMES:
            pb.button_press("right")
            right_held = True
            cycle_frame = 0

        pb.tick()
        x = pb.memory[0xC202]

        if locked_at is None and x == 81 and f > 10:
            locked_at = f

        if locked_at is not None and f > locked_at and x != 81:
            stopped_at_frame = f
            break

        for row in range(HUD_ROWS, ROWS):
            new_vals = read_row(row)
            for bx in range(COLS):
                if new_vals[bx] != slot_val[row][bx]:
                    slot_wc[row][bx] += COLS
                    slot_val[row][bx] = new_vals[bx]
                    combined[(slot_wc[row][bx], row)] = new_vals[bx]

    world_x_reached = x_spawn if locked_at is None else 81 + ((stopped_at_frame or max_frames) - locked_at)
    pb.button_release("right")
    pb.stop()

    print(f"spawn x={x_spawn}, camera lock engaged at frame {locked_at}")
    if stopped_at_frame is not None:
        print(
            f"stopped at frame {stopped_at_frame}: screen X left 81 after the "
            f"camera lock, treated as a death/respawn, not a real further scroll"
        )
    print(f"safely captured up to world column ~{world_x_reached // 8}")

    max_col = max(k[0] for k in combined)
    min_col = min(k[0] for k in combined)
    width = max_col - min_col + 1

    OUT.mkdir(parents=True, exist_ok=True)
    cells = bytearray(width * ROWS)
    for row in range(ROWS):
        for c in range(width):
            cells[row * width + c] = combined.get((min_col + c, row), 44)

    map_blob = b"SMLM" + bytes([1]) + struct.pack("<HH", width, ROWS) + bytes(cells)
    out_path = OUT / "level_1_1_stitched_partial.tmap"
    out_path.write_bytes(map_blob)
    print(f"wrote {out_path} ({width}x{ROWS}, world columns {min_col}..{max_col})")


if __name__ == "__main__":
    sys.exit(main())
