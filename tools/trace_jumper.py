# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Trace object kind 0x0E frame by frame, with the camera held still.

The summary that `measure_enemy_walk.py` prints suits something that walks:
step size, frames between steps, reversals. Kind 0x0E moves up to 4 px in a
frame on both axes and then sits still for 54 frames, which that summary can
only describe as noise. The shape of a jump needs the raw trace.

Same instrument as the walk measurement. The camera only scrolls while Mario
moves, so releasing right freezes it and every pixel the slot coordinates
change after that is the object's own. This prints the arc, splits it into
excursions above the resting row, and reports each one's height, duration, and
how far it carried the object sideways.

Usage: uv run tools/trace_jumper.py [kind] [frames]
"""

import sys

sys.path.insert(0, "tools")

from dump_object_slots import SLOTS, slot
from run_through_levels import FLY_Y, MARIO_Y
from sml_boot import boot_to_gameplay

WATCH = 900
APPROACH = 2600
SETTLE = 8


def find(pb, want):
    pb.button_press("right")
    found = None
    for _ in range(APPROACH):
        pb.memory[MARIO_Y] = FLY_Y
        pb.tick()
        for s in range(SLOTS):
            state = slot(pb, s)
            if state[0] == want and 40 <= state[3] <= 150:
                found = s
                break
        if found is not None:
            break
    pb.button_release("right")
    return found


def excursions(trace, rest):
    """Split the trace into stretches where the object is above its rest row."""
    out = []
    start = None
    for i, (_, y) in enumerate(trace):
        if y < rest and start is None:
            start = i
        elif y >= rest and start is not None:
            out.append((start, i))
            start = None
    return out


def main():
    want = int(sys.argv[1], 0) if len(sys.argv) > 1 else 0x0E
    watch = int(sys.argv[2]) if len(sys.argv) > 2 else WATCH

    pb = boot_to_gameplay()
    found = find(pb, want)
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

    if not trace:
        print("the slot emptied before anything could be read")
        return 1

    ys = [y for _, y in trace]
    rest = max(set(ys), key=ys.count)
    print(f"kind 0x{want:02X} in slot {found}, {len(trace)} frames traced")
    print(f"resting row at slot Y {rest}, lowest reached {min(ys)} "
          f"({rest - min(ys)} px above it)")

    hops = excursions(trace, rest)
    print(f"\n{len(hops)} excursions above the resting row")
    print(" start  frames  height   dx")
    for start, end in hops:
        height = rest - min(y for _, y in trace[start:end])
        dx = trace[end - 1][0] - trace[start][0]
        print(f" {start:5d}  {end - start:6d}  {height:6d}  {dx:+3d}")

    if len(hops) > 1:
        apart = [b[0] - a[0] for a, b in zip(hops, hops[1:])]
        print(f"\nstarts {apart} frames apart")
    grounded = sum(1 for _, y in trace if y >= rest)
    print(f"{grounded} of {len(trace)} frames at the resting row")

    print("\nbetween excursions:")
    for (_, end), (start, _) in zip(hops, hops[1:]):
        dx = trace[start][0] - trace[end - 1][0]
        print(f"  {start - end + 1} frames on the ground, {dx:+d} px sideways")

    # The first excursion is whatever was already in progress when the camera
    # stopped, so the second is the first complete one.
    if len(hops) > 1:
        start, end = hops[1]
        print(f"\nthe second excursion, frames {start} to {end}, "
              f"as height above the resting row:")
        heights = [rest - y for _, y in trace[start - 1 : end + 1]]
        print("  " + " ".join(str(h) for h in heights))
        steps = [
            (b[0] - a[0], a[1] - b[1])
            for a, b in zip(trace[start - 1 : end + 1], trace[start : end + 2])
            if (b[0], b[1]) != (a[0], a[1])
        ]
        print(f"  {len(steps)} position updates, (dx, up) each: "
              + " ".join(f"{dx:+d}/{dy:+d}" for dx, dy in steps))
    return 0


if __name__ == "__main__":
    sys.exit(main())
