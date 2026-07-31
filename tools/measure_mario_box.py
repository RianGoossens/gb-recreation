# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Measure how wide small Mario is for collision, by walling him in.

The lift measurement put a number on this sideways: the lift holds him across
a 29 px window and is 16 px of surface, so the cartridge's Mario is nearer 14
px wide than the 8 the engine uses. That was inferred from one experiment and
deserves its own.

Build a corridor. Super Mario Land tests collision against the background
tilemap in video RAM, so with the camera still, two columns of wall tiles can
be written either side of Mario. Walk him into the right one and read his X
byte, walk him into the left one and read it again. The distance he covered
is the corridor minus his own width, and the constant offset between his X
byte and his left edge cancels out of the subtraction, so it never has to be
known:

    width = corridor_pixels - (x_right_stop - x_left_stop)

Usage: uv run tools/measure_mario_box.py
"""

import sys

sys.path.insert(0, "tools")

from run_through_levels import MAP_BASE, MAP_WIDTH, SCREEN_X
from sml_boot import boot_to_gameplay

SCX = 0xFF43
MARIO_Y = 0xC201
OAM = 0xFE00
SPRITES = 40

WALL_ROWS = range(10, 16)
WALL_TILE = 0x60
# One corridor could be answering with an off-by-one in the wall placement
# rather than with Mario's width, so every width that fits on screen runs.
CORRIDORS = [(-2, 2), (-3, 3), (-4, 4), (-2, 4), (-4, 2)]
PUSH_FRAMES = 240


def wall_at(pb, screen_column):
    ring = ((pb.memory[SCX] // 8) + screen_column) % MAP_WIDTH
    for row in WALL_ROWS:
        pb.memory[MAP_BASE + row * MAP_WIDTH + ring] = WALL_TILE


def push(pb, direction):
    pb.button_press(direction)
    for _ in range(PUSH_FRAMES):
        pb.tick()
    pb.button_release(direction)
    for _ in range(4):
        pb.tick()
    return pb.memory[SCREEN_X]


def sprite_extent(pb):
    """Screen x of Mario's leftmost and rightmost drawn sprite columns."""
    xs = []
    for i in range(SPRITES):
        y = pb.memory[OAM + i * 4]
        x = pb.memory[OAM + i * 4 + 1]
        if 0 < y < 160 and 0 < x < 168:
            xs.append(x - 8)
    if not xs:
        return None
    return min(xs), max(xs) + 7


def trial(left_column, right_column):
    pb = boot_to_gameplay()
    for _ in range(30):
        pb.tick()

    scx = pb.memory[SCX]
    mario_column = (pb.memory[SCREEN_X] - 8) // 8
    left = mario_column + left_column
    right = mario_column + right_column
    wall_at(pb, left)
    wall_at(pb, right)
    corridor = 8 * (right - left - 1)

    x_right = push(pb, "right")
    x_left = push(pb, "left")
    moved = pb.memory[SCX] != scx
    pb.stop()
    if moved:
        return None
    return corridor, x_right - x_left


def main():
    pb = boot_to_gameplay()
    for _ in range(30):
        pb.tick()
    extent = sprite_extent(pb)
    print(f"Mario's X byte {pb.memory[SCREEN_X]}, SCX {pb.memory[SCX]}")
    if extent:
        print(f"drawn sprites span screen x {extent[0]}..{extent[1]} "
              f"({extent[1] - extent[0] + 1} px)")
    pb.stop()

    print("\n corridor  travelled  width")
    widths = []
    for left_column, right_column in CORRIDORS:
        result = trial(left_column, right_column)
        if result is None:
            print("  (the camera moved, trial void)")
            continue
        corridor, travelled = result
        widths.append(corridor - travelled)
        print(f" {corridor:8d}  {travelled:9d}  {corridor - travelled:5d}")

    if len(set(widths)) == 1:
        print(f"\nsmall Mario is {widths[0]} px wide for collision")
        return 0
    print(f"\ncorridors disagree: {sorted(set(widths))}")
    return 1


if __name__ == "__main__":
    sys.exit(main())
