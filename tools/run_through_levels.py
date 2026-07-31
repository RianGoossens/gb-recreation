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
Y position every frame holds him in the air above them. The height matters:
sweeping it from a snapshot at World 1-2's opening, 60 loses four lives in
2500 frames and 32 loses none. Only Y: pinning the
rise/fall phase byte as well freezes him at the spawn of any level he did
not start in, at screen x 50, and the flattened ground is not solid so he
cannot simply be left to walk.

Nothing needs to track the camera. The game writes a column into the
tilemap once, when it scrolls in at the right edge, roughly every eight
frames, and the ring column it writes to advances by one each time. Any
ring column that changes is fresh level data, in world order. It is
captured and flattened `FLATTEN_DELAY` frames later, long before Mario
arrives.

Crossing into the next level needs the poking to stop the moment a level
ends, and a way to tell when the next one has started. Both are the same
lesson: a level's tail stays on display for a few hundred frames and matches
its own screen list, while the bonus game between levels matches nothing the
ROM points at for thousands. The length of that gap is the marker.

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
from sml_level import VISIBLE_COLUMNS, known_screens

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
LEVEL_GAP_FRAMES = 400
# A level can go quiet for a couple of hundred frames without being over, so
# the end-of-level threshold is generous. The bonus game between levels
# matches no screen the ROM points at, for thousands of frames. A level's own tail matches its own list within a few
# hundred, which is why a plain "matched nothing" test does not separate them.
BONUS_FRAMES = 800
SAMPLE_EVERY = 10
RELEASE_FRAMES = 30
SCREEN_X = 0xC202
PHASE = 0xC207
SPAWN_X = 55

MARIO_Y = 0xC201
LIVES = 0xDA15
FLY_Y = 32

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

    def __init__(self, pb, frame):
        self.start_map = {ring: read_column(pb, ring) for ring in range(MAP_WIDTH)}
        self.known = dict(self.start_map)
        self.columns = []
        self.last_ring = None
        # Flatten from the outset rather than after the first column is
        # captured. Waiting deadlocks at a level that starts Mario in front of
        # something solid: he cannot move until the terrain goes, and the
        # terrain does not go until he moves.
        self.pending = {ring: frame + FLATTEN_DELAY for ring in range(MAP_WIDTH)}

    def seed(self, first_ring):
        """The columns already drawn when the level started, in world order.

        Read from the snapshot taken at construction, not from the live map,
        which has been flattened by now. Every ring is flattened, not just the
        ones that get captured: a seeded ring keeps its opening-screen content
        otherwise, and when the game later writes an identical column into it
        nothing changes and the column is missed. That cost exactly two
        columns of World 1-1, at world columns 58 and 59.
        """
        self.columns = [self.start_map[r] for r in range(first_ring)]

    def step(self, pb, frame):
        found = False
        for ring in range(MAP_WIDTH):
            column = read_column(pb, ring)
            if column == self.known[ring] or is_flat(column):
                continue
            self.known[ring] = column
            if not self.columns:
                self.seed(ring)
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

    screens = known_screens()
    pb = boot_to_gameplay()
    levels = []
    capture = Capture(pb, 0)
    playing = True
    quiet = 0
    blank = 0

    pb.button_press("right")
    for frame in range(frames):
        if playing:
            pb.memory[MARIO_Y] = FLY_Y
            # Leave the rise/fall phase alone until Mario is under way.
            # Pinning it at a level's spawn freezes him there.
            if pb.memory[SCREEN_X] > SPAWN_X:
                pb.memory[PHASE] = 0
        pb.tick()

        # Running out of lives sends the game to the title screen and its
        # attract demo, which draws real level columns but not in level order.
        if pb.memory[LIVES] == 0:
            print(f"  frame {frame}: out of lives, stopping")
            break

        if playing:
            if capture.step(pb, frame):
                quiet = 0
            else:
                quiet += 1
            # The end-of-level sequence needs the game left alone. Poking
            # through it freezes Mario at the exit gate forever.
            if capture.columns and quiet > LEVEL_GAP_FRAMES:
                print(f"  frame {frame}: level {len(levels)} ended, "
                      f"{len(capture.columns)} columns")
                levels.append(capture.columns)
                playing = False
                blank = 0
            continue

        # A bonus game sits between levels and waits for input. Only A: Start
        # pauses Super Mario Land, and a tap landing just as the next level
        # opens leaves it paused, with Mario frozen at the spawn and no
        # columns ever written.
        if frame % 40 == 0:
            pb.button_press("a")
        elif frame % 40 == 12:
            pb.button_release("a")

        if frame % SAMPLE_EVERY:
            continue
        visible = [read_column(pb, ring) for ring in range(VISIBLE_COLUMNS)]
        hit = screens.get(repr(visible))
        if not hit:
            blank += SAMPLE_EVERY
        elif blank >= BONUS_FRAMES:
            opening = sorted({p for _, p in hit})
            print(f"  frame {frame}: level {len(levels)} opens on "
                  + ", ".join(f"0x{p:04X}" for p in opening))
            # Right has to be let go of and pressed again for the new level,
            # and one frame of release is not enough for the game to see the
            # edge. Holding it across the transition looks the same from here
            # and does nothing: Mario stands at the spawn and never moves.
            pb.button_release("right")
            for _ in range(RELEASE_FRAMES):
                pb.tick()
            pb.button_press("right")
            capture = Capture(pb, frame)
            playing = True
            quiet = 0
        else:
            blank = 0

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
