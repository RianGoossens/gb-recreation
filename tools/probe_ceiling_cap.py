# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Does a ceiling over part of Mario cap his jump, and is the pocket a dead end?

World 1-3 opens with a block at row 10 spanning columns 7 to 11 and a two-tile
wall at columns 13 and 14. Walking the floor at row 14 takes Mario under the
block and up against the wall, and in our engine he cannot get out: the wall's
top is 16 pixels above his feet and the block caps his rise 4 pixels short of
that. Our rise is capped because *any* part of his 11 pixel width is under the
block, and here he is under it by a single column, so a cartridge that tests a
narrower head would let him through.

The question the cartridge can answer directly is the one that matters: walk
right from World 1-3's opening and see whether he gets stuck at column 11.

Two controls, since a probe that can only answer "stuck" is measuring itself:

  free      the same jump with nothing overhead, at column 3, which has to
            come out higher than the one under the block
  no jump   walking right without pressing A, which has to stop at the wall

Reaching World 1-3 uses the flatten-and-fly walkthrough, which rewrites the
terrain ahead of Mario as he goes. That stops the frame the level opens, so
1-3's own opening screen is the game's, and the tool prints the rows it reads
back out of the tilemap so a flattened one is visible rather than silent.

Usage: uv run tools/probe_ceiling_cap.py [level] [column]
"""

import sys

sys.path.insert(0, "tools")

from run_through_levels import LIVES, MARIO_Y, PHASE, SCREEN_X
from sml_boot import restore, snapshot
from trace_level_objects import NAMES, reach_level

# The background tilemap, and the first tile id that is solid.
TILEMAP = 0x9800
SOLID_FROM = 0x60
SETTLE = 90
WATCH = 240


def tilemap_rows(pb, columns=20, rows=range(2, 16)):
    """The visible tilemap as text, so a flattened screen is not silent.

    The playfield sits below two rows of status bar, and the game scrolls by
    rewriting a 32-column ring, so at a level's opening the screen column and
    the ring column are the same.
    """
    out = []
    for row in rows:
        line = ""
        for column in range(columns):
            tile = pb.memory[TILEMAP + row * 32 + column]
            line += "#" if tile >= SOLID_FROM else "."
        out.append(f"  row {row - 2:2}: {line}")
    return out


def land(pb):
    """Let go of everything and let him fall to the floor."""
    for _ in range(SETTLE):
        pb.tick()
    return pb.memory[MARIO_Y]


def run(pb, state, column, jump):
    """Put him at `column` and hold right, tapping A unless `jump` is off.

    Returns (furthest screen x, highest point, whether a life was lost).
    """
    restore(pb, state)
    pb.memory[SCREEN_X] = column * 8
    pb.memory[PHASE] = 0
    lives = pb.memory[LIVES]
    pb.button_press("right")
    furthest, highest, held = 0, 255, 0
    path = []
    for frame in range(WATCH):
        # A jump needs a released frame in front of it: the button is latched,
        # so holding it down gives one jump and then nothing.
        if jump and frame % 30 == 0:
            pb.button_press("a")
            held = 20
        if held:
            held -= 1
            if held == 0:
                pb.button_release("a")
        pb.tick()
        if pb.memory[LIVES] < lives:
            pb.button_release("right")
            pb.button_release("a")
            return furthest, highest, True, path
        furthest = max(furthest, pb.memory[SCREEN_X])
        highest = min(highest, pb.memory[MARIO_Y])
        path.append((pb.memory[SCREEN_X], pb.memory[MARIO_Y]))
    pb.button_release("right")
    pb.button_release("a")
    return furthest, highest, False, path


def standing_jump(pb, state, x):
    """One standing jump from screen x. Returns how far he rose, in pixels."""
    restore(pb, state)
    pb.memory[SCREEN_X] = x
    pb.memory[PHASE] = 0
    start = pb.memory[MARIO_Y]
    highest = start
    pb.button_press("a")
    for frame in range(80):
        if frame == 20:
            pb.button_release("a")
        pb.tick()
        highest = min(highest, pb.memory[MARIO_Y])
    pb.button_release("a")
    return start - highest


def main():
    level = sys.argv[1] if len(sys.argv) > 1 else "1-3"
    column = int(sys.argv[2]) if len(sys.argv) > 2 else 11

    pb, _ = reach_level(NAMES.index(level))
    if pb is None:
        print(f"never reached World {level}")
        return 1
    pb.button_release("right")
    floor = land(pb)
    print(f"World {level} open, Mario resting at y {floor}")
    print("the opening screen as the game has it:")
    print("\n".join(tilemap_rows(pb)))

    here = snapshot(pb)
    for label, at, jump in (
        ("free jump at column 3 (control)", 3, True),
        ("walk only, no jump (control)", column, False),
        (f"jump under the block at column {column}", column, True),
    ):
        far, high, died, path = run(pb, here, at, jump)
        rise = floor - high
        print(f"  {label:34} -> furthest x {far}, rose {rise} px"
              + (", lost a life" if died else ""))
        # The aggregates cannot tell a capped jump from a jump that landed on
        # top of the block and set off again, so print the path as well.
        print("     " + " ".join(f"{x},{y}" for x, y in path[::6][:22]))

    # A standing jump, a pixel at a time across the block's edges. Away from it
    # the rise is the free 24; under it the block cuts the rise short. Where
    # that changes is the edge of whatever part of Mario the ceiling test uses,
    # and the free jumps either side are the control that says the sweep is
    # wide enough to hold the whole window.
    print("\nstanding jumps across the block at columns 7 to 11 (x 56 to 95):")
    rises = [(x, standing_jump(pb, here, x)) for x in range(40, 112)]
    free = max(r for _, r in rises)
    capped = [x for x, r in rises if r < free]
    print("  " + " ".join(f"{x}:{r}" for x, r in rises))
    if not capped:
        print("  nothing capped the jump anywhere, so this reads nothing")
    else:
        print(f"  free rise {free}, cut short from x {capped[0]} to "
              f"{capped[-1]}, which is {capped[-1] - capped[0] + 1} px")
    pb.stop()
    return 0


if __name__ == "__main__":
    sys.exit(main())
