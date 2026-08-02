# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Sweep Mario across an object in any level and find where contact hurts.

`measure_enemy_box.py` does this in World 1-1 and reboots the game for every
offset, which is fine there and hopeless for a boss: reaching World 1-3 means
playing through two levels first, so 37 offsets would be 37 playthroughs.
This reaches the level once, snapshots the moment the object is on screen,
and restores that snapshot for each trial.

The probe it replaces (`tools/probe_boss_contact.py`) tested a single offset
and found the boss harmless, and found an ordinary enemy in the same room
harmless too, which is what said the offset was the problem rather than the
boss: the slot's anchor and Mario's are not the same point, so a difference of
zero between them need not be an overlap at all.

The control kind is swept first and has to produce a window with both edges
inside the swept range. Without that, "the boss never hurt him" says nothing.

Result: **still inconclusive, and the failure moved.** Sweeping the whole
range in World 1-3 leaves the positive control with no window at all: kind
`0x3F`, an ordinary enemy, does not cost Mario a life at any of the 41
offsets. So the offset was not the problem, and something about reaching this
level the way `reach_level` does leaves Mario in a state nothing collides
with. The obvious suspect is the fly loop, which pins his Y byte every frame
for thousands of frames to carry him over the level; `measure_enemy_box.py`
works in World 1-1 where the same loop runs for far fewer frames.

That is two failures on this question, so it is recorded as unmeasured
(`docs/reference/objects.md`) rather than attempted a third time. What would
break the tie is a control that does not depend on an enemy at all: whether
walking Mario into a pit in the same restored state costs him a life. If it
does not, the state is the problem and no contact probe from here can work.

Usage: uv run tools/measure_boss_box.py [level] [kind] [control-kind] [axis]
"""

import sys

sys.path.insert(0, "tools")

from dump_object_slots import SLOTS, SLOT_BASE, SLOT_SIZE, slot
from run_through_levels import FLY_Y, LIVES, MARIO_Y, PHASE, SCREEN_X, SPAWN_X
from sml_boot import restore, snapshot
from trace_level_objects import NAMES, reach_level

APPROACH = 5000
# A death is not instant: forcing Mario into World 1-1's first walker costs a
# life on frame 212, after an animation the counter does not move during.
WATCH = 300
# Mario's box, measured on the cartridge by walling him into corridors.
MARIO_WIDTH = 11
MARIO_HEIGHT = 12
SPAN = range(-20, 21)


def approach(pb, kind):
    """Fly right until `kind` is on screen in a readable place."""
    for _ in range(APPROACH):
        pb.memory[MARIO_Y] = FLY_Y
        if pb.memory[SCREEN_X] > SPAWN_X:
            pb.memory[PHASE] = 0
        pb.tick()
        if pb.memory[LIVES] == 0:
            return None
        for s in range(SLOTS):
            state = slot(pb, s)
            if state[0] == kind and 60 <= state[3] <= 120:
                pb.button_release("right")
                return s
    pb.button_release("right")
    return None


def trial(pb, state_bytes, s, ox, oy, axis, offset):
    """Hold Mario at an offset from the object and report whether it hurt."""
    restore(pb, state_bytes)
    mx = ox + offset if axis == "x" else ox
    my = oy if axis == "x" else oy + offset
    lives = pb.memory[LIVES]
    for _ in range(WATCH):
        # The object too, every frame. A boss that leaps 20 pixels would carry
        # itself out of the offset being tested otherwise.
        pb.memory[SLOT_BASE + s * SLOT_SIZE + 3] = ox
        pb.memory[SLOT_BASE + s * SLOT_SIZE + 2] = oy
        pb.memory[SCREEN_X] = max(min(mx, 250), 8)
        pb.memory[MARIO_Y] = max(min(my, 250), 8)
        pb.tick()
        if pb.memory[LIVES] < lives:
            return True
    return False


def sweep(pb, kind, axis):
    """Sweep one kind and report its window, or None if it never hurt."""
    s = approach(pb, kind)
    if s is None:
        print(f"  kind 0x{kind:02X} never came on screen")
        return None
    state = slot(pb, s)
    ox, oy = state[3], state[2]
    here = snapshot(pb)
    print(f"  kind 0x{kind:02X} in slot {s} at x {ox}, y {oy}")

    hits = [o for o in SPAN if trial(pb, here, s, ox, oy, axis, o)]
    if not hits:
        print("  nothing hurt him anywhere in the sweep")
        return None
    lo, hi = hits[0], hits[-1]
    print(f"  hurt from {lo:+d} to {hi:+d}" +
          ("" if hits == list(range(lo, hi + 1)) else f" with holes: {hits}"))
    if lo == SPAN.start or hi == SPAN.stop - 1:
        print("  the window runs off the end of the sweep, so it is not a size")
        return (lo, hi, False)
    mario = MARIO_WIDTH if axis == "x" else MARIO_HEIGHT
    print(f"  a window of {hi - lo + 1} minus Mario's {mario} gives "
          f"{hi - lo + 1 - mario + 1} across this axis")
    return (lo, hi, True)


def main():
    level = sys.argv[1] if len(sys.argv) > 1 else "1-3"
    kind = int(sys.argv[2], 0) if len(sys.argv) > 2 else 0x08
    control = int(sys.argv[3], 0) if len(sys.argv) > 3 else 0x3F
    axis = sys.argv[4] if len(sys.argv) > 4 else "x"

    pb, _ = reach_level(NAMES.index(level))
    if pb is None:
        print(f"never reached World {level}")
        return 1

    print(f"positive control, kind 0x{control:02X}, sweeping {axis}")
    ok = sweep(pb, control, axis)
    if ok is None:
        print("\nthe control found no window, so this instrument cannot "
              "detect a death and any result below would be meaningless")
        pb.stop()
        return 1

    print(f"\nthe boss, kind 0x{kind:02X}, sweeping {axis}")
    sweep(pb, kind, axis)
    pb.stop()
    return 0


if __name__ == "__main__":
    sys.exit(main())
