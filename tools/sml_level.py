# /// script
# requires-python = ">=3.10"
# dependencies = []
# ///
"""Decoding a level out of the ROM, for the observation tools.

The product's decoder is Rust (`src/assets/level.rs`); this is the same
format read from Python so a running emulator can be compared against it
without shelling out. Format notes live in `docs/reference/level-1-1.md`.

Not a runnable tool: import it, e.g. `from sml_level import known_screens`.
"""

ROWS = 16
VISIBLE_COLUMNS = 20

ROM_PATH = "super_mario_land.gb"
FILLER = 44
REPEAT = 0xFD
COLUMN_END = 0xFE
LIST_END = 0xFF
BANK_BASE = 0x8000
BANK_WINDOW = 0x4000
LIST_STARTS = (0x0A190, 0x0A1B7, 0x0A1DA)


def decode_column(rom, i):
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
            for n in range(count):
                column[row + n] = rom[i]
                i += 1
    return None, i


def decode_screen(rom, pointer):
    columns = []
    i = pointer - BANK_WINDOW + BANK_BASE
    for _ in range(VISIBLE_COLUMNS):
        column, i = decode_column(rom, i)
        if column is None:
            return None
        columns.append(column)
    return columns


def known_screens():
    """Every screen every candidate list points at, keyed by its columns."""
    rom = open(ROM_PATH, "rb").read()
    screens = {}
    for start in LIST_STARTS:
        i = start
        while rom[i] != LIST_END:
            pointer = rom[i] | (rom[i + 1] << 8)
            i += 2
            columns = decode_screen(rom, pointer)
            if columns is not None:
                screens.setdefault(repr(columns), []).append((start, pointer))
    return screens




def screen_list(rom, start):
    pointers = []
    i = start
    while i + 1 < len(rom) and rom[i] != LIST_END:
        pointers.append(rom[i] | (rom[i + 1] << 8))
        i += 2
    return pointers


def decode_level(rom, start):
    columns = []
    for pointer in screen_list(rom, start):
        screen = decode_screen(rom, pointer)
        if screen is None:
            break
        columns += screen
    return columns
