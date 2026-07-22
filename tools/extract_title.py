# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy", "pillow"]
# ///
"""Extract the Super Mario Land title screen from a running emulator.

We boot the verified ROM headlessly with PyBoy, let it reach the title screen,
then read the background tile map and tile data straight out of VRAM. That is
the "observe a real emulator" approach: we take what the game actually draws,
not a guess at ROM offsets.

Output (all gitignored, never committed as ROM data) goes to assets/extracted/:
  title.tiles   our SMLT tile-sheet format (deduplicated tiles + palette)
  title.tmap    our SMLM tile-map format (20x18 indices into the sheet)
  title_ref.png a reference screenshot for local eyeballing

Run: uv run tools/extract_title.py
"""

import struct
import sys
from pathlib import Path

from pyboy import PyBoy

ROM = "super_mario_land.gb"
OUT = Path("assets/extracted")
BOOT_FRAMES = 600
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
    pb = PyBoy(ROM, window="null")
    for _ in range(BOOT_FRAMES):
        pb.tick()

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

    # Walk the visible grid, deduplicate tiles, build a remapped index map.
    unique = {}
    order = []
    cells = []
    for row in range(ROWS):
        for col in range(COLS):
            mx = (scx // 8 + col) & 31
            my = (scy // 8 + row) & 31
            map_index = pb.memory[map_base + my * 32 + mx]
            decoded = decode_tile(read_tile(map_index))
            if decoded not in unique:
                unique[decoded] = len(order)
                order.append(decoded)
            cells.append(unique[decoded])

    OUT.mkdir(parents=True, exist_ok=True)

    # SMLT tile sheet: magic, version, palette, u32 count, 64 bytes per tile.
    tiles_blob = b"SMLT" + bytes([1, bgp]) + struct.pack("<I", len(order))
    for tile in order:
        tiles_blob += tile
    (OUT / "title.tiles").write_bytes(tiles_blob)

    # SMLM tile map: magic, version, u16 width, u16 height, index bytes.
    map_blob = b"SMLM" + bytes([1]) + struct.pack("<HH", COLS, ROWS) + bytes(cells)
    (OUT / "title.tmap").write_bytes(map_blob)

    # Reference screenshot for local comparison (gitignored).
    pb.screen.image.save(OUT / "title_ref.png")
    pb.stop()

    print(f"LCDC=0x{lcdc:02X} BGP=0x{bgp:02X} SCX={scx} SCY={scy}")
    print(f"tile addressing: {'unsigned 0x8000' if unsigned else 'signed 0x8800'}")
    print(f"background map base: 0x{map_base:04X}")
    print(f"unique tiles: {len(order)} of {COLS * ROWS} cells")
    print(f"wrote {OUT/'title.tiles'}, {OUT/'title.tmap'}, {OUT/'title_ref.png'}")


if __name__ == "__main__":
    sys.exit(main())
