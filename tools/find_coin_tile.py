# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Find which background tile id is a coin, by watching one get collected.

Coins are drawn in the background tilemap rather than as sprites, which is
why a column re-read after Mario has walked through it is missing the ones
he took. That also makes them findable without decoding the game's object
table: play until the coin counter goes up, then look at which tilemap cell
changed on that frame.

Usage: uv run tools/find_coin_tile.py
"""

import sys
from collections import Counter

sys.path.insert(0, "tools")

import sml_hud
from sml_boot import boot_to_gameplay

MAP_BASE = 0x9800
MAP_WIDTH = 32
FIRST_ROW = 2
ROWS = 16

MARIO_Y = 0xC201
PHASE = 0xC207
LIVES = 0xDA15
FRAMES = 1600

SOLID_FROM = 0x60
PASSABLE = 0xF4
GROUND_ROW = 14
SKY_ROWS = 2


def is_solid(tile):
    return tile >= SOLID_FROM and tile != PASSABLE


def clear_walls(pb, ring):
    """Remove everything solid from a column, leaving the coins in place."""
    for r in range(SKY_ROWS, GROUND_ROW):
        addr = MAP_BASE + (FIRST_ROW + r) * MAP_WIDTH + ring
        if is_solid(pb.memory[addr]):
            pb.memory[addr] = 0x00


def playfield(pb):
    return [
        pb.memory[MAP_BASE + (FIRST_ROW + r) * MAP_WIDTH + c]
        for r in range(ROWS)
        for c in range(MAP_WIDTH)
    ]


def main():
    taken = Counter()
    # Mario has to touch a coin to take one, so flying is a height sweep: each
    # pass covers the two tile rows his box spans.
    for fly_y in range(56, 130, 8):
        pb = boot_to_gameplay()
        before = playfield(pb)
        coins = sml_hud.coins(pb)
        pb.button_press("right")
        for _ in range(FRAMES):
            pb.memory[MARIO_Y] = fly_y
            pb.memory[PHASE] = 0
            pb.memory[LIVES] = 5
            pb.tick()
            for ring in range(MAP_WIDTH):
                clear_walls(pb, ring)
            now = sml_hud.coins(pb)
            after = playfield(pb)
            if now != coins:
                for old, new in zip(before, after):
                    if old != new:
                        taken[(old, new)] += 1
                coins = now
            before = after
        pb.button_release("right")
        print(f"fly y {fly_y:3d}: {coins} coins")
        pb.stop()

    print("\ntile changes on the frames the coin counter moved:")
    for (old, new), count in taken.most_common(8):
        print(f"  {old:3d} -> {new:3d}   {count} times")
    return 0


if __name__ == "__main__":
    sys.exit(main())
