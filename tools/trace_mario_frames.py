# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Which of Mario's frames does the cartridge show, and when?

His frames were found in the object atlas without running anything: a still
pose and two walking poses per size, plus five blocks per row nobody had
identified (`docs/reference/sprites.md`). What the atlas cannot say is which
one plays at which moment, so the engine picks: the still pose standing and
airborne, the two walk poses alternating on its own animator. That is logged
as a stand-in.

This asks the game. Every frame it reads the top left tile of the sprite the
game drew for Mario, alongside the state bytes that would explain a choice:
whether he is moving, and the rising/falling byte. Standing still, walking,
and jumping each get their own stretch, so the answer is a table of state to
frame rather than one number.

Mario is found in OAM by position rather than by tile id, which would assume
the answer.

Usage: uv run tools/trace_mario_frames.py
"""

import sys

sys.path.insert(0, "tools")

from run_through_levels import LIVES, MARIO_Y, PHASE, SCREEN_X
from sml_boot import boot_to_gameplay

OAM = 0xFE00
SPRITES = 40
NEAR = 12
MARIO_TILES = 0x40
MARIO_BLOCKS = 6


def mario_tiles(pb):
    """The tile ids of the sprites drawn at Mario's own position."""
    x, y = pb.memory[SCREEN_X], pb.memory[MARIO_Y]
    found = []
    for i in range(SPRITES):
        sy, sx, tile, attr = (pb.memory[OAM + i * 4 + f] for f in range(4))
        if sy == 0 or sx == 0:
            continue
        # Mario is drawn from the first four rows of the sheet, so an id at
        # or past 0x40 belongs to something standing next to him. Without this
        # a walker he is about to meet is collected as one of his own poses.
        # Mario is drawn from the first four rows of the sheet, and only from
        # the first six blocks of each: rendering the atlas shows the last two
        # blocks of those rows hold other characters. Without this a walker he
        # is about to meet is collected as one of his own poses, and so is
        # whatever the black ball in block 7 belongs to.
        if tile >= MARIO_TILES or tile % 16 >= MARIO_BLOCKS * 2:
            continue
        if abs(sx - x) <= NEAR and abs(sy - y) <= NEAR + 8:
            found.append((sy, sx, tile, attr))
    found.sort()
    return found


def block(pb):
    """The top left tile id, which names the 2x2 block, and the flip bit."""
    found = mario_tiles(pb)
    if not found:
        return None, None
    top = found[0]
    return top[2], bool(top[3] & 0x20)


def run(pb, label, frames, button=None, tap=None, also=None, until_death=False):
    print(f"\n{label}:")
    buttons = [b for b in (button, also) if b]
    for b in buttons:
        pb.button_press(b)
    seen = {}
    order = []
    dying = {}
    lives = pb.memory[LIVES]
    for f in range(frames):
        if tap is not None and f == tap:
            pb.button_press("a")
        if tap is not None and f == tap + 14:
            pb.button_release("a")
        pb.tick()
        tile, flip = block(pb)
        if tile is None:
            continue
        key = (tile, flip, pb.memory[PHASE])
        seen.setdefault(key, []).append(f)
        order.append(tile)
        if until_death and pb.memory[LIVES] < lives:
            print(f"  he died on frame {f}")
            for d in range(240):
                pb.tick()
                tile, _flip = block(pb)
                if tile is not None:
                    dying.setdefault(tile, []).append(d)
            break
    for b in buttons:
        pb.button_release(b)
    if dying:
        for tile, at in sorted(dying.items()):
            print(f"  dying: tile 0x{tile:02X} for {len(at)} frames "
                  f"({at[0]}..{at[-1]})")
    for (tile, flip, phase), frames_at in sorted(seen.items()):
        span = f"{frames_at[0]}..{frames_at[-1]}"
        print(f"  tile 0x{tile:02X}  flip {int(flip)}  phase 0x{phase:02X}  "
              f"{len(frames_at):4d} frames  ({span})")
    holds = []
    for tile in order:
        if holds and holds[-1][0] == tile:
            holds[-1][1] += 1
        else:
            holds.append([tile, 1])
    runs = [n for _t, n in holds[2:-1]]
    if runs:
        print(f"  each pose is held {sorted(set(runs))} frames "
              f"over {len(runs)} changes")


def main():
    pb = boot_to_gameplay()
    for _ in range(40):
        pb.tick()

    run(pb, "standing still", 120)
    run(pb, "walking right", 240, button="right")
    run(pb, "walking left", 240, button="left")
    run(pb, "jumping from a stand", 120, tap=10)
    run(pb, "jumping while running", 160, button="right", tap=20)
    run(pb, "running right with b held", 240, button="right", also="b")
    run(pb, "skidding: right then left", 120, button="left")
    run(pb, "walking into whatever kills him", 1200, button="right",
        also="b", until_death=True)
    pb.stop()
    return 0


if __name__ == "__main__":
    sys.exit(main())
