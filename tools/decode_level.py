# /// script
# requires-python = ">=3.10"
# dependencies = []
# ///
"""Decode World 1-1 straight out of the ROM, instead of playing through it.

Found by observation, not by reading a disassembly. `find_level_data.py`
first ruled out the easy possibilities: no row or column of the live tilemap
appears anywhere in the 64K ROM, no 2x2 tile block does either, and no
constant offset applied to the tile indices changes that. A density scan for
the tile ids the opening screen uses put the data in bank 2, and
`find_level_pointer.py` (which watches RAM for a banked ROM pointer that only
moves forward) landed on 0xD010, holding a bank-2 address a few hundred bytes
below the column data. Hexdumping there showed both halves of the format.

Two layers.

A **column record** is a list of runs, terminated by 0xFE:

    (row << 4) | count    place `count` tiles starting at `row`
    <count tile bytes>    the tile ids, top to bottom
    ...                   more runs
    0xFE                  end of column

with two special cases:

    count == 0            means a full 16 rows, not an empty run
    0xFD <tile>           in place of the tile bytes, repeat one tile
                          for the whole run

Anything no run covers is the background filler (tile 44). Worked example,
world column 87:

    02 53 40 | 37 fd f4 | e2 60 61 | fe
    02          -> row 0, 2 tiles: 83, 64
    37 fd f4    -> row 3, 7 tiles, all 244
    e2          -> row 14, 2 tiles: 96, 97

A **level** is a list of 16-bit little-endian pointers to those records,
terminated by 0xFF. Each pointer starts one screen, and a screen is exactly
20 columns, the width of the Game Boy display. Pointers repeat: World 1-1
reuses six of its fifteen screens, which is why world columns 0-19 are
byte-identical to columns 40-59 (both are the screen at 0x62BE).

That repeat is what made an earlier, screen-less reading of this data look
right while being wrong. Decoding linearly from one guessed offset matched
the running game for the first 88 columns purely because 1-1's first five
screens happen to sit in near-linear order, and a 20-column agreement fits
in two places on a level that repeats every 40 columns. Two different start
offsets were published before this one.

The current reading is scored against every column the running game reveals
(0 to 87, captured by `capture_columns.py`), and against every candidate
list start in the surrounding 192 bytes:

    list at 0x0A198   88/88  (100%)
    list at 0x0A1AA   40/80   (50%)
    list at 0x0A1A6   40/88   (45%)

Usage: uv run tools/decode_level.py [--verify] [columns.json]
"""

import json
import sys

ROM_PATH = "super_mario_land.gb"
ROWS = 16  # the playfield, below the two status bar rows
SCREEN_COLUMNS = 20
FILLER = 44
REPEAT = 0xFD
COLUMN_END = 0xFE
LIST_END = 0xFF

# World 1-1's screen list, and the bank-2 window it points through. Bank 2
# is mapped at CPU 0x4000, so a pointer of 0x62BE is ROM offset 0xA2BE.
LEVEL_1_1_LIST = 0x0A198
BANK_BASE = 0x8000
BANK_WINDOW = 0x4000


def rom_offset(pointer):
    return pointer - BANK_WINDOW + BANK_BASE


def decode_column(rom, i):
    """One column record starting at `rom[i]`. Returns (column, next_i).

    Returns (None, i) if a run does not fit the column, which is what tells
    level data apart from whatever else is nearby in the ROM.
    """
    column = [FILLER] * ROWS
    while i < len(rom):
        header = rom[i]
        if header == COLUMN_END:
            return column, i + 1
        row, count = header >> 4, (header & 0x0F) or ROWS
        i += 1
        if row + count > ROWS or i >= len(rom):
            return None, i
        if rom[i] == REPEAT:
            tile = rom[i + 1]
            i += 2
            for n in range(count):
                column[row + n] = tile
        else:
            if i + count > len(rom):
                return None, i
            for n in range(count):
                column[row + n] = rom[i]
                i += 1
    return None, i


def decode_screen(rom, pointer):
    """The 20 columns a single screen pointer draws."""
    columns = []
    i = rom_offset(pointer)
    for _ in range(SCREEN_COLUMNS):
        column, i = decode_column(rom, i)
        if column is None:
            return None
        columns.append(column)
    return columns


def screen_list(rom, start):
    """The 0xFF-terminated pointer list at `start`."""
    pointers = []
    i = start
    while i + 1 < len(rom) and rom[i] != LIST_END:
        pointers.append(rom[i] | (rom[i + 1] << 8))
        i += 2
    return pointers


def decode_level(rom, start=LEVEL_1_1_LIST):
    """Every column of the level whose screen list starts at `start`."""
    columns = []
    for pointer in screen_list(rom, start):
        screen = decode_screen(rom, pointer)
        if screen is None:
            break
        columns += screen
    return columns


def score(rom, start, truth):
    """How many of the observed columns a candidate list start reproduces."""
    columns = decode_level(rom, start)
    overlap = [n for n in truth if n < len(columns)]
    if not overlap:
        return 0.0, 0, 0, len(columns)
    hits = sum(1 for n in overlap if columns[n] == truth[n])
    return hits / len(overlap), hits, len(overlap), len(columns)


def verify(rom, truth):
    """Score every plausible list start, not just the expected one.

    Checking only the expected offset is what let two wrong answers stand.
    """
    scores = []
    for start in range(LEVEL_1_1_LIST - 0x60, LEVEL_1_1_LIST + 0x60):
        rate, hits, total, length = score(rom, start, truth)
        if total >= 80:
            scores.append((rate, hits, total, start, length))
    scores.sort(reverse=True)

    print("best screen lists (scored against every observed column):")
    for rate, hits, total, start, length in scores[:3]:
        print(f"  0x{start:05X}: {hits}/{total} ({rate:.0%}), {length} columns")

    rate, hits, total, start, length = scores[0]
    ok = start == LEVEL_1_1_LIST and rate == 1.0
    print(
        f"\n{'PASS' if ok else 'FAIL'}: expected 0x{LEVEL_1_1_LIST:05X} at 100%, "
        f"best was 0x{start:05X} at {rate:.0%}"
    )
    return ok


def render(columns):
    for row in range(ROWS):
        line = "".join(
            "." if c[row] == FILLER else ("#" if c[row] in (96, 97) else "o")
            for c in columns
        )
        print(f"{row:2d} {line}")


def main():
    rom = open(ROM_PATH, "rb").read()
    args = [a for a in sys.argv[1:] if a != "--verify"]

    if "--verify" in sys.argv:
        path = args[0] if args else "columns.json"
        truth = {int(k): v for k, v in json.load(open(path)).items()}
        return 0 if verify(rom, truth) else 1

    pointers = screen_list(rom, LEVEL_1_1_LIST)
    columns = decode_level(rom)
    print(f"{len(pointers)} screens, {len(columns)} columns")
    print("  " + " ".join(f"{p:04X}" for p in pointers) + "\n")
    render(columns)
    return 0


if __name__ == "__main__":
    sys.exit(main())
