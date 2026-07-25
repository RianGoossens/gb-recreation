# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Measure the cartridge's stomp bounce by landing a real stomp, deterministically.

Two earlier attempts (see docs/reference/physics.md) reacted to "an enemy is
near" and jumped, then looked for a bounce in the trace. Neither ever landed
real contact: the jumps either arced over the enemy or the "kill" was an
off-screen despawn while Mario stood on the ground.

This one changes both halves of that:

* Ground truth is Mario's own vertical phase byte `0xC207`, not an OAM count.
  A stomp is the only thing that makes the phase go 2 (falling) -> 1 (rising)
  while `0xC20A` says Mario is still airborne. An enemy leaving OAM for any
  other reason cannot produce that.
* The approach is a save-state sweep instead of a live reaction. Walk right
  until an enemy sprite is actually on screen ahead of Mario, snapshot there,
  then replay that same frame with every (wait, hold) jump combination until
  one of them connects. Same starting state every trial, so a hit is
  reproducible rather than lucky.

Usage: uv run tools/measure_stomp_bounce.py
"""

import sys

from sml_boot import boot_to_gameplay, restore, snapshot

MARIO_Y = 0xC201
MARIO_X = 0xC202
PHASE = 0xC207  # 0 grounded, 1 rising, 2 falling
GROUNDED = 0xC20A

OAM = 0xFE00
MARIO_SLOTS = range(3, 7)  # Mario's own sprite, pinned in an earlier session



def enemy_sprites(pb):
    """On-screen OAM sprites that are not Mario's, as (screen_x, screen_y)."""
    out = []
    for slot in range(40):
        if slot in MARIO_SLOTS:
            continue
        y = pb.memory[OAM + slot * 4]
        x = pb.memory[OAM + slot * 4 + 1]
        if 16 < y < 160 and 0 < x < 168:
            out.append((x - 8, y - 16))
    return out


def find_approach(pb, max_frames=600):
    """Walk right until an enemy is on screen ahead of Mario. Returns a state."""
    pb.button_press("right")
    for _ in range(max_frames):
        pb.tick()
        mario_x = pb.memory[MARIO_X]
        ahead = [s for s in enemy_sprites(pb) if 24 < s[0] - mario_x < 72]
        if ahead and pb.memory[GROUNDED] == 1:
            pb.button_release("right")
            return snapshot(pb), ahead[0][0] - mario_x
    return None, None


def trial(pb, state, wait, hold, frames=70):
    """Replay from `state`: walk right `wait` frames, jump holding A `hold`.

    Returns the per-frame trace as (y, phase, grounded, enemy_count) tuples.

    PyBoy's button state is not part of a save state, so it survives
    load_state and leaks from the previous trial into this one. Releasing
    first is what makes two trials with the same arguments give the same
    answer (before this, a sweep and a single re-run of one of its hits
    disagreed, because the sweep's earlier trials had left Right held).
    """
    pb.button_release("right")
    pb.button_release("a")
    restore(pb, state)
    pb.button_press("right")
    for _ in range(wait):
        pb.tick()
    pb.button_press("a")
    trace = []
    for f in range(frames):
        if f == hold:
            pb.button_release("a")
        pb.tick()
        trace.append(
            (
                pb.memory[MARIO_Y],
                pb.memory[PHASE],
                pb.memory[GROUNDED],
                len(enemy_sprites(pb)),
                pb.memory[MARIO_X],
                enemy_sprites(pb),
            )
        )
    pb.button_release("right")
    return trace


def find_stomp(trace):
    """Index of the falling->rising flip while airborne, or None.

    Nothing else in this stretch of World 1-1 turns a fall back into a rise
    without touching the ground. Confirmed by rendering the frames around
    one hit (`tools/stomp_frames.py`): Mario comes down on the Chibibo, it
    vanishes, the 100-point popup appears, and the rise starts.
    """
    for i in range(1, len(trace)):
        prev, cur = trace[i - 1], trace[i]
        if prev[1] == 2 and cur[1] == 1 and prev[2] == 0 and cur[2] == 0:
            return i
    return None


def bounce_arc(trace, at):
    """Rise height and duration of the bounce starting at frame `at`.

    Ends at the frame the phase byte flips back to falling, which is the
    game's own statement that the upward speed reached zero.
    """
    start_y = trace[at - 1][0]
    end = at
    while end + 1 < len(trace) and trace[end][1] == 1:
        end += 1
    peak = min(t[0] for t in trace[at - 1 : end + 1])
    return start_y - peak, end - (at - 1)


def print_trace(label, trace, at):
    print(label)
    print("  frame    y   dy  phase  grounded")
    for i in range(max(0, at - 4), min(len(trace), at + 18)):
        y, phase, ground = trace[i][0], trace[i][1], trace[i][2]
        dy = y - trace[i - 1][0] if i > 0 else 0
        mark = "  <- bounce" if i == at else ""
        print(f"  {i:5d} {y:4d} {dy:+4d} {phase:6d} {ground:9d}{mark}")
    print()


def main():
    pb = boot_to_gameplay()
    state, gap = find_approach(pb)
    if state is None:
        print("no enemy came into view while walking right")
        return 1
    print(f"snapshot taken with an enemy {gap}px ahead of Mario\n")

    hits = []
    for wait in range(0, 40):
        for hold in (2, 4, 6, 8, 12, 16, 70):
            trace = trial(pb, state, wait, hold)
            at = find_stomp(trace)
            if at is not None:
                hits.append((wait, hold, at, trace))

    if not hits:
        print("no stomp landed across the sweep")
        return 1

    first = hits[0]
    print_trace(f"one full bounce (wait={first[0]} hold={first[1]})", first[3], first[2])

    print(f"{len(hits)} stomps landed. Bounce arc per trial:\n")
    print("  hold  held at bounce  rise(px)  frames  v0 = 2d/t")
    groups = {}
    for wait, hold, at, trace in hits:
        rise, frames = bounce_arc(trace, at)
        if frames == 0:
            continue
        held = hold > at
        v0 = 2 * rise / frames
        groups.setdefault((hold, held), []).append((rise, frames, v0))

    for (hold, held), rows in sorted(groups.items()):
        rise = sum(r[0] for r in rows) / len(rows)
        frames = sum(r[1] for r in rows) / len(rows)
        v0 = sum(r[2] for r in rows) / len(rows)
        print(
            f"  {hold:4d}  {str(held):14s}  {rise:8.2f}  {frames:6.2f}  "
            f"{v0:9.3f}   (n={len(rows)})"
        )

    all_rows = [r for rows in groups.values() for r in rows]
    v0 = sum(r[2] for r in all_rows) / len(all_rows)
    print(f"\noverall v0 = {v0:.3f} px/frame = {round(v0 * 256)} subpixel units")
    return 0


if __name__ == "__main__":
    sys.exit(main())
