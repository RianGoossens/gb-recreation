# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Walk the real game through level after level, capturing every column.

Ground truth for the ROM decode past World 1-1. A scripted walker cannot
finish a level: the best reactive one this project built died at world
column 78 of 300. Two pokes remove both obstacles.

Super Mario Land tests collision against the background tilemap in video
RAM (see `probe_solidity.py`), so the terrain can be flattened out from
under the problem: every column Mario has not reached yet becomes open sky
over solid ground. That handles pits, pipes and walls, but not the enemies,
which still take all three lives inside a thousand frames. Pinning Mario's
Y position holds him in the air above them, and pinning the vertical phase
byte stops the game pulling him back down.

Nothing needs to track the camera. The game writes a column into the
tilemap once, when it scrolls in at the right edge, roughly every eight
frames, and the ring column it writes to advances by one each time. Any
ring column that changes is fresh level data, in world order. It is
captured and flattened `FLATTEN_DELAY` frames later, long before Mario
arrives.

Two things this got wrong before logging the ring index caught them. The
top two playfield rows are a skyline the game keeps redrawing, so
flattening them made every column look freshly written on the next frame.
And the columns already on screen when a level starts are never "written",
so they have to be read out of the initial map instead.

Usage: uv run tools/run_through_levels.py [out.txt] [frames]
"""

import sys

sys.path.insert(0, "tools")

from sml_boot import boot_to_gameplay

MAP_BASE = 0x9800
MAP_WIDTH = 32
FIRST_ROW = 2
ROWS = 16

GROUND_ROW = 14
SKY_ROWS = 2

# Flattening has to be unmistakable. Filling with the level's own background
# tile does not work: plenty of real columns are nothing but background over
# ground, and those then read as already flat and get skipped. Tile 0 never
# appears in level data, and being below 0x60 it is not solid, which does not
# matter while Mario is pinned in the air anyway.
FLAT_FILL = 0x00

FLATTEN_DELAY = 24
LEVEL_GAP_FRAMES = 90

MARIO_Y = 0xC201
PHASE = 0xC207
FLY_Y = 60

FRAMES = 20000


def read_column(pb, ring):
    return [pb.memory[MAP_BASE + (FIRST_ROW + r) * MAP_WIDTH + ring] for r in range(ROWS)]


def flatten(pb, ring):
    for r in range(SKY_ROWS, ROWS):
        pb.memory[MAP_BASE + (FIRST_ROW + r) * MAP_WIDTH + ring] = FLAT_FILL


def is_flat(column):
    return all(t == FLAT_FILL for t in column[SKY_ROWS:])


class Capture:
    """Columns of one level, in world order."""

    def __init__(self, pb):
        self.start_map = {ring: read_column(pb, ring) for ring in range(MAP_WIDTH)}
        self.known = dict(self.start_map)
        self.pending = {}
        self.columns = []
        self.last_ring = None

    def seed(self, first_ring, frame):
        """The columns already drawn when the level started, in world order.

        Every ring is queued for flattening here, not just the ones that get
        captured. A seeded ring keeps its opening-screen content otherwise,
        and when the game later writes a column into it that happens to be
        identical, nothing changes and the column is missed. That cost
        exactly two columns of World 1-1, at world columns 58 and 59.
        """
        self.columns = [self.start_map[r] for r in range(first_ring)]
        for ring in range(MAP_WIDTH):
            self.pending.setdefault(ring, frame + FLATTEN_DELAY)

    def step(self, pb, frame):
        found = False
        for ring in range(MAP_WIDTH):
            column = read_column(pb, ring)
            if column == self.known[ring] or is_flat(column):
                continue
            self.known[ring] = column
            if not self.columns:
                self.seed(ring, frame)
            # The game fills a column over several frames. Waiting for it to
            # settle drops columns instead: rewrite the last one in place.
            if ring == self.last_ring:
                self.columns[-1] = column
            else:
                self.columns.append(column)
                self.last_ring = ring
            self.pending[ring] = frame + FLATTEN_DELAY
            found = True
        for ring, due in list(self.pending.items()):
            if frame >= due:
                flatten(pb, ring)
                self.known[ring] = read_column(pb, ring)
                del self.pending[ring]
        return found


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else "assets/extracted/captured_columns.txt"
    frames = int(sys.argv[2]) if len(sys.argv) > 2 else FRAMES

    pb = boot_to_gameplay()
    levels = []
    capture = Capture(pb)
    last_seen = 0

    pb.button_press("right")
    for frame in range(frames):
        pb.memory[MARIO_Y] = FLY_Y
        pb.memory[PHASE] = 0
        pb.tick()

        # No column is written while one level ends and the next loads, so a
        # long quiet stretch is the boundary between two levels.
        if capture.columns and frame - last_seen > LEVEL_GAP_FRAMES:
            print(f"  frame {frame}: level {len(levels)} ended, {len(capture.columns)} columns")
            levels.append(capture.columns)
            capture = Capture(pb)
            last_seen = frame

        if capture.step(pb, frame):
            last_seen = frame

    pb.button_release("right")
    if capture.columns:
        levels.append(capture.columns)
    pb.stop()

    for i, level in enumerate(levels):
        print(f"level {i}: {len(level)} columns")
    # One column per line, 16 tile ids, a blank line between levels. Plain
    # enough that the Rust side reads it without a JSON dependency.
    with open(out, "w") as f:
        for i, level in enumerate(levels):
            if i:
                f.write("\n")
            for column in level:
                f.write(" ".join(str(t) for t in column) + "\n")
    print(f"wrote {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
