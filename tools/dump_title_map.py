# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Dump the raw VRAM tile map indices for the title screen.

This is a one-time tool to capture the 20x18 VRAM tile indices that the
game's level loader writes to 0x9800 for the title screen. The output is
used to hard-code the tilemap in the Rust extraction code, after which
this script is no longer needed.

Run: uv run tools/dump_title_map.py
"""

import sys

from sml_boot import boot_to_title

COLS, ROWS = 20, 18


def main():
    pb = boot_to_title()

    lcdc = pb.memory[0xFF40]
    scx = pb.memory[0xFF43]
    scy = pb.memory[0xFF42]
    map_base = 0x9C00 if (lcdc >> 3) & 1 else 0x9800

    indices = []
    for row in range(ROWS):
        for col in range(COLS):
            mx = (scx // 8 + col) & 31
            my = (scy // 8 + row) & 31
            map_index = pb.memory[map_base + my * 32 + mx]
            indices.append(map_index)

    pb.stop()

    # Print as a Rust array literal.
    print(f"// {COLS}x{ROWS} = {len(indices)} VRAM tile indices for the title screen")
    print(f"// LCDC=0x{lcdc:02X}, map_base=0x{map_base:04X}, SCX={scx}, SCY={scy}")
    print("[")
    for row in range(ROWS):
        row_data = indices[row * COLS : (row + 1) * COLS]
        line = ", ".join(f"0x{b:02X}" for b in row_data)
        print(f"    {line},")
    print("]")


if __name__ == "__main__":
    sys.exit(main())
