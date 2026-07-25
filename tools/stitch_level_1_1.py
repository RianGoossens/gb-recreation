# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy", "numpy"]
# ///
"""Stitch World 1-1's tilemap past the initially-loaded opening screen.

The background tilemap at 0x9800 is a 32-column ring buffer. At spawn it
already holds real data a few columns past the visible 20x18 screen (the
game preloads ahead of Mario's position), and it streams more in as he
walks right, reusing each buffer column roughly every 32 tiles of world
distance. There is no reliable way to compute which world column a given
buffer slot currently holds from a position estimate alone (tried and
discarded: dead-reckoning the estimate and picking the closest or the
next wraparound both mis-happened near lap boundaries, since preload
timing does not line up neatly with any fixed margin).

So this script does not guess. Every frame, it directly watches whether
each of the 32 buffer columns' contents changed since the previous frame.
The buffer only ever streams forward (never rewritten with older data),
so a slot's world-column identity starts at its raw buffer index (true at
spawn, confirmed against the static opening-screen tilemap) and increases
by exactly 32 every time that slot's value changes. That much needs no position estimate at all.

Watching only for a slot's value to change stalls silently across long
uniform terrain (a long flat stretch of repeated ground tile, for example),
where nothing ever looks different lap to lap. Two successive attempts to
fix that by estimating Mario's world position from WRAM were both wrong,
and the second was wrong for a long time before anyone noticed. See
`docs/reference/level-1-1.md` for the trail.

Position now comes from `sml_scroll.ScrollTracker`, which measures how far
the background actually moved between two rendered frames. It needs no
model of Mario's speed, it is validated against known physics (exactly
1 px/frame at his saturated walk), and it stops dead when the game does.
That last part matters: the death sequence freezes the level for about 150
frames while Mario's WRAM bytes keep reading as though he were alive and
walking, so a position integrated from those bytes runs on through a death
forever.

Getting past hazards is a search, not a policy. `sml_walker.ReactiveWalker`
holds Right and jumps at enemies and at whatever stops the scroll, but no
fixed setting of it survives the flying enemy on a pillar at world column
78. So every 120 frames of real progress this saves a `Checkpoint` (the
emulator, the scroll tracker and the stitched map together, since rewinding
one without the others would leave the map holding tiles from a future that
no longer happens), and on a death it rewinds and reseeds the walker.
Backing up goes deeper each time the same depth fails again, otherwise the
segment leading in just succeeds and puts Mario back in the same spot.

Run: uv run tools/stitch_level_1_1.py
"""

import struct
import sys
from pathlib import Path

from sml_boot import boot_to_gameplay, restore, snapshot
from sml_scroll import ScrollTracker
from sml_walker import ReactiveWalker

OUT = Path("assets/extracted")
MAP_BASE = 0x9800
MARIO_SCREEN_X = 81  # where the camera lock pins him
FROZEN_FRAMES = 5  # identical frames that mean the level stopped running
STALE_LAPS = 1  # a full 32-tile lap of no detected change before inferring a repeat
# How far ahead of Mario the buffer is ever seen to have streamed content in.
# Measured at 0 to about 17 tiles across 84 sampled transitions.
PRELOAD_MARGIN = 20
SEGMENT = 120  # frames of survival before a new checkpoint is taken
FRAME_BUDGET = 20000
MAX_ATTEMPTS = 25  # reseeded retries from one checkpoint before backing up
MIN_PROGRESS = 24  # pixels a segment must cover to count as progress
# Rows 0-1 are the status bar (score/coins/time), redrawn every frame
# regardless of scroll; they are not level geometry, so they are excluded.
HUD_ROWS = 2
ROWS = 18
COLS = 32


