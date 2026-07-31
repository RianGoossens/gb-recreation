# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Read every World 1 level's object list start, by playing to each level.

World 1-1's list start needed no searching: 0xD010 already holds it when the
level opens. The other two levels are the same question with a longer walk,
so this drives the flatten-and-fly walkthrough from `run_through_levels.py`
across the level boundaries and reads the pointer each time a level opens.

The reading has to happen once the level is under way rather than on the
first frame it is recognised. The game rewrites 0xD010 as part of loading a
level, and a pointer read too early is still the previous level's.

Usage: uv run tools/find_object_lists.py
"""

import sys

sys.path.insert(0, "tools")

from run_through_levels import (
    BONUS_FRAMES,
    Capture,
    FLY_Y,
    LEVEL_GAP_FRAMES,
    LIVES,
    MARIO_Y,
    PHASE,
    RELEASE_FRAMES,
    SAMPLE_EVERY,
    SCREEN_X,
    SPAWN_X,
    read_column,
)
from sml_boot import boot_to_gameplay
from sml_level import VISIBLE_COLUMNS, known_screens
from trace_object_spawns import pointer, rom_offset

FRAMES = 12000
NAMES = ["1-1", "1-2", "1-3"]


def main():
    rom = open("super_mario_land.gb", "rb").read()
    screens = known_screens()
    pb = boot_to_gameplay()
    capture = Capture(pb, 0)
    playing = True
    quiet = 0
    blank = 0
    found = []

    def record_start(index):
        value = pointer(pb)
        at = rom_offset(value)
        name = NAMES[index] if index < len(NAMES) else f"level {index}"
        head = " ".join(f"{b:02X}" for b in rom[at : at + 12])
        print(f"World {name}: 0xD010 = 0x{value:04X}, rom 0x{at:05X}")
        print(f"  first bytes: {head}")
        found.append((name, at))

    record_start(0)

    pb.button_press("right")
    for frame in range(FRAMES):
        if playing:
            pb.memory[MARIO_Y] = FLY_Y
            if pb.memory[SCREEN_X] > SPAWN_X:
                pb.memory[PHASE] = 0
        pb.tick()
        if pb.memory[LIVES] == 0:
            print(f"  frame {frame}: out of lives, stopping")
            break

        if playing:
            quiet = 0 if capture.step(pb, frame) else quiet + 1
            if capture.columns and quiet > LEVEL_GAP_FRAMES:
                playing = False
                blank = 0
            continue

        if frame % 40 == 0:
            pb.button_press("a")
        elif frame % 40 == 12:
            pb.button_release("a")

        if frame % SAMPLE_EVERY:
            continue
        hit = screens.get(repr([read_column(pb, r) for r in range(VISIBLE_COLUMNS)]))
        if not hit:
            blank += SAMPLE_EVERY
        elif blank >= BONUS_FRAMES:
            pb.button_release("right")
            for _ in range(RELEASE_FRAMES):
                pb.tick()
            pb.button_press("right")
            record_start(len(found))
            capture = Capture(pb, frame)
            playing = True
            quiet = 0
        else:
            blank = 0
    pb.button_release("right")
    pb.stop()

    print("\nlist starts:")
    for name, at in found:
        print(f"  {name}: 0x{at:05X}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
