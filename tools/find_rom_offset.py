# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Find where a piece of emulator memory actually lives in the ROM file.

This is the preferred way to pin a ROM offset: boot the verified ROM, let the
game run to the state you care about, read the bytes out of the emulator's
memory (VRAM, OAM, wherever the game put them), and search the ROM file for
that exact byte sequence. Whatever offset it is found at is where those bytes
really come from, bank switching included. No disassembly reading required:
the ROM file and the emulator's own behavior are the only inputs.

Usage:
  uv run tools/find_rom_offset.py <mem_start_hex> <length> [frames]

Example, the title screen's menu/logo tile block:
  uv run tools/find_rom_offset.py 0x9300 64 600

Prints every ROM offset where the byte sequence occurs, plus which bank that
implies (offset // 0x4000) and the CPU address it corresponds to in the
bank-switched window ($4000-$7FFF) if the bank is not 0.
"""

import sys
from pyboy import PyBoy

ROM = "super_mario_land.gb"


def main():
    if len(sys.argv) < 3:
        print(__doc__)
        return 1

    mem_start = int(sys.argv[1], 0)
    length = int(sys.argv[2], 0)
    frames = int(sys.argv[3], 0) if len(sys.argv) > 3 else 600

    pb = PyBoy(ROM, window="null")
    for _ in range(frames):
        pb.tick()
    needle = bytes(pb.memory[mem_start + i] for i in range(length))
    pb.stop()

    rom = open(ROM, "rb").read()
    print(f"searching for {length} bytes read from emulator memory 0x{mem_start:04X}")
    print(f"bytes: {needle.hex()}")

    found = []
    start = 0
    while True:
        idx = rom.find(needle, start)
        if idx == -1:
            break
        found.append(idx)
        start = idx + 1

    if not found:
        print("not found in the ROM file (try a shorter or different length/region)")
        return 1

    for off in found:
        bank, local = off // 0x4000, off % 0x4000
        cpu_addr = f"0x{0x4000 + local:04X}" if bank else f"0x{local:04X}"
        print(f"  ROM offset 0x{off:04X}: bank {bank}, CPU addr {cpu_addr} (when that bank is switched in)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