def nearest_world_col(bx, position_estimate, min_wc):
    """Nearest multiple-of-32 (relative to bx) to a precise position
    estimate, used only when a real value change was just observed.

    Bounded on both sides. Never below min_wc: the level cannot have
    negative columns, and a slot's world column cannot legitimately regress
    once established. Never above the position estimate plus PRELOAD_MARGIN
    either: the game streams a slot in shortly before Mario reaches it, so
    a label further ahead than the buffer ever preloads would be a fiction.
    Returns None when no lap satisfies both, meaning the slot should be left
    alone rather than given a number the run cannot support."""
    k = round((position_estimate - bx) / COLS)
    wc = bx + COLS * k
    while wc < min_wc:
        k += 1
        wc = bx + COLS * k
    return wc if wc <= position_estimate + PRELOAD_MARGIN else None


class Capture:
    """The stitched map, and the per-slot bookkeeping behind it."""

    def __init__(self, slot_wc, slot_val, combined, inferred):
        self.slot_wc = slot_wc
        self.slot_val = slot_val
        self.combined = combined
        self.inferred = inferred

    def absorb(self, row, new_vals, position):
        for bx in range(COLS):
            changed = new_vals[bx] != self.slot_val[row][bx]
            stale = position - self.slot_wc[row][bx] > STALE_LAPS * COLS
            if not changed and not stale:
                continue
            wc = nearest_world_col(bx, position, self.slot_wc[row][bx] + 1)
            if changed:
                self.slot_val[row][bx] = new_vals[bx]
            if wc is None:
                continue
            self.slot_wc[row][bx] = wc
            self.combined[(wc, row)] = self.slot_val[row][bx]
            # A detected change is a direct observation. A stale slot is an
            # inference: the buffer would almost certainly have refreshed it
            # by now if its content were going to differ (measured margin, 0
            # to 17 tiles during ongoing streaming, well under a full lap),
            # so its bytes are taken as a genuine repeat. Kept apart in the
            # output so the two are never confused.
            if changed:
                self.inferred.discard((wc, row))
            else:
                self.inferred.add((wc, row))

    def snapshot(self):
        return (
            [r[:] for r in self.slot_wc],
            [r[:] for r in self.slot_val],
            dict(self.combined),
            set(self.inferred),
        )

    def rollback(self, snap):
        self.slot_wc, self.slot_val, self.combined, self.inferred = snap


class Checkpoint:
    """A point the run can be rewound to: emulator, scroll, and map alike.

    All three have to move together. Rewinding the emulator alone would
    leave the map holding tiles from a future that no longer happens.
    """

    def __init__(self, pb, tracker, capture):
        self.state = snapshot(pb)
        self.scroll = tracker.scroll
        self.prev = tracker.prev.copy()
        self.capture = capture.snapshot()

    def restore(self, pb, tracker, capture):
        restore(pb, self.state)
        tracker.scroll = self.scroll
        tracker.prev = self.prev.copy()
        tracker.frozen = 0
        capture.rollback(self.capture)


