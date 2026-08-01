# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Map gameplay's tile graphics back to where they live in the ROM.

The title screen's tile blocks were pinned by this project already, and
World 1-1 uses no tile data outside them. It does not use the same VRAM
layout, though: rendering a level with the title screen's destinations
draws font glyphs, because a level's tile ids index different addresses.

So this reads video RAM's tile data after the level has loaded and finds
each chunk of it in the ROM file, the same way `find_rom_offset.py` pins a
single offset. Contiguous chunks with a constant ROM-to-VRAM delta are
coalesced into blocks, which is what the Rust side needs.

A level's tile ids index whatever atlas is loaded when it is playing, and
that is not one atlas for the whole cartridge: World 2's geometry decodes
correctly and renders as garbage through World 1's blocks. So the level to
read from is an argument.

This gives a partial picture on purpose: it works in 0x40-byte chunks and
drops any chunk that is uniform or that turns up more than once in the ROM,
so a real copy shows up here with gaps in it. `level::tile_overlay` reads the
cartridge's own copy tables instead and covers every byte; the two agree
everywhere this tool has something to say.

Usage: uv run tools/find_gameplay_tile_blocks.py [level]
"""

import sys

sys.path.insert(0, "tools")

from sml_boot import boot_to_gameplay
from trace_level_objects import NAMES, reach_level

VRAM_TILE_BASE = 0x8000
VRAM_TILE_SIZE = 0x1800
CHUNK = 0x40


def main():
    rom = open("super_mario_land.gb", "rb").read()
    want = sys.argv[1] if len(sys.argv) > 1 else "1-1"
    if want == "1-1":
        pb = boot_to_gameplay()
    else:
        pb, _capture = reach_level(NAMES.index(want))
        if pb is None:
            print(f"never reached World {want}")
            return 1
    vram = bytes(pb.memory[VRAM_TILE_BASE + i] for i in range(VRAM_TILE_SIZE))
    pb.stop()
    print(f"World {want}:")

    # ROM offset for each chunk of tile data, where it can be found uniquely.
    placements = []
    for start in range(0, VRAM_TILE_SIZE, CHUNK):
        chunk = vram[start : start + CHUNK]
        if len(set(chunk)) <= 1:
            placements.append((start, None))
            continue
        hits = []
        at = rom.find(chunk)
        while at != -1 and len(hits) < 3:
            hits.append(at)
            at = rom.find(chunk, at + 1)
        placements.append((start, hits[0] if len(hits) == 1 else None))

    blocks = []
    for vram_off, rom_off in placements:
        if rom_off is None:
            continue
        if blocks and blocks[-1][2] == rom_off - blocks[-1][1] - (vram_off - blocks[-1][0]) + 0:
            pass
        if blocks:
            v0, r0, size = blocks[-1]
            if vram_off == v0 + size and rom_off == r0 + size:
                blocks[-1] = (v0, r0, size + CHUNK)
                continue
        blocks.append((vram_off, rom_off, CHUNK))

    print(f"{len(blocks)} blocks of gameplay tile data:")
    for vram_off, rom_off, size in blocks:
        print(f"  rom 0x{rom_off:05X} -> vram 0x{VRAM_TILE_BASE + vram_off:04X}, "
              f"size 0x{size:04X} ({size // 16} tiles)")
    unmatched = sum(1 for _, r in placements if r is None)
    print(f"{unmatched} of {len(placements)} chunks were blank or not uniquely locatable")
    return 0


if __name__ == "__main__":
    sys.exit(main())
