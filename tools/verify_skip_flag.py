# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Check the candidate skip flag over a whole level rather than two records.

The work-RAM sweep turned up one address that added exactly the two skipped
records inside its window, 0xFF9A, and four more that filled every slot at
once, which is what a poke into the stack looks like rather than a flag.

Two records is a small sample. Walk all of World 1-1 with the byte held at a
value and record which records fill a slot, then compare against a plain run.
A flag that selects the skipped records makes the sixteen become thirty-seven,
or swaps one set for the other; anything else is a coincidence.

Usage: uv run tools/verify_skip_flag.py [address] [value]
"""

import sys

sys.path.insert(0, "tools")

from dump_object_slots import EMPTY, SLOTS, slot
from run_through_levels import Capture, FLY_Y, LIVES, MARIO_Y, PHASE, SCREEN_X, SPAWN_X
from sml_boot import ROM, boot_to_gameplay

FRAMES = 2400
DEFAULT_ADDRESS = 0xFF9A


def run(address, value):
    """Walk 1-1, holding `address` at `value` every frame. Returns the fills."""
    pb = boot_to_gameplay(ROM)
    capture = Capture(pb, 0)
    types = [slot(pb, s)[0] for s in range(SLOTS)]
    fills = []

    pb.button_press("right")
    for frame in range(FRAMES):
        pb.memory[MARIO_Y] = FLY_Y
        if pb.memory[SCREEN_X] > SPAWN_X:
            pb.memory[PHASE] = 0
        if address is not None:
            pb.memory[address] = value
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
    address = int(sys.argv[1], 0) if len(sys.argv) > 1 else DEFAULT_ADDRESS
    value = int(sys.argv[2], 0) if len(sys.argv) > 2 else 1

    plain = run(None, 0)
    held = run(address, value)
    print(f"plain run: {len(plain)} objects spawned")
    print(f"{address:04X} held at 0x{value:02X}: {len(held)} objects spawned\n")

    plain_set = set(plain)
    for column, kind in held:
        mark = " " if (column, kind) in plain_set else "+"
        print(f"{mark} column {column:3d} kind 0x{kind:02X}")
    missing = [f for f in plain if f not in set(held)]
    for column, kind in missing:
        print(f"- column {column:3d} kind 0x{kind:02X} (gone)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
