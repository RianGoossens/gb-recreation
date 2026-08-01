# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Does the drop block (kind 0x36) hold Mario before it goes away?

`probe_drop_block.py` dropped him on it and found the slot empties on the
frame his feet reach the surface. That cannot separate a block that catches
him for a frame and then gives way from one with no surface at all, because
he is falling three pixels a frame and the object leaves on the same frame it
would have caught him.

So this stops dropping him. It places him at rest height (the slot's y minus
10, where a lift holds him), tells the game he is on the ground, and then
writes nothing more: whatever happens next is the game's own collision. Three
trials from one snapshot each:

  object    the block is there
  freed     the same position with the slot's kind byte set to 0xFF first,
            which is the negative control: no object, so he must fall
  lift      the same procedure on World 1-1's lift, the positive control:
            a known surface, so he must stay

It also prints every slot's kind on the frame the object leaves and the two
after, which answers the other half: whether the block is removed outright or
handed off to a falling object in another slot.

Usage: uv run tools/probe_drop_block_support.py [kind] [level]
"""

import sys

sys.path.insert(0, "tools")

from dump_object_slots import EMPTY, SLOT_BASE, SLOT_SIZE, SLOTS, slot
from measure_object_sprites import entries_near
from run_through_levels import FLY_Y, LIVES, MARIO_Y, PHASE, SCREEN_X, SPAWN_X
from sml_boot import restore, snapshot
from trace_level_objects import NAMES, reach_level

APPROACH = 5000
REST = 10
WATCH = 90


def find(pb, want):
    """Walk right until a slot holds `want` somewhere near the middle."""
    for _ in range(APPROACH):
        pb.memory[MARIO_Y] = FLY_Y
        if pb.memory[SCREEN_X] > SPAWN_X:
            pb.memory[PHASE] = 0
        pb.tick()
        if pb.memory[LIVES] == 0:
            return None
        for s in range(SLOTS):
            state = slot(pb, s)
            if state[0] == want and 60 <= state[3] <= 120:
                return s
    return None


def kinds(pb):
    return [slot(pb, s)[0] for s in range(SLOTS)]


def place(pb, s, ox, oy):
    pb.memory[SCREEN_X] = ox
    pb.memory[MARIO_Y] = oy - REST
    pb.memory[PHASE] = 0


def trial(pb, state, s, ox, oy, want, free_it):
    restore(pb, state)
    if free_it:
        pb.memory[SLOT_BASE + s * SLOT_SIZE] = EMPTY
    place(pb, s, ox, oy)

    rows = []
    gone_at = None
    after = []
    for f in range(WATCH):
        pb.tick()
        my = pb.memory[MARIO_Y]
        here = slot(pb, s)[0]
        rows.append((f, my, oy - REST - my, here, pb.memory[PHASE],
                     slot(pb, s)[2]))
        if gone_at is None and here != want:
            gone_at = f
            after = [kinds(pb)]
        elif gone_at is not None and len(after) < 3:
            after.append(kinds(pb))
    return rows, gone_at, after


def report(label, rows, gone_at, after):
    print(f"\n--- {label}")
    print("frame  mario y  fallen  slot  phase  slot y  gap")
    for f, my, fallen, here, phase, sy in rows:
        name = "free" if here == EMPTY else f"0x{here:02X}"
        print(f"{f:5d}  {my:7d}  {fallen:6d}  {name:>4}  {phase:5d}  "
              f"{sy:6d}  {sy - my:4d}")
    held = 0
    for _f, _my, fallen, *_rest in rows:
        if fallen != 0:
            break
        held += 1
    print(f"he stayed at rest height for {held} frames, "
          f"and had fallen {rows[-1][2]} px after {len(rows)}")
    if gone_at is not None:
        print(f"the slot kind changed on frame {gone_at}; slot kinds that frame "
              f"and the two after:")
        for row in after:
            print("   " + " ".join("--" if k == EMPTY else f"{k:02X}" for k in row))


def sweep(pb, state, s, ox, oy, want):
    """Which offsets of Mario's own x does the block hold him at?

    The trigger fires on the first frame he is standing at rest height, and it
    shows up as the slot's kind byte changing, so three frames per offset is
    enough and the whole window costs one continuous run.
    """
    held = []
    for offset in range(-12, 45):
        restore(pb, state)
        target = ox + offset
        if not 8 <= target <= 250:
            continue
        pb.memory[SCREEN_X] = target
        pb.memory[MARIO_Y] = oy - REST
        pb.memory[PHASE] = 0
        caught = False
        for _ in range(3):
            pb.tick()
            if slot(pb, s)[0] != want:
                caught = True
        if caught:
            held.append(offset)
    return held


def main():
    want = int(sys.argv[1], 0) if len(sys.argv) > 1 else 0x36
    level = sys.argv[2] if len(sys.argv) > 2 else "1-2"

    pb, _capture = reach_level(NAMES.index(level))
    if pb is None:
        print("the run died before reaching the level")
        return 1
    s = find(pb, want)
    pb.button_release("right")
    if s is None:
        print(f"kind 0x{want:02X} never came on screen in {level}")
        pb.stop()
        return 1

    state = slot(pb, s)
    ox, oy = state[3], state[2]
    print(f"kind 0x{want:02X} in slot {s} at x {ox}, y {oy}, "
          f"resting height {oy - REST}")
    saved = snapshot(pb)

    # Its own sprite, which the sprite survey never caught: with Mario pinned
    # far above it there is nothing else inside the window.
    print("every slot in play, so a neighbour cannot be read as this one:")
    for other in range(SLOTS):
        state = slot(pb, other)
        if state[0] != EMPTY:
            print(f"    slot {other}: kind 0x{state[0]:02X} "
                  f"at x {state[3]}, y {state[2]}")
    print("the sprites drawn near it, as offsets from its position:")
    for dx, dy, tile, attr in entries_near(pb, ox, oy):
        print(f"    dx {dx:+3d} dy {dy:+3d}  tile 0x{tile:02X}  attr 0x{attr:02X}")

    for label, free_it in (("object", False), ("freed", True)):
        rows, gone_at, after = trial(pb, saved, s, ox, oy, want, free_it)
        report(label, rows, gone_at, after)

    held = sweep(pb, saved, s, ox, oy, want)
    if held and held == list(range(held[0], held[-1] + 1)):
        print(f"\nheld across offsets {held[0]} to {held[-1]}, "
              f"a window of {len(held)}")
    else:
        print(f"\nthe held offsets have holes in them: {held}")
    pb.stop()
    return 0


if __name__ == "__main__":
    sys.exit(main())
