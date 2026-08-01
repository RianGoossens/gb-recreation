# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""What does object kind 0x36 do when Mario lands on it?

It is the most-used kind nothing had watched: nine of the twelve levels place
it. Two measurements narrow it down and neither finishes the job.
`measure_level_kind.py` traced it for 600 frames in World 1-2 and it never
moved a pixel on either axis. `probe_object_contact.py` swept the whole
overlap and it never cost Mario a life, so it is not an enemy. But at the one
offset where his feet are on top of it, the object went away, and the 600
frame trace is the control that says it does not go away on its own.

So this drops him on it and watches both of them frame by frame: his Y, the
slot's kind byte, and the slot's Y. That separates the three things it could
be. A platform holds him and stays. A block that gives way holds him and then
goes. Something with no surface at all lets him fall straight through, and the
disappearance was him passing through rather than standing on it.

Usage: uv run tools/probe_drop_block.py [kind] [1-2|1-3]
"""

import sys

sys.path.insert(0, "tools")

from dump_object_slots import EMPTY, SLOTS, slot
from run_through_levels import FLY_Y, LIVES, MARIO_Y, PHASE, SCREEN_X, SPAWN_X
from trace_level_objects import NAMES, reach_level

APPROACH = 5000
# Above it and clear, so the landing is a real fall rather than a placement
# already overlapping.
DROP = 28
WATCH = 240


def main():
    want = int(sys.argv[1], 0) if len(sys.argv) > 1 else 0x36
    level = sys.argv[2] if len(sys.argv) > 2 else "1-2"

    pb, _capture = reach_level(NAMES.index(level))
    if pb is None:
        print("the run died before reaching the level")
        return 1

    found = None
    for _ in range(APPROACH):
        pb.memory[MARIO_Y] = FLY_Y
        if pb.memory[SCREEN_X] > SPAWN_X:
            pb.memory[PHASE] = 0
        pb.tick()
        if pb.memory[LIVES] == 0:
            break
        for s in range(SLOTS):
            state = slot(pb, s)
            if state[0] == want and 60 <= state[3] <= 120:
                found = s
                break
        if found is not None:
            break
    pb.button_release("right")
    if found is None:
        print(f"kind 0x{want:02X} never came on screen in {level}")
        pb.stop()
        return 1

    state = slot(pb, found)
    ox, oy = state[3], state[2]
    print(f"kind 0x{want:02X} in slot {found} at x {ox}, y {oy}\n")

    # Line him up over it and let go.
    for _ in range(4):
        pb.memory[MARIO_Y] = FLY_Y
        pb.tick()
    pb.memory[SCREEN_X] = ox
    pb.memory[MARIO_Y] = max(oy - DROP, 8)

    print("frame  mario y  slot kind  slot y")
    resting = None
    for f in range(WATCH):
        pb.tick()
        now = slot(pb, found)
        my = pb.memory[MARIO_Y]
        gone = now[0] != want
        if f < 60 or gone or f % 20 == 0:
            print(f"{f:5d}  {my:7d}  {'gone' if gone else f'0x{now[0]:02X}':>9}  "
                  f"{'-' if gone else now[2]:>6}")
        if gone:
            print(f"\nthe object went away on frame {f}, "
                  f"with Mario at y {my} ({oy - my} above its last y)")
            break
        if resting is None and my == oy - 10:
            resting = f
    else:
        print(f"\nthe object was still there after {WATCH} frames")
    if resting is not None:
        print(f"he was resting on it (slot y minus 10) from frame {resting}")
    else:
        print("he never rested at the height a lift holds him at")
    pb.stop()
    return 0


if __name__ == "__main__":
    sys.exit(main())
