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

The stream starts at world column 21, one screen in. Nothing decodes the
columns before that, anywhere in the ROM, which fits how a scrolling game
is usually built: the opening screen is drawn once when the level loads,
and this data is what streams in as the camera moves. Reading those first
columns out of the ROM directly is the one piece still missing.

Usage: uv run tools/decode_level.py [--verify]
"""

import sys

ROM_PATH = "super_mario_land.gb"
COLUMN_START = 0xFE
ROWS = 16  # the playfield, below the two status bar rows
FILLER = 44

# Where World 1-1's streaming column records start, and which world column
# the first record draws.
#
# Both numbers were wrong twice before landing here, in the same way each
# time: checked against too little data. A first attempt pinned 0x0A2BD by
# finding the one tile run on the opening screen that is unique in the whole
# ROM (`36 71 73`) and stepping back two records, which gave 20 consecutive
# columns matching the game at spawn exactly. That is not proof on a level
# that repeats every 40 columns, and it was fitted to the wrong one of the
# two places a 20-column window fits.
#
# Scored against every world column the running game reveals (0 to 87, read
# off the ring buffer as the camera moves, each column captured the first
# time it appears so Mario cannot alter it by collecting a coin first):
#
#   record k = column k + 21   66/67 (99%)
#   record k = column k - 19   21/88 (24%)
#   record k = column k + 19   12/69 (17%)
SEGMENT_START = 0x0A206
SEGMENT_FIRST_COLUMN = 21


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


def ground_truth(pb, tracker, walker, frames=900):
    """Every world column the running game reveals, read from the ring buffer.

    Sampling as the camera moves gives real columns well past the opening
    screen, which is what makes the alignment check below meaningful.

    Each column is captured the first time it appears and never overwritten.
    Coins live in the background tilemap, so a column re-read after Mario has
    walked through it is missing the ones he collected. Keeping the later
    reads instead costs about 4 points of match rate.
    """
    truth = {}

    def sample():
        camera = tracker.scroll // 8
        for i in range(20):
            n = camera + i
            if n not in truth:
                truth[n] = [
                    pb.memory[0x9800 + r * 32 + (n % 32)] for r in range(2, 18)
                ]

    sample()
    for _ in range(frames):
        walker.step(pb, tracker)
        pb.tick()
        tracker.update(pb)
        if tracker.frozen > 5:
            break
        sample()
    return truth


def verify(rom):
    """Check the decode against columns the running game actually revealed.

    Comparing only at spawn is what produced the wrong offset in the first
    place: this level repeats every 40 columns, so a 20-column window fits
    in two places. Scoring every available column against every candidate
    offset is what separates them.
    """
    sys.path.insert(0, "tools")
    from sml_boot import boot_to_gameplay
    from sml_scroll import ScrollTracker
    from sml_walker import ReactiveWalker

    decoded = decode_columns(rom, SEGMENT_START)
    pb = boot_to_gameplay()
    tracker = ScrollTracker(pb)
    walker = ReactiveWalker(pb)
    truth = ground_truth(pb, tracker, walker)
    pb.stop()

    columns = sorted(truth)
    print(f"decoded {len(decoded)} columns from 0x{SEGMENT_START:05X}")
    print(f"ground truth: world columns {columns[0]}..{columns[-1]}\n")

    scores = []
    for offset in range(0, 64):
        overlap = [n for n in columns if 0 <= n - offset < len(decoded)]
        if len(overlap) < 16:
            continue
        hits = sum(1 for n in overlap if decoded[n - offset] == truth[n])
        scores.append((hits / len(overlap), hits, len(overlap), offset))
    scores.sort(reverse=True)
    print("best alignments (record k against world column k + offset):")
    for rate, hits, total, offset in scores[:3]:
        print(f"  offset {offset:2d}: {hits}/{total} columns ({rate:.0%})")

    rate, hits, total, offset = scores[0]
    ok = offset == SEGMENT_FIRST_COLUMN and rate > 0.95
    print(
        f"\n{'PASS' if ok else 'FAIL'}: expected offset {SEGMENT_FIRST_COLUMN}, "
        f"best was {offset} at {rate:.0%}"
    )
    return ok


def main():
    rom = open(ROM_PATH, "rb").read()

    if "--verify" in sys.argv:
        return 0 if verify(rom) else 1

    columns = decode_columns(rom, SEGMENT_START)
    print(f"decoded {len(columns)} columns from 0x{SEGMENT_START:05X}\n")
    for row in range(ROWS):
        line = "".join(
            "." if col[row] == FILLER else ("#" if col[row] in (96, 97) else "o")
            for col in columns
        )
        print(f"{row:2d} {line}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
