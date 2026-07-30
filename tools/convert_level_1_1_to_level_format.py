# /// script
# requires-python = ">=3.10"
# dependencies = []
# ///
"""Convert World 1-1 from the ROM into our plain-text level format.

Reads all 300 columns out of the cartridge with `decode_level.py` (no
emulator involved) and writes `assets/extracted/level_1_1.txt`, which
`Level::from_file` already loads. Generated on demand from the verified ROM
and gitignored, never committed.

Solidity is not stored per column, so it comes from the tile id:

    a tile is solid if id >= 0x60, except 0xF4, which is not

That rule is the cartridge's own answer, not a fit. `probe_solidity.py`
writes each of the 256 tile ids into the background tilemap in front of
Mario and lets the game's collision code decide, which covers every id
instead of only the ones this level happens to contain. All 43 of the
level's ids are settled by direct observation, including the 36 that were
previously decided by inference.

Four ids (0x68, 0x69, 0x6A, 0x7C) hold Mario up but do not block him
sideways. None of them appear in World 1-1, and our level format has one
notion of solid, so they are treated as solid here.

The rule this replaces was `0x60 <= id <= 0xE8`, a fit to 7 observed tiles
plus "the level can be finished". It produces the same grid for this level,
because 1-1 has no id between 0xE8 and 0xF4 and none of the four semi-solid
ids, but its upper bound was wrong as a general rule: 0xF4 is a single
exception, not a boundary.

Run: uv run tools/convert_level_1_1_to_level_format.py
"""

import sys
from pathlib import Path

sys.path.insert(0, "tools")

from decode_level import ROWS, decode_level

OUT_PATH = Path("assets/extracted/level_1_1.txt")

SOLID_FROM = 0x60
PASSABLE = {0xF4}
SEMI_SOLID = {0x68, 0x69, 0x6A, 0x7C}

SPAWN_COLUMN = 6
GROUND_ROW = 14


def is_solid(tile):
    return tile >= SOLID_FROM and tile not in PASSABLE


def to_text(columns):
    rows = [
        ["#" if is_solid(columns[c][r]) else "." for c in range(len(columns))]
        for r in range(ROWS)
    ]
    rows[GROUND_ROW - 1][SPAWN_COLUMN] = "M"
    rows[GROUND_ROW - 1][len(columns) - 2] = "E"
    return "\n".join("".join(row) for row in rows) + "\n"


def main():
    rom = open("super_mario_land.gb", "rb").read()
    columns = decode_level(rom)

    OUT_PATH.parent.mkdir(parents=True, exist_ok=True)
    OUT_PATH.write_text(to_text(columns))

    solid = sum(1 for col in columns for t in col if is_solid(t))
    pits = [c for c, col in enumerate(columns) if not any(is_solid(t) for t in col)]
    print(f"wrote {OUT_PATH} ({len(columns)}x{ROWS}), {solid} solid cells")
    print(f"columns you can fall through: {pits}")

    semi = sorted({t for col in columns for t in col if t in SEMI_SOLID})
    if semi:
        print(f"note: semi-solid ids flattened to solid: {semi}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
