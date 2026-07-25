# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Find where the game keeps World 1-1's scroll position, by scanning WRAM.

Every attempt at stitching the full 1-1 tilemap so far has had to *derive*
Mario's world position (dead reckoning, integrating the speed byte, guessing
lap numbers), and every one of those derivations has needed a correction
later. See `docs/reference/level-1-1.md` for the trail.

`SCX`, the hardware register that shifts the background left and right, is
not readable once per frame here: Super Mario Land splits the screen with a
mid-scanline STAT interrupt so the status bar draws at `SCX = 0` and the
playfield draws at the real scroll value, and a VBlank-time sample almost
always catches the reset-to-zero.

But the game has to keep the real value somewhere in WRAM to write it from.
That variable has a distinctive shape: it sits at 0 at spawn, and once the
camera lock engages it climbs by exactly Mario's walking speed every frame
and never goes backwards. Scanning every 16-bit little-endian pair in WRAM
for that shape finds it without knowing its address ahead of time, the same
way `find_mario_speed.py` found the speed register.

Usage: uv run tools/find_scroll_position.py
"""

import sys

from sml_boot import boot_to_gameplay

WRAM_START = 0xC000
WRAM_END = 0xE000
MARIO_X = 0xC202
# Holding Right walks into the level's first enemy and dies around frame 338
# (see docs/reference/level-1-1.md), which resets everything. Stop before it.
FRAMES = 320


def main():
    pb = boot_to_gameplay()
    pb.button_press("right")

    snaps = []
    screen_x = []
    for _ in range(FRAMES):
        pb.tick()
        snaps.append(bytes(pb.memory[WRAM_START:WRAM_END]))
        screen_x.append(pb.memory[MARIO_X])
    pb.button_release("right")

    # The camera lock is the frame Mario's screen X stops moving for good.
    lock = next(
        f for f in range(len(screen_x)) if all(x == screen_x[-1] for x in screen_x[f:])
    )
    expected = FRAMES - lock  # ~1 px/frame at saturated walking speed
    print(f"camera locks at frame {lock}, screen x {screen_x[-1]}")
    print(f"expecting the scroll variable to advance about {expected} px\n")

    size = WRAM_END - WRAM_START
    scan(snaps, size, "16-bit counter", expected, 1, lock)
    # A pixel-scroll byte wraps at 256, so it cannot be monotonic; accumulate
    # its wrapped deltas instead. A tile-column counter advances once per 8
    # pixels, so check that scale too.
    scan_wrapped(snaps, size, "8-bit pixel scroll", expected, 1, lock)
    scan_wrapped(snaps, size, "8-bit tile column", expected / 8, 1 / 8, lock)
    return 0


def scan(snaps, size, label, expected, step, lock):
    hits = []
    for off in range(size - 1):
        for (lo, hi), name in ((((0, 1)), "LE"), (((1, 0)), "BE")):
            values = [s[off + lo] | (s[off + hi] << 8) for s in snaps]
            if not (0.7 * expected <= values[-1] - values[0] <= 1.3 * expected):
                continue
            if any(b < a for a, b in zip(values, values[1:])):
                continue
            hits.append((f"0x{WRAM_START + off:04X} {name}", values))
    report(label, hits, lock)


def scan_wrapped(snaps, size, label, expected, step, lock):
    hits = []
    for off in range(size):
        values = [s[off] for s in snaps]
        total = 0
        ok = True
        for a, b in zip(values, values[1:]):
            d = (b - a) % 256
            if d > 4 * max(step, 1):  # a scroll never leaps; anything else is noise
                ok = False
                break
            total += d
        if ok and 0.7 * expected <= total <= 1.3 * expected:
            hits.append((f"0x{WRAM_START + off:04X}", values))
    report(label, hits, lock)


def report(label, hits, lock):
    if not hits:
        print(f"{label}: no match\n")
        return
    print(f"{label}: {len(hits)} candidate(s)")
    for name, values in hits[:8]:
        print(f"  {name}: {values[0]} -> {values[-1]}, at lock {values[lock]}")
        print(f"    samples: {values[::40]}")
    print()


if __name__ == "__main__":
    sys.exit(main())
