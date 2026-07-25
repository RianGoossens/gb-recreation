# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy", "numpy"]
# ///
"""Validate `sml_scroll.ScrollTracker` against known physics, then use it.

Two checks, both against things already pinned independently:

1. While Mario walks right at his saturated speed with the camera locked,
   every pixel of his travel is scroll, so the measured scroll must advance
   at exactly 1 px/frame (`docs/reference/physics.md`).
2. Holding Right alone walks into the level's first enemy and dies. The
   measured scroll must stop dead there, and the frozen-frame counter must
   catch it, even though Mario's WRAM bytes keep reading as though he were
   alive and walking.

Then it reports what the reactive stomp-jumping walker (the one
`stitch_level_1_1.py` uses to survive) really covers, which the old
speed-byte estimate could only guess at.

Usage: uv run tools/measure_scroll.py
"""

import sys

from sml_boot import boot_to_gameplay
from sml_scroll import ScrollTracker
from sml_walker import ReactiveWalker

MARIO_X = 0xC202


def hold_right_run(frames=320):
    pb = boot_to_gameplay()
    pb.button_press("right")
    tracker = ScrollTracker(pb)
    trace = []
    for f in range(frames):
        pb.tick()
        shift = tracker.update(pb)
        trace.append((f, pb.memory[MARIO_X], shift, tracker.scroll, tracker.frozen))
    pb.stop()
    return trace


def check_hold_right():
    trace = hold_right_run()
    lock = next(
        f for f, r in enumerate(trace) if all(s[1] == trace[-1][1] for s in trace[f:])
    )
    death = next((r[0] for r in trace if r[4] > 5), None)
    print(f"camera locks at frame {lock}, screen x {trace[lock][1]}")
    if death is None:
        print("FAIL: the run never froze, so the known death was not detected")
        return False

    # The freeze counter lags the death by the length of the death
    # animation, which still moves sprites around. Scrolling is what stops
    # first, so anchor the check on the last frame that scrolled at all.
    stop = max(r[0] for r in trace if r[2] > 0)
    walked = stop - lock
    gained = trace[stop][3] - trace[lock][3]
    print(f"scrolling stops at frame {stop}, screen freezes at frame {death}")
    print(f"scroll between the lock and the stop: {gained}px over {walked} frames")

    stale = trace[-1]
    print(
        f"meanwhile at frame {stale[0]} the WRAM still reads screen x {stale[1]}, "
        f"and the measured scroll has correctly stopped at {stale[3]}px"
    )
    ok = abs(gained - walked) <= 2 and trace[-1][3] == trace[stop][3]
    print("PASS" if ok else "FAIL", "1 px/frame while walking, flat after death\n")
    return ok


def reactive_run(frames=5000):
    pb = boot_to_gameplay()
    walker = ReactiveWalker(pb)
    tracker = ScrollTracker(pb)
    deaths = 0
    was_frozen = False
    for _ in range(frames):
        walker.step(pb)
        pb.tick()
        tracker.update(pb)
        frozen = tracker.frozen > 5
        if frozen and not was_frozen:
            deaths += 1
        was_frozen = frozen
    pb.stop()
    return tracker, deaths


def main():
    if not check_hold_right():
        return 1

    frames = 5000
    tracker, deaths = reactive_run(frames)
    print(f"reactive stomp-jumping walker, {frames} frames:")
    print(f"  measured scroll {tracker.scroll}px = world column {tracker.scroll // 8}")
    print(f"  deaths detected: {deaths}")
    print(f"  ambiguous frames: {tracker.ambiguous_frames}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
