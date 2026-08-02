# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""How does object kind 0x42, Bunbun, move?

World 1-2 carries 19 of them, more than any other kind in any World 1 level,
and the extractor drops every one because the engine has nothing to place. So
it is the largest single piece of World 1 still missing.

The method is `measure_lift_phase.py`'s: catch the slot on the frame the game
fills it, then stop the camera, so every pixel its coordinates change after
that is the object's own motion. What that tool reports is the first direction
and the first reversal, which is enough for a platform on a fixed cycle. A
flyer needs three more things.

The cadence, because "188 px in 319 frames" is a rate and not a rule: the
frames between one pixel step and the next say whether it moves every other
frame, or two pixels every third, or something with a pattern in it.

Whether the flight depends on Mario. The walkers do not, and a flyer that
tracks him would be a different kind of thing to write. Two controls for that,
both cheap once the level is snapshotted: pin Mario at two different heights
and compare the traces, and once the flyer has passed him, put him back on its
far side and see whether it turns.

Whether terrain stops it. World 1-2 is built out of floating platforms and the
trace runs along one height, so the run reports which of the columns it crossed
had a solid tile at its own row. A flyer that goes through them has its answer
without writing anything.

Usage: uv run tools/measure_flyer.py [level] [kind]
"""

import sys

sys.path.insert(0, "tools")

from dump_object_slots import SLOTS, slot
from measure_lift_phase import fly, wait_for
from run_through_levels import FLY_Y, MARIO_Y, SCREEN_X
from sml_boot import restore, snapshot
from trace_level_objects import NAMES, reach_level

WATCH = 400
TILEMAP = 0x9800
HUD_ROWS = 2
# Where Mario is held for the second trace. FLY_Y is 24, near the top of the
# playfield, so this is most of a screen lower and still off the floor.
LOW_Y = 96
# Screen x to put him at once the flyer has gone past, for the turn control.
BEHIND_X = 140


def trace(pb, s, kind, mario_y, chase_from=None):
    """Read the slot every frame with the camera still.

    Returns [(x, y)]. With `chase_from`, Mario is moved to the right of the
    flyer once its x drops below that, which asks whether it follows him.
    """
    out = []
    chasing = False
    for _ in range(WATCH):
        pb.memory[MARIO_Y] = mario_y
        if chasing:
            pb.memory[SCREEN_X] = BEHIND_X
        pb.tick()
        state = slot(pb, s)
        if state[0] != kind:
            break
        out.append((state[3], state[2]))
        if chase_from is not None and state[3] < chase_from:
            chasing = True
    return out


def runs(path, i):
    """Per-frame deltas on one axis, run-length encoded, zeros included."""
    out = []
    for a, b in zip(path, path[1:]):
        d = b[i] - a[i]
        if out and out[-1][0] == d:
            out[-1][1] += 1
        else:
            out.append([d, 1])
    return out


def row_tiles(pb, row):
    """The 32 tilemap ids on one screen row, to say what the flyer crossed."""
    return [pb.memory[TILEMAP + (row + HUD_ROWS) * 32 + c] for c in range(32)]


def report(label, kind, path, pb=None):
    if len(path) < 4:
        print(f"  {label}: the slot emptied after {len(path)} frames")
        return
    print(f"  {label}: {len(path)} frames, x {path[0][0]} to {path[-1][0]}, "
          f"y {path[0][1]} to {path[-1][1]}")
    for axis, i in (("x", 0), ("y", 1)):
        steps = [(f, b[i] - a[i])
                 for f, (a, b) in enumerate(zip(path, path[1:]))
                 if b[i] != a[i]]
        if not steps:
            print(f"    {axis}: never moved")
            continue
        sizes = sorted({d for _, d in steps})
        shape = runs(path, i)
        drawn = " ".join(f"{d:+d}x{n}" for d, n in shape[:14])
        print(f"    {axis}: {len(steps)} steps of {sizes}, first on frame "
              f"{steps[0][0]}, per frame {drawn}"
              + (" ..." if len(shape) > 14 else ""))
    if pb is not None:
        row = path[0][1] // 8 - HUD_ROWS
        tiles = row_tiles(pb, row)
        solid = [c for c, t in enumerate(tiles) if t >= 0x60]
        print(f"    it flew along screen row {row}, whose solid columns are "
              f"{solid if solid else 'none'}")


def main():
    level = sys.argv[1] if len(sys.argv) > 1 else "1-2"
    kind = int(sys.argv[2], 0) if len(sys.argv) > 2 else 0x42

    pb, capture = reach_level(NAMES.index(level))
    if pb is None:
        print(f"never reached World {level}")
        return 1
    print(f"World {level} open, tracing kind 0x{kind:02X}")

    trials = [
        ("held high", FLY_Y, None),
        ("held low", LOW_Y, None),
        ("moved behind it", FLY_Y, 60),
    ]
    seen = set()
    for label, mario_y, chase in trials:
        here = snapshot(pb)
        s = wait_for(pb, capture, kind, seen)
        if s is None:
            print(f"  {label}: kind 0x{kind:02X} never appeared")
            restore(pb, here)
            continue
        seen.add(s)
        report(label, kind, trace(pb, s, kind, mario_y, chase), pb)
        restore(pb, here)
        # The restore puts the un-flattened terrain back, and any flattening
        # still owed from this trial is owed at a frame number the next trial
        # never reaches. Drop it; the next changed column schedules its own.
        capture.pending.clear()
        seen.discard(s)

    # A cadence read off one instance can be that instance's phase rather than
    # the kind's rule, so walk on and trace the next two the level creates.
    for n in (2, 3):
        s = wait_for(pb, capture, kind, seen)
        if s is None:
            print(f"  instance {n}: never appeared")
            break
        seen.add(s)
        report(f"instance {n}", kind, trace(pb, s, kind, FLY_Y), pb)
    pb.stop()
    return 0


if __name__ == "__main__":
    sys.exit(main())
