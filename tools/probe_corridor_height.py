# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy", "numpy"]
# ///
"""How much headroom does Mario need to walk down a corridor?

Our engine gives him a 12 pixel body and tests all of it against the terrain
when he moves sideways, so a corridor with 8 pixels of headroom is a wall. That
stops the World 1-3 walk at column 183: the floor at row 13 runs on to column
200 and a slab at row 11 covers columns 185 to 192, leaving one free row
underneath. Nobody measured whether the cartridge's Mario can walk in there.

Rather than play to column 183, which needs the flatten-and-fly walkthrough
that would erase the corridor on the way, this writes a corridor into World
1-1's own tilemap a few columns in front of him and watches whether he walks
through it. Patching the tilemap is how the walker-turn question was settled,
and the game's collision reads it.

Three runs, and the first two are the controls:

  nothing      no ceiling written, so he has to walk the whole way
  16 px        a ceiling two rows above the floor, which he fits under
  8 px         one row above, which is the corridor in question

Usage: uv run tools/probe_corridor_height.py [gap-rows]
"""

import sys

sys.path.insert(0, "tools")

from run_through_levels import LIVES, SCREEN_X
from sml_boot import boot_to_gameplay, restore, snapshot
from sml_scroll import ScrollTracker

TILEMAP = 0x9800
# Two rows of status bar sit above the playfield, so level row r is tilemap
# row r + 2.
HUD_ROWS = 2
FLOOR_ROW = 14
WALK = 260
# Where the ceiling goes, in columns ahead of where he starts.
FROM, TO = 6, 14


def write_ceiling(pb, start_column, rows, tile):
    for row in rows:
        for column in range(start_column + FROM, start_column + TO):
            pb.memory[TILEMAP + (row + HUD_ROWS) * 32 + column] = tile


def walk(pb, state, rows, tile):
    """Hold right for a while and report how far right he actually got.

    His screen x is no measure of that: once the camera is moving he stays
    around x 81 however far he walks, which is what made the first run of this
    report the control as blocked too. The scroll has to come off the screen,
    since `SCX` reads 0 (the game rewrites the tilemap ring instead of moving
    the background) and nothing in RAM tracks it.
    """
    restore(pb, state)
    start = pb.memory[SCREEN_X]
    if rows:
        write_ceiling(pb, start // 8, rows, tile)
    lives = pb.memory[LIVES]
    tracker = ScrollTracker(pb)
    pb.button_press("right")
    furthest = 0
    for _ in range(WALK):
        pb.tick()
        tracker.update(pb)
        if pb.memory[LIVES] < lives:
            break
        furthest = max(furthest, tracker.scroll + pb.memory[SCREEN_X] - start)
    pb.button_release("right")
    return furthest


def main():
    pb = boot_to_gameplay()
    for _ in range(60):
        pb.tick()
    # The floor's own tile id, so the ceiling is written out of the level's
    # own graphics rather than a guessed number.
    tile = pb.memory[TILEMAP + (FLOOR_ROW + HUD_ROWS) * 32 + 4]
    print(f"floor tile 0x{tile:02X}, Mario at x {pb.memory[SCREEN_X]}")
    here = snapshot(pb)

    for label, rows in (
        ("nothing overhead (control)", ()),
        # The instrument's own control: a wall he cannot be anywhere but
        # stopped by. Without this a run where the writes never landed reads
        # exactly like a run where nothing blocks him.
        ("a solid wall, rows 11 to 13 (control)", (11, 12, 13)),
        ("ceiling at row 11, 16 px of headroom", (11,)),
        ("ceiling at row 12, 8 px of headroom", (12,)),
        ("row 13 only, the 8 px just above the floor", (13,)),
        ("rows 12 and 13, his whole body", (12, 13)),
    ):
        far = walk(pb, here, rows, tile)
        blocked = far // 8 < TO
        print(f"  {label:36} -> {far} px, {far // 8} columns"
              + (", stopped in the corridor" if blocked else ", walked through"))
    pb.stop()
    return 0


if __name__ == "__main__":
    sys.exit(main())
