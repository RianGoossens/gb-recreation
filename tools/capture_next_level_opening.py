# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Read the opening screen of the level after World 1-1, straight off the map.

Where World 1-2's screen list starts is the open question. Each candidate
predicts a different first screen, and the first screen is drawn into the
background tilemap before Mario takes a step, so there is no need to capture
a whole level to tell them apart: reach 1-2 and read the map.

Getting there reuses the walkthrough from `run_through_levels.py`: flatten
the terrain ahead of Mario and pin him in the air so nothing can stop him.
Two additions. All poking has to stop the moment the level ends, because
flattening through the end-of-level sequence freezes Mario at the exit gate
forever, at screen x 160, with the game waiting on tiles that are no longer
there. And a bonus game sits between the two levels, so the run keeps
tapping Start and A to get through it.

Rather than guess when 1-2 has loaded, every screen the game draws is
matched against every pointer in every screen list the ROM contains. The
tail of 1-1 is still up for a while after the level ends and matches its own
list, and the bonus game matches nothing, so the run prints the whole
timeline rather than stopping at the first hit.

Usage: uv run tools/capture_next_level_opening.py [out.txt]
"""

import sys

sys.path.insert(0, "tools")

from sml_boot import boot_to_gameplay
from sml_level import ROWS, VISIBLE_COLUMNS, known_screens

MAP_BASE = 0x9800
MAP_WIDTH = 32
FIRST_ROW = 2

SKY_ROWS = 2
FLAT_FILL = 0x00

MARIO_Y = 0xC201
PHASE = 0xC207
FLY_Y = 60

QUIET_FRAMES = 90
SAMPLE_EVERY = 10
BONUS_FRAMES = 800
SETTLE_FRAMES = 12000
FRAMES = 20000

def read_column(pb, ring):
    return [pb.memory[MAP_BASE + (FIRST_ROW + r) * MAP_WIDTH + ring] for r in range(ROWS)]


def flatten(pb, ring):
    for r in range(SKY_ROWS, ROWS):
        pb.memory[MAP_BASE + (FIRST_ROW + r) * MAP_WIDTH + ring] = FLAT_FILL


def is_flat(column):
    return all(t == FLAT_FILL for t in column[SKY_ROWS:])


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else "assets/extracted/next_level_opening.txt"
    screens = known_screens()
    print(f"{len(screens)} distinct screens across the ROM's screen lists")
    pb = boot_to_gameplay()

    matched = None
    last_label = None
    blank = 0
    known = {ring: read_column(pb, ring) for ring in range(MAP_WIDTH)}
    pending = {}
    quiet = 0
    ended = None

    pb.button_press("right")
    for frame in range(FRAMES):
        if ended is None:
            pb.memory[MARIO_Y] = FLY_Y
            pb.memory[PHASE] = 0
        pb.tick()

        if ended is not None:
            # A bonus game sits between levels and waits for input.
            if frame % 40 == 0:
                pb.button_press("start")
            elif frame % 40 == 8:
                pb.button_release("start")
            elif frame % 40 == 16:
                pb.button_press("a")
            elif frame % 40 == 24:
                pb.button_release("a")

            if frame % SAMPLE_EVERY == 0:
                visible = [read_column(pb, ring) for ring in range(VISIBLE_COLUMNS)]
                hit = screens.get(repr(visible))
                if not hit:
                    blank += SAMPLE_EVERY
                else:
                    # The tail of World 1-1 keeps matching its own list for a
                    # few hundred frames. The bonus game between levels
                    # matches nothing for thousands, which is the marker that
                    # the next level is what comes next.
                    if blank >= BONUS_FRAMES:
                        print(f"  frame {frame:6d}: the next level opens on "
                              + ", ".join(f"0x{p:04X} (list 0x{s:05X})"
                                          for s, p in sorted(set(hit))))
                        matched = visible
                        break
                    if last_label != repr(hit):
                        print(f"  frame {frame:6d}: still World 1-1, "
                              + ", ".join(f"0x{p:04X}" for _, p in sorted(set(hit))))
                        last_label = repr(hit)
                    blank = 0
            if frame - ended > SETTLE_FRAMES:
                break
            continue

        wrote = False
        for ring in range(MAP_WIDTH):
            column = read_column(pb, ring)
            if column == known[ring] or is_flat(column):
                continue
            known[ring] = column
            pending[ring] = frame + 24
            wrote = True
        quiet = 0 if wrote else quiet + 1
        if quiet > QUIET_FRAMES:
            ended = frame
            pending.clear()
            print(f"frame {frame}: World 1-1 ended, leaving the game alone")
            continue
        for ring, due in list(pending.items()):
            if frame >= due:
                flatten(pb, ring)
                known[ring] = read_column(pb, ring)
                del pending[ring]
    pb.button_release("right")

    columns = matched or [read_column(pb, ring) for ring in range(VISIBLE_COLUMNS)]
    pb.stop()

    with open(out, "w") as f:
        for column in columns:
            f.write(" ".join(str(t) for t in column) + "\n")
    print(f"wrote {out}")
    for r in range(ROWS):
        print(f"  row {r:2d} " + " ".join(f"{c[r]:3d}" for c in columns))
    return 0


if __name__ == "__main__":
    sys.exit(main())
