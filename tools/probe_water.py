# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Does Mario swim in World 2-1's water, or does it kill him?

The plan has carried "swimming, so World 2's levels can be finished" since the
geometry walker stopped at column 69 of both 2-1 and 2-2, where the level has
no floor. Two things found since then point the other way. The decoded tiles
put the animated water line in the bottom row of the screen for 300 of 2-1's
320 columns, so the water is below the playfield rather than a body he moves
through, and 2-1's own object list puts two horizontal lifts inside the gap,
at columns 66 and 74.

Swimming would be a mechanic invented for this project if the cartridge does
not have it, which is the thing the faithfulness rule exists to stop. So ask
the cartridge. Walk to the gap, stop pinning him, let him fall in, and watch
the life counter and his y.

Two outcomes and they are not close together:

    he loses a life        the water is a pit, and the lifts are the crossing
    he stays alive in it   he swims, and how he moves in it is the next thing
                           to measure

A death takes 212 frames to register, so the watch has to outlast that; the
control for the instrument is the life counter moving at all in the run.

Usage: uv run tools/probe_water.py [level] [column]
"""

import sys

sys.path.insert(0, "tools")

from dump_object_slots import EMPTY, SLOTS, slot
from run_through_levels import FLY_Y, LIVES, MARIO_Y, PHASE, SCREEN_X, SPAWN_X
from trace_level_objects import NAMES, reach_level

# Well inside the gap: 2-1 has no floor from about column 62 to 81.
GAP_COLUMN = 70
WATCH = 400
COLUMN_FRAMES = 8


def main():
    level = sys.argv[1] if len(sys.argv) > 1 else "2-1"
    column = int(sys.argv[2]) if len(sys.argv) > 2 else GAP_COLUMN

    pb, capture = reach_level(NAMES.index(level))
    if pb is None:
        print("the run died before reaching the level")
        return 1

    # Fly along the top until the camera has the gap under it, the same way
    # every other tool crosses a level.
    lives = pb.memory[LIVES]
    print(f"{level} open with {lives} lives, flying to column {column}")
    while len(capture.columns) < column + 6:
        pb.memory[MARIO_Y] = FLY_Y
        if pb.memory[SCREEN_X] > SPAWN_X:
            pb.memory[PHASE] = 0
        pb.tick()
        capture.step(pb, len(capture.columns) * COLUMN_FRAMES)
        capture.pending.clear()
        if pb.memory[LIVES] < lives:
            print("he died on the way there")
            pb.stop()
            return 1
    pb.button_release("right")

    # Drop him. Nothing is written after this, so what happens is the game's.
    print(f"at column {len(capture.columns)}, dropping him in")
    print("frame  mario y  mario x  phase  lives")
    for f in range(WATCH):
        pb.tick()
        y, phase, now = pb.memory[MARIO_Y], pb.memory[PHASE], pb.memory[LIVES]
        if f % 20 == 0 or now < lives:
            print(f"{f:5d}  {y:7d}  {pb.memory[SCREEN_X]:7d}  "
                  f"{phase:5d}  {now:5d}")
        if now < lives:
            print(f"\nhe lost a life on frame {f}: the water is a pit")
            pb.stop()
            return 0
    print(f"\nhe was still alive after {WATCH} frames at y "
          f"{pb.memory[MARIO_Y]}: he does not simply drown")
    print("what is on screen with him:")
    for i in range(SLOTS):
        state = slot(pb, i)
        if state[0] != EMPTY:
            print(f"    slot {i}: kind 0x{state[0]:02X} at x {state[3]}, "
                  f"y {state[2]}")
    pb.stop()
    return 0


if __name__ == "__main__":
    sys.exit(main())
