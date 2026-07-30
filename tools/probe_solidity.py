# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Ask the cartridge itself which tile ids are solid.

Every earlier attempt watched Mario walk past whatever tiles World 1-1
happens to contain. That settled 7 of its 43 tile ids and left the shipped
rule deciding the other 36 with no evidence. This asks the game directly:
write a chosen tile id into the background tilemap in front of Mario, hold
right, and let the game's own collision code answer.

That works because Super Mario Land tests collision against the background
tilemap in video RAM, not against the level data it streams from ROM. Poking
video RAM changes what Mario can walk through, which is what makes every one
of the 256 ids reachable instead of only the ones this level contains.

Two questions per id, because they are not the same question:

* `wall`    a tall wall of the id, four tile columns ahead. Mario is camera
            locked at screen x 81, so a run that ends short of that was
            stopped by the wall. Ground under the wall is left untouched.
* `floor`   the ground band ahead replaced by the id. Mario either keeps
            walking across it or drops through.

Usage: uv run tools/probe_solidity.py [out.json]
"""

import json
import sys

sys.path.insert(0, "tools")

from sml_boot import boot_to_gameplay, restore, snapshot

MAP_BASE = 0x9800
MAP_WIDTH = 32

SCREEN_X = 0xC202
MARIO_Y = 0xC201

SETTLE_FRAMES = 30
PUSH_FRAMES = 90

WALL_COLUMN = 7
WALL_ROWS = range(2, 16)
FLOOR_COLUMNS = range(7, 20)
FLOOR_ROWS = (16, 17)

FREE_X = 81
BLOCKED_X = 70

SOLID_CONTROL = 96
SKY_CONTROL = 44


def cell(row, column):
    return MAP_BASE + row * MAP_WIDTH + (column % MAP_WIDTH)


def run(pb, state, cells, tile):
    restore(pb, state)
    for _ in range(SETTLE_FRAMES):
        pb.tick()
    for address in cells:
        pb.memory[address] = tile
    pb.button_press("right")
    for _ in range(PUSH_FRAMES):
        pb.tick()
    pb.button_release("right")
    return pb.memory[SCREEN_X], pb.memory[MARIO_Y]


def wall_test(pb, state, tile):
    cells = [cell(r, WALL_COLUMN) for r in WALL_ROWS]
    x, _ = run(pb, state, cells, tile)
    return x


def floor_test(pb, state, tile):
    cells = [cell(r, c) for r in FLOOR_ROWS for c in FLOOR_COLUMNS]
    _, y = run(pb, state, cells, tile)
    return y


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else "solidity.json"
    pb = boot_to_gameplay()
    for _ in range(120):
        pb.tick()
    state = snapshot(pb)

    ground_y = floor_test(pb, state, SOLID_CONTROL)
    pit_y = floor_test(pb, state, SKY_CONTROL)
    solid_x = wall_test(pb, state, SOLID_CONTROL)
    sky_x = wall_test(pb, state, SKY_CONTROL)
    print(f"controls: wall {SOLID_CONTROL} -> x {solid_x}, wall {SKY_CONTROL} -> x {sky_x}")
    print(f"controls: floor {SOLID_CONTROL} -> y {ground_y}, floor {SKY_CONTROL} -> y {pit_y}")
    if not (solid_x <= BLOCKED_X < FREE_X <= sky_x):
        print("wall control failed; the probe is not measuring collision")
        pb.stop()
        return 1

    results = {}
    for tile in range(256):
        x = wall_test(pb, state, tile)
        y = floor_test(pb, state, tile)
        results[tile] = {
            "x": x,
            "y": y,
            "wall": x <= BLOCKED_X,
            "floor": abs(y - ground_y) <= 2,
        }
        print(f"{tile:3d} 0x{tile:02X}  x={x:3d} y={y:3d}  "
              f"wall={'Y' if results[tile]['wall'] else 'n'} "
              f"floor={'Y' if results[tile]['floor'] else 'n'}")
    pb.stop()

    with open(out, "w") as f:
        json.dump(results, f, indent=1)

    walls = [t for t, r in results.items() if r["wall"]]
    floors = [t for t, r in results.items() if r["floor"]]
    print(f"\nwrote {out}")
    print(f"{len(walls)} ids block horizontally, {len(floors)} support Mario")
    print("blocking:", walls)
    print("mismatch (one but not the other):",
          sorted(set(walls) ^ set(floors)))
    return 0


if __name__ == "__main__":
    sys.exit(main())
