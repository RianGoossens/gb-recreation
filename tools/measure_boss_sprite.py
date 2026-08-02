# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Which atlas tiles is one object kind drawn from, with Mario moved aside?

`measure_object_sprites.py` sweeps every kind in one pass and skips any object
with a neighbour or with Mario within 28 pixels, because both are drawn from
the same atlas and would be collected as the object's own tiles. That skip is
why it has never measured the boss: the walkthrough flies Mario straight over
it, so it is never far enough away while the boss is on screen.

This reaches one kind, stops, moves Mario to the far side of the screen, and
then reads OAM (the hardware's table of 40 sprite entries: y, x, tile, flags)
around the slot for a whole leap cycle. The window is wider than the survey's,
since a boss is bigger than a walker.

The control is Gao, whose tiles are already measured in this same level by the
survey. If this tool cannot reproduce those, its answer for the boss is worth
nothing.

Usage: uv run tools/measure_boss_sprite.py [kind] [level]
"""

import sys

sys.path.insert(0, "tools")

from dump_object_slots import EMPTY, SLOTS, slot
from probe_object_contact import approach
from run_through_levels import SCREEN_X

OAM = 0xFE00
SPRITES = 40
# A boss is bigger than the 2x2 block the survey's 20 was sized for.
NEAR = 32
# Where to park Mario so his own sprite is not collected. The slot is only
# read while it sits between x 50 and 140, so either screen edge clears it.
MARIO_PARK = 8
# One full leap cycle, so a pose that only appears in the air is seen.
WATCH = 200


def entries_near(pb, x, y):
    """Every OAM entry within NEAR of (x, y), as offsets from it."""
    out = []
    for i in range(SPRITES):
        sy, sx, tile, attr = (pb.memory[OAM + i * 4 + f] for f in range(4))
        if sy == 0 or sx == 0:
            continue
        if abs(sx - x) <= NEAR and abs(sy - y) <= NEAR:
            out.append((sx - x, sy - y, tile, attr))
    return sorted(out, key=lambda e: (e[1], e[0]))


def measure(kind, level):
    """Park Mario away from the object and collect the sprites drawn on it."""
    pb, s = approach(kind, level)
    if pb is None:
        print(f"  kind 0x{kind:02X} never came on screen in World {level}")
        return None

    poses = {}
    seen = {}
    for _ in range(WATCH):
        pb.memory[SCREEN_X] = MARIO_PARK
        pb.tick()
        state = slot(pb, s)
        if state[0] != kind:
            break
        found = entries_near(pb, state[3], state[2])
        # Keyed on the tiles alone, so the same drawing at a different height
        # counts once and a genuinely different pose does not.
        tiles = tuple(e[2] for e in found)
        poses.setdefault(tiles, found)
        seen[tiles] = seen.get(tiles, 0) + 1
    pb.stop()
    return poses, seen


def report(label, kind, poses, seen):
    print(f"{label}: kind 0x{kind:02X}")
    if not poses:
        return
    for tiles, found in sorted(poses.items(), key=lambda p: -seen[p[0]]):
        ids = " ".join(f"{t:02X}" for t in tiles)
        print(f"  {seen[tiles]:3d} frames, {len(tiles)} sprites: {ids}")
        for dx, dy, tile, attr in found:
            print(f"    dx {dx:+4d}  dy {dy:+4d}  tile 0x{tile:02X}  attr 0x{attr:02X}")


def main():
    kind = int(sys.argv[1], 0) if len(sys.argv) > 1 else 0x08
    level = sys.argv[2] if len(sys.argv) > 2 else "1-3"

    control, control_seen = measure(0x3F, level) or (None, None)
    report("control, Gao, already measured by the survey", 0x3F,
           control or {}, control_seen or {})
    if not control:
        print("\nthe control drew nothing, so the result below means nothing")
        return 1
    print()

    poses, seen = measure(kind, level) or (None, None)
    report("the kind asked for", kind, poses or {}, seen or {})
    return 0


if __name__ == "__main__":
    sys.exit(main())
