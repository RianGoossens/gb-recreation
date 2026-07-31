# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Measure how an object kind moves, in any World 1 level.

`measure_enemy_walk.py` and `trace_jumper.py` both start in World 1-1, which
is where four of the five measured kinds live. World 1-3 introduces two more,
`0x02` and `0x0C`, and it has no kind whose movement is known at all, so its
extracted level currently carries no enemies. Getting to them means playing
through two levels first, which `trace_level_objects.reach_level` now does.

From there it is the same instrument as always: the camera only scrolls while
Mario moves, so releasing right freezes it, and every pixel the object's slot
coordinates change after that is its own. This prints the raw per-frame trace
and a summary of both axes, which covers a walker and a hopper alike.

Usage: uv run tools/measure_level_kind.py [1-1|1-2|1-3] [kind] [frames]
"""

import sys

sys.path.insert(0, "tools")

from dump_object_slots import SLOTS, slot
from run_through_levels import FLY_Y, LIVES, MARIO_Y, PHASE, SCREEN_X, SPAWN_X
from trace_level_objects import NAMES, reach_level

APPROACH = 5000
SETTLE = 8
WATCH = 600


def summarise(axis, moves):
    if not moves:
        print(f"{axis}: never moved")
        return
    gaps = [b[0] - a[0] for a, b in zip(moves, moves[1:])]
    sizes = sorted({d for _, d in moves})
    print(f"{axis}: {len(moves)} moves of {sizes} px, "
          f"{sum(abs(d) for _, d in moves)} px travelled, "
          f"{sum(d for _, d in moves):+d} px net")
    if gaps:
        spread = sorted(set(gaps))
        print(f"   frames between moves: "
              + ", ".join(f"{g}x{gaps.count(g)}" for g in spread[:6]))
    turns = [b[0] for a, b in zip(moves, moves[1:]) if (a[1] > 0) != (b[1] > 0)]
    if turns:
        apart = [b - a for a, b in zip(turns, turns[1:])]
        print(f"   {len(turns)} reversals, {sorted(set(apart))[:6]} frames apart")


def main():
    want_level = sys.argv[1] if len(sys.argv) > 1 else "1-3"
    want = int(sys.argv[2], 0) if len(sys.argv) > 2 else 0x02
    watch = int(sys.argv[3]) if len(sys.argv) > 3 else WATCH

    pb, _capture = reach_level(NAMES.index(want_level))
    if pb is None:
        print(f"never reached World {want_level}")
        return 1
    print(f"World {want_level} open, looking for kind 0x{want:02X}")

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
            if state[0] == want and 40 <= state[3] <= 150:
                found = s
                break
        if found is not None:
            break
    pb.button_release("right")
    if found is None:
        print(f"no object of kind 0x{want:02X} came on screen")
        pb.stop()
        return 1
    for _ in range(SETTLE):
        pb.tick()

    trace = []
    for _ in range(watch):
        pb.memory[MARIO_Y] = FLY_Y
        pb.tick()
        state = slot(pb, found)
        if state[0] != want:
            break
        trace.append((state[3], state[2]))
    pb.stop()

    if len(trace) < 2:
        print("the slot emptied before anything could be read")
        return 1
    print(f"slot {found}, {len(trace)} frames traced, "
          f"x {trace[0][0]}->{trace[-1][0]}, y {trace[0][1]}->{trace[-1][1]}")

    steps = [
        (i, (b[0] - a[0], b[1] - a[1]))
        for i, (a, b) in enumerate(zip(trace, trace[1:]))
        if a != b
    ]
    for axis, index in (("x", 0), ("y", 1)):
        summarise(axis, [(f, d[index]) for f, d in steps if d[index]])

    print("\nfirst 40 changes (frame: dx, dy):")
    print("  " + " ".join(f"{f}:{d[0]:+d},{d[1]:+d}" for f, d in steps[:40]))
    return 0


if __name__ == "__main__":
    sys.exit(main())
