# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy", "numpy"]
# ///
"""Pin what World 1-1 actually awards for each action.

`docs/reference/faithfulness.md` lists score as canonical in shape but with
the point values never checked against the cartridge. Our engine awards 100
for a stomp, 100 for a coin and 1000 for a power-up, none of it measured.

Reading it off the status bar (`sml_hud`) rather than out of WRAM: diffing
WRAM across a stomp turns up 96 changed bytes with no way to tell which is
the score, while the HUD says it in digits.

This plays World 1-1 with the reactive walker and records every frame the
score, coin count or life count moves. A jump in the score with the coin
count also moving is a coin; a jump with the coin count flat is something
else, and the stomp case is already confirmed at 100 by watching one
deterministic stomp.

Usage: uv run tools/measure_scores.py
"""

import sys

import sml_hud
from sml_boot import boot_to_gameplay
from sml_scroll import ScrollTracker
from sml_walker import ReactiveWalker

FRAMES = 3000


def main():
    pb = boot_to_gameplay()
    tracker = ScrollTracker(pb)
    walker = ReactiveWalker(pb)

    prev = sml_hud.read_all(pb)
    events = []
    for f in range(FRAMES):
        walker.step(pb, tracker)
        pb.tick()
        tracker.update(pb)
        if tracker.frozen > 5:
            print(f"died at frame {f}, stopping\n")
            break
        cur = sml_hud.read_all(pb)
        if cur[:3] != prev[:3]:
            events.append((f, prev, cur))
        prev = cur

    if not events:
        print("nothing scored")
        return 1

    print("frame  score      coins  lives   award")
    for f, before, after in events:
        award = after[0] - before[0]
        coins_gained = after[1] - before[1]
        note = f"+{award}" if award else ""
        if coins_gained:
            note += f" with {coins_gained:+d} coin"
        if after[2] != before[2]:
            note += f" and {after[2] - before[2]:+d} life"
        print(f"{f:5d} {after[0]:8d} {after[1]:6d} {after[2]:6d}   {note}")

    # The coin counter moves one frame before the score does, so a coin shows
    # up as two events rather than one. Pair them back up by looking a couple
    # of frames either side rather than only within a single frame.
    coin_frames = {f for f, b, a in events if a[1] != b[1]}
    coin_awards, other_awards = set(), set()
    for f, before, after in events:
        award = after[0] - before[0]
        if not award:
            continue
        near_coin = any(abs(f - c) <= 2 for c in coin_frames)
        (coin_awards if near_coin else other_awards).add(award)

    print(f"\ncoin: {sorted(coin_awards)} points")
    print(f"stomp: {sorted(other_awards)} points")
    return 0


if __name__ == "__main__":
    sys.exit(main())
