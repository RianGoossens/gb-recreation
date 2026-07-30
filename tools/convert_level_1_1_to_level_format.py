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

    a tile is solid if 0x60 <= id <= 0xE8

That is one rule for the whole level, and it replaces a per-tile allow-list
plus two invented structural rules that were wrong. Rian played the real
cartridge and reported the previous output had far too few pipes, far too
few blocks to collide with, and none of the holes you can fall through. He
was right on all three: the earlier reading produced a level with **zero**
columns you could fall through, on a level that has nine.

Evidence for the threshold:

* Every tile `classify_solid_tiles.py` settled agrees with it, 7 for 7 with
  no mismatches. The one observed solid tile is 96, which is 0x60 exactly.
  The six observed non-solid tiles (44, 49, 50, 51, 54, 94) are all below it.
* It reproduces the level's real structure with no special cases: multiple
  pipes, floating platforms, and nine columns of genuine pit at world columns
  89-90, 138-139, 247-249 and 261-262.
* The level can be walked from spawn to its exit gate, which a plain `>= 0x60`
  cannot: that leaves tile 244 (0xF4) solid, and 244 blocks the route dead at
  world column 269. 244 is decoration. It appears 18 times, as isolated single
  cells on alternating rows at column 277 and as a seven-tall bar floating in
  open sky at column 87 with nothing under it.
* The two rules it removes existed only to paper over its absence. A "fill
  propagates downward" rule was needed because tile 232, the body of a raised
  platform, went unrecognised; 232 is 0xE8. A "column with no solid cell
  stands on its lowest non-sky tile" rule was needed because the level's final
  ground band went unrecognised too; those tiles are 142 and 143, 0x8E and
  0x8F. Both are solid under the threshold, and both hacks disappear.

Still a hypothesis rather than a confirmed read of the cartridge's own
collision test, which has not been located in the ROM. Only 7 of the level's
43 tile ids have been observed directly; the rule decides the other 36. The
upper bound is the weakest part: this level has no tile between 0xE8 and 0xF4,
so the data pins it only to somewhere in that gap.

Run: uv run tools/convert_level_1_1_to_level_format.py
"""

import sys
from pathlib import Path

sys.path.insert(0, "tools")

from decode_level import ROWS, decode_level

OUT_PATH = Path("assets/extracted/level_1_1.txt")
SOLID_FROM = 0x60
SOLID_TO = 0xE8

SPAWN_COLUMN = 6
GROUND_ROW = 14


def is_solid(tile):
    return SOLID_FROM <= tile <= SOLID_TO


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
    return 0


if __name__ == "__main__":
    sys.exit(main())
