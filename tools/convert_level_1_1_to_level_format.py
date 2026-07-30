# /// script
# requires-python = ">=3.10"
# dependencies = []
# ///
"""Convert World 1-1 from the ROM into our plain-text level format.

Reads all 300 columns out of the cartridge with `decode_level.py` (no
emulator involved) and writes `assets/extracted/level_1_1.txt`, which
`Level::from_file` already loads. Generated on demand from the verified ROM
and gitignored, never committed.

Solidity is not in the level data, so it comes from
`tools/classify_solid_tiles.py` plus one structural rule:

1. Tile 96 is solid. Observed: it held Mario up on 577 frames of a real run
   and he never once rested inside it.
2. Tiles 44, 49, 50, 51, 54 and 94 are non-solid. Observed the same way, and
   49/50/51/54/94 are the pyramid tiles, which an earlier session established
   independently.
3. **Fill rule (inference, not observation).** A cell whose neighbour above is
   solid is solid too, unless its tile is one of the observed non-solid ones.
   This is what makes tile 97 (under the ground) and tile 232 (the body of a
   raised platform) solid. Without it, the platform at world columns 61-64
   becomes a pit the original does not have, since rows 14 and 15 there are
   232 and 97 rather than ground.

4. **Tile 99, the pipe, is solid.** Settled by playing both readings rather
   than by looking at it. Driving Mario right through the real cartridge, his
   grounded feet row over this stretch goes `14 14 14 - 10 10 14 14`: he
   climbs something and comes back down. Our engine with tile 99 passable
   runs flat across at row 14 with no obstacle at all; with it solid it jumps
   at the same place and stands on top. `--passable-pipe` rebuilds the other
   reading for comparison.

Everything else is treated as non-solid background. That is a real gap rather
than a finished answer: 36 of the level's 43 tile ids have no solidity
evidence either way.

Run: uv run tools/convert_level_1_1_to_level_format.py [--passable-pipe]
"""

import sys
from pathlib import Path

sys.path.insert(0, "tools")

from decode_level import ROWS, decode_level

OUT_PATH = Path("assets/extracted/level_1_1.txt")
OBSERVED_SOLID = {96}
OBSERVED_NON_SOLID = {44, 49, 50, 51, 54, 94}
PIPE = 99  # a 2x3 block on the ground at world columns 34-35

SPAWN_COLUMN = 6
GROUND_ROW = 14


def solid_grid(columns, extra_solid=frozenset()):
    """Per-cell solidity: observed tiles, then fill propagated downward."""
    solid = [[False] * ROWS for _ in columns]
    for c, column in enumerate(columns):
        for r in range(ROWS):
            tile = column[r]
            if tile in OBSERVED_SOLID or tile in extra_solid:
                solid[c][r] = True
            elif r > 0 and solid[c][r - 1] and tile not in OBSERVED_NON_SOLID:
                solid[c][r] = True
        floor_rule(column, solid[c])
    return solid


def floor_rule(column, solid):
    """A column with no solid cell stands on its lowest non-sky tile.

    Inference, and the level cannot be finished without it. World 1-1's last
    eighteen columns have sky at every row except row 15, which carries a
    142/143 band, and nothing there is observable because the walker stops at
    world column 68. Read literally the level ends in a pit that swallows
    Mario sixteen columns short of its own exit gate, so the band is being
    treated as the floor for that stretch.
    """
    if any(solid):
        return
    for r in reversed(range(ROWS)):
        if column[r] not in OBSERVED_NON_SOLID:
            solid[r] = True
            return


def to_text(columns, solid):
    rows = [["#" if solid[c][r] else "." for c in range(len(columns))] for r in range(ROWS)]
    rows[GROUND_ROW - 1][SPAWN_COLUMN] = "M"
    rows[GROUND_ROW - 1][len(columns) - 2] = "E"
    return "\n".join("".join(row) for row in rows) + "\n"


def main():
    extra = frozenset() if "--passable-pipe" in sys.argv else {PIPE}
    rom = open("super_mario_land.gb", "rb").read()
    columns = decode_level(rom)
    solid = solid_grid(columns, extra)

    OUT_PATH.parent.mkdir(parents=True, exist_ok=True)
    OUT_PATH.write_text(to_text(columns, solid))
    filled = sum(sum(col) for col in solid)
    print(f"wrote {OUT_PATH} ({len(columns)}x{ROWS}), {filled} solid cells")
    print("pipe (tile 99): " + ("passable" if not extra else "solid"))
    return 0


if __name__ == "__main__":
    sys.exit(main())
