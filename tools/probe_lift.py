# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Do the two objects by World 1-1's exit hold Mario up?

Kinds 0x0A and 0x0B are the last two records in the level, at columns 284 and
293, either side of the exit door. Neither moves on more than one axis, both
run at a flat pixel every two frames, and both reverse on a strict cycle: 120
frames over 60 pixels up and down for 0x0A, 106 frames over 53 pixels left and
right for 0x0B. That is the motion of a lift, and World 1-1's raised exit needs
some way of being reached, but a guess is a guess.

Mario settles it. Put him directly above the object, let go, and see where he
stops. Landing on it means his Y comes to rest at the object's height and then
follows it; falling through means he carries on to the ground far below.

World 1-3 introduces two more kinds that move on one axis only, `0x02`
(16 px down, pause, 16 px back up, on a 200-frame cycle) and `0x0C` (still,
then straight down at a pixel a frame). The same question applies to both, so
the probe takes a level as well as a kind.

A third argument, `carve`, clears the background tilemap around the object
before the drop. World 1-3's records sit inside terrain, so Mario lands on that
and the drop answers nothing; with the terrain gone the object is the only
thing left that can hold him.

Usage: uv run tools/probe_lift.py [kind] [1-1|1-2|1-3] [carve]
"""

import sys

sys.path.insert(0, "tools")

from dump_object_slots import EMPTY, SLOTS, slot
from run_through_levels import FLY_Y, MAP_BASE, MAP_WIDTH, MARIO_Y, PHASE, SPAWN_X
from sml_boot import boot_to_gameplay
from trace_level_objects import NAMES, reach_level

SCREEN_X = 0xC202
APPROACH = 2600
DROP = 40
WATCH = 200


def main():
    want = int(sys.argv[1], 0) if len(sys.argv) > 1 else 0x0A
    level = sys.argv[2] if len(sys.argv) > 2 else "1-1"
    carve = len(sys.argv) > 3 and sys.argv[3] == "carve"
    if level == "1-1":
        pb = boot_to_gameplay()
        pb.button_press("right")
    else:
        pb, _capture = reach_level(NAMES.index(level))
        if pb is None:
            print(f"never reached World {level}")
            return 1

    found = None
    for _ in range(APPROACH):
        pb.memory[MARIO_Y] = FLY_Y
        if pb.memory[SCREEN_X] > SPAWN_X:
            pb.memory[PHASE] = 0
        pb.tick()
        for s in range(SLOTS):
            state = slot(pb, s)
            if state[0] == want and 50 <= state[3] <= 140:
                found = s
                break
        if found is not None:
            break
    pb.button_release("right")
    if found is None:
        print(f"kind 0x{want:02X} never came on screen")
        pb.stop()
        return 1

    state = slot(pb, found)
    print(f"kind 0x{want:02X} in slot {found} at x {state[3]}, y {state[2]}")

    if carve:
        # Everything except the object, so nothing else can catch him.
        for row in range(2, 18):
            for column in range(MAP_WIDTH):
                pb.memory[MAP_BASE + row * MAP_WIDTH + column] = 0x00
        print("cleared the background tilemap around it")

    # Line Mario up over it and drop him from a little way above.
    for _ in range(4):
        pb.memory[MARIO_Y] = FLY_Y
        pb.tick()
    pb.memory[SCREEN_X] = state[3]
    pb.memory[MARIO_Y] = max(state[2] - DROP, 8)
    print(f"dropped mario at x {pb.memory[SCREEN_X]}, y {pb.memory[MARIO_Y]}\n")

    print("frame  mario y   object x   object y")
    rest = None
    for frame in range(WATCH):
        pb.tick()
        state = slot(pb, found)
        if state[0] != want:
            print(f"{frame:5d}  the object went away")
            break
        my = pb.memory[MARIO_Y]
        if frame % 5 == 0 or rest is None:
            print(f"{frame:5d}  {my:7d}   {state[3]:8d}   {state[2]:8d}")
        rest = my
    pb.stop()

    print(f"\nmario ended at y {rest}, the object at y {state[2]}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
