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

Result: **still unmeasured, and the tie-breaker has now been run.** Sweeping
the whole range in World 1-3 left the positive control with no window at all:
kind `0x3F`, an ordinary enemy, cost Mario no life at any of the 41 offsets.
So the offset was never the problem, and something about the state these
trials run in leaves him unable to be hurt.

The control that settles that uses no enemy: take the floor out of the tilemap
under him, write nothing else at all, and see whether the game's own fall
costs him a life. It does not. So the state is the problem, and no contact
probe from here can work whatever it points at. (Pinning his Y byte below the
screen is not a fall and answers "unharmed" from any state, which is why the
first version of this control was worthless.)

Two suspects, and the second is the better one. The fly loop pins his Y byte
for thousands of frames to carry him across two levels, and `settle` is here
to undo that: it lets go for 90 frames so he lands, which is what
`tools/probe_ceiling_cap.py` does before walking him, and there he collides
with terrain normally. That leaves the snapshot. Every trial here begins with
`restore`, and `tools/measure_lift.py` already recorded that restoring a save
state per trial silently breaks a placement experiment: it dropped Mario
through a lift at every offset, including ones a continuous run held him at.
The next thing to try is the pit control with no restore in front of it,
inside one continuous approach.

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
# Frames of letting go before a trial, so he lands and the fly loop's state
# is behind him.
SETTLE = 90
# Mario's box, measured on the cartridge by walling him into corridors.
MARIO_WIDTH = 11
MARIO_HEIGHT = 12
SPAN = range(-20, 21)
TILEMAP = 0x9800


def settle(pb):
    """Stop writing to him and let him land.

    Two earlier runs of this failed with the positive control finding no
    window anywhere, which said the state Mario was left in by the fly loop
    was the problem rather than the offset. The loop pins his Y byte every
    frame for thousands of frames to carry him over the level, and whatever
    that leaves him in, nothing collides with it. Letting go for a while and
    letting him fall to the floor is what `tools/probe_ceiling_cap.py` does
    before it walks him, and there he collides with terrain normally.
    """
    pb.button_release("right")
    for _ in range(SETTLE):
        pb.tick()


def pit_control(pb, state_bytes):
    """Does anything at all cost him a life from this state?

    A control that uses no enemy. If a pit does not cost a life either, no
    contact probe run from here can mean anything, and that is a fact about
    the state rather than about the boss.

    The pit is a real one: the floor is taken out of the tilemap under him and
    then nothing is written at all, so what follows is the game's own falling
    and its own death. Pinning his Y byte below the screen instead is not a
    fall, and it answers "unharmed" whatever the state is.
    """
    restore(pb, state_bytes)
    column = pb.memory[SCREEN_X] // 8
    for row in range(2, 18):
        for c in range(max(0, column - 2), column + 3):
            pb.memory[TILEMAP + (row % 32) * 32 + (c % 32)] = 0x00
    lives = pb.memory[LIVES]
    for _ in range(WATCH):
        pb.tick()
        if pb.memory[LIVES] < lives:
            return True
    return False


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
    settle(pb)
    state = slot(pb, s)
    ox, oy = state[3], state[2]
    here = snapshot(pb)
    print(f"  kind 0x{kind:02X} in slot {s} at x {ox}, y {oy}")

    if not pit_control(pb, here):
        print("  a pit does not cost him a life from this state either, so "
              "nothing measured from here would mean anything")
        return None
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
