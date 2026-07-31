# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Tie the column counter to the camera, so a spawn can be placed in the level.

Counting the columns the game writes into the tilemap is exact (it is what
captured all 300 columns of World 1-1), but a count is not a camera
position until the offset between them is known: the game draws a column
some way before it is visible.

The start of a level pins that offset without any scroll measurement. The
camera does not move until Mario reaches the middle of the screen, so his
screen X (0xC202) climbing to its locked value of 81 is a scroll of zero,
and every pixel he gains after that is a pixel of scroll. Watching the
count against that gives the phase directly: which pixel of scroll turns
the counter over.

Usage: uv run tools/measure_spawn_column.py [frames]
"""

import sys

sys.path.insert(0, "tools")

from run_through_levels import Capture, SCREEN_X
from sml_boot import boot_to_gameplay

FRAMES = 260


def main():
    frames = int(sys.argv[1]) if len(sys.argv) > 1 else FRAMES
    pb = boot_to_gameplay()
    capture = Capture(pb, 0)
    capture.pending.clear()  # leave the terrain alone, nothing has to move

    print(f"columns already drawn when the level opens: seeded on first write")
    locked = None
    count = 0
    pb.button_press("right")
    for frame in range(frames):
        pb.tick()
        capture.step(pb, frame)
        x = pb.memory[SCREEN_X]
        if locked is None and x >= 81:
            locked = frame
            print(f"frame {frame}: mario reached screen x {x}, camera unlocks")
        if len(capture.columns) != count:
            count = len(capture.columns)
            scroll = "-" if locked is None else frame - locked
            print(f"frame {frame:4d}  mario x {x:3d}  scroll {scroll:>4}  "
                  f"columns {count}")
    pb.button_release("right")
    pb.stop()
    return 0


if __name__ == "__main__":
    sys.exit(main())
