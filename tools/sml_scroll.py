# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy", "numpy"]
# ///
"""Measure how far World 1-1 has scrolled, by watching the screen move.

Every earlier attempt to know Mario's world position *derived* it, and every
derivation needed a correction later (see `docs/reference/level-1-1.md`).
Two direct sources were checked and both are dead ends. `SCX`, the hardware
register that shifts the background sideways, reads 0 at every VBlank sample
because Super Mario Land rewrites it mid-frame for the status bar split. A
full scan of WRAM, OAM and HRAM for a byte or 16-bit pair tracking the
expected scroll (`tools/find_scroll_position.py`) finds nothing.

The scroll is still directly observable, on the screen rather than in memory:
between two frames the whole background shifts left by exactly the scroll
delta. Cross-correlating consecutive playfield captures over a small range of
candidate shifts recovers that delta, and summing gives the true scroll with
no model of Mario's speed involved.

Two frames that are byte-identical mean the game is not running the level at
all, which is what a death looks like: the death sequence freezes the world
for about 150 frames before respawning. That matters because Mario's WRAM
state goes stale rather than blank while this happens (`0xC202` keeps
reading 81, `0xC20A` keeps reading grounded, `0xC20C` keeps reading a
saturated 6), so anything integrating those bytes keeps accumulating
distance through a death that never happened. `frozen` here is the honest
signal for that.

Not a runnable tool: import `ScrollTracker` from another `uv run` script.
"""

import numpy as np

# Below the status bar, which is drawn at SCX 0 and never scrolls.
PLAYFIELD_TOP = 24
PLAYFIELD_BOTTOM = 136
MAX_SHIFT = 8


def playfield(pb):
    return np.asarray(pb.screen.ndarray, dtype=np.int16)[
        PLAYFIELD_TOP:PLAYFIELD_BOTTOM, :, 0
    ]


class ScrollTracker:
    """Accumulated horizontal scroll, measured frame to frame.

    `update` must be called once per `tick`, since it works on the change
    between consecutive frames.
    """

    def __init__(self, pb):
        self.prev = playfield(pb)
        self.scroll = 0
        self.frozen = 0
        self.ambiguous_frames = 0

    def update(self, pb):
        cur = playfield(pb)
        if np.array_equal(self.prev, cur):
            self.frozen += 1
            self.prev = cur
            return 0
        self.frozen = 0

        scores = []
        for shift in range(MAX_SHIFT + 1):
            width = self.prev.shape[1] - shift
            scores.append(np.abs(self.prev[:, shift:] - cur[:, :width]).mean())
        best = int(np.argmin(scores))
        runner_up = min(s for i, s in enumerate(scores) if i != best)
        # A real shift wins clearly. If two candidates score nearly the same
        # the frame carries no usable evidence, so count it and keep the
        # best guess rather than pretend the reading is solid.
        if runner_up - scores[best] < 0.05:
            self.ambiguous_frames += 1

        self.scroll += best
        self.prev = cur
        return best
