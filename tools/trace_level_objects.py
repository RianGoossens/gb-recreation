# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Trace object spawns in any pinned level, not just the first one.

World 1-3 introduces kinds the position mapping does not place: nine records
of kind 0x02 whose `y` byte has a high nibble of 4 or C, seven of kind 0x0C,
one of 0x36. Guessing at the encoding from the bytes went nowhere, so this
puts the same question to the game that settled 1-1: play to the level, watch
the read pointer, and print what the record actually put in the slot.

Getting there is the flatten-and-fly walkthrough from `run_through_levels.py`.
Once the requested level is up, the flattening keeps going (Mario still has to
cross the level for the pointer to advance) and every slot fill is reported
with the record that caused it.

Usage: uv run tools/trace_level_objects.py [level]
"""

import sys

sys.path.insert(0, "tools")

from dump_object_slots import EMPTY, SLOTS, slot
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

NAMES = ["1-1", "1-2", "1-3", "2-1", "2-2", "2-3"]
FRAMES = 30000


def reach_level(target, on_open=None):
    """Play from a cold boot until the `target` level is open.

    Returns (pb, capture) with right still held and the level's first columns
    already captured, or (None, None) if the run died on the way. Split out of
    the tracer so other tools can start their measurement inside 1-2 or 1-3
    instead of only in 1-1.
    """
    screens = known_screens()
    pb = boot_to_gameplay()
    capture = Capture(pb, 0)
    playing = True
    level = 0
    quiet = 0
    blank = 0

    pb.button_press("right")
    for frame in range(FRAMES):
        if level == target:
            return pb, capture
        if playing:
            pb.memory[MARIO_Y] = FLY_Y
            if pb.memory[SCREEN_X] > SPAWN_X:
                pb.memory[PHASE] = 0
        pb.tick()
        if pb.memory[LIVES] == 0:
            print(f"  frame {frame}: out of lives before reaching the level")
            pb.stop()
            return None, None

        if playing:
            quiet = 0 if capture.step(pb, frame) else quiet + 1
            if capture.columns and quiet > LEVEL_GAP_FRAMES:
                playing = False
                blank = 0
            continue

        # Between levels: tap A through the bonus game.
        if frame % 40 == 0:
            pb.button_press("a")
        elif frame % 40 == 12:
            pb.button_release("a")
        if frame % SAMPLE_EVERY:
            continue
        if not screens.get(repr([read_column(pb, r) for r in range(VISIBLE_COLUMNS)])):
            blank += SAMPLE_EVERY
            continue
        if blank < BONUS_FRAMES:
            blank = 0
            continue
        pb.button_release("right")
        for _ in range(RELEASE_FRAMES):
            pb.tick()
        pb.button_press("right")
        level += 1
        capture = Capture(pb, frame)
        playing = True
        quiet = 0
        if on_open:
            on_open(pb, level)
    pb.stop()
    return None, None


def main():
    want = sys.argv[1] if len(sys.argv) > 1 else "1-3"
    target = NAMES.index(want)
    rom = open("super_mario_land.gb", "rb").read()
    screens = known_screens()

    pb = boot_to_gameplay()
    capture = Capture(pb, 0)
    playing = True
    level = 0
    quiet = 0
    blank = 0
    last = pointer(pb)
    types = [slot(pb, s)[0] for s in range(SLOTS)]
    record = None
    start = None

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

            if level == target:
                now = pointer(pb)
                if now != last:
                    at = rom_offset(last)
                    record = rom[at : at + 3]
                    print(f"column {len(capture.columns):3d}  "
                          f"record {record[0]:02X} {record[1]:02X} {record[2]:02X} "
                          f"at 0x{at:05X}")
                    last = now
                for s in range(SLOTS):
                    state = slot(pb, s)
                    if types[s] == EMPTY and state[0] != EMPTY:
                        print(f"           -> slot {s}: "
                              + " ".join(f"{b:02X}" for b in state[:8]))
                    types[s] = state[0]

            if capture.columns and quiet > LEVEL_GAP_FRAMES:
                if level == target:
                    break
                playing = False
                blank = 0
            continue

        if frame % 40 == 0:
            pb.button_press("a")
        elif frame % 40 == 12:
            pb.button_release("a")

        if frame % SAMPLE_EVERY:
            continue
        if not screens.get(repr([read_column(pb, r) for r in range(VISIBLE_COLUMNS)])):
            blank += SAMPLE_EVERY
            continue
        if blank < BONUS_FRAMES:
            blank = 0
            continue
        pb.button_release("right")
        for _ in range(RELEASE_FRAMES):
            pb.tick()
        pb.button_press("right")
        level += 1
        capture = Capture(pb, frame)
        playing = True
        quiet = 0
        if level == target:
            last = pointer(pb)
            start = rom_offset(last)
            types = [slot(pb, s)[0] for s in range(SLOTS)]
            print(f"World {NAMES[level]} open, list at 0x{start:05X}\n")
    pb.button_release("right")
    pb.stop()
    return 0


if __name__ == "__main__":
    sys.exit(main())
