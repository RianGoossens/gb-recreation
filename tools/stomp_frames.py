# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy", "pillow"]
# ///
"""Dump screenshots around one candidate stomp, to see what actually happened.

`measure_stomp_bounce.py` finds frames where Mario's vertical phase flips
from falling to rising while airborne. That is the right signature, but the
sprite dump alone does not say whether an enemy was under him. This renders
the frames so it can be looked at instead of inferred.

Usage: uv run tools/stomp_frames.py <wait> <hold>
"""

import sys

from measure_stomp_bounce import find_approach, find_stomp, trial
from sml_boot import boot_to_gameplay, restore

OUT = "/tmp/stomp"


def main():
    wait = int(sys.argv[1])
    hold = int(sys.argv[2])

    pb = boot_to_gameplay()
    state, gap = find_approach(pb)
    if state is None:
        print("no enemy came into view")
        return 1

    print(f"enemy {gap}px ahead at snapshot")
    trace = trial(pb, state, wait, hold)
    at = find_stomp(trace, require_vanish=False)
    if at is None:
        print("no phase flip for that combination")
        print("phases:", "".join(str(t[1]) for t in trace))
        return 1

    import os

    os.makedirs(OUT, exist_ok=True)

    restore(pb, state)
    pb.button_press("right")
    for _ in range(wait):
        pb.tick()
    pb.button_press("a")
    for f in range(at + 6):
        if f == hold:
            pb.button_release("a")
        pb.tick()
        if at - 4 <= f <= at + 5:
            pb.screen.image.save(f"{OUT}/f{f:03d}.png")
    print(f"wrote frames {at - 4}..{at + 5} to {OUT} (bounce at {at})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
