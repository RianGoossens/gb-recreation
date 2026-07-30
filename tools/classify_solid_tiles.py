# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy", "numpy"]
# ///
"""Classify which World 1-1 tile ids are solid, by watching Mario hit them.

The level format carries tile ids and nothing else, so solidity has to come
from observation. Two kinds of evidence, both read off a real run:

* **Overlap.** Mario's own bounding box is inside a tile. He cannot stand
  inside a solid, so that tile is non-solid.
* **Support.** Mario is grounded and the tile directly under his feet is
  what is holding him up, so that tile is solid.

Both are tested against a box inset by `INSET` pixels on each side, and that
inset is doing real work rather than being a safety margin. Mario is 16
pixels wide, so a box flush against a wall still laps 1 or 2 pixels into the
wall's own tile column, and the raw box reports every wall Mario is blocked
by as a tile he walked through. That misread tile 99 (a pipe) and tile 232
(the fill under a raised platform) as non-solid on the first run. Insetting
by 4 keeps only cells Mario is genuinely inside.

A cell also has to stay occupied for `MIN_STREAK` frames before it counts as
walked through. Landing on a raised platform clips Mario a few frames into
the fill below its surface, which read as him walking through tile 232 on the
second run even with the inset. A solid can be entered for a frame or two
during collision resolution; nothing rests inside one.

Verdicts use the ratio of support to overlap rather than thresholds on each,
because the two overlap legitimately: standing at the edge of a ledge puts
sky under half of Mario, and landing puts him a pixel into the ground.

Coordinates, calibrated against the emulator at spawn rather than assumed.
Mario is four sprites in OAM slots 3-6 (the hardware sprite table), giving a
16x16 box; OAM stores y biased by 16 and x by 8. Playfield row `r` (row 0
being the first row below the two status bar rows) is drawn at screen y
`(r + 2) * 8`. At spawn Mario's box bottom is screen y 128, which is
playfield row 14, and row 14 there is tile 96, the ground. That is the
calibration.

Tiles are looked up in the level decoded straight from the ROM
(`decode_level.py`), so this doubles as a check on that decode: a tile id
that never appears where the run says Mario was would mean the two disagree.

Coverage is bounded by how far the walker gets, so tiles the run never
touches come back unclassified and stay that way rather than being guessed.

Usage: uv run tools/classify_solid_tiles.py [out.json]
"""

import json
import sys
from collections import Counter

sys.path.insert(0, "tools")

from decode_level import decode_level
from sml_boot import boot_to_gameplay
from sml_scroll import ScrollTracker
from sml_walker import ReactiveWalker

OAM_BASE = 0xFE00
MARIO_SLOTS = (3, 4, 5, 6)
OAM_Y_BIAS = 16
OAM_X_BIAS = 8
GROUNDED = 0xC20A
PHASE = 0xC207
HUD_ROWS = 2
ROWS = 16
FRAMES = 3000
MIN_SIGHTINGS = 8
INSET = 4  # pixels trimmed from each side of Mario's box; see the module docstring
MIN_STREAK = 8  # frames a cell must stay occupied before it counts as walked through
SOLID_RATIO = 0.5
NON_SOLID_RATIO = 0.1


def mario_box(pb):
    """Mario's screen-space bounding box, or None if his sprites are gone."""
    ys, xs = [], []
    for slot in MARIO_SLOTS:
        base = OAM_BASE + slot * 4
        y, x = pb.memory[base], pb.memory[base + 1]
        if y == 0 or y >= 160:
            return None
        ys.append(y - OAM_Y_BIAS)
        xs.append(x - OAM_X_BIAS)
    return min(xs), min(ys), max(xs) + 8, max(ys) + 8


def tile_at(level, column, row):
    if not (0 <= row < ROWS) or not (0 <= column < len(level)):
        return None
    return level[column][row]


def observe(pb, level, tracker, walker, frames=FRAMES):
    overlap, support = Counter(), Counter()
    streaks = {}
    for _ in range(frames):
        walker.step(pb, tracker)
        pb.tick()
        tracker.update(pb)
        if tracker.frozen > 5:
            break

        box = mario_box(pb)
        if box is None:
            continue
        left, top, right, bottom = box
        first_col = (tracker.scroll + left + INSET) // 8
        last_col = (tracker.scroll + right - 1 - INSET) // 8
        first_row = (top + INSET) // 8 - HUD_ROWS
        last_row = (bottom - 1 - INSET) // 8 - HUD_ROWS

        inside = {
            (c, r)
            for c in range(first_col, last_col + 1)
            for r in range(first_row, last_row + 1)
            if tile_at(level, c, r) is not None
        }
        streaks = {cell: streaks.get(cell, 0) + 1 for cell in inside}
        for cell, run in streaks.items():
            if run >= MIN_STREAK:
                overlap[level[cell[0]][cell[1]]] += 1

        if pb.memory[GROUNDED] and pb.memory[PHASE] == 0:
            for c in range(first_col, last_col + 1):
                tile = tile_at(level, c, bottom // 8 - HUD_ROWS)
                if tile is not None:
                    support[tile] += 1
    return overlap, support


def classify(overlap, support):
    """Solid, non-solid or contested, for every tile the run actually met."""
    out = {}
    for tile in sorted(set(overlap) | set(support)):
        held, through = support[tile], overlap[tile]
        total = held + through
        ratio = held / total if total else 0.0
        if total < MIN_SIGHTINGS:
            verdict = "too-few-sightings"
        elif ratio >= SOLID_RATIO:
            verdict = "solid"
        elif ratio <= NON_SOLID_RATIO:
            verdict = "non-solid"
        else:
            verdict = "contested"
        out[tile] = {
            "verdict": verdict,
            "stood_on": held,
            "passed_through": through,
            "support_ratio": round(ratio, 3),
        }
    return out


def main():
    out_path = sys.argv[1] if len(sys.argv) > 1 else "solid_tiles.json"
    rom = open("super_mario_land.gb", "rb").read()
    level = decode_level(rom)

    pb = boot_to_gameplay()
    tracker = ScrollTracker(pb)
    walker = ReactiveWalker(pb)
    overlap, support = observe(pb, level, tracker, walker)
    pb.stop()

    result = classify(overlap, support)
    seen = set(result)
    present = {t for column in level for t in column}
    print(f"reached world column {tracker.scroll // 8}")
    print(f"classified {len(seen)} of {len(present)} tile ids in the level\n")
    print("tile        verdict            stood on   passed through")
    for tile, info in sorted(result.items(), key=lambda kv: -kv[1]["stood_on"]):
        print(
            f"{tile:3d} 0x{tile:02X}   {info['verdict']:18s} "
            f"{info['stood_on']:6d}   {info['passed_through']:6d}"
        )
    unseen = sorted(present - seen)
    print(f"\nunclassified ({len(unseen)}): " + " ".join(str(t) for t in unseen))

    with open(out_path, "w") as f:
        json.dump({str(k): v for k, v in result.items()}, f, indent=2)
    print(f"\nwrote {out_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
