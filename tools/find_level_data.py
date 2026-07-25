# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Look for World 1-1's level data in the ROM, instead of playing to find it.

Every attempt at the level so far has driven Mario through it and recorded
what streamed past, which caps at however far a scripted run survives (world
column 78 at the moment). If the geometry is in the ROM, all of it is
available at once.

Same technique as the title screen's tile offsets (`find_rom_offset.py`):
take bytes the game actually produced, and search the ROM file for them. If
the level is stored as a raw tilemap, a run of the live tilemap will be
found verbatim. If it is stored as something more compact (a table of
chunk indices, run-length data, a per-screen block list) the raw search
fails, and the shape of that failure is itself information: which runs
appear and which do not narrows down how it is encoded.

Usage: uv run tools/find_level_data.py
"""

import sys
from collections import Counter

from sml_boot import boot_to_gameplay

MAP_BASE = 0x9800
ROWS = 18
COLS = 32
HUD_ROWS = 2
ROM_PATH = "super_mario_land.gb"


def read_map(pb):
    return [
        [pb.memory[MAP_BASE + row * 32 + col] for col in range(COLS)]
        for row in range(ROWS)
    ]


def occurrences(rom, needle):
    out, start = [], 0
    while True:
        i = rom.find(needle, start)
        if i == -1:
            return out
        out.append(i)
        start = i + 1


def search_rows(rom, tilemap):
    """Look for each visible row of the tilemap, longest run first."""
    print("Raw row search (is the level stored as a plain tilemap?)\n")
    any_hit = False
    for row in range(HUD_ROWS, ROWS):
        for length in (20, 12, 8):
            needle = bytes(tilemap[row][:length])
            if len(set(needle)) < 3:
                continue  # a run of one repeated tile matches everywhere
            hits = occurrences(rom, needle)
            if hits:
                any_hit = True
                where = ", ".join(f"0x{h:05X}" for h in hits[:4])
                print(f"  row {row:2d}, first {length} tiles: {len(hits)} hit(s) at {where}")
                break
        else:
            print(f"  row {row:2d}: no run of 8 or more found")
    if not any_hit:
        print("\n  Nothing. The level is not a raw tilemap in the ROM.")
    return any_hit


def search_columns(rom, tilemap):
    """Look for each column of the tilemap, which is how it streams in."""
    print("\nRaw column search (the buffer streams one column at a time)\n")
    hits_found = 0
    for col in range(COLS):
        needle = bytes(tilemap[row][col] for row in range(HUD_ROWS, ROWS))
        if len(set(needle)) < 3:
            continue
        hits = occurrences(rom, needle)
        if hits:
            hits_found += 1
            where = ", ".join(f"0x{h:05X}" for h in hits[:4])
            print(f"  column {col:2d} ({len(needle)} tiles): {len(hits)} hit(s) at {where}")
    if not hits_found:
        print("  Nothing. Columns are not stored verbatim either.")
    return hits_found


def report_tile_stats(tilemap):
    counts = Counter(t for row in tilemap[HUD_ROWS:] for t in row)
    print("\nTile makeup of the opening screen (most common first):")
    for tile, n in counts.most_common(8):
        print(f"  tile {tile:3d}: {n:3d} cells")
    print(f"  {len(counts)} distinct tiles across {sum(counts.values())} cells")


def blocks_2x2(tilemap):
    """Distinct 2x2 tile groups on the even grid, as (top-left) -> bytes.

    A 16x16 pixel block built from four 8x8 tiles is the usual way a Game Boy
    platformer stores level geometry compactly. If that is what SML does, the
    screen should decompose into a small alphabet of these.
    """
    seen = {}
    for row in range(HUD_ROWS, ROWS - 1, 2):
        for col in range(0, COLS - 1, 2):
            quad = (
                tilemap[row][col],
                tilemap[row][col + 1],
                tilemap[row + 1][col],
                tilemap[row + 1][col + 1],
            )
            seen.setdefault(quad, []).append((col, row))
    return seen


def search_blocks(rom, tilemap):
    """Do the 2x2 groups appear in the ROM, and do their hits cluster?"""
    groups = blocks_2x2(tilemap)
    print(f"\n2x2 block search: {len(groups)} distinct groups on the opening screen\n")

    buckets = Counter()
    found = 0
    for quad in groups:
        if len(set(quad)) < 3:
            continue  # too bland to be distinctive
        a, b, c, d = quad
        for order, name in (((a, b, c, d), "rows"), ((a, c, b, d), "cols")):
            for off in occurrences(rom, bytes(order)):
                buckets[off // 0x100] += 1
                found += 1
        del name

    if not found:
        print("  No 2x2 group appears anywhere in the ROM.")
        return
    print(f"  {found} hits. Busiest 256-byte regions (a chunk table would cluster):")
    for page, n in buckets.most_common(10):
        print(f"    0x{page * 0x100:05X}: {n} hits")


def search_shifted(rom, tilemap):
    """The same column search, with every tile index shifted by a constant.

    The tile numbers read out of VRAM are what the game wrote there, and the
    level data could store them in a different index space (the background
    uses signed tile addressing, so an offset would be unsurprising). A sweep
    over every possible constant offset rules that out cheaply.
    """
    print("\nColumn search with a constant tile-index offset applied\n")
    columns = []
    for col in range(COLS):
        needle = [tilemap[row][col] for row in range(HUD_ROWS, ROWS)]
        if len(set(needle)) >= 3:
            columns.append((col, needle))

    hits = []
    for shift in range(256):
        for col, needle in columns:
            shifted = bytes((t + shift) & 0xFF for t in needle)
            for off in occurrences(rom, shifted):
                hits.append((shift, col, off))
    if not hits:
        print(f"  No offset works, across all 256 shifts and {len(columns)} columns.")
        return
    for shift, col, off in hits[:10]:
        print(f"  shift {shift}: column {col} found at 0x{off:05X}")


def search_density(rom, tilemap):
    """Where in the ROM do the observed tile IDs cluster?

    The screen layout is not in the ROM in any tile-index form, so the level
    must be indices into a table whose entries are tile IDs. That table is in
    the ROM, and its bytes are drawn from the same small alphabet the screen
    uses. Scanning for windows dense in that alphabet finds candidate tables
    without knowing the format.
    """
    alphabet = {t for row in tilemap[HUD_ROWS:] for t in row}
    # Drop the background filler: it is a single very common value and would
    # make any run of zeros-equivalent look like a hit.
    filler = Counter(t for row in tilemap[HUD_ROWS:] for t in row).most_common(1)[0][0]
    alphabet.discard(filler)
    print(f"\nDensity scan for the {len(alphabet)} non-filler tile IDs used on screen")
    print(f"  alphabet: {sorted(alphabet)}\n")

    window = 64
    scores = []
    for start in range(0, len(rom) - window, 16):
        chunk = rom[start : start + window]
        hits = sum(1 for b in chunk if b in alphabet)
        scores.append((hits, start))
    scores.sort(reverse=True)
    print(f"  densest {window}-byte windows (out of {window} bytes):")
    shown, last = 0, -1
    for hits, start in scores:
        if start - last < window:
            continue
        print(f"    0x{start:05X}: {hits}/{window} bytes are screen tiles")
        last, shown = start, shown + 1
        if shown == 8:
            break


def main():
    pb = boot_to_gameplay()
    tilemap = read_map(pb)
    pb.stop()

    rom = open(ROM_PATH, "rb").read()
    print(f"ROM is {len(rom)} bytes ({len(rom) // 0x4000} banks of 16K)\n")

    report_tile_stats(tilemap)
    print()
    search_rows(rom, tilemap)
    search_columns(rom, tilemap)
    search_blocks(rom, tilemap)
    search_shifted(rom, tilemap)
    search_density(rom, tilemap)
    return 0


if __name__ == "__main__":
    sys.exit(main())
