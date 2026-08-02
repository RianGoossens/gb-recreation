# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Find a state inside the flown walkthrough where Mario can still die.

`tools/measure_boss_box.py` could not measure anything in World 1-3 because
nothing costs Mario a life there: not the boss, not an ordinary enemy, not a
pit. It named the save state as the last suspect. The save state is innocent.
Two other things were wrong, and each on its own is enough to make every
contact result meaningless.

1. The fly loop leaves Mario unstepped. It pins `0xC201` (his y) and `0xC207`
   (his vertical phase, 0 for grounded) every frame to carry him across two
   levels, and when it lets go he stays where it left him forever: 240 frames
   standing still and 240 more holding right, all at y 22, phase 0. Diffing
   his bytes against an ordinary World 1-1 Mario shows three that differ,
   `0xC200`, `0xC203` and `0xC210`, and writing all three back restarts him:
   he falls and lands on the level's own floor.
2. The pit dug the wrong columns. The background map is a 32 column ring, so
   a screen position is not a map column once the level has scrolled, and
   `SCX` reads 0 from work RAM because the status bar is drawn at 0 and the
   playfield's value is set mid-frame (`tools/sml_scroll.py`). Clearing all
   32 columns needs no scroll and cannot miss.

With both fixed, the pit kills him on frame 192 in World 1-3, from a state a
contact sweep can be built on. `thaw` and `dig` are here for that sweep to
import.

Usage: uv run tools/probe_death_state.py [control|flown]
"""

import sys

sys.path.insert(0, "tools")

from run_through_levels import FLY_Y, LIVES, MARIO_Y, PHASE, SCREEN_X, SPAWN_X
from sml_boot import boot_to_gameplay
from trace_level_objects import NAMES, reach_level

WATCH = 300
TILEMAP = 0x9800
BLOCK = range(0xC200, 0xC220)
# The three bytes of Mario's block the fly loop leaves wrong, with the values
# an ordinary World 1-1 Mario has.
THAW = {0xC200: 0x80, 0xC203: 0x00, 0xC210: 0x80}
# Landing takes about 200 frames to begin after the write, so give it room.
THAW_FRAMES = 300


def block(pb):
    """Mario's own bytes, for diffing one state against another."""
    return " ".join(f"{pb.memory[a]:02X}" for a in BLOCK)


def thaw(pb, frames=THAW_FRAMES):
    """Hand Mario back to the game after the fly loop has pinned him."""
    for address, value in THAW.items():
        pb.memory[address] = value
    for _ in range(frames):
        pb.tick()


def dig(pb):
    """Take every solid tile out of the playfield and write nothing else.

    What follows is the game's own fall and the game's own death. Pinning his
    y byte below the screen instead is not a fall and answers "unharmed" from
    any state at all, which is why the first version of this control was
    worth nothing.
    """
    for row in range(2, 18):
        for column in range(32):
            pb.memory[TILEMAP + row * 32 + column] = 0x00


def watch_for_death(pb, frames=WATCH):
    """Tick and report the frame a life goes, or None."""
    lives = pb.memory[LIVES]
    for frame in range(frames):
        pb.tick()
        if pb.memory[LIVES] < lives:
            return frame
    return None


def control():
    """Does `dig` kill an ordinary Mario, in a state nothing has touched?

    The pit control is itself an instrument, so it needs one. World 1-1 from
    a cold boot, with no fly loop, no pinning and no save state, is the
    plainest state the game has.
    """
    pb = boot_to_gameplay()
    pb.button_press("right")
    for _ in range(120):
        pb.tick()
    pb.button_release("right")
    for _ in range(30):
        pb.tick()
    print(f"World 1-1 from a cold boot: y {pb.memory[MARIO_Y]}, phase "
          f"{pb.memory[PHASE]}, screen x {pb.memory[SCREEN_X]}")
    print("his block: " + block(pb))
    dig(pb)
    frame = watch_for_death(pb)
    print(f"a life went on frame {frame}" if frame is not None
          else f"no life went, ending at y {pb.memory[MARIO_Y]}")
    pb.stop()
    return 0 if frame is not None else 1


def flown(level="1-3", frames=800):
    """The same pit, partway through a level reached by flying to it."""
    pb, _ = reach_level(NAMES.index(level))
    if pb is None:
        print(f"never reached World {level}")
        return 1
    for _ in range(frames):
        pb.memory[MARIO_Y] = FLY_Y
        if pb.memory[SCREEN_X] > SPAWN_X:
            pb.memory[PHASE] = 0
        pb.tick()
    pb.button_release("right")

    frozen = []
    for frame in range(240):
        pb.tick()
        if frame % 60 == 0:
            frozen.append((pb.memory[MARIO_Y], pb.memory[PHASE]))
    print(f"World {level} after the fly loop lets go: {frozen}")
    print("his block: " + block(pb))

    thaw(pb)
    print(f"after his block is put back: y {pb.memory[MARIO_Y]}, phase "
          f"{pb.memory[PHASE]}")

    dig(pb)
    frame = watch_for_death(pb)
    print(f"a life went on frame {frame}, so a contact sweep can run from here"
          if frame is not None
          else f"no life went, ending at y {pb.memory[MARIO_Y]}")
    pb.stop()
    return 0 if frame is not None else 1


def main():
    mode = sys.argv[1] if len(sys.argv) > 1 else "flown"
    return control() if mode == "control" else flown()


if __name__ == "__main__":
    sys.exit(main())
