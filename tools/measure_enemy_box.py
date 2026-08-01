# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""How big is an enemy's collision box?

The engine has used 8 by 8 for every kind since before any object list was
read, and nothing has ever measured it. This measures it the way the lift's
surface was measured: sweep Mario across the object a pixel at a time and see
where contact starts and stops costing him a life. The window that produces is
the enemy's size plus Mario's minus one, and Mario's box is already known from
the cartridge at 11 by 12 (`tools/measure_mario_box.py`), so the enemy's falls
out of it.

The catch a walker adds over a lift is that it walks. Over the frames a death
takes to register it covers more than a screen, so it would reach Mario from
any starting offset and every trial would report a hit. So both positions are
written every frame, which holds the two of them still relative to each other
for as long as the trial runs.

Two controls, because a probe that only says yes is measuring itself. Well
clear of the object on either side has to come back unharmed, and the window
has to have both edges inside the swept range rather than running off the end.

Usage: uv run tools/measure_enemy_box.py [kind] [axis]
  axis is `x` or `y`; `x` sweeps across the object, `y` down through it.
"""

import sys

sys.path.insert(0, "tools")

from dump_object_slots import SLOTS, slot, SLOT_BASE, SLOT_SIZE
from run_through_levels import FLY_Y, LIVES, MARIO_Y, PHASE, SCREEN_X, SPAWN_X
from sml_boot import boot_to_gameplay

APPROACH = 5000
# A death is not instant. Forcing Mario into World 1-1's first walker costs him
# a life on frame 212, after an animation the counter does not move during.
WATCH = 320
# Mario's box, measured on the cartridge by walling him into corridors.
MARIO_WIDTH = 11
MARIO_HEIGHT = 12
SPAN = range(-18, 19)


def approach(kind):
    pb = boot_to_gameplay()
    pb.button_press("right")
    for _ in range(APPROACH):
        pb.memory[MARIO_Y] = FLY_Y
        if pb.memory[SCREEN_X] > SPAWN_X:
            pb.memory[PHASE] = 0
        pb.tick()
        if pb.memory[LIVES] == 0:
            break
        for s in range(SLOTS):
            state = slot(pb, s)
            if state[0] == kind and 60 <= state[3] <= 120:
                pb.button_release("right")
                return pb, s
    pb.button_release("right")
    pb.stop()
    return None, None


def trial(kind, axis, offset):
    """Hold Mario at an offset from the object and report whether it hurt."""
    pb, s = approach(kind)
    if pb is None:
        return None
    state = slot(pb, s)
    ox, oy = state[3], state[2]
    # Fully overlapping on the axis not being swept, so the sweep is measuring
    # one edge at a time rather than a corner.
    mx = ox + offset if axis == "x" else ox
    my = oy if axis == "x" else oy + offset
    lives = pb.memory[LIVES]
    hurt = False
    for _ in range(WATCH):
        # Both of them, every frame. A walker covers more than a screen in the
        # time a death takes to register, so leaving it to move would have it
        # reach Mario from every starting offset.
        pb.memory[SLOT_BASE + s * SLOT_SIZE + 3] = ox
        pb.memory[SLOT_BASE + s * SLOT_SIZE + 2] = oy
        pb.memory[SCREEN_X] = max(min(mx, 250), 8)
        pb.memory[MARIO_Y] = max(min(my, 250), 8)
        pb.tick()
        if pb.memory[LIVES] < lives:
            hurt = True
            break
    pb.stop()
    return hurt


def main():
    kind = int(sys.argv[1], 0) if len(sys.argv) > 1 else 0x00
    axis = sys.argv[2] if len(sys.argv) > 2 else "x"
    mario = MARIO_WIDTH if axis == "x" else MARIO_HEIGHT

    print(f"kind 0x{kind:02X}, sweeping {axis}\n")
    hits = []
    for offset in SPAN:
        hurt = trial(kind, axis, offset)
        if hurt is None:
            print(f"  kind 0x{kind:02X} never came on screen")
            return 1
        print(f"  offset {offset:+3d}: {'lost a life' if hurt else 'unharmed'}")
        if hurt:
            hits.append(offset)

    if not hits:
        print("\nnothing hurt him anywhere, so this measured nothing")
        return 1
    lo, hi = hits[0], hits[-1]
    if lo == SPAN.start or hi == SPAN.stop - 1:
        print("\nthe window runs off the end of the sweep, so it is not a measurement")
        return 1
    if hits != list(range(lo, hi + 1)):
        print(f"\nthe window has holes in it: {hits}")
        return 1
    window = hi - lo + 1
    print(f"\nhurt from {lo:+d} to {hi:+d}, a window of {window}")
    print(f"minus Mario's {mario} gives an enemy {window - mario + 1} across this axis")
    return 0


if __name__ == "__main__":
    sys.exit(main())
