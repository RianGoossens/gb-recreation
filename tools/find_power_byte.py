# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Which byte makes Mario big?

Big Mario's box is 11 by 16 in the engine, of which only the width is
measured: no run has ever reached a mushroom on the cartridge, so the height
is the sprite's two tiles rather than a number off the game. It is one of the
last open discrepancies in `docs/reference/faithfulness.md`.

Reaching a mushroom is hard. Making the game think he already has one is not,
if the byte that says so can be found, and that search can be exhaustive
rather than clever, which is how the expert-mode flag was found
(`tools/find_skip_flag.py`). Snapshot at a known frame, poke one byte, run a
few frames, and check whether the sprite drawn at Mario's position is a big
Mario block. Restore and move on.

Mario's own state is known to sit around 0xC200 (his y at 0xC201, his x at
0xC202, the rising/falling byte at 0xC207), so that page is swept first and
the rest of work RAM only if it turns up nothing.

The check is the sprite rather than any byte, because a byte that merely looks
like a size flag is what this is trying to avoid assuming. Big Mario's blocks
are atlas rows 2 and 3, tiles 0x20 and up, and small Mario's are rows 0 and 1
below 0x20 (`docs/reference/sprites.md`).

What this found: nothing. All of work RAM, 0xC000 to 0xE000, single-byte
pokes of 1, and Mario never drew a big frame. That result is weak on its own,
because there is no way to make him big by other means and so no positive
control: a probe with no known yes is not much better than one with no known
no. What it does say is where not to look next.

Do not point it at high RAM. 0xFF80 to 0xFFFF holds the stack and the
interrupt enable register, and sweeping it hangs the emulator rather than
reporting anything.

Usage: uv run tools/find_power_byte.py [start] [end]
"""

import sys

sys.path.insert(0, "tools")

from run_through_levels import MARIO_Y, SCREEN_X
from sml_boot import boot_to_gameplay

OAM = 0xFE00
SPRITES = 40
NEAR = 12
SETTLE = 40
TRY = 16
VALUES = (1,)
# Big Mario's frames start at tile 0x20 and run to the end of his two rows.
BIG = range(0x20, 0x40)


def mario_tile(pb):
    x, y = pb.memory[SCREEN_X], pb.memory[MARIO_Y]
    best = None
    for i in range(SPRITES):
        sy, sx, tile = (pb.memory[OAM + i * 4 + f] for f in range(3))
        if sy == 0 or sx == 0:
            continue
        if abs(sx - x) <= NEAR and abs(sy - y) <= NEAR + 8:
            if best is None or (sy, sx) < best[0]:
                best = ((sy, sx), tile)
    return best[1] if best else None


def main():
    start = int(sys.argv[1], 0) if len(sys.argv) > 1 else 0xC200
    end = int(sys.argv[2], 0) if len(sys.argv) > 2 else 0xC300

    pb = boot_to_gameplay()
    for _ in range(SETTLE):
        pb.tick()
    import io

    snapshot = io.BytesIO()
    pb.save_state(snapshot)

    small = mario_tile(pb)
    print(f"before any poke, Mario draws tile 0x{small:02X}")
    if small in BIG:
        print("he is already big, so this measures nothing")
        return 1

    hits = []
    for address in range(start, end):
        for value in VALUES:
            snapshot.seek(0)
            pb.load_state(snapshot)
            # A button pressed straight after a restore does not register
            # until a tick has gone by; the same caution applies to a poke.
            pb.tick()
            pb.memory[address] = value
            for _ in range(TRY):
                pb.tick()
            tile = mario_tile(pb)
            if tile is not None and tile in BIG:
                print(f"  0x{address:04X} = {value}: Mario draws tile 0x{tile:02X}")
                hits.append((address, value))
    pb.stop()

    if not hits:
        print(f"\nnothing in 0x{start:04X}..0x{end:04X} made him big")
        return 1
    print(f"\n{len(hits)} candidates: " + ", ".join(f"0x{a:04X}={v}" for a, v in hits))
    return 0


if __name__ == "__main__":
    sys.exit(main())
