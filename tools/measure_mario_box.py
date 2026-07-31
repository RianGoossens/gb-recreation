# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Measure small Mario's collision box by walling him in.

The lift measurement put a number on the width sideways: the lift holds him
across a 29 px window and is 16 px of surface, so the cartridge's Mario is
wider than the 8 px the engine uses. That was inferred from one experiment and
deserves its own.

Build a corridor. Super Mario Land tests collision against the background
tilemap in video RAM, so with the camera still, two columns of wall tiles can
be written either side of Mario. Walk him into the right one and read his X
byte, walk him into the left one and read it again. The room he had minus the
room he used is his width, and the constant offset between his X byte and his
left edge cancels out of the subtraction, so it never has to be known:

    width = corridor_pixels - (x_right_stop - x_left_stop)

The height comes out of the same subtraction stood on end: put a ceiling over
him, jump, and take the headroom minus how far he rose. One trap there, and it
is the reason the free jump gets measured first: a ceiling above the jump's own
apex returns the apex, which looks like a perfectly good number.

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
BODY_ROWS = (14, 15)
WALL_TILE = 0x60
SOLID_FROM = 0x60
# One corridor could be answering with an off-by-one in the wall placement
# rather than with Mario's width, so every width that fits on screen runs.
CORRIDORS = [(-2, 2), (-3, 3), (-4, 4), (-2, 4), (-4, 2), (-3, 5), (-2, 6)]
PUSH_FRAMES = 240
FLOOR_ROW = 16
CEILINGS = [13, 12, 11, 10, 9]


def wall_at(pb, screen_column):
    ring = ((pb.memory[SCX] // 8) + screen_column) % MAP_WIDTH
    for row in WALL_ROWS:
        pb.memory[MAP_BASE + row * MAP_WIDTH + ring] = WALL_TILE


def solid(pb, screen_column):
    ring = ((pb.memory[SCX] // 8) + screen_column) % MAP_WIDTH
    return any(
        pb.memory[MAP_BASE + row * MAP_WIDTH + ring] >= SOLID_FROM for row in BODY_ROWS
    )


def blockers(pb, mario_column):
    """The nearest solid column either side of Mario, at body height.

    The walls this writes are not always what stops him. World 1-1 opens with
    two columns of scenery standing on the ground three columns to Mario's
    left, so a wall written further out than that never gets reached and the
    corridor is narrower than it was asked for. Reading the tilemap back
    removes the assumption.
    """
    left = mario_column - 1
    while left >= 0 and not solid(pb, left):
        left -= 1
    right = mario_column + 1
    while right < 20 and not solid(pb, right):
        right += 1
    return left, right


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


def ceiling_at(pb, map_row):
    for column in range(MAP_WIDTH):
        pb.memory[MAP_BASE + map_row * MAP_WIDTH + column] = WALL_TILE


def height_trial(ceiling_row=None):
    """Same measurement stood on end: jump into a ceiling and read the Y byte.

    The constant offset between the Y byte and Mario's top pixel drops out of
    the subtraction the same way the X offset does, so the room he has to move
    in minus the room he uses is his height.
    """
    pb = boot_to_gameplay()
    for _ in range(30):
        pb.tick()
    standing = pb.memory[MARIO_Y]
    if ceiling_row is not None:
        ceiling_at(pb, ceiling_row)

    pb.button_press("a")
    peak = standing
    for _ in range(120):
        pb.tick()
        peak = min(peak, pb.memory[MARIO_Y])
    pb.button_release("a")
    pb.stop()

    if ceiling_row is None:
        return None, standing - peak
    gap = 8 * (FLOOR_ROW - ceiling_row - 1)
    return gap, standing - peak


def trial(left_column, right_column):
    pb = boot_to_gameplay()
    for _ in range(30):
        pb.tick()

    scx = pb.memory[SCX]
    mario_column = (pb.memory[SCREEN_X] - 8) // 8
    wall_at(pb, mario_column + left_column)
    wall_at(pb, mario_column + right_column)
    left, right = blockers(pb, mario_column)
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

    # A ceiling higher than the jump reaches measures the jump instead, so the
    # free rise is measured first and any trial that only matches it is void.
    _, free_rise = height_trial()
    print(f"\nan unobstructed jump rises {free_rise} px")
    print("  headroom  rose  height")
    heights = []
    for ceiling_row in CEILINGS:
        gap, rose = height_trial(ceiling_row)
        if rose >= free_rise:
            print(f" {gap:9d}  {rose:4d}  (never reached the ceiling)")
            continue
        heights.append(gap - rose)
        print(f" {gap:9d}  {rose:4d}  {gap - rose:6d}")

    ok = True
    if len(set(widths)) == 1:
        print(f"\nsmall Mario is {widths[0]} px wide for collision")
    else:
        print(f"\ncorridors disagree on width: {sorted(set(widths))}")
        ok = False
    if len(set(heights)) == 1:
        print(f"small Mario is {heights[0]} px tall for collision")
    else:
        print(f"ceilings disagree on height: {sorted(set(heights))}")
        ok = False
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
