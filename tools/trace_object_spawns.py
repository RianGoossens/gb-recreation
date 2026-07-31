# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Watch the game read its object table, and note where it is standing.

0xD010 is the one 16-bit value in RAM that walks forward through banked ROM
as a level scrolls (`find_level_pointer.py` found it while the column
records were being pinned). It starts World 1-1 at 0x6002, which is ROM
0x0A002, two bytes past the last column record. Hexdumping there shows
three-byte groups whose first byte only ever increases, ending on 0xFF.
That looks like a spawn list, and the first byte looks like a position.

Guessing which position is not necessary. The pointer moves while the game
runs, so the question can be asked directly: when the pointer steps past a
record, how far into the level is the camera? Column counting comes from
`run_through_levels.py`, which needs no camera tracking, since the game
writes each column into the tilemap once as it scrolls in.

Usage: uv run tools/trace_object_spawns.py [frames]
"""

import sys

sys.path.insert(0, "tools")

from run_through_levels import Capture, LIVES, MARIO_Y, PHASE, SCREEN_X, SPAWN_X, FLY_Y
from sml_boot import boot_to_gameplay

OBJECT_POINTER = 0xD010
BANK_WINDOW = 0x4000
# The switchable banks, as file offsets. World 1's data is in bank 2; World 2's
# is in bank 1, so which bank a pointer resolves against has to be observed
# rather than assumed.
BANK_BASES = (0x4000, 0x8000, 0xC000)
BANK_BASE = 0x8000

FRAMES = 3000


def pointer(pb):
    return pb.memory[OBJECT_POINTER] | (pb.memory[OBJECT_POINTER + 1] << 8)


def rom_offset(value, bank=BANK_BASE):
    return value - BANK_WINDOW + bank


def current_bank(pb, rom, at=BANK_WINDOW, length=64):
    """Which ROM bank is switched in, by matching its bytes.

    PyBoy exposes no bank register, and the bank cannot be read off the
    pointer. Comparing what the CPU currently sees in the 0x4000 window
    against the same span of each bank in the ROM file identifies it by
    construction, the same technique `find_rom_offset.py` uses. Returns None
    if more than one bank matches, so an ambiguous window is visible rather
    than guessed at.
    """
    window = bytes(pb.memory[at : at + length])
    hits = [b for b in BANK_BASES
            if rom[at - BANK_WINDOW + b : at - BANK_WINDOW + b + length] == window]
    return hits[0] if len(hits) == 1 else None


def main():
    frames = int(sys.argv[1]) if len(sys.argv) > 1 else FRAMES
    rom = open("super_mario_land.gb", "rb").read()

    pb = boot_to_gameplay()
    capture = Capture(pb, 0)
    start = pointer(pb)
    print(f"level start: 0xD010 = 0x{start:04X} (rom 0x{rom_offset(start):05X})")

    last = start
    steps = []
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
            steps.append((frame, len(capture.columns), last, now))
            last = now
    pb.button_release("right")
    pb.stop()

    print(f"\n{len(steps)} pointer moves over {len(capture.columns)} columns\n")
    print("frame  column  pointer          record consumed")
    for frame, column, before, after in steps:
        at = rom_offset(before)
        record = " ".join(f"{b:02X}" for b in rom[at : rom_offset(after)])
        print(f"{frame:5d}  {column:6d}  0x{before:04X}->0x{after:04X}  {record}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
