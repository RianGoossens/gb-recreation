# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Measure how an object kind moves, with the camera held still.

An object's slot X is a screen coordinate, so while the camera is scrolling it
mixes the object's own speed with Mario's. Super Mario Land makes that easy to
separate: the camera only moves when Mario does, and it stops the frame he
stops. Walk right until the object is on screen, let go, and every pixel the
slot X loses after that is the object's own.

World 1-1's first record is kind 0x00, which the list uses nine times, always
on the ground. This measures it: which way it walks, how fast, and whether it
keeps going or turns.

Usage: uv run tools/measure_enemy_walk.py [kind]
"""

import sys

sys.path.insert(0, "tools")

from dump_object_slots import EMPTY, SLOTS, slot
from run_through_levels import FLY_Y, MARIO_Y
from sml_boot import boot_to_gameplay

SCREEN_X = 0xC202
WATCH = 1200
APPROACH = 2600


def main():
    want = int(sys.argv[1], 0) if len(sys.argv) > 1 else 0x00
    pb = boot_to_gameplay()

    found = None
    pb.button_press("right")
    for _ in range(APPROACH):
        # Mario has to survive the walk to the kinds that appear late in the
        # level, so he flies over everything on the way there.
        pb.memory[MARIO_Y] = FLY_Y
        pb.tick()
        for s in range(SLOTS):
            state = slot(pb, s)
            if state[0] == want and 40 <= state[3] <= 150:
                found = s
                break
        if found is not None:
            break
    pb.button_release("right")
    if found is None:
        print(f"no object of kind 0x{want:02X} came on screen")
        pb.stop()
        return 1

    # Give the camera a few frames to come to rest before measuring.
    for _ in range(8):
        pb.tick()

    state = slot(pb, found)
    print(f"kind 0x{want:02X} in slot {found}, camera stopped at mario x "
          f"{pb.memory[SCREEN_X]}")
    last = (state[3], state[2])
    steps = []
    for frame in range(WATCH):
        # Mario has to stay out of the way for a long measurement, and out of
        # reach: the walk ends early otherwise, with him dead.
        pb.memory[MARIO_Y] = FLY_Y
        pb.tick()
        state = slot(pb, found)
        if state[0] != want:
            print(f"frame {frame}: slot emptied")
            break
        now = (state[3], state[2])
        if now != last:
            steps.append((frame, (now[0] - last[0], now[1] - last[1])))
            last = now
    pb.stop()

    if not steps:
        print("it did not move at all")
        return 0
    for axis, index in (("x", 0), ("y", 1)):
        moves = [(f, d[index]) for f, d in steps if d[index]]
        if not moves:
            print(f"{axis}: never moved")
            continue
        gaps = [b[0] - a[0] for a, b in zip(moves, moves[1:])]
        spread = sorted(set(gaps))
        travel = sum(d for _, d in moves)
        reach = sum(abs(d) for _, d in moves)
        span = moves[-1][0] - moves[0][0]
        print(f"{axis}: {len(moves)} moves of {sorted({d for _, d in moves})} px, "
              f"{reach} px travelled, {travel:+d} px net over {span} frames")
        print(f"   frames between moves: "
              f"{', '.join(f'{g}x{gaps.count(g)}' for g in spread[:6])}")
        turns = [
            b[0]
            for a, b in zip(moves, moves[1:])
            if (a[1] > 0) != (b[1] > 0)
        ]
        if turns:
            apart = [b - a for a, b in zip(turns, turns[1:])]
            print(f"   {len(turns)} reversals, {sorted(set(apart))[:6]} frames apart")
    return 0


if __name__ == "__main__":
    sys.exit(main())
