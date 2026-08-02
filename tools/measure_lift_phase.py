# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Which way does a lift set off, and where in its cycle does it start?

`src/core/lift.rs` builds every lift with `direction: 1` and `phase: 0`, which
nobody measured. `measure_enemy_walk.py` gave the cadence and the half cycles
by watching a lift that had been running for a while, and that trace cannot
say which way it went from the position its record decodes to.

It decides a crossing. World 1-2's third gap runs from column 223 to 236 with
one horizontal lift, whose record puts it at column 232. Setting off right it
travels to column 238, leaving a 72 pixel gap from the ledge that no jump in
the engine covers. Setting off left it comes back to column 225, 19 pixels
from the ledge, and the crossing is one ordinary jump with a wait in front of
it.

So catch the slot on the frame the game fills it, rather than once the object
has drifted somewhere readable, and freeze the camera straight away: after
that every pixel the slot's coordinates change is the object's own. What comes
out is the first direction on each axis and the frame of the first reversal,
which is the phase it was created at (a full half cycle means it starts at one
end).

Two kinds are traced in one run, since World 1-1 carries both either side of
its exit door. The camera has to move again between them or the second is
never created, so the first trace ends and the fly loop resumes.

Usage: uv run tools/measure_lift_phase.py [level] [kind,kind...]
"""

import sys

sys.path.insert(0, "tools")

from dump_object_slots import SLOTS, slot
from run_through_levels import FLY_Y, LIVES, MARIO_Y, PHASE, SCREEN_X, SPAWN_X
from trace_level_objects import NAMES, reach_level

APPROACH = 6000
WATCH = 400


def fly(pb, capture, frame):
    """One frame of the flatten-and-fly walkthrough.

    The flattening is not optional once a run leaves World 1-1: pinning
    Mario's Y carries him over the ground but not through anything at that
    height, and World 1-2's block at column 212 stopped him dead between the
    two lifts this is here to compare.
    """
    pb.memory[MARIO_Y] = FLY_Y
    if pb.memory[SCREEN_X] > SPAWN_X:
        pb.memory[PHASE] = 0
    capture.step(pb, frame)
    pb.tick()
    return pb.memory[LIVES] > 0


def wait_for(pb, capture, kind, seen):
    """Fly right until a slot newly holds `kind`. Returns the slot index."""
    pb.button_press("right")
    for frame in range(APPROACH):
        if not fly(pb, capture, frame):
            return None
        for s in range(SLOTS):
            if slot(pb, s)[0] == kind and s not in seen:
                pb.button_release("right")
                return s
    pb.button_release("right")
    return None


def trace(pb, s, kind):
    """Read the slot every frame with the camera still. Returns [(x, y)]."""
    out = []
    for _ in range(WATCH):
        pb.memory[MARIO_Y] = FLY_Y
        pb.tick()
        state = slot(pb, s)
        if state[0] != kind:
            break
        out.append((state[3], state[2]))
    return out


def report(kind, path):
    if len(path) < 4:
        print(f"  kind 0x{kind:02X}: the slot emptied after {len(path)} frames")
        return
    print(f"  kind 0x{kind:02X}: {len(path)} frames from x {path[0][0]}, "
          f"y {path[0][1]}")
    for axis, i in (("x", 0), ("y", 1)):
        steps = [(f, b[i] - a[i])
                 for f, (a, b) in enumerate(zip(path, path[1:]))
                 if b[i] != a[i]]
        if not steps:
            print(f"    {axis}: never moved")
            continue
        first = steps[0]
        turn = next((f for (f, d) in steps if (d > 0) != (first[1] > 0)), None)
        print(f"    {axis}: sets off {'+' if first[1] > 0 else '-'} on frame "
              f"{first[0]}, {sum(abs(d) for _, d in steps)} px travelled")
        if turn is None:
            print(f"      no reversal in {len(path)} frames")
        else:
            print(f"      first reversal on frame {turn}, so it was created "
                  f"{turn} frames into that leg")


def main():
    level = sys.argv[1] if len(sys.argv) > 1 else "1-1"
    kinds = [int(k, 0) for k in (sys.argv[2] if len(sys.argv) > 2
                                 else "0x0A,0x0B").split(",")]

    pb, capture = reach_level(NAMES.index(level))
    if pb is None:
        print(f"never reached World {level}")
        return 1
    print(f"World {level} open")

    seen = set()
    for kind in kinds:
        s = wait_for(pb, capture, kind, seen)
        if s is None:
            print(f"  kind 0x{kind:02X} never appeared")
            continue
        seen.add(s)
        report(kind, trace(pb, s, kind))
    pb.stop()
    return 0


if __name__ == "__main__":
    sys.exit(main())
