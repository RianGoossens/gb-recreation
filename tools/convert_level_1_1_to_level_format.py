# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Convert World 1-1's opening screen into our level format.

Reads the raw background tilemap straight from a live emulator (the same
20x18 opening screen extract_level_1_1.py captures) and writes
assets/extracted/level_1_1_opening.txt in the plain-text level format
Level::from_file already loads (see docs/reference/level-format.md):
'#' solid, '.' empty, 'M' Mario's spawn.

This is deliberately not committed as a shipped level. It is generated
on demand from the verified ROM, and it is also incomplete: only this
opening screen has directly-confirmed tile solidity (see
docs/reference/level-1-1.md). Tile classification here:

- 96 (ground), 97 (underground fill): solid. 96 is directly confirmed
  by observed collision; 97 is presumed by level-design consistency
  (never independently confirmed, see level-1-1.md).
- 44 (sky/blank): non-solid, directly confirmed by observed jump arcs.
- The pyramid structure's tiles (49, 50, 51, 54, 94, 112, 113, 114, 115,
  129): non-solid, at least against a horizontal approach. This looked
  the other way at first (a rendered stacked-block staircase, presumed
  solid by level-design consistency), but building this exact level with
  that presumption and walking it is what disproved it: Mario got stuck
  oscillating at spawn, contradicting every real-emulator trace this
  session showing him walk straight through with no collision at all.
  See level-1-1.md for the full account. Solidity from above (a fall
  onto its top surface) was never tested and is a separate question.
- Rows 0-1 are the status bar (score/coins/time) bleeding into the raw
  tilemap read, not level geometry, and are always written as empty.
- Everything else (the mountain silhouette, clouds, palm trees):
  treated as non-solid background decoration by the universal Mario
  convention (backgrounds are never collidable), not independently
  confirmed either.

Run: uv run tools/convert_level_1_1_to_level_format.py
"""

import sys
from pathlib import Path

from sml_boot import boot_to_gameplay

OUT_PATH = Path("assets/extracted/level_1_1_opening.txt")
MAP_BASE = 0x9800
COLS, ROWS = 20, 18

HUD_ROWS = 2  # rows 0-1: status bar, not level geometry
GROUND_ROW = 16
SPAWN_COL = 6
SPAWN_ROW = GROUND_ROW - 1

SOLID_TILES = {
    96, 97,  # ground, underground fill
}


def main():
    pb = boot_to_gameplay()

    rows = []
    for row in range(ROWS):
        line = []
        for col in range(COLS):
            if row < HUD_ROWS:
                line.append(".")
                continue
            tile = pb.memory[MAP_BASE + row * 32 + col]
            line.append("#" if tile in SOLID_TILES else ".")
        rows.append(line)
    pb.stop()

    rows[SPAWN_ROW][SPAWN_COL] = "M"

    OUT_PATH.parent.mkdir(parents=True, exist_ok=True)
    text = "\n".join("".join(row) for row in rows) + "\n"
    OUT_PATH.write_text(text)
    print(f"wrote {OUT_PATH} ({COLS}x{ROWS})")


if __name__ == "__main__":
    sys.exit(main())
