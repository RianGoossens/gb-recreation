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

To get past a hazard, this reads OAM every frame: whenever a non-Mario
sprite is within DANGER_RADIUS pixels of Mario's on-screen X, it releases
Right and jumps (a stomp attempt, cooldown-limited). Mario's own sprite
always occupies OAM slots 3-6 (confirmed across every OAM dump this
session), so those are excluded when scanning for danger. Just releasing
Right and waiting was tried first and does not work past the first
hazard or so: an enemy that is already within range keeps closing in
even while Mario stands still, so waiting only delays contact. Jumping
at it instead let a run survive past world column 1880 without dying
once, over a 15000-frame run.

That survival distance outran what the tile-tracking used to be able to
confirm: watching only for a slot's value to change stalls silently
across long uniform terrain (a long flat stretch of repeated ground
tile, for example), where nothing ever looks different lap to lap.
Fixed by tracking Mario's true world position precisely instead of
periodically: 0xC20C (his horizontal speed, in 1/6-pixel units, see
physics.md) is integrated every frame into a running subpixel counter.
When a slot's tile value does change, its new world column is picked as
the nearest multiple of 32 (relative to the slot's buffer index) to that
precise position estimate, not a blind "+32 from whatever it was
labeled before". That closes the mislabeling risk a blind increment
would have if a long unchanging stretch skipped several real laps
before the next detected change: the new value gets the correct lap
number outright, from position, not from a possibly stale label.

0xC20C is only trustworthy for this while Mario is grounded (0xC20A ==
1). Checked directly: it does not hold horizontal speed at all while
airborne, it climbs unboundedly, one unit per frame, well past the
walking cap of 6, for as long as a jump lasts, some other counter
reusing the address mid-flight. Integrating it regardless of grounded
state was tried first and badly inflated the position estimate, since
this tool's own stomp-reaction jumps often; caught by a sanity check
against real screenshots showing almost no visual change over a span
the buggy estimate claimed covered hundreds of tiles. Fixed by only
reading 0xC20C while grounded and, while airborne, continuing to add
whatever speed was last read on the ground (horizontal motion is not
affected by jumping, confirmed both in physics.md and by on-screen X
advancing at a steady 1 px/frame through a jump when already at max
speed beforehand).

The blind spot on totally uniform terrain is now also addressed, within
a stated confidence level rather than by direct observation alone.
Measured how far ahead of Mario's precise position a freshly detected
transition's world column typically sits during ongoing (not initial)
streaming: 0 to about 17 tiles across 84 sampled transitions, well under
a full 32-tile lap. So if a slot has gone more than a full lap
(STALE_LAPS) without a detected change, the buffer would almost
certainly have refreshed it by now if its content were going to differ;
its current bytes are inferred to be a genuine repeat, and its label is
advanced to the current lap without waiting for a value change. This is
an inference from a measured margin, not a direct observation the way a
detected change is, and is reported separately in the output so it is
never confused with directly-confirmed data.

Run: uv run tools/stitch_level_1_1.py
"""

import struct
import sys
from pathlib import Path

from sml_boot import boot_to_gameplay

OUT = Path("assets/extracted")
MAP_BASE = 0x9800
OAM_BASE = 0xFE00
MARIO_OAM_SLOTS = {3, 4, 5, 6}
DANGER_RADIUS = 90
STOMP_COOLDOWN = 30
JUMP_HOLD = 10
STALE_LAPS = 1  # a full 32-tile lap of no detected change before inferring a repeat
# Rows 0-1 are the status bar (score/coins/time), redrawn every frame
# regardless of scroll; they are not level geometry, so they are excluded.
HUD_ROWS = 2
ROWS = 18
COLS = 32


def nearby_danger(pb, mario_x):
    for i in range(40):
        if i in MARIO_OAM_SLOTS:
            continue
        base = OAM_BASE + i * 4
        y, x = pb.memory[base], pb.memory[base + 1]
        if y == 0 or y >= 160:
            continue
        if abs(x - mario_x) <= DANGER_RADIUS:
            return True
    return False


def nearest_world_col(bx, position_estimate, min_wc):
    """Nearest multiple-of-32 (relative to bx) to a precise position
    estimate, used only when a real value change was just observed.
    Never returns less than min_wc: the level cannot have negative
    columns, and a slot's world column cannot legitimately regress once
    established, so out-of-range candidates are rejected in favor of the
    next viable lap rather than picked for being numerically closer."""
    k = round((position_estimate - bx) / COLS)
    wc = bx + COLS * k
    while wc < min_wc:
        k += 1
        wc = bx + COLS * k
    return wc


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

    pb.button_press("right")
    right_held = True
    locked_at = None
    stopped_at_frame = None
    max_frames = 5000
    jump_cooldown = 0
    a_held = 0
    # Precise world position, integrated every frame from the real
    # horizontal speed register (1/6-pixel units, see physics.md). Only
    # trustworthy while grounded: 0xC20C does not hold horizontal speed
    # while airborne at all (confirmed directly: it climbs unboundedly,
    # 1 unit/frame, well past the walking cap of 6, for as long as a jump
    # lasts, some other counter reusing the address mid-flight). While
    # airborne this instead keeps adding whatever speed was last read on
    # the ground, since horizontal motion is not affected by jumping
    # (established in physics.md; also directly observed: on-screen X
    # advances at a steady 1 px/frame through a jump when already at max
    # speed beforehand). Blindly integrating 0xC20C regardless of
    # grounded state was tried first and produced a wildly inflated
    # position during this tool's own frequent stomp-jumps, caught by a
    # sanity check against real screenshots showing almost no visual
    # change over a span this claimed covered hundreds of tiles.
    world_x_subpixel = x_spawn * 6
    last_grounded_speed = 0

    for f in range(1, max_frames + 1):
        danger = nearby_danger(pb, pb.memory[0xC202])
        grounded = pb.memory[0xC20A]
        if danger:
            if right_held:
                pb.button_release("right")
                right_held = False
            if grounded and jump_cooldown <= 0:
                pb.button_press("a")
                a_held = JUMP_HOLD
                jump_cooldown = STOMP_COOLDOWN
        elif not right_held:
            pb.button_press("right")
            right_held = True
        if a_held > 0:
            a_held -= 1
            if a_held == 0:
                pb.button_release("a")
        if jump_cooldown > 0:
            jump_cooldown -= 1

        pb.tick()
        x = pb.memory[0xC202]
        grounded = pb.memory[0xC20A]
        if grounded:
            last_grounded_speed = pb.memory[0xC20C]
        world_x_subpixel += last_grounded_speed
        position_estimate = world_x_subpixel // 6 // 8  # -> world column

        if locked_at is None and x == 81 and f > 10:
            locked_at = f

        if locked_at is not None and f > locked_at and x != 81:
            stopped_at_frame = f
            break

        for row in range(HUD_ROWS, ROWS):
            new_vals = read_row(row)
            for bx in range(COLS):
                if new_vals[bx] != slot_val[row][bx]:
                    slot_wc[row][bx] = nearest_world_col(
                        bx, position_estimate, slot_wc[row][bx] + 1
                    )
                    slot_val[row][bx] = new_vals[bx]
                    combined[(slot_wc[row][bx], row)] = new_vals[bx]
                    inferred.discard((slot_wc[row][bx], row))
                elif position_estimate - slot_wc[row][bx] > STALE_LAPS * COLS:
                    # No detected change in over a full lap: the buffer
                    # would almost certainly have refreshed this slot by
                    # now if its content were going to differ (measured
                    # margin: 0-17 tiles during ongoing streaming, well
                    # under one lap). Infer a genuine repeat rather than
                    # leave it stuck; marked as inferred, not observed.
                    slot_wc[row][bx] = nearest_world_col(
                        bx, position_estimate, slot_wc[row][bx] + 1
                    )
                    combined[(slot_wc[row][bx], row)] = slot_val[row][bx]
                    inferred.add((slot_wc[row][bx], row))

    world_x_reached = world_x_subpixel // 6
    pb.button_release("right")
    pb.stop()

    print(f"spawn x={x_spawn}, camera lock engaged at frame {locked_at}")
    if stopped_at_frame is not None:
        print(
            f"stopped at frame {stopped_at_frame}: screen X left 81 after the "
            f"camera lock, treated as a death/respawn, not a real further scroll"
        )
    print(f"safely captured up to world column ~{world_x_reached // 8}")
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
