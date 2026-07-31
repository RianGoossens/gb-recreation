# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Ask the game what the top bit of an object's type byte does.

World 1-1's object list holds 37 records, and only 16 of them ever fill a
slot: exactly the ones whose type byte has the top bit clear. That is a
correlation, and the game can be made to answer it directly. Clear the bit
in one skipped record, in a scratch copy of the cartridge, and see whether
that record starts spawning.

The scratch ROM never leaves the temporary directory and is not an
extracted asset; it is a question put to the emulator.

Usage: uv run tools/probe_object_type_flag.py
"""

import shutil
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, "tools")

from dump_object_slots import EMPTY, SLOTS, slot
from run_through_levels import Capture, LIVES, MARIO_Y, PHASE, SCREEN_X, SPAWN_X, FLY_Y
from sml_boot import ROM, boot_to_gameplay

OBJECT_LIST = 0x0A002
RECORD = 3
FRAMES = 2400

# Every record of World 1-1's list, for reporting which one changed.
SKIPPED_RECORD = 2  # 13 0C 84, the third record, never spawns as shipped


def run(rom_path):
    pb = boot_to_gameplay(rom_path)
    capture = Capture(pb, 0)
    types = [slot(pb, s)[0] for s in range(SLOTS)]
    fills = []

    pb.button_press("right")
    for frame in range(FRAMES):
        pb.memory[MARIO_Y] = FLY_Y
        if pb.memory[SCREEN_X] > SPAWN_X:
            pb.memory[PHASE] = 0
        pb.tick()
        if pb.memory[LIVES] == 0:
            break
        capture.step(pb, frame)
        for s in range(SLOTS):
            now = slot(pb, s)[0]
            if types[s] == EMPTY and now != EMPTY:
                fills.append((len(capture.columns), now))
            types[s] = now
    pb.button_release("right")
    pb.stop()
    return fills


def main():
    rom = Path(ROM).read_bytes()
    at = OBJECT_LIST + SKIPPED_RECORD * RECORD + 2
    print(f"record {SKIPPED_RECORD} type byte at rom 0x{at:05X} is 0x{rom[at]:02X}")

    with tempfile.TemporaryDirectory() as tmp:
        patched = Path(tmp) / "patched.gb"
        shutil.copy(ROM, patched)
        data = bytearray(rom)
        data[at] &= 0x7F
        patched.write_bytes(bytes(data))
        print(f"scratch copy has 0x{data[at]:02X} there\n")

        before = run(ROM)
        after = run(str(patched))

    print(f"shipped: {len(before)} objects spawned")
    print(f"patched: {len(after)} objects spawned")
    new = [f for f in after if f not in before]
    print(f"only in the patched run: {new}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
