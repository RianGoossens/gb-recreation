# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""What turns kind 0x04 around?

Kind 0x00 walks one pixel every three frames and never turned in 143 steps.
Kind 0x04 shares that cadence and turned exactly once in 888 frames, which is
not enough to tell a wall from a ledge from a timer.

Putting the obstacle there on purpose is. Super Mario Land tests collision
against the background tilemap in video RAM, so with the camera frozen a wall
or a pit can be written into the object's path and it either reacts or does
not. Both cases run here, and kind 0x00 is worth running through the same
thing as a control, since its ledge behaviour is already known.

The obstacle goes in front of whichever way the object is currently walking,
which is read off two samples of its slot X before anything is written.

Usage: uv run tools/probe_walker_turn.py [kind] [wall|pit]
"""

import sys

sys.path.insert(0, "tools")

from dump_object_slots import SLOTS, slot
from run_through_levels import Capture, FLY_Y, MAP_BASE, MAP_WIDTH, MARIO_Y
from sml_boot import boot_to_gameplay

OAM_OFFSET = 8
CAMERA_LAG = 27
GROUND_ROWS = (16, 17)
AIR_ROWS = range(10, 16)
SOLID_FROM = 0x60
WALL_TILE = 0x60
OPEN = 0x00

GAP = 3
APPROACH = 2600
WATCH = 500


def main():
    want = int(sys.argv[1], 0) if len(sys.argv) > 1 else 0x04
    mode = sys.argv[2] if len(sys.argv) > 2 else "wall"

    pb = boot_to_gameplay()
    capture = Capture(pb, 0)
    capture.pending.clear()

    found = None
    pb.button_press("right")
    for frame in range(APPROACH):
        pb.memory[MARIO_Y] = FLY_Y
        pb.tick()
        capture.step(pb, frame)
        # Capture re-arms its flattening for every fresh column, so clearing it
        # once at the start is not enough. The terrain has to stay: an object
        # that spawns over a flattened column falls instead of walking, and
        # the obstacle this probe writes lands in ground that has gone.
        capture.pending.clear()
        for s in range(SLOTS):
            state = slot(pb, s)
            if state[0] == want and 60 <= state[3] <= 150 and 120 <= state[2] <= 150:
                found = s
                break
        if found is not None:
            break
    pb.button_release("right")
    if found is None:
        print(f"no kind 0x{want:02X} settled on the ground on screen")
        pb.stop()
        return 1

    # Two samples, far enough apart to see a pixel of movement at 1 per 3 frames.
    before = slot(pb, found)[3]
    for _ in range(12):
        pb.memory[MARIO_Y] = FLY_Y
        pb.tick()
    state = slot(pb, found)
    heading = 1 if state[3] > before else -1
    print(f"kind 0x{want:02X} in slot {found} at screen x {state[3] - OAM_OFFSET}, "
          f"walking {'right' if heading > 0 else 'left'}")

    camera = len(capture.columns) - CAMERA_LAG
    column = camera + (state[3] - OAM_OFFSET) // 8
    ring = column % MAP_WIDTH
    under = [pb.memory[MAP_BASE + r * MAP_WIDTH + ring] for r in GROUND_ROWS]
    if not all(t >= SOLID_FROM for t in under):
        print(f"the computed column {column} is not ground ({under})")
        pb.stop()
        return 1

    for step in range(1, GAP + 1):
        at = (ring + heading * step) % MAP_WIDTH
        if mode == "pit":
            for row in GROUND_ROWS:
                pb.memory[MAP_BASE + row * MAP_WIDTH + at] = OPEN
        else:
            for row in AIR_ROWS:
                pb.memory[MAP_BASE + row * MAP_WIDTH + at] = WALL_TILE
    print(f"put a {mode} {GAP} columns ahead of it\n")

    last = (state[3], state[2])
    turned = False
    fell = False
    for frame in range(WATCH):
        pb.memory[MARIO_Y] = FLY_Y
        pb.tick()
        state = slot(pb, found)
        if state[0] != want:
            print(f"frame {frame}: slot emptied")
            break
        now = (state[3], state[2])
        if now != last:
            if (now[0] - last[0]) * heading < 0:
                turned = True
            if now[1] > last[1]:
                fell = True
            last = now

    pb.stop()
    print(f"turned back: {turned}")
    print(f"fell: {fell}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
