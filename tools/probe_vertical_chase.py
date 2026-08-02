# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Does this object kind swim towards Mario's height?

`measure_flyer.py` runs two trials at two heights, which is enough to say that
a flight ignores Mario when both come back identical. World 2-3's `0x1D` did
not: with him pinned high the object rose 55 pixels and was still rising when
it left the screen, and with him pinned low it ended within a pixel of him
having moved both up and down on the way. Two points fit "it chases him" and
they also fit "it rises until something stops it".

This asks the question directly. Snapshot the frame the slot fills, then for
each of several heights restore, pin Mario there, and read where the object's
y ends up. A chaser converges on him at every height, including the ones below
where it started.

The control is the trial that pins Mario at the object's own starting height.
A chaser has nothing to do there and holds still; a thing that simply drifts
up moves anyway. That trial can fail, which is what makes the others worth
reading.

Usage: uv run tools/probe_vertical_chase.py [level] [kind] [frames]
"""

import sys

sys.path.insert(0, "tools")

from dump_object_slots import slot
from measure_lift_phase import wait_for
from run_through_levels import MARIO_Y
from sml_boot import restore, snapshot
from trace_level_objects import NAMES, reach_level

WATCH = 300
# Heights to pin him at, as offsets from the object's own y when the trial
# starts. Above it, level with it, and below it, so a chase has to change
# direction across the set.
OFFSETS = [-56, -24, 0, 24, 56]


def trial(pb, s, kind, mario_y):
    """Pin Mario at a height and read the object's y each frame."""
    out = []
    for _ in range(WATCH):
        pb.memory[MARIO_Y] = mario_y
        pb.tick()
        state = slot(pb, s)
        if state[0] != kind:
            break
        out.append(state[2])
    return out


def main():
    level = sys.argv[1] if len(sys.argv) > 1 else "2-3"
    kind = int(sys.argv[2], 0) if len(sys.argv) > 2 else 0x1D
    watch = int(sys.argv[3]) if len(sys.argv) > 3 else WATCH

    pb, capture = reach_level(NAMES.index(level))
    if pb is None:
        print(f"never reached World {level}")
        return 1
    print(f"World {level} open, kind 0x{kind:02X}, {watch} frames a trial")

    seen = set()
    s = wait_for(pb, capture, kind, seen)
    if s is None:
        print(f"kind 0x{kind:02X} never appeared")
        pb.stop()
        return 1
    here = snapshot(pb)
    start = slot(pb, s)[2]
    print(f"slot {s}, the object starts at y {start}\n")

    for offset in OFFSETS:
        restore(pb, here)
        capture.pending.clear()
        mario_y = start + offset
        path = trial(pb, s, kind, mario_y)
        if len(path) < 4:
            print(f"  Mario at {mario_y:3d} ({offset:+3d}): "
                  f"the slot emptied after {len(path)} frames")
            continue
        moved = path[-1] - path[0]
        towards = "towards him" if moved * offset > 0 else "away from him"
        if moved == 0:
            towards = "not at all"
        label = "  control, level with it" if offset == 0 else f"  Mario at {mario_y:3d}"
        print(f"{label} ({offset:+3d}): y {path[0]} -> {path[-1]} "
              f"over {len(path)} frames, {moved:+d}, {towards}, "
              f"closest {min(abs(y - mario_y) for y in path)} px off him")
    pb.stop()
    return 0


if __name__ == "__main__":
    sys.exit(main())
