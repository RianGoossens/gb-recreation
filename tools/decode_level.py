# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Decode World 1-1 straight out of the ROM, instead of playing through it.

Found by observation, not by reading a disassembly. `find_level_data.py`
first ruled out the easy possibilities: no row or column of the live tilemap
appears anywhere in the 64K ROM, no 2x2 tile block does either, and no
constant offset applied to the tile indices changes that. So the level is
not stored as tiles laid out the way they appear.

A density scan for the tile IDs the opening screen actually uses put the
busiest windows in bank 2, and hexdumping there showed an obvious repeating
delimiter: `fe`, then `53 40`, which are tiles 83 and 64, the two that make
up the top of every column on screen. Lining those records up against the
live tilemap column by column decoded the whole thing:

    0xFE                      start of a column
    (row << 4) | count        place `count` tiles starting at `row`
    <count tile bytes>        the tile ids, top to bottom
    ... more runs ...
    (until the next 0xFE)

Everything not covered by a run is the background filler (tile 44). `0xFE`
cannot be mistaken for a run header, since row 15 with a count of 14 would
run off the bottom of a 16-row column.

Worked example, the third column of the level:

    fe 02 53 40 b1 5e e2 60 61
       02          -> row 0, 2 tiles: 83, 64
          53 40
             b1    -> row 11, 1 tile: 94
                5e
                   e2 -> row 14, 2 tiles: 96, 97
                      60 61

which is exactly what the emulator has in that column.

**Not finished.** The first 20 columns decode to exactly what the emulator
renders, all 16 rows, every tile. From column 20 the stream desynchronises:
the ROM record there holds tiles (82, and 232 further on) that the running
game does not draw until around world column 68, so the decoder is running
ahead of the game. At least one control code or record form is still
unaccounted for. `--verify` reports this rather than hiding it.

Usage: uv run tools/decode_level.py [--verify]
"""

import sys

ROM_PATH = "super_mario_land.gb"
COLUMN_START = 0xFE
ROWS = 16  # the playfield, below the two status bar rows
FILLER = 44

# Where World 1-1's column records begin. Pinned by the one tile run on the
# opening screen that is unique in the whole ROM (`36 71 73`, the 54/113/115
# of the pyramid's second column) and stepping back two records from it.
LEVEL_1_1_START = 0x0A2BD


def decode_column(rom, i):
    """One column starting at the 0xFE at `rom[i]`. Returns (column, next_i).

    Parsed strictly forward, consuming exactly what each run header asks for.
    Splitting the stream on every 0xFE instead looks right for the first
    47 columns and then breaks, because a tile id can itself be 0xFE and a
    0xFE inside a run is data, not the next column.

    Returns (None, i) if a run does not fit the column, which is what tells
    the level's data apart from whatever follows it in the ROM.
    """
    column = [FILLER] * ROWS
    i += 1
    while i < len(rom) and rom[i] != COLUMN_START:
        row, count = rom[i] >> 4, rom[i] & 0x0F
        i += 1
        if count == 0 or row + count > ROWS or i + count > len(rom):
            return None, i
        for n in range(count):
            column[row + n] = rom[i]
            i += 1
    return column, i


def decode_columns(rom, start, limit=None):
    """Successive columns from `start` until the records stop parsing."""
    columns = []
    i = start
    while i < len(rom) and rom[i] == COLUMN_START:
        column, i = decode_column(rom, i)
        if column is None:
            break
        columns.append(column)
        if limit and len(columns) >= limit:
            break
    return columns


def live_columns(pb, camera_col):
    """The 20 visible columns, as world columns, from the running game.

    The background map is a 32-column ring buffer, so world column N lives at
    buffer index N % 32.
    """
    return [
        (
            camera_col + i,
            [pb.memory[0x9800 + r * 32 + ((camera_col + i) % 32)] for r in range(2, 18)],
        )
        for i in range(20)
    ]


def compare(decoded, live, label):
    matched = 0
    for world_col, want in live:
        if world_col >= len(decoded):
            continue
        if want == decoded[world_col]:
            matched += 1
        elif matched + 3 > len(live):
            pass
        else:
            print(f"  world column {world_col} differs:")
            print(f"    emulator: {want}")
            print(f"    decoded:  {decoded[world_col]}")
    print(f"{label}: {matched}/{len(live)} columns match")
    return matched == len(live)


def verify(rom):
    """Compare the decode against the running game, at spawn and scrolled.

    Matching only at spawn would not prove much: the opening screen is what
    the format was worked out against. Scrolling well into the level and
    matching there is the real test.
    """
    sys.path.insert(0, "tools")
    from sml_boot import boot_to_gameplay
    from sml_scroll import ScrollTracker
    from sml_walker import ReactiveWalker

    decoded = decode_columns(rom, LEVEL_1_1_START)
    print(f"decoded {len(decoded)} columns from 0x{LEVEL_1_1_START:05X}\n")

    pb = boot_to_gameplay()
    ok = compare(decoded, live_columns(pb, 0), "at spawn")

    tracker = ScrollTracker(pb)
    walker = ReactiveWalker(pb)
    checkpoints = [24, 40, 56]
    for target in checkpoints:
        while tracker.scroll // 8 < target:
            walker.step(pb, tracker)
            pb.tick()
            tracker.update(pb)
            if tracker.frozen > 5:
                print(f"died before reaching camera column {target}")
                pb.stop()
                return False
        camera_col = tracker.scroll // 8
        ok &= compare(decoded, live_columns(pb, camera_col), f"at camera column {camera_col}")
    pb.stop()
    return ok


def main():
    rom = open(ROM_PATH, "rb").read()

    if "--verify" in sys.argv:
        return 0 if verify(rom) else 1

    columns = decode_columns(rom, LEVEL_1_1_START)
    print(f"decoded {len(columns)} columns from 0x{LEVEL_1_1_START:05X}\n")
    for row in range(ROWS):
        line = "".join(
            "." if col[row] == FILLER else ("#" if col[row] in (96, 97) else "o")
            for col in columns
        )
        print(f"{row:2d} {line}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
