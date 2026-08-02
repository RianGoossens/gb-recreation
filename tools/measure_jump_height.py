# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""How high does Mario actually jump, and does running change it?

`docs/reference/physics.md` puts the max-hold jump at 24 to 25 pixels, traced
from a standing start. World 1-1's own geometry disagrees: a pillar at columns
78 and 79 rises four tiles above the ground either side of it, so getting past
it needs a 32 pixel climb, and the level is verified column for column against
the running cartridge. Two measurements that cannot both be right, and the one
nobody has varied is the run-up.

Super Mario Bros scales jump velocity with horizontal speed. If Super Mario
Land does the same, a standing 24 and a running 40 are both true and the
engine is missing the term.

So jump from a standstill and jump at full speed from the same snapshot, and
read the peak off `0xC201` either way. The control is the standing trial
reproducing the 24 already in the document: if it does not, the instrument is
wrong rather than the model.

Usage: uv run tools/measure_jump_height.py
"""

import sys

sys.path.insert(0, "tools")

from sml_boot import boot_to_gameplay, restore, snapshot

MARIO_Y = 0xC201
SPEED = 0xC20C
HOLD = 40
WATCH = 90


def trial(pb, state, run_up, run_b, air_right, air_b, hold=HOLD):
    """Peak rise in pixels and the speed jumped at.

    `run_up` frames of running right first (with B if `run_b`), then A held
    for `hold` frames with Right and B held during the flight only as
    `air_right` and `air_b` say. Separating the run-up from the flight is the
    whole point: a run-up of one frame leaves the speed register at 0 and
    still changes the jump, so what matters is not the speed reached.
    """
    restore(pb, state)
    for name, on in (("right", run_up > 0), ("b", run_b)):
        (pb.button_press if on else pb.button_release)(name)
    for _ in range(run_up):
        pb.tick()
    start = pb.memory[MARIO_Y]
    speed = pb.memory[SPEED]
    peak = start
    for name, on in (("right", air_right), ("b", air_b)):
        (pb.button_press if on else pb.button_release)(name)
    pb.button_press("a")
    for f in range(WATCH):
        if f == hold:
            pb.button_release("a")
        pb.tick()
        peak = min(peak, pb.memory[MARIO_Y])
    for name in ("a", "right", "b"):
        pb.button_release(name)
    return speed, start - peak


def main():
    pb = boot_to_gameplay()
    state = snapshot(pb)

    print("per-frame dy for the three heights")
    for label, (run_up, run_b, air_right, air_b) in (
        ("standing 24", (0, False, False, False)),
        ("moving   33", (0, False, True, False)),
        ("running  41", (30, True, True, True)),
    ):
        restore(pb, state)
        for name, on in (("right", run_up > 0), ("b", run_b)):
            (pb.button_press if on else pb.button_release)(name)
        for _ in range(run_up):
            pb.tick()
        for name, on in (("right", air_right), ("b", air_b)):
            (pb.button_press if on else pb.button_release)(name)
        pb.button_press("a")
        prev = pb.memory[MARIO_Y]
        dys = []
        for _ in range(34):
            pb.tick()
            now = pb.memory[MARIO_Y]
            dys.append(now - prev)
            prev = now
        for name in ("a", "right", "b"):
            pb.button_release(name)
        print(f"  {label}: {dys}")

    print("run-up  runB  airRight  airB  speed  rise (px)")
    cases = [
        (0, False, False, False),
        (0, False, True, False),
        (0, False, True, True),
        (0, False, False, True),
        (30, False, False, False),
        (30, False, True, False),
        (30, True, False, False),
        (30, True, True, True),
        (30, True, True, False),
        (30, True, False, True),
    ]
    for run_up, run_b, air_right, air_b in cases:
        speed, rise = trial(pb, state, run_up, run_b, air_right, air_b)
        print(f"{run_up:6d}  {run_b!s:>5}  {air_right!s:>8}  {air_b!s:>4}  "
              f"{speed:5d}  {rise:8d}")
    pb.stop()
    return 0


if __name__ == "__main__":
    sys.exit(main())
