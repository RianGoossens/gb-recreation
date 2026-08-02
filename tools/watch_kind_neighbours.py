# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""What appears next to a kind that never moves.

Gao (`0x3F`) does not move a pixel in 700 frames with the camera frozen
(`tools/measure_level_kind.py 1-3 0x3F 700`), and the sprite survey found a
kind named "fire breath" (`0x1E`) in the same atlas band. A stationary object
that breathes fire spawns the fire as another object, so the thing to watch
is not the kind itself but every other slot while it sits there.

Same instrument as always: fly to the level, find the kind, release right so
the camera freezes, then watch. Every slot that fills or empties is reported
with its position relative to the kind being watched.

Usage: uv run tools/watch_kind_neighbours.py [level] [kind] [frames]
"""

import sys

sys.path.insert(0, "tools")

from dump_object_slots import EMPTY, SLOTS, slot
from run_through_levels import FLY_Y, LIVES, MARIO_Y, PHASE, SCREEN_X, SPAWN_X
from trace_level_objects import NAMES, reach_level

APPROACH = 5000
WATCH = 900


def find(pb, kind):
    for _ in range(APPROACH):
        pb.memory[MARIO_Y] = FLY_Y
        if pb.memory[SCREEN_X] > SPAWN_X:
            pb.memory[PHASE] = 0
        pb.tick()
        if pb.memory[LIVES] == 0:
            return None
        for s in range(SLOTS):
            if slot(pb, s)[0] == kind and 60 <= slot(pb, s)[3] <= 120:
                pb.button_release("right")
                return s
    pb.button_release("right")
    return None


def main():
    level = sys.argv[1] if len(sys.argv) > 1 else "1-3"
    kind = int(sys.argv[2], 0) if len(sys.argv) > 2 else 0x3F
    frames = int(sys.argv[3]) if len(sys.argv) > 3 else WATCH

    pb, _ = reach_level(NAMES.index(level))
    if pb is None:
        print(f"never reached World {level}")
        return 1
    s = find(pb, kind)
    if s is None:
        print(f"kind 0x{kind:02X} never came on screen")
        pb.stop()
        return 1

    ox, oy = slot(pb, s)[3], slot(pb, s)[2]
    print(f"watching kind 0x{kind:02X} in slot {s} at x {ox}, y {oy}")
    was = [slot(pb, i)[0] for i in range(SLOTS)]
    seen = {}
    for frame in range(frames):
        pb.tick()
        for i in range(SLOTS):
            state = slot(pb, i)
            if state[0] == was[i]:
                continue
            if state[0] != EMPTY:
                dx, dy = state[3] - ox, state[2] - oy
                print(f"  frame {frame}: slot {i} filled with 0x{state[0]:02X} "
                      f"at {dx:+d}, {dy:+d} from it")
                seen[state[0]] = seen.get(state[0], 0) + 1
            else:
                print(f"  frame {frame}: slot {i} emptied")
            was[i] = state[0]
    print(f"kinds that appeared: "
          + (", ".join(f"0x{k:02X} x{n}" for k, n in sorted(seen.items()))
             or "none"))
    pb.stop()
    return 0


if __name__ == "__main__":
    sys.exit(main())
