# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy", "numpy"]
# ///
"""Capture World 1-1's real background columns from the running game.

Ground truth for anything decoded out of the ROM. Written to a JSON file so
the decoding work can be re-scored without booting the emulator each time.

Each column is captured the first time it appears and never overwritten:
coins live in the background tilemap, so a column re-read after Mario has
walked through it is missing the ones he collected.

Usage: uv run tools/capture_columns.py [out.json]
"""

import json
import sys

sys.path.insert(0, "tools")

from sml_boot import boot_to_gameplay
from sml_scroll import ScrollTracker
from sml_walker import ReactiveWalker

MAP_BASE = 0x9800
MAP_WIDTH = 32
FIRST_ROW = 2
ROWS = 16
VISIBLE_COLUMNS = 20
FRAMES = 900


def capture(pb, tracker, walker, frames=FRAMES):
    truth = {}

    def sample():
        camera = tracker.scroll // 8
        for i in range(VISIBLE_COLUMNS):
            n = camera + i
            if n not in truth:
                truth[n] = [
                    pb.memory[MAP_BASE + r * MAP_WIDTH + (n % MAP_WIDTH)]
                    for r in range(FIRST_ROW, FIRST_ROW + ROWS)
                ]

    sample()
    for _ in range(frames):
        walker.step(pb, tracker)
        pb.tick()
        tracker.update(pb)
        if tracker.frozen > 5:
            break
        sample()
    return truth


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else "columns.json"
    pb = boot_to_gameplay()
    tracker = ScrollTracker(pb)
    walker = ReactiveWalker(pb)
    truth = capture(pb, tracker, walker)
    pb.stop()

    with open(out, "w") as f:
        json.dump({str(k): v for k, v in sorted(truth.items())}, f)
    columns = sorted(truth)
    print(f"wrote {len(truth)} columns ({columns[0]}..{columns[-1]}) to {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
