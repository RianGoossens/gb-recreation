# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy", "pillow"]
# ///
"""Extract World 1-1's opening screen from a running emulator.

Same technique as extract_title.py: boot the verified ROM headlessly with
PyBoy, let it reach gameplay, then read the background tile map and tile
data straight out of VRAM. This captures only the screen visible at spawn
(one 20x18 view, no scrolling yet); stitching the full scrolling width is
follow-up work.

Output (all gitignored, never committed as ROM data) goes to assets/extracted/:
  level_1_1_opening.tiles   our SMLT tile-sheet format (deduplicated tiles + palette)
  level_1_1_opening.tmap    our SMLM tile-map format (20x18 indices into the sheet)
  level_1_1_opening_ref.png a reference screenshot for local eyeballing

Run: uv run tools/extract_level_1_1.py
"""

import struct
import sys
from pathlib import Path

from sml_boot import boot_to_gameplay

OUT = Path("assets/extracted")
COLS, ROWS = 20, 18  # visible Game Boy tiles


def decode_tile(tile_bytes):
    """16 bytes of 2bpp Game Boy tile data -> 64 color indices (0..3), row major."""
    out = []
    for row in range(8):
        low = tile_bytes[row * 2]
        high = tile_bytes[row * 2 + 1]
        for x in range(8):
            bit = 7 - x
            lo = (low >> bit) & 1
            hi = (high >> bit) & 1
            out.append((hi << 1) | lo)
    return bytes(out)


def main():
    pb = boot_to_gameplay()

    lcdc = pb.memory[0xFF40]
    bgp = pb.memory[0xFF47]
    scx = pb.memory[0xFF43]
    scy = pb.memory[0xFF42]
    unsigned = (lcdc >> 4) & 1
    map_base = 0x9C00 if (lcdc >> 3) & 1 else 0x9800

    def tile_addr(index):
        if unsigned:
            return 0x8000 + index * 16
        signed = index - 256 if index >= 128 else index
        return 0x9000 + signed * 16

    def read_tile(index):
        base = tile_addr(index)
        return bytes(pb.memory[base + i] for i in range(16))

    unique = {}
    order = []
    raw_indices = []
    cells = []
    for row in range(ROWS):
        for col in range(COLS):
            mx = (scx // 8 + col) & 31
            my = (scy // 8 + row) & 31
            map_index = pb.memory[map_base + my * 32 + mx]
            raw_indices.append(map_index)
            decoded = decode_tile(read_tile(map_index))
            if decoded not in unique:
                unique[decoded] = len(order)
                order.append(decoded)
            cells.append(unique[decoded])

    OUT.mkdir(parents=True, exist_ok=True)

    tiles_blob = b"SMLT" + bytes([1, bgp]) + struct.pack("<I", len(order))
    for tile in order:
        tiles_blob += tile
    (OUT / "level_1_1_opening.tiles").write_bytes(tiles_blob)

    map_blob = b"SMLM" + bytes([1]) + struct.pack("<HH", COLS, ROWS) + bytes(cells)
    (OUT / "level_1_1_opening.tmap").write_bytes(map_blob)

    pb.screen.image.save(OUT / "level_1_1_opening_ref.png")
    pb.stop()

    print(f"LCDC=0x{lcdc:02X} BGP=0x{bgp:02X} SCX={scx} SCY={scy}")
    print(f"tile addressing: {'unsigned 0x8000' if unsigned else 'signed 0x8800'}")
    print(f"background map base: 0x{map_base:04X}")
    print(f"unique tiles: {len(order)} of {COLS * ROWS} cells")
    print(f"distinct raw tile indices: {sorted(set(raw_indices))}")
    print(f"wrote {OUT/'level_1_1_opening.tiles'}, {OUT/'level_1_1_opening.tmap'}, {OUT/'level_1_1_opening_ref.png'}")


if __name__ == "__main__":
    sys.exit(main())
