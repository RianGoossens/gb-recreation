# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Does World 1-3's boss hurt Mario, and can he be landed on?

`measure_level_kind.py` says what King Totomesu (kind `0x08`) does with the
camera frozen: he never moves sideways and leaps 20 pixels straight up and
back down on a 162-frame cycle. What that cannot say is what happens on
contact, which is the half that decides whether the engine treats him as an
enemy, a wall, or something with its own rule.

Three placements, each holding Mario at a fixed offset from the boss's slot
for long enough that a death would register (212 frames on the cartridge):

    beside      overlapping him at his own height
    on top      resting where a surface would hold Mario, 10px above the slot
    clear       well to the left, which is the control

The control is what makes a "no death" result worth anything: a probe that
can only answer yes is measuring itself. If the clear trial also reports a
death then something else in the room is killing him and the other two say
nothing.

Both positions are written every frame, so the boss's own leap cannot carry
him out of the offset being tested.

Result so far: **inconclusive, and the positive control is what says so.**
Kind `0x3F`, an ordinary enemy standing in the same room, comes back
"unharmed" from the `beside` placement too, so this instrument cannot detect
a death at that offset and the boss's "unharmed" means nothing. A single
offset was the mistake: `measure_enemy_box.py` sweeps a range because the
slot's anchor and Mario's are not the same point, and dx=0 need not be an
overlap at all. Sweeping is the next instrument to build here.

Both kinds do leave their slot under the `on top` placement, which is a
stomp for the ordinary enemy and an open question for the boss.

Usage: uv run tools/probe_boss_contact.py [level] [kind] [control-kind]
"""

import sys

sys.path.insert(0, "tools")

from dump_object_slots import EMPTY, SLOTS, SLOT_BASE, SLOT_SIZE, slot
from run_through_levels import FLY_Y, LIVES, MARIO_Y, PHASE, SCREEN_X, SPAWN_X
from trace_level_objects import NAMES, reach_level

APPROACH = 5000
# A death is not instant: forcing Mario into World 1-1's first walker costs a
# life on frame 212, after an animation the counter does not move during.
WATCH = 320
# Mario rests this far above a surface's own slot y (measured on the lifts).
REST = 10


def find(pb, want):
    """Fly right until a slot holds `want`, and return the slot index."""
    for _ in range(APPROACH):
        pb.memory[MARIO_Y] = FLY_Y
        if pb.memory[SCREEN_X] > SPAWN_X:
            pb.memory[PHASE] = 0
        pb.tick()
        if pb.memory[LIVES] == 0:
            return None
        for i in range(SLOTS):
            if slot(pb, i)[0] == want:
                return i
    return None


def hold(pb, s, dx, dy):
    """Pin Mario at (dx, dy) from the boss's slot and watch the life counter."""
    lives = pb.memory[LIVES]
    for _ in range(WATCH):
        state = slot(pb, s)
        if state[0] == EMPTY:
            return "the object left its slot"
        pb.memory[SCREEN_X] = max(0, min(255, state[3] + dx))
        pb.memory[MARIO_Y] = max(0, min(255, state[2] + dy))
        pb.memory[PHASE] = 0
        pb.tick()
        if pb.memory[LIVES] < lives:
            return "cost a life"
    return "unharmed"


def probe(pb, want):
    s = find(pb, want)
    if s is None:
        print(f"kind 0x{want:02X} never appeared")
        return
    state = slot(pb, s)
    print(f"kind 0x{want:02X} in slot {s} at x {state[3]}, y {state[2]}")
    pb.button_release("right")
    for label, dx, dy in (("clear (control)", -60, 0), ("beside", 0, 0),
                          ("on top", 0, -REST)):
        print(f"  {label:16} -> {hold(pb, s, dx, dy)}")
        now = slot(pb, s)[0]
        if now != want:
            print(f"     the slot now holds 0x{now:02X}, so later trials "
                  f"would be measuring something else")
            break


def main():
    level = sys.argv[1] if len(sys.argv) > 1 else "1-3"
    want = int(sys.argv[2], 0) if len(sys.argv) > 2 else 0x08
    # The positive control: an ordinary enemy in the same room, which has to
    # come back "cost a life" or the instrument cannot detect one.
    control = int(sys.argv[3], 0) if len(sys.argv) > 3 else 0x3F

    pb, _ = reach_level(NAMES.index(level))
    if pb is None:
        print(f"never reached World {level}")
        return 1
    print(f"positive control first, kind 0x{control:02X}")
    probe(pb, control)
    print(f"the boss, kind 0x{want:02X}")
    probe(pb, want)
    pb.stop()
    return 0


if __name__ == "__main__":
    sys.exit(main())
