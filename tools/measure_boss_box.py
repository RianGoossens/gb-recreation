# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Sweep Mario across an object in any level and find where contact hurts.

`measure_enemy_box.py` does this in World 1-1 and reboots the game for every
offset, which is fine there and hopeless for a boss: reaching World 1-3 means
playing through two levels first, so 41 offsets would be 41 playthroughs.
This reaches the level once, snapshots the moment the object is on screen,
and restores that snapshot for each trial.

Three earlier versions of this measured nothing, and what was wrong is
recorded in `docs/reference/objects.md` and in `tools/probe_death_state.py`.
The short version: crossing two levels by pinning Mario's y and phase bytes
leaves him unstepped, so nothing can hurt him and nothing can kill him, and
the pit control that should have caught it was clearing the wrong tilemap
columns. `thaw` puts three bytes of his block back and hands him to the game
again; `dig` clears all 32 columns of the map.

Two controls, in this order, and both have to pass before any number here
means anything:

1. The pit. If taking the floor away does not cost him a life, nothing
   measured from this state would.
2. An ordinary enemy in the same room, swept the same way, which has to give
   a window with both edges inside the swept range.

**Result: the pit control now passes and the enemy control still does not.**
From a restored, thawed World 1-3, a dug pit costs him a life, and an
ordinary enemy laid exactly on top of him for 300 frames costs him nothing,
at every one of the 41 offsets. Two things that could have explained that are
ruled out. Writing the object rather than Mario is not it: the same rig run
in World 1-1 (`uv run tools/measure_boss_box.py 1-1 0x00 0x00 x`) hurts him
at every offset in the sweep. And his screen position is not it either, since
the object is placed from his own bytes each frame.

What is left is a difference in the bytes. Byte 1 of the slot record holds 1
for World 1-1's walker while it is hurting him, and 0 for every object in
World 1-3 reached by flying. Writing 1 into it every frame does not stick:
the game puts it back to 0 on the same frame. Mario also stands at y 22 in
mid air here against y 134 on the floor in World 1-1, and `land` cannot fix
that without losing the object. So the named next step is to find out what
byte 1 of a slot record is (`tools/watch_object_slot.py` follows one slot
through a plain walk with the terrain intact and can say when it turns 1).
If it means the object has been woken by Mario arriving, the
flatten-and-fly walkthrough can never measure contact and the route has to be
a real playthrough.

Usage: uv run tools/measure_boss_box.py [level] [kind] [control-kind] [axis]
"""

import sys

sys.path.insert(0, "tools")

from dump_object_slots import SLOTS, SLOT_BASE, SLOT_SIZE, slot
from probe_death_state import dig, thaw, watch_for_death
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


def pit_control(pb, state_bytes):
    """Does anything at all cost him a life from this state?

    A control that uses no enemy, run from a restored snapshot exactly as the
    trials are, so it tests the state the trials actually run in.
    """
    restore(pb, state_bytes)
    dig(pb)
    return watch_for_death(pb, WATCH) is not None


def trial(pb, state_bytes, s, axis, offset):
    """Hold the object at an offset from Mario and report whether it hurt.

    Mario is not written at all. The earlier version moved him instead, by
    writing `0xC202`, and that is his position on screen rather than his
    position in the level: at the start of World 1-1 the two are the same
    number, which is where every sweep that worked was run, and by World 1-3
    they are not. The object's slot holds a screen position too, so putting
    the object where Mario is measures the same thing with nothing but the
    object being written.
    """
    restore(pb, state_bytes)
    lives = pb.memory[LIVES]
    for _ in range(WATCH):
        mx, my = pb.memory[SCREEN_X], pb.memory[MARIO_Y]
        x = mx - offset if axis == "x" else mx
        y = my if axis == "x" else my - offset
        pb.memory[SLOT_BASE + s * SLOT_SIZE + 3] = max(min(x, 250), 8)
        pb.memory[SLOT_BASE + s * SLOT_SIZE + 2] = max(min(y, 250), 8)
        pb.tick()
        if pb.memory[LIVES] < lives:
            return True
    return False


def land(pb):
    """Thaw him and try to put him on the level's own floor.

    Thawing alone leaves him standing in mid air at the height the fly loop
    left him. Right lands him, at y 38 from y 22, and 240 frames of it scroll
    the object out of every slot; left does not land him at all, which is
    what this does now, so a sweep still runs with him in the air.
    """
    thaw(pb, frames=0)
    pb.button_press("left")
    for _ in range(30):
        pb.tick()
    pb.button_release("left")
    for _ in range(150):
        pb.tick()
    print(f"  standing at y {pb.memory[MARIO_Y]}, phase {pb.memory[PHASE]}")


def watch(pb, state_bytes, s, axis, offset):
    """Print what one trial actually does, frame by frame."""
    restore(pb, state_bytes)
    for frame in range(WATCH):
        mx, my = pb.memory[SCREEN_X], pb.memory[MARIO_Y]
        x = mx - offset if axis == "x" else mx
        y = my if axis == "x" else my - offset
        pb.memory[SLOT_BASE + s * SLOT_SIZE + 3] = max(min(x, 250), 8)
        pb.memory[SLOT_BASE + s * SLOT_SIZE + 2] = max(min(y, 250), 8)
        pb.tick()
        if frame % 60 == 0:
            state = slot(pb, s)
            record = " ".join(f"{b:02X}" for b in state)
            him = " ".join(f"{pb.memory[a]:02X}" for a in range(0xC200, 0xC210))
            print(f"    frame {frame}: lives {pb.memory[LIVES]}\n"
                  f"      slot   {record}\n"
                  f"      Mario  {him}")


def sweep(pb, kind, axis, check_pit, verbose=False):
    """Sweep one kind and report its window, or None if it never hurt."""
    s = approach(pb, kind)
    if s is None:
        print(f"  kind 0x{kind:02X} never came on screen")
        return None
    land(pb)
    # Landing takes 300 frames, and the object can change slots in that time,
    # so ask which slot holds the kind now rather than trusting the old one.
    held = [i for i in range(SLOTS) if slot(pb, i)[0] == kind]
    if not held:
        print(f"  kind 0x{kind:02X} left every slot while he was landing")
        return None
    s = held[0]
    state = slot(pb, s)
    ox, oy = state[3], state[2]
    here = snapshot(pb)
    print(f"  kind 0x{kind:02X} in slot {s} at x {ox}, y {oy}")

    if check_pit and not pit_control(pb, here):
        print("  a pit does not cost him a life from this state, so nothing "
              "measured from here would mean anything")
        return None
    if verbose:
        watch(pb, here, s, axis, 0)
    hits = [o for o in SPAN if trial(pb, here, s, axis, o)]
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
    ok = sweep(pb, control, axis, check_pit=True, verbose=True)
    if ok is None:
        print("\nthe control found no window, so this instrument cannot "
              "detect a death and any result below would be meaningless")
        pb.stop()
        return 1

    print(f"\nthe boss, kind 0x{kind:02X}, sweeping {axis}")
    sweep(pb, kind, axis, check_pit=False)
    pb.stop()
    return 0


if __name__ == "__main__":
    sys.exit(main())
