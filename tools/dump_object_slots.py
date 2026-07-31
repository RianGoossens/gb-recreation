# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Watch an object slot fill, and print what the record put in it.

`find_object_slots.py` traced the third byte of every record to a table at
0xD100 with a stride of 16, so byte 0 of a slot is the object's type and
0xFF marks a slot as free. That leaves the other two bytes of the record.
Neither turns up in RAM unchanged, which is expected: a stored position has
to become a live one.

So this triggers on the slot rather than on the pointer. When a slot's type
byte stops being 0xFF, it prints the record the pointer had just consumed
and then the slot's bytes over the following frames, which separates the
fields that were written once from the ones the game updates.

Usage: uv run tools/dump_object_slots.py [frames]
"""

import sys

sys.path.insert(0, "tools")

from run_through_levels import Capture, LIVES, MARIO_Y, PHASE, SCREEN_X, SPAWN_X, FLY_Y
from sml_boot import boot_to_gameplay
from trace_object_spawns import pointer, rom_offset

SLOT_BASE = 0xD100
SLOT_SIZE = 0x10
SLOTS = 10
EMPTY = 0xFF
FOLLOW = 4
FRAMES = 1400


def slot(pb, s):
    return bytes(pb.memory[SLOT_BASE + s * SLOT_SIZE + i] for i in range(SLOT_SIZE))


def main():
    frames = int(sys.argv[1]) if len(sys.argv) > 1 else FRAMES
    rom = open("super_mario_land.gb", "rb").read()

    pb = boot_to_gameplay()
    capture = Capture(pb, 0)
    last = pointer(pb)
    record = None
    types = [slot(pb, s)[0] for s in range(SLOTS)]
    following = []

    pb.button_press("right")
    for frame in range(frames):
        pb.memory[MARIO_Y] = FLY_Y
        if pb.memory[SCREEN_X] > SPAWN_X:
            pb.memory[PHASE] = 0
        pb.tick()
        if pb.memory[LIVES] == 0:
            break
        capture.step(pb, frame)

        now = pointer(pb)
        if now != last:
            record = rom[rom_offset(last) : rom_offset(last) + 3]
            last = now

        for s in range(SLOTS):
            state = slot(pb, s)
            if types[s] == EMPTY and state[0] != EMPTY:
                shown = " ".join(f"{b:02X}" for b in record) if record else "-"
                print(f"frame {frame:5d} column {len(capture.columns):3d} "
                      f"slot {s} filled, last record {shown}")
                following.append((s, frame + FOLLOW))
            types[s] = state[0]

        for s, until in list(following):
            print(f"  +{until - frame:>2} " + " ".join(f"{b:02X}" for b in slot(pb, s)))
            if frame >= until:
                following.remove((s, until))
    pb.button_release("right")
    pb.stop()
    return 0


if __name__ == "__main__":
    sys.exit(main())
