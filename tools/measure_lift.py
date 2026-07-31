# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""How wide is a lift, and where does Mario stand on it?

`probe_lift.py` showed that World 1-1's last two objects carry Mario. Building
them needs two more numbers that are easy to assume and worth measuring: how
far either side of the object he is still held up, and how high above it he
comes to rest in a coordinate our engine can use.

Both come from dropping him repeatedly at different offsets. A save state
taken once the lift is on screen makes that cheap: every offset restarts from
the same frame, so the lift is in the same phase of its cycle each time.

The resting height is reported against ordinary ground as well as against the
object, since Mario's own Y byte is in a coordinate of its own. Standing him
on World 1-1's opening ground, whose top edge is known from the geometry
decode, converts one to the other.

Usage: uv run tools/measure_lift.py [kind]
"""

import sys

sys.path.insert(0, "tools")

from dump_object_slots import SLOTS, slot
from run_through_levels import FLY_Y, MARIO_Y
from sml_boot import boot_to_gameplay

SCREEN_X = 0xC202
GROUND_ROW = 14  # World 1-1's ground; its top edge is screen y 8 * 14 = 112
GROUND_TOP = 112

APPROACH = 2600
DROP = 40
SETTLE = 90
OFFSETS = range(-8, 49, 2)


def main():
    want = int(sys.argv[1], 0) if len(sys.argv) > 1 else 0x0A
    pb = boot_to_gameplay()

    # Mario's Y byte while standing on ordinary ground, before anything is poked.
    for _ in range(30):
        pb.tick()
    ground_y = pb.memory[MARIO_Y]
    print(f"mario stands on ground (top edge screen y {GROUND_TOP}) at y byte {ground_y}")

    found = None
    pb.button_press("right")
    for _ in range(APPROACH):
        pb.memory[MARIO_Y] = FLY_Y
        pb.tick()
        for s in range(SLOTS):
            if slot(pb, s)[0] == want and 50 <= slot(pb, s)[3] <= 140:
                found = s
                break
        if found is not None:
            break
    pb.button_release("right")
    if found is None:
        print(f"kind 0x{want:02X} never came on screen")
        pb.stop()
        return 1

    state = slot(pb, found)
    print(f"kind 0x{want:02X} in slot {found} at x {state[3]}, y {state[2]}\n")


    # Every offset runs inside the same approach, repositioning Mario between
    # trials. A save state looked like the tidier way to do this and is not:
    # restoring one and then placing him drops him through the lift at every
    # offset, including the ones a plain run holds him at.
    print("offset  landed  mario x  object x  mario y  object y  rest")
    held = []
    for offset in OFFSETS:
        state = slot(pb, found)
        for _ in range(4):
            pb.memory[MARIO_Y] = FLY_Y
            pb.tick()
        pb.memory[SCREEN_X] = max(state[3] + offset, 8)
        pb.memory[MARIO_Y] = max(state[2] - DROP, 8)
        for _ in range(SETTLE):
            pb.tick()
        now = slot(pb, found)
        my = pb.memory[MARIO_Y]
        # Held up means resting above the object and moving with it. Being
        # merely near it is not enough: down on the floor at y 142, he passes
        # a distance test whenever the lift happens to be low in its cycle.
        landed = 0 < now[2] - my < 24
        if landed:
            held.append(offset)
        print(f"{offset:6d}  {'yes' if landed else 'no ':>6}  "
              f"{pb.memory[SCREEN_X]:7d}  {now[3]:8d}  {my:7d}  {now[2]:8d}  "
              f"{now[2] - my if landed else '-':>4}")
    pb.stop()

    if not held:
        print("\nnothing held him up")
        return 1
    print(f"\nheld from offset {held[0]} to {held[-1]}, "
          f"which is {held[-1] - held[0] + 2} px of surface at this step")
    return 0


if __name__ == "__main__":
    sys.exit(main())
