# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""What starts World 1-3's kind 0x0C falling?

Its motion and its contact behaviour are both measured: X frozen, then a
straight drop at exactly one pixel a frame, and a sweep that matches a
walker's, so it hurts Mario from every side except a stomp. One trace had it
stand still for 125 frames first, which could be a timer or could be Mario
walking under it.

The two readings come apart if Mario is somewhere else. Freeze the camera the
frame the object comes on screen, park Mario at a chosen screen X, and count
frames until the object's Y moves. A timer gives the same count wherever he
stands; a trigger only fires when he is near.

Usage: uv run tools/probe_faller_trigger.py [kind] [1-1|1-2|1-3]
"""

import sys

sys.path.insert(0, "tools")

from dump_object_slots import EMPTY, SLOTS, slot
from run_through_levels import FLY_Y, LIVES, MARIO_Y, PHASE, SCREEN_X, SPAWN_X
from trace_level_objects import NAMES, reach_level

APPROACH = 5000
WATCH = 900
# Directly under it, a screen away, and as far left as the screen goes.
PARK = ["under", 60, 8]


def trial(kind, level, park):
    pb, _capture = reach_level(NAMES.index(level))
    if pb is None:
        return None

    # Catch the slot the frame it fills, not the frame the object becomes
    # visible. The timer starts when the game creates the object, and it is
    # created off the right of the screen, so waiting for it to scroll into
    # view spends part of the count before anything is being measured.
    found = None
    types = [slot(pb, s)[0] for s in range(SLOTS)]
    for _ in range(APPROACH):
        pb.memory[MARIO_Y] = FLY_Y
        if pb.memory[SCREEN_X] > SPAWN_X:
            pb.memory[PHASE] = 0
        pb.tick()
        if pb.memory[LIVES] == 0:
            break
        for s in range(SLOTS):
            now = slot(pb, s)[0]
            if types[s] == EMPTY and now == kind:
                found = s
            types[s] = now
        if found is not None:
            break
    pb.button_release("right")
    if found is None:
        pb.stop()
        return None

    state = slot(pb, found)
    at = state[3] if park == "under" else park
    start_y = state[2]
    for frame in range(WATCH):
        # Keep him where he was put, and out of reach vertically so the object
        # landing on him does not end the trial early.
        pb.memory[MARIO_Y] = FLY_Y
        pb.memory[SCREEN_X] = at
        pb.tick()
        now = slot(pb, found)
        if now[0] != kind:
            pb.stop()
            return ("the slot emptied", state[3], at)
        if now[2] != start_y:
            pb.stop()
            return (f"fell after {frame} frames", state[3], at)
    pb.stop()
    return (f"never fell in {WATCH} frames", state[3], at)


def main():
    kind = int(sys.argv[1], 0) if len(sys.argv) > 1 else 0x0C
    level = sys.argv[2] if len(sys.argv) > 2 else "1-3"

    print(f"World {level}, kind 0x{kind:02X}: frames from the object being "
          f"created to its first pixel of fall\n")
    for park in PARK:
        result = trial(kind, level, park)
        if result is None:
            print(f"  mario at {park}: never found the object")
            continue
        note, object_x, at = result
        where = f"mario at screen x {at}, object at {object_x}"
        print(f"  {where}: {note}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
