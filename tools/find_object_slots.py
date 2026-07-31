# /// script
# requires-python = """>=3.10"""
# dependencies = ["pyboy"]
# ///
"""Find where a spawned object lands in RAM, by looking for its own bytes.

`trace_object_spawns.py` pins the frame: the moment 0xD010 steps past a
three-byte record is the moment the game acts on it. So the record's bytes
are known, and the question is only where they went. This watches work RAM
for each of the three values turning up at an address that did not hold it
the frame before, and counts how often each address does that across all 36
of World 1-1's records.

An address that takes the same field of every record is the field's slot.
A table of slots shows up as several addresses at a fixed stride, each
taking a share of the records.

Usage: uv run tools/find_object_slots.py [frames]
"""

import sys
from collections import Counter

sys.path.insert(0, "tools")

from run_through_levels import Capture, LIVES, MARIO_Y, PHASE, SCREEN_X, SPAWN_X, FLY_Y
from sml_boot import boot_to_gameplay
from trace_object_spawns import pointer, rom_offset

WRAM = range(0xC000, 0xE000)
SETTLE = 3
FRAMES = 2400


def snapshot(pb):
    return bytes(pb.memory[a] for a in WRAM)


def main():
    frames = int(sys.argv[1]) if len(sys.argv) > 1 else FRAMES
    rom = open("super_mario_land.gb", "rb").read()

    pb = boot_to_gameplay()
    capture = Capture(pb, 0)
    last = pointer(pb)
    before = snapshot(pb)

    took = [Counter() for _ in range(3)]
    spawns = 0
    pending = None
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
            pending = (record, before, frame + SETTLE)
            last = now
        if pending and frame >= pending[2]:
            record, was, _ = pending
            after = snapshot(pb)
            spawns += 1
            for field, value in enumerate(record):
                for i, (a, b) in enumerate(zip(was, after)):
                    if b == value and a != value:
                        took[field][WRAM.start + i] += 1
            pending = None
        before = snapshot(pb)
    pb.button_release("right")
    pb.stop()

    print(f"{spawns} records\n")
    for field, counts in enumerate(took):
        print(f"field {field}: addresses that took its value")
        for addr, count in sorted(counts.items(), key=lambda kv: -kv[1])[:12]:
            print(f"  0x{addr:04X}  {count}/{spawns}")
        print()
    return 0


if __name__ == "__main__":
    sys.exit(main())
