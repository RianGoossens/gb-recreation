# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy", "numpy"]
# ///
"""Find the RAM pointer the game uses to walk World 1-1's column stream.

`decode_level.py` decodes the column records but had to guess where they
start, and guessed wrong twice. The game itself does not guess: to stream a
new column in every 8 pixels of scroll it must hold a read position
somewhere in RAM and advance it. Finding that variable settles the start
offset by observation instead of by pattern matching, and reading it on the
first frame of the level says exactly where the opening screen comes from.

The signature is narrow. Bank 2 mapped at 0x4000 puts the known records at
CPU address 0x6206, so the pointer is a 16-bit value in 0x4000..0x7FFF that
never decreases and climbs by roughly one record (a handful of bytes) per
column of scroll.

Usage: uv run tools/find_level_pointer.py
"""

import sys

sys.path.insert(0, "tools")

from sml_boot import boot_to_gameplay
from sml_scroll import ScrollTracker
from sml_walker import ReactiveWalker

RAM_RANGES = [(0xC000, 0xE000), (0xFF80, 0xFFFF)]
SAMPLE_EVERY = 30
SAMPLES = 30
ROM_WINDOW_LOW = 0x4000
ROM_WINDOW_HIGH = 0x8000


def read_ram(pb):
    return {
        base: bytes(pb.memory[base:end]) for base, end in [(a, b) for a, b in RAM_RANGES]
    }


def le16(block, i):
    return block[i] | (block[i + 1] << 8)


def collect(pb, tracker, walker):
    """RAM plus measured scroll, sampled while Mario walks right."""
    samples = [(0, read_ram(pb))]
    for _ in range(SAMPLES):
        for _ in range(SAMPLE_EVERY):
            walker.step(pb, tracker)
            pb.tick()
            tracker.update(pb)
        if tracker.frozen > 5:
            break
        samples.append((tracker.scroll, read_ram(pb)))
    return samples


def candidates(samples):
    """Addresses holding a banked-ROM pointer that only ever moves forward."""
    scrolls = [s for s, _ in samples]
    columns = (scrolls[-1] - scrolls[0]) / 8
    found = []
    for base, end in RAM_RANGES:
        for off in range(end - base - 1):
            values = [le16(ram[base], off) for _, ram in samples]
            if not all(ROM_WINDOW_LOW <= v < ROM_WINDOW_HIGH for v in values):
                continue
            if any(b < a for a, b in zip(values, values[1:])):
                continue
            if values[-1] == values[0]:
                continue
            per_column = (values[-1] - values[0]) / max(columns, 1)
            found.append((base + off, values[0], values[-1], per_column))
    return found, columns


def main():
    pb = boot_to_gameplay()
    tracker = ScrollTracker(pb)
    walker = ReactiveWalker(pb)
    samples = collect(pb, tracker, walker)
    pb.stop()

    found, columns = candidates(samples)
    print(f"{len(samples)} samples, {columns:.0f} columns of scroll\n")
    if not found:
        print("no monotonic banked-ROM pointer found")
        return 1

    print("addr    start   end     bytes/column")
    for addr, first, last, per_column in found:
        print(f"{addr:04X}   {first:04X}    {last:04X}    {per_column:5.1f}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
