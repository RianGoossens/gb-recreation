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

from run_through_levels import MARIO_Y, PHASE, SCREEN_X
from sml_boot import boot_to_gameplay

OAM = 0xFE00
SPRITES = 40
NEAR = 12


def mario_tiles(pb):
    """The tile ids of the sprites drawn at Mario's own position."""
    x, y = pb.memory[SCREEN_X], pb.memory[MARIO_Y]
    found = []
    for i in range(SPRITES):
        sy, sx, tile, attr = (pb.memory[OAM + i * 4 + f] for f in range(4))
        if sy == 0 or sx == 0:
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


def run(pb, label, frames, button=None, tap=None):
    print(f"\n{label}:")
    if button:
        pb.button_press(button)
    seen = {}
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
    if button:
        pb.button_release(button)
    for (tile, flip, phase), frames_at in sorted(seen.items()):
        span = f"{frames_at[0]}..{frames_at[-1]}"
        print(f"  tile 0x{tile:02X}  flip {int(flip)}  phase 0x{phase:02X}  "
              f"{len(frames_at):4d} frames  ({span})")


def main():
    pb = boot_to_gameplay()
    for _ in range(40):
        pb.tick()

    run(pb, "standing still", 120)
    run(pb, "walking right", 240, button="right")
    run(pb, "walking left", 240, button="left")
    run(pb, "jumping from a stand", 120, tap=10)
    run(pb, "jumping while running", 160, button="right", tap=20)
    pb.stop()
    return 0


if __name__ == "__main__":
    sys.exit(main())
