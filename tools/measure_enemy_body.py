# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Where is the edge an enemy walks into walls with?

Every enemy in the engine uses 8 by 8, which nothing ever measured. Contact
with Mario is a separate box and is measured (5 by 5,
`tools/measure_enemy_box.py`); this is the other one, the body the terrain
stops.

Method, borrowed from `probe_walker_turn.py`: the game tests an object against
the background tilemap in video RAM, so with the camera frozen a wall can be
written into its path. Walk it into the wall, let it settle, and read its slot
x against the wall's own screen x.

The instrument needs its own control, and the obvious one is worthless here:
"the column under it is ground" is true for almost any column, because the
ground is continuous. Taking the floor out from under its own computed column
is the control that works, and it is what caught a wrong reading. An earlier
version took the column from the scroll register, which reads 0 throughout
(the game scrolls by rewriting the tilemap ring rather than by moving the
background), and the object walked straight through the wall it was given. The
pit control failed on that version and passes on this one: it drops 56 px in
60 frames.

What came out, for kind 0x00 walking left:

    it stops with its slot x four pixels inside the wall's own column,
    at two wall placements sixteen pixels apart

Its drawing starts one pixel right of its slot x and is eight wide, so the
edge the terrain stops is three pixels inside the drawing rather than at its
left column.

What did not come out is the other edge. It needs an object walking right into
a wall, and kind 0x00 never turns, while kind 0x04 (which does) kept losing
its slot to another object partway through the run. So one number is measured
and the box is not: a single point tested at slot x plus four would produce
this reading, and so would a box whose left edge is there, and this run cannot
separate them. `ENEMY_SIZE` stays 8 until it can.