def main():
    pb = boot_to_gameplay()

    x_spawn = pb.memory[0xC202]

    def read_row(row):
        return [pb.memory[MAP_BASE + row * 32 + bx] for bx in range(COLS)]

    # Trusted starting point: at spawn (before any movement) buffer index
    # bx holds world column bx exactly, already confirmed against the
    # static opening-screen tilemap.
    slot_wc = [[bx for bx in range(COLS)] for _ in range(ROWS)]
    slot_val = [read_row(row) for row in range(ROWS)]
    combined = {}
    inferred = set()
    for row in range(HUD_ROWS, ROWS):
        for bx in range(COLS):
            combined[(slot_wc[row][bx], row)] = slot_val[row][bx]

    tracker = ScrollTracker(pb)
    walker = ReactiveWalker(pb)

    capture = Capture(slot_wc, slot_val, combined, inferred)

    def advance(frames):
        """Run forward. Returns the frame the level stopped running, or None."""
        for i in range(1, frames + 1):
            walker.step(pb, tracker)
            pb.tick()
            tracker.update(pb)
            if tracker.frozen > FROZEN_FRAMES:
                return i
            # The camera's left edge in world pixels, plus Mario's fixed
            # screen position once locked, is his world position; the buffer
            # preloads a few columns past that.
            position = (tracker.scroll + MARIO_SCREEN_X) // 8
            for row in range(HUD_ROWS, ROWS):
                capture.absorb(row, read_row(row), position)
        return None

    # Checkpoint and retry instead of tuning a policy that survives
    # everything. No fixed policy gets past the flying enemy at world column
    # 78 (see sml_walker), and retrying the same policy from the same state
    # reproduces the same death, so each retry reseeds the walker.
    checkpoints = [Checkpoint(pb, tracker, capture)]
    emulated = 0
    attempt = 0
    failures = 0
    backoff = 1
    last_stuck_depth = None
    while emulated < FRAME_BUDGET and checkpoints:
        before = tracker.scroll
        died = advance(SEGMENT)
        emulated += SEGMENT
        # Surviving is not the same as getting anywhere: a walker that
        # stands still survives every segment forever. A segment only counts
        # when it also covers ground.
        if died is None and tracker.scroll - before >= MIN_PROGRESS:
            attempt = 0
            checkpoints.append(Checkpoint(pb, tracker, capture))
            print(f"  checkpoint {len(checkpoints) - 1}: scroll {tracker.scroll}px")
            continue
        failures += 1
        attempt += 1
        if attempt > MAX_ATTEMPTS:
            # Retrying from just before a hazard is not always enough runway:
            # by then Mario may be committed to an approach that cannot work.
            # Backing up one checkpoint is often not enough either, because
            # the segment leading in succeeds and lands him in the same spot.
            # So the backup gets deeper each time the same depth fails again.
            if len(checkpoints) == 1:
                break
            depth = len(checkpoints) - 1
            backoff = backoff * 2 if depth == last_stuck_depth else 1
            last_stuck_depth = depth
            del checkpoints[max(1, len(checkpoints) - backoff) :]
            attempt = 1
            print(f"  backing up {backoff} to checkpoint {len(checkpoints) - 1}")
        walker.release_all(pb)
        checkpoints[-1].restore(pb, tracker, capture)
        walker.reseed(failures)
        walker.resume(pb, tracker.scroll)

    stalled = not checkpoints or emulated >= FRAME_BUDGET
    slot_wc, slot_val, combined, inferred = (
        capture.slot_wc,
        capture.slot_val,
        capture.combined,
        capture.inferred,
    )

    pb.stop()

    print(f"spawn x={x_spawn}")
    print(
        f"{len(checkpoints) - 1} segments of real progress over {emulated} "
        f"emulated frames, after {failures} rewinds"
    )
    if not checkpoints:
        print("gave up: the search rewound all the way back to the start")
    elif stalled:
        print(f"ran out of the {FRAME_BUDGET}-frame emulation budget")
    print(
        f"measured scroll {tracker.scroll}px, so Mario reached world column "
        f"{(tracker.scroll + MARIO_SCREEN_X) // 8}"
    )
    if tracker.ambiguous_frames:
        print(f"{tracker.ambiguous_frames} frames had an ambiguous scroll reading")
    print(
        f"{len(combined) - len(inferred)} cells directly observed, "
        f"{len(inferred)} inferred as repeats (not directly reconfirmed)"
    )

    max_col = max(k[0] for k in combined)
    min_col = min(k[0] for k in combined)
    width = max_col - min_col + 1

    OUT.mkdir(parents=True, exist_ok=True)
    cells = bytearray(width * ROWS)
    for row in range(ROWS):
        for c in range(width):
            cells[row * width + c] = combined.get((min_col + c, row), 44)

    map_blob = b"SMLM" + bytes([1]) + struct.pack("<HH", width, ROWS) + bytes(cells)
    out_path = OUT / "level_1_1_stitched_partial.tmap"
    out_path.write_bytes(map_blob)
    print(f"wrote {out_path} ({width}x{ROWS}, world columns {min_col}..{max_col})")


if __name__ == "__main__":
    sys.exit(main())
