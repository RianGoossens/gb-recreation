# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Does the ground walker turn at a ledge, or walk off it?

Our engine turns it around, which came from another game rather than from
this one. World 1-1's own pits are awkward to use: they are a long way from
any walker's spawn, and the camera has to stay frozen for the walk to be
readable, which pins how far the walker can get.

So the ledge gets made instead. Super Mario Land tests collision against the
background tilemap in video RAM (see `probe_solidity.py`), and with the camera
stopped nothing redraws it, so clearing the ground out of a few columns in
front of the walker leaves a pit that the game treats as real.

Placing that pit needs the walker's column, which follows from two numbers
already pinned: the camera sits 27 columns behind the column counter, and the
walker's screen X is its slot byte minus the 8-pixel OAM offset. The script
checks its work before cutting anything, by confirming the column it computed
for the walker's feet actually holds ground.

Falling shows as the slot's Y climbing, turning as its X going back.

Usage: uv run tools/probe_enemy_ledge.py
"""

import sys

sys.path.insert(0, "tools")

from dump_object_slots import EMPTY, SLOTS, slot
from run_through_levels import Capture, FLY_Y, MAP_BASE, MAP_WIDTH, MARIO_Y
from sml_boot import boot_to_gameplay

WALKER = 0x00
OAM_OFFSET = 8
CAMERA_LAG = 27
GROUND_ROWS = (16, 17)  # tilemap rows for playfield rows 14 and 15
SOLID_FROM = 0x60
OPEN = 0x00

GAP = 3
APPROACH = 2000
WATCH = 900


def ring_tile(pb, ring, row):
    return pb.memory[MAP_BASE + row * MAP_WIDTH + ring]


def main():
    pb = boot_to_gameplay()
    capture = Capture(pb, 0)
    capture.pending.clear()  # the terrain has to stay; the pit is cut by hand

    found = None
    pb.button_press("right")
    for frame in range(APPROACH):
        pb.memory[MARIO_Y] = FLY_Y
        pb.tick()
        capture.step(pb, frame)
        for s in range(SLOTS):
            state = slot(pb, s)
            # On screen, with room to the left, and standing rather than falling.
            if state[0] == WALKER and 60 <= state[3] <= 150 and state[2] == 136:
                found = s
                break
        if found is not None:
            break
    pb.button_release("right")
    if found is None:
        print("no walker settled on screen")
        pb.stop()
        return 1

    state = slot(pb, found)
    camera = len(capture.columns) - CAMERA_LAG
    column = camera + (state[3] - OAM_OFFSET) // 8
    ring = column % MAP_WIDTH
    print(f"walker in slot {found} at screen x {state[3] - OAM_OFFSET}, y {state[2]}")
    print(f"column counter {len(capture.columns)}, camera at column {camera}, "
          f"walker at column {column} (ring {ring})")

    under = [ring_tile(pb, ring, row) for row in GROUND_ROWS]
    print(f"tiles under the walker: {under}")
    if not all(t >= SOLID_FROM for t in under):
        print("the computed column is not ground, so the mapping is wrong")
        pb.stop()
        return 1

    # A bottomless pit cannot tell a fall from a removal, and World 1-1's
    # ground is only two rows deep, so "shallow" cuts the upper row alone and
    # leaves a floor 8 pixels down to land on.
    shallow = "--shallow" in sys.argv
    rows = GROUND_ROWS[:1] if shallow else GROUND_ROWS
    for step in range(1, GAP + 1):
        cut = (ring - step) % MAP_WIDTH
        for row in rows:
            pb.memory[MAP_BASE + row * MAP_WIDTH + cut] = OPEN
    depth = "one row" if shallow else "both rows"
    print(f"cut {GAP} columns of ground ({depth}) to its left\n")

    print("frame    x    y")
    last = (state[3], state[2])
    turned = False
    fell = False
    for frame in range(WATCH):
        pb.memory[MARIO_Y] = FLY_Y
        pb.tick()
        state = slot(pb, found)
        if state[0] != WALKER:
            print(f"{frame:5d}  slot emptied")
            break
        now = (state[3], state[2])
        if now != last:
            turned |= now[0] > last[0]
            fell |= now[1] > last[1]
            print(f"{frame:5d}  {now[0]:3d}  {now[1]:3d}")
            last = now
    pb.stop()

    print(f"\nturned back: {turned}")
    print(f"fell: {fell}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