Usage: uv run tools/measure_enemy_body.py [kind] [gap] [""|selfpit]
"""

import sys

sys.path.insert(0, "tools")

from dump_object_slots import SLOTS, slot
from run_through_levels import Capture, FLY_Y, MAP_BASE, MAP_WIDTH, MARIO_Y
from sml_boot import boot_to_gameplay

SCX = 0xFF43
CAMERA_LAG = 27
OAM_OFFSET = 8
AIR_ROWS = range(10, 16)
WALL_TILE = 0x60
APPROACH = 2600
SETTLE = 900


def find(pb, capture, want):
    pb.button_press("right")
    for frame in range(APPROACH):
        pb.memory[MARIO_Y] = FLY_Y
        pb.tick()
        capture.step(pb, frame)
        capture.pending.clear()
        for s in range(SLOTS):
            state = slot(pb, s)
            if state[0] == want and 40 <= state[3] <= 130 and 120 <= state[2] <= 150:
                pb.button_release("right")
                return s
    pb.button_release("right")
    return None


def heading_of(pb, s):
    before = slot(pb, s)[3]
    for _ in range(12):
        pb.memory[MARIO_Y] = FLY_Y
        pb.tick()
    return slot(pb, s)[3] - before


def run(want, gap, wall=True, turn_back=False):
    pb = boot_to_gameplay()
    capture = Capture(pb, 0)
    capture.pending.clear()
    s = find(pb, capture, want)
    if s is None:
        print(f"no kind 0x{want:02X} settled on the ground on screen")
        pb.stop()
        return None

    drift = heading_of(pb, s)
    if drift == 0:
        print("it did not move in 12 frames, so there is no heading to use")
        pb.stop()
        return None
    heading = 1 if drift > 0 else -1
    x = slot(pb, s)[3]

    # The map column it stands in. The scroll register reads 0 here (the game
    # scrolls by rewriting the ring rather than by moving the background), so
    # the column comes from how many columns have gone past, the same way
    # `probe_walker_turn.py` gets it. The pit control below is what says this
    # is right: an earlier version computed the ring from the scroll register
    # and the object walked straight through the wall it was given.
    scroll = pb.memory[SCX]
    camera = len(capture.columns) - CAMERA_LAG
    ring = (camera + (x - OAM_OFFSET) // 8) % MAP_WIDTH
    wall_ring = (ring + heading * gap) % MAP_WIDTH
    # Screen x of that column. A ring index is a slot in a 32 column buffer,
    # not a position on screen, so this counts tiles from the object's own
    # column rather than from the ring.
    wall_left = ((x - OAM_OFFSET) // 8 + heading * gap) * 8

    # The instrument's own control: the column the object is standing in has
    # to be ground. If it is not, the ring is wrong and the wall goes
    # somewhere the object never walks.
    under = [pb.memory[MAP_BASE + row * MAP_WIDTH + ring] for row in (16, 17)]
    print(f"ring {ring} under it holds {[hex(t) for t in under]}")
    if not all(t >= 0x60 for t in under):
        print("that is not ground, so the ring is wrong; stopping")
        pb.stop()
        return None

    if wall == "selfpit":
        # The instrument's positive control. Ground is continuous, so "the
        # column under it is ground" is true for almost any ring and proves
        # nothing. Taking the floor out from under its own computed ring does:
        # if the ring is right it falls, and if it is wrong it walks on.
        for row in (16, 17):
            pb.memory[MAP_BASE + row * MAP_WIDTH + ring] = 0x00
        start_y = slot(pb, s)[2]
        for _ in range(60):
            pb.memory[MARIO_Y] = FLY_Y
            pb.tick()
        fell = slot(pb, s)[2] - start_y
        print(f"pit under its own ring: it dropped {fell} px in 60 frames")
        pb.stop()
        return fell
    if wall:
        for row in AIR_ROWS:
            pb.memory[MAP_BASE + row * MAP_WIDTH + wall_ring] = WALL_TILE

    print(f"kind 0x{want:02X} in slot {s}: slot x {x}, walking "
          f"{'right' if heading > 0 else 'left'}, scroll {scroll}")
    print(f"{'wall' if wall else 'no wall (control)'} at ring {wall_ring}, "
          f"screen x {wall_left} to {wall_left + 7}")

    reach = x
    settled = x
    for _ in range(SETTLE):
        pb.memory[MARIO_Y] = FLY_Y
        pb.tick()
        state = slot(pb, s)
        if state[0] != want:
            print(f"the slot became kind 0x{state[0]:02X} at slot x "
                  f"{state[3]}, y {state[2]}, stopping")
            break
        reach = max(reach, state[3]) if heading > 0 else min(reach, state[3])
        if state[3] == reach:
            settled = state[3]

    # The other edge. Kind 0x04 turns at a wall, so the wall that stopped it
    # walking one way is what sets it walking the other, and a second wall in
    # the new direction measures the second edge in the same run.
    if turn_back:
        x2 = slot(pb, s)[3]
        column2 = (x2 - OAM_OFFSET) // 8
        back_ring = (ring + (column2 - (x - OAM_OFFSET) // 8) - heading * gap) % MAP_WIDTH
        back_left = (column2 - heading * gap) * 8
        for row in AIR_ROWS:
            pb.memory[MAP_BASE + row * MAP_WIDTH + back_ring] = WALL_TILE
        print(f"second wall at ring {back_ring}, screen x {back_left} to "
              f"{back_left + 7}")
        back = x2
        for _ in range(SETTLE):
            pb.memory[MARIO_Y] = FLY_Y
            pb.tick()
            state = slot(pb, s)
            if state[0] != want:
                break
            back = min(back, state[3]) if heading > 0 else max(back, state[3])
        screen2 = back - OAM_OFFSET
        if heading > 0:
            print(f"turned back to slot x {back} (screen {screen2}), against a "
                  f"wall whose right edge is {back_left + 7}: "
                  f"{screen2 - (back_left + 7)} px past it")
        else:
            print(f"turned back to slot x {back} (screen {screen2}), against a "
                  f"wall whose left edge is {back_left}: "
                  f"{back_left - screen2} px short of it")

    pb.stop()
    screen = reach - OAM_OFFSET
    if heading > 0:
        margin = wall_left - screen
        print(f"furthest slot x {reach} (screen {screen}), wall's left edge "
              f"{wall_left}, so it stopped {margin} px short of it")
    else:
        margin = screen - (wall_left + 7)
        print(f"furthest slot x {reach} (screen {screen}), wall's right edge "
              f"{wall_left + 7}, so it stopped {margin} px past it")
    return margin


def main():
    want = int(sys.argv[1], 0) if len(sys.argv) > 1 else 0x04
    gap = int(sys.argv[2]) if len(sys.argv) > 2 else 3
    run(want, gap, sys.argv[3] if len(sys.argv) > 3 else True, want == 0x04)
    return 0


if __name__ == "__main__":
    sys.exit(main())
