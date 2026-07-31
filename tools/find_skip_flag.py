# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Find the byte of memory that decides whether a skipped record spawns.

Twenty-one of World 1-1's thirty-seven object records carry the top bit of
their type byte, and none of those twenty-one ever fills a slot in normal
play. Clearing the bit in a scratch cartridge makes one spawn, so the bit is
read rather than being leftover data. What it is read *against* is the open
question: if the game only ever skips them, they are dead weight in every
level of the cartridge, which is not a sensible thing to ship.

So ask the machine. Walk until the read pointer is sitting on a skipped
record, snapshot there, then for every byte of work RAM and high RAM: restore
the snapshot, poke that byte, walk the same short distance, and see whether a
slot fills that did not fill before. A flag that gates the skip has to be one
of them.

Two records are inside the window, so a real flag shows up as two extra
spawns rather than one, which throws out most of the noise a random poke
makes on its own.

Usage: uv run tools/find_skip_flag.py [value]
"""

import sys

sys.path.insert(0, "tools")

from dump_object_slots import EMPTY, SLOTS, slot
from run_through_levels import FLY_Y, MARIO_Y
from sml_boot import boot_to_gameplay, restore, snapshot
from trace_object_spawns import pointer

LIST_START = 0x6002
FIRST_SKIPPED = LIST_START + 3
WINDOW = 140

WRAM = range(0xC000, 0xE000)
HRAM = range(0xFF80, 0xFFFF)
# Poking inside the object slot table makes an object by definition, and the
# read pointer itself just moves the game to a different record.
EXCLUDED = set(range(0xD100, 0xD200)) | {0xD010, 0xD011}


def walk(pb, frames):
    """Hold right for `frames`, keeping Mario in the air, counting slot fills."""
    was = [slot(pb, s)[0] for s in range(SLOTS)]
    fills = 0
    pb.button_press("right")
    for _ in range(frames):
        pb.memory[MARIO_Y] = FLY_Y
        pb.tick()
        for s in range(SLOTS):
            now = slot(pb, s)[0]
            if was[s] == EMPTY and now != EMPTY:
                fills += 1
            was[s] = now
    return fills


def main():
    value = int(sys.argv[1], 0) if len(sys.argv) > 1 else 0x01

    pb = boot_to_gameplay()
    pb.button_press("right")
    for _ in range(4000):
        pb.memory[MARIO_Y] = FLY_Y
        pb.tick()
        if pointer(pb) >= FIRST_SKIPPED:
            break
    pb.button_release("right")
    if pointer(pb) != FIRST_SKIPPED:
        print(f"pointer sat at {pointer(pb):04X}, not the first skipped record")
        pb.stop()
        return 1
    state = snapshot(pb)

    restore(pb, state)
    baseline = walk(pb, WINDOW)
    print(f"baseline: {baseline} slot fills in {WINDOW} frames, "
          f"pointer ended at {pointer(pb):04X}")

    candidates = [a for a in list(WRAM) + list(HRAM) if a not in EXCLUDED]
    print(f"sweeping {len(candidates)} addresses with value 0x{value:02X}")
    hits = []
    for i, address in enumerate(candidates):
        restore(pb, state)
        pb.memory[address] = value
        fills = walk(pb, WINDOW)
        if fills > baseline:
            hits.append((address, fills))
            print(f"  {address:04X} -> {fills} fills")
        if i % 512 == 511:
            print(f"  ...{i + 1}/{len(candidates)}", flush=True)
    pb.stop()

    print(f"\n{len(hits)} addresses raised the spawn count")
    for address, fills in sorted(hits, key=lambda h: -h[1]):
        print(f"  {address:04X}: {fills}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
