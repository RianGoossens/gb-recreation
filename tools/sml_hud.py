# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Read Super Mario Land's status bar straight off the background tilemap.

Score, coins, lives and the timer are all on screen every frame, and the
digit tiles turn out to be the digits themselves: tile 0 draws '0', tile 9
draws '9', and tile 44 is the blank used for leading spaces. Confirmed by
reading row 1 with the score at 0 (`44 44 44 44 44 0`) and again right after
a stomp (`44 44 44 1 0 0`, which is 100).

That is a better source than a WRAM address for this. Diffing WRAM across a
stomp turns up 96 changed bytes (sprite tables, the stack, animation
counters), with no way to tell which of them is the score. The HUD is
unambiguous.

Not a runnable tool: import the readers from another `uv run` script.
"""

MAP_BASE = 0x9800
BLANK = 44

SCORE = (1, 0, 6)  # row, first column, width
COINS = (1, 9, 2)
LIVES = (0, 6, 2)
TIMER = (1, 17, 3)


def _field(pb, field):
    row, col, width = field
    digits = [pb.memory[MAP_BASE + row * 32 + col + i] for i in range(width)]
    text = "".join(str(d) for d in digits if d != BLANK)
    return int(text) if text else 0


def score(pb):
    """The displayed score. It shows the top 6 digits; SML pads with a
    trailing 0, so a displayed 000100 is 100 points."""
    return _field(pb, SCORE)


def coins(pb):
    return _field(pb, COINS)


def lives(pb):
    return _field(pb, LIVES)


def timer(pb):
    return _field(pb, TIMER)


def read_all(pb):
    return score(pb), coins(pb), lives(pb), timer(pb)
