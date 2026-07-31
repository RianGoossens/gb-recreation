# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Does touching this object kind hurt Mario?

Motion alone does not say what an object is. World 1-3's `0x02` moves 16 px up
and down on a 200-frame cycle, which fits a lift and fits a crusher equally
well, and dropping Mario from above answers nothing because those records sit
inside terrain he lands on first.

Putting him inside the object does answer it. Write his position straight into
the two bytes the game reads, leave him there, and watch the life counter. A
hazard takes one within a few frames; a platform does not.

Two controls run alongside, because an instrument that says "hurt" to
everything is worth nothing. The positive control is World 1-1's kind `0x00`,
a walker that is definitely an enemy. The negative is Mario left in open air
at the same height with no object at all.

Usage: uv run tools/probe_object_contact.py [kind] [1-1|1-2|1-3]
"""

import sys

sys.path.insert(0, "tools")

from dump_object_slots import SLOTS, slot
from run_through_levels import FLY_Y, LIVES, MARIO_Y, PHASE, SCREEN_X, SPAWN_X
from sml_boot import boot_to_gameplay
from trace_level_objects import NAMES, reach_level

APPROACH = 5000
# A death is not instant: forcing Mario into World 1-1's first walker costs him
# a life on frame 212, after a death animation the counter does not move
# during. An earlier 180-frame window reported the walker as harmless.
WATCH = 400
# Standing on a lift puts Mario's Y byte 10 below the object's slot Y
# (`tools/measure_lift.py`). That is not the offset to test contact at: feet on
# top of a walker is a stomp, and the first run of this probe had its positive
# control come back clean for exactly that reason. So the whole overlap is
# swept and the object's own slot is watched, which separates "it hurt him"
# from "he landed on it and killed it".
OFFSETS = [10, 6, 2, 0, -4, -8]


def approach(kind, level):
    if level == "1-1":
        pb = boot_to_gameplay()
        pb.button_press("right")
    else:
        pb, _capture = reach_level(NAMES.index(level))
        if pb is None:
            return None, None
    for _ in range(APPROACH):
        pb.memory[MARIO_Y] = FLY_Y
        if pb.memory[SCREEN_X] > SPAWN_X:
            pb.memory[PHASE] = 0
        pb.tick()
        if pb.memory[LIVES] == 0:
            break
        for s in range(SLOTS):
            state = slot(pb, s)
            if state[0] == kind and 50 <= state[3] <= 140:
                pb.button_release("right")
                return pb, s
    pb.button_release("right")
    pb.stop()
    return None, None


def watch(pb, s, kind, at_x, at_y):
    """Put Mario at a screen position and report what happens to each of them."""
    lives = pb.memory[LIVES]
    pb.memory[SCREEN_X] = at_x
    pb.memory[MARIO_Y] = at_y
    hurt = False
    gone = False
    for _ in range(WATCH):
        pb.tick()
        if pb.memory[LIVES] < lives:
            hurt = True
            break
        if s is not None and slot(pb, s)[0] != kind:
            gone = True
    return hurt, gone


def sweep(kind, level, label):
    """Try every overlap, and report the first that costs Mario a life."""
    print(f"{label}:")
    hurt_at = None
    for offset in OFFSETS:
        pb, s = approach(kind, level)
        if pb is None:
            print(f"  kind 0x{kind:02X} never came on screen")
            return None
        state = slot(pb, s)
        hurt, gone = watch(pb, s, kind, state[3], state[2] - offset)
        pb.stop()
        # The slot also empties when Mario dies and the level resets, so this
        # only means anything on a trial he survived.
        note = " (and the object went away)" if gone and not hurt else ""
        print(f"  offset {offset:+3d}: {'lost a life' if hurt else 'unharmed'}{note}")
        if hurt and hurt_at is None:
            hurt_at = offset
    return hurt_at


def clear_air(kind, level):
    """The same sweep with Mario placed well away from any object."""
    pb, s = approach(kind, level)
    if pb is None:
        return None
    state = slot(pb, s)
    hurt, _gone = watch(pb, None, kind, (state[3] + 60) % 150, state[2] - OFFSETS[0])
    pb.stop()
    return hurt


def main():
    kind = int(sys.argv[1], 0) if len(sys.argv) > 1 else 0x02
    level = sys.argv[2] if len(sys.argv) > 2 else "1-3"

    control = sweep(0x00, "1-1", "positive control, World 1-1 kind 0x00 (a walker)")
    print()
    clear = clear_air(0x00, "1-1")
    print(f"negative control, Mario 60 px to the side: "
          f"{'lost a life' if clear else 'unharmed'}\n")

    if control is None or clear:
        print("the controls disagree with themselves, so nothing below is trusted")
        return 1
    print(f"the control is hurt from offset {control:+d}\n")

    hurt_at = sweep(kind, level, f"World {level}, kind 0x{kind:02X}")
    if hurt_at is None:
        print(f"\nkind 0x{kind:02X} never hurt Mario at any overlap")
    else:
        print(f"\nkind 0x{kind:02X} hurts Mario, from offset {hurt_at:+d}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
