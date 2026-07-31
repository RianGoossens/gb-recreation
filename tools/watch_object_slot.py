# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Follow one object slot through a plain walk, with the terrain left alone.

The flatten-and-fly walker is the wrong instrument for this. It removes the
ground, so every object that spawns is in free fall from its first frame and
its resting position never shows. This just holds right from the start of
World 1-1 and prints slot 0 every frame, which is enough to see the first
enemy spawn, fall onto the ground, and settle.

Usage: uv run tools/watch_object_slot.py [frames]
"""

import sys

sys.path.insert(0, "tools")

from dump_object_slots import EMPTY, SLOT_BASE, SLOT_SIZE, slot
from sml_boot import boot_to_gameplay

SCREEN_X = 0xC202
FRAMES = 400


def main():
    frames = int(sys.argv[1]) if len(sys.argv) > 1 else FRAMES
    pb = boot_to_gameplay()

    pb.button_press("right")
    last = None
    for frame in range(frames):
        pb.tick()
        state = slot(pb, 0)
        if state[0] == EMPTY:
            continue
        if state != last:
            print(f"frame {frame:4d} mario x {pb.memory[SCREEN_X]:3d}  "
                  + " ".join(f"{b:02X}" for b in state[:8]))
            last = state
    pb.button_release("right")
    pb.stop()
    return 0


if __name__ == "__main__":
    sys.exit(main())
