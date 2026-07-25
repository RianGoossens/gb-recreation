# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy", "numpy"]
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

Getting past hazards is `sml_walker.ReactiveWalker`'s job: hold Right, and
jump at enemies that come close and at whatever stops the scroll. See that
module for why an earlier version of it barely moved at all.

That survival distance outran what the tile-tracking used to be able to
confirm: watching only for a slot's value to change stalls silently
across long uniform terrain (a long flat stretch of repeated ground
tile, for example), where nothing ever looks different lap to lap. Two
successive attempts to fix that by estimating Mario's world position
from WRAM were both wrong, and the second was wrong for a long time
before anyone noticed. See `docs/reference/level-1-1.md` for the trail.

Position now comes from `sml_scroll.ScrollTracker`, which measures how far
the background actually moved between two rendered frames. It needs no
model of Mario's speed, it is validated against known physics (exactly
1 px/frame at his saturated walk), and it stops dead when the game does.
That last part matters: the death sequence freezes the level for about 150
frames while Mario's WRAM bytes keep reading as though he were alive and
walking, so a position integrated from those bytes runs on through a death
forever. This stops capturing at the first death instead.

Run: uv run tools/stitch_level_1_1.py
"""

import struct
import sys
from pathlib import Path

from sml_boot import boot_to_gameplay
from sml_scroll import ScrollTracker
from sml_walker import ReactiveWalker

OUT = Path("assets/extracted")
MAP_BASE = 0x9800
MARIO_SCREEN_X = 81  # where the camera lock pins him
FROZEN_FRAMES = 5  # identical frames that mean the level stopped running
STALE_LAPS = 1  # a full 32-tile lap of no detected change before inferring a repeat
# How far ahead of Mario the buffer is ever seen to have streamed content in.
# Measured at 0 to about 17 tiles across 84 sampled transitions.
PRELOAD_MARGIN = 20
# Rows 0-1 are the status bar (score/coins/time), redrawn every frame
# regardless of scroll; they are not level geometry, so they are excluded.
HUD_ROWS = 2
ROWS = 18
COLS = 32


def nearest_world_col(bx, position_estimate, min_wc):
    """Nearest multiple-of-32 (relative to bx) to a precise position
    estimate, used only when a real value change was just observed.

    Bounded on both sides. Never below min_wc: the level cannot have
    negative columns, and a slot's world column cannot legitimately regress
    once established. Never above the position estimate plus PRELOAD_MARGIN
    either: the game streams a slot in shortly before Mario reaches it, so
    a label further ahead than the buffer ever preloads would be a fiction.
    Returns None when no lap satisfies both, meaning the slot should be left
    alone rather than given a number the run cannot support."""
    k = round((position_estimate - bx) / COLS)
    wc = bx + COLS * k
    while wc < min_wc:
        k += 1
        wc = bx + COLS * k
    return wc if wc <= position_estimate + PRELOAD_MARGIN else None


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
    inferred = set()
    for row in range(HUD_ROWS, ROWS):
        for bx in range(COLS):
            combined[(slot_wc[row][bx], row)] = slot_val[row][bx]

    tracker = ScrollTracker(pb)
    walker = ReactiveWalker(pb)
    max_frames = 5000
    died_at = None

    for f in range(1, max_frames + 1):
        walker.step(pb, tracker)
        pb.tick()
        tracker.update(pb)
        if tracker.frozen > FROZEN_FRAMES:
            died_at = f
            break
        # The camera's left edge in world pixels, plus Mario's fixed screen
        # position once locked, is his world position; the buffer preloads a
        # few columns past that.
        position_estimate = (tracker.scroll + MARIO_SCREEN_X) // 8

        for row in range(HUD_ROWS, ROWS):
            new_vals = read_row(row)
            for bx in range(COLS):
                if new_vals[bx] != slot_val[row][bx]:
                    wc = nearest_world_col(
                        bx, position_estimate, slot_wc[row][bx] + 1
                    )
                    slot_val[row][bx] = new_vals[bx]
                    if wc is None:
                        continue
                    slot_wc[row][bx] = wc
                    combined[(wc, row)] = new_vals[bx]
                    inferred.discard((wc, row))
                elif position_estimate - slot_wc[row][bx] > STALE_LAPS * COLS:
                    # No detected change in over a full lap: the buffer
                    # would almost certainly have refreshed this slot by
                    # now if its content were going to differ (measured
                    # margin: 0-17 tiles during ongoing streaming, well
                    # under one lap). Infer a genuine repeat rather than
                    # leave it stuck; marked as inferred, not observed.
                    wc = nearest_world_col(
                        bx, position_estimate, slot_wc[row][bx] + 1
                    )
                    if wc is None:
                        continue
                    slot_wc[row][bx] = wc
                    combined[(wc, row)] = slot_val[row][bx]
                    inferred.add((wc, row))

    pb.stop()

    print(f"spawn x={x_spawn}")
    if died_at is not None:
        print(
            f"stopped at frame {died_at}: the screen went static, which is the "
            f"death sequence. Everything after it would be a reloaded level."
        )
    print(
        f"measured scroll {tracker.scroll}px, so Mario reached world column "
        f"{(tracker.scroll + MARIO_SCREEN_X) // 8}"
    )
    if tracker.ambiguous_frames:
        print(f"{tracker.ambiguous_frames} frames had an ambiguous scroll reading")
    print(
        f"{len(combined) - len(inferred)} cells directly observed, "
        f"{len(inferred)} inferred as repeats (not directly reconfirmed)"
    )

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
