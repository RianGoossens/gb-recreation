# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy", "numpy"]
# ///
"""How wide is the part of Mario a ceiling stops?

`probe_corridor_height.py` measured what the terrain stops when he walks: the
bottom 8 pixels of him, not his 12. This is the same question for the other
axis. Our engine caps a jump when any of his 11 pixel width is under a solid
tile, which nobody measured, and World 1-3's opening is where that decides
something: he is under a block by a single column there and cannot get out.

`probe_ceiling_cap.py` swept a jump across World 1-3's own block and got a
shape nothing fits: capped from 6 pixels inside the block to 10 pixels past
its far end, alternating between two cap heights on an 8 pixel cadence. That
block is five tiles wide with terrain around it, so several things could be
producing that at once.

This writes a ceiling of a chosen width into World 1-1's own tilemap instead,
over flat ground with open sky around it, and sweeps a jump under it a pixel
at a time. The window where the jump is capped is `ceiling + head - 1`, which
is how the lift's 24 pixel surface and 6 pixel foot came out of a 29 pixel
window.

Two controls. Open sky either side of the window has to give the free rise, or
the sweep is too narrow to hold the window and its edges mean nothing. And a
ceiling of two different widths has to move the window by the difference, or
what is being measured is not a width at all.

Each direction is swept separately, because the cap in World 1-3 depended on
which one was held: 12 pixels standing and pressing left, 33 pressing right,
at the same spot under the same tile.

Usage: uv run tools/measure_head_width.py [ceiling-row] [tiles]
"""

import sys

sys.path.insert(0, "tools")

from run_through_levels import LIVES, MARIO_Y, PHASE, SCREEN_X
from sml_boot import boot_to_gameplay, restore, snapshot

TILEMAP = 0x9800
HUD_ROWS = 2
FLOOR_ROW = 14
# Where the ceiling sits, in level rows. Row 9 leaves 5 free rows over the
# floor, which is more than a standing jump and less than a moving one, so a
# capped jump and a free one are far apart.
CEILING_ROW = 9
RUN_UP = 40
AIRBORNE = 70
HOLD = 20


def write_ceiling(pb, column, tiles, row, tile):
    for c in range(column, column + tiles):
        pb.memory[TILEMAP + (row + HUD_ROWS) * 32 + (c % 32)] = tile


def jump(pb, state, x, direction, ceiling, tiles, row, tile):
    """One jump from screen x with x pinned. Returns the rise in pixels."""
    restore(pb, state)
    if ceiling is not None:
        write_ceiling(pb, ceiling, tiles, row, tile)
    pb.memory[SCREEN_X] = x
    pb.memory[PHASE] = 0
    start = pb.memory[MARIO_Y]
    if direction:
        pb.button_press(direction)
    for _ in range(RUN_UP):
        pb.memory[SCREEN_X] = x
        pb.tick()
    highest = pb.memory[MARIO_Y]
    pb.button_press("a")
    for frame in range(AIRBORNE):
        pb.memory[SCREEN_X] = x
        if frame == HOLD:
            pb.button_release("a")
        pb.tick()
        highest = min(highest, pb.memory[MARIO_Y])
    pb.button_release("a")
    if direction:
        pb.button_release(direction)
    return start - highest


def sweep(pb, state, ceiling, tiles, row, tile, direction):
    """Rises across the ceiling, and the window where they are cut short."""
    left = ceiling * 8 - 20
    rises = [
        (x, jump(pb, state, x, direction, ceiling, tiles, row, tile))
        for x in range(left, ceiling * 8 + tiles * 8 + 20)
    ]
    free = max(r for _, r in rises)
    capped = [x for x, r in rises if r < free]
    label = direction or "still"
    if not capped:
        print(f"  {label:5}: nothing capped the jump, free rise {free}")
        return None
    if rises[0][1] != free or rises[-1][1] != free:
        print(f"  {label:5}: the sweep does not start and end in free air, "
              f"so its edges say nothing ({rises[0][1]} .. {rises[-1][1]})")
        return None
    window = capped[-1] - capped[0] + 1
    holes = window != len(capped)
    print(f"  {label:5}: free {free}, capped from x {capped[0]} to "
          f"{capped[-1]}, a window of {window}"
          + (" with holes in it" if holes else "")
          + f", so a head of {window - tiles * 8 + 1}")
    return window


def main():
    row = int(sys.argv[1]) if len(sys.argv) > 1 else CEILING_ROW
    widths = [int(t) for t in (sys.argv[2] if len(sys.argv) > 2
                               else "1,3").split(",")]

    pb = boot_to_gameplay()
    for _ in range(60):
        pb.tick()
    tile = pb.memory[TILEMAP + (FLOOR_ROW + HUD_ROWS) * 32 + 4]
    here = snapshot(pb)
    print(f"floor tile 0x{tile:02X}, ceiling at level row {row}")

    # The control that says a written ceiling caps anything at all.
    free = jump(pb, here, 60, None, None, 0, row, tile)
    print(f"  free jump with nothing written: {free} px")

    for tiles in widths:
        print(f"a ceiling {tiles} tile(s) wide at column 10:")
        for direction in (None, "left", "right"):
            sweep(pb, here, 10, tiles, row, tile, direction)
    pb.stop()
    return 0


if __name__ == "__main__":
    sys.exit(main())
