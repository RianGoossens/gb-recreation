# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""What are the sprites near World 1-3's boss that are not the boss?

`measure_boss_sprite.py` collected the nine sprites King Totomesu is drawn
from, and reported leftover tiles (`C4` `C5`, `D4` `D5`, `FE`) in some frames
that moved leftwards. Leftwards across the screen, away from a boss whose own x
never moves, is the shape of a projectile, and that is all that was known.

This parks Mario at the far side of the screen the same way and then tracks
every OAM entry frame by frame, chaining each into the track it continues by
proximity, rather than reading one frame at a time. A track's tiles, its
lifetime and its per-frame motion say what it is.

Two controls run in the same pass, because a tracker that reports motion for
everything says nothing:

  * The boss's own sprites are tracked alongside. Its x never moves and its y
    runs 20 px up and back down on a 162-frame cycle, so the tracker has to
    come back with vertical-only tracks for it. If it cannot follow the thing
    that is already measured, it cannot follow anything.
  * The same tracking runs on Gao (`0x3F`) in the same level, which is an
    ordinary enemy with no projectile. Any tile group that only appears in the
    boss run belongs to the boss.

Usage: uv run tools/measure_boss_fire.py [kind] [level]
"""

import sys

sys.path.insert(0, "tools")

from dump_object_slots import EMPTY, SLOTS, slot
from probe_object_contact import approach
from run_through_levels import FLY_Y, MARIO_Y, SCREEN_X

OAM = 0xFE00
SPRITES = 40
# Where to park Mario. The object is only reached while it sits between x 50
# and 140, so the left edge keeps his own sprites out of the reading.
MARIO_PARK = 8
# Anything this close to Mario's parked position is his own drawing. The first
# run of this cut on x alone, at 24, and every projectile "vanished" at x 33,
# which is 8 + 24 + 1: the instrument's own edge, read as the game's. He is
# parked in the air well above the fire's line, so cutting on both axes
# separates them.
MARIO_NEAR_X = 12
MARIO_NEAR_Y = 20
# Long enough for several leap cycles (the boss's is 162 frames), so a thing
# that only happens once a cycle is seen more than once.
WATCH = 700
# How far a sprite may move in one frame and still be the same sprite. The
# fastest thing measured on this hardware moves a couple of pixels a frame.
# This has to stay under 8, the spacing of the grid the boss's own nine
# sprites sit on: at 8 a track hops to its neighbour and comes back holding
# tiles from half the drawing, which is what the first run of this did.
STEP = 3
# Sprites the hardware is not showing. OAM y is 16 past the top of the screen,
# so anything at or past this is parked below it.
HIDDEN_Y = 160
HIDDEN_X = 168
# A track this short is a flicker, not an object.
MIN_LIFE = 3


class Track:
    def __init__(self, frame, x, y, tile):
        self.start = frame
        self.last = frame
        self.first_pos = (x, y)
        self.pos = (x, y)
        self.tiles = {tile}
        self.frames = 1
        self.history = [(frame, x, y, tile)]

    def extend(self, frame, x, y, tile):
        self.last = frame
        self.pos = (x, y)
        self.tiles.add(tile)
        self.frames += 1
        self.history.append((frame, x, y, tile))

    @property
    def life(self):
        return self.last - self.start + 1

    def key(self):
        return tuple(sorted(self.tiles))


def sprites(pb):
    out = []
    for i in range(SPRITES):
        sy, sx, tile, attr = (pb.memory[OAM + i * 4 + f] for f in range(4))
        if sy == 0 or sx == 0 or sy >= HIDDEN_Y or sx >= HIDDEN_X:
            continue
        if (abs(sx - (MARIO_PARK + 8)) <= MARIO_NEAR_X
                and abs(sy - (FLY_Y + 16)) <= MARIO_NEAR_Y):
            continue
        out.append((sx, sy, tile, attr))
    return out


def track(pb, s, kind):
    """Follow every sprite on screen for WATCH frames, and note the slots."""
    open_tracks = []
    done = []
    slots_seen = set()
    events = []
    ref = []
    held = [slot(pb, k)[0] for k in range(SLOTS)]
    for frame in range(WATCH):
        pb.memory[SCREEN_X] = MARIO_PARK
        pb.memory[MARIO_Y] = FLY_Y
        pb.tick()
        if slot(pb, s)[0] != kind:
            break
        for k in range(SLOTS):
            state = slot(pb, k)
            if state[0] != EMPTY:
                slots_seen.add(state[0])
            if state[0] != held[k]:
                events.append((frame, k, held[k], state[0]))
                held[k] = state[0]
        state = slot(pb, s)
        ref.append((frame, state[3], state[2]))

        unmatched = []
        for x, y, tile, _attr in sprites(pb):
            best = None
            for t in open_tracks:
                if t.last == frame:
                    continue
                px, py = t.pos
                if abs(px - x) <= STEP and abs(py - y) <= STEP:
                    cost = abs(px - x) + abs(py - y)
                    if best is None or cost < best[0]:
                        best = (cost, t)
            if best is None:
                unmatched.append((x, y, tile))
            else:
                best[1].extend(frame, x, y, tile)
        for x, y, tile in unmatched:
            open_tracks.append(Track(frame, x, y, tile))

        still_open = []
        for t in open_tracks:
            (done if t.last < frame else still_open).append(t)
        open_tracks = still_open
    done += open_tracks
    return done, slots_seen, events, ref


def group(tracks):
    """Collect tracks by the tiles they were drawn from."""
    out = {}
    for t in tracks:
        if t.life < MIN_LIFE:
            continue
        out.setdefault(t.key(), []).append(t)
    return out


def report(label, tracks, slots_seen):
    print(f"{label}")
    print(f"  object kinds in slots during the run: "
          + " ".join(f"0x{k:02X}" for k in sorted(slots_seen)))
    grouped = group(tracks)
    if not grouped:
        print("  nothing tracked")
        return grouped
    for tiles, group_tracks in sorted(grouped.items(), key=lambda g: -len(g[1])):
        ids = " ".join(f"{t:02X}" for t in tiles)
        print(f"  tiles {ids}: {len(group_tracks)} tracks")
        for t in sorted(group_tracks, key=lambda t: t.start)[:6]:
            (x0, y0), (x1, y1) = t.first_pos, t.pos
            dx, dy = x1 - x0, y1 - y0
            speed = f"{dx / max(t.life - 1, 1):+.2f} px/frame x"
            print(f"    frames {t.start:4d}-{t.last:4d} ({t.life:3d}): "
                  f"({x0:3d},{y0:3d}) -> ({x1:3d},{y1:3d})  "
                  f"d ({dx:+4d},{dy:+4d})  {speed}")
        if len(group_tracks) > 6:
            print(f"    ... {len(group_tracks) - 6} more")
    return grouped


def detail(tracks, events, ref):
    """The parts that say what a flying thing is: cadence, where it starts,
    what the object it came from was doing, and whether it took a slot."""
    flying = [t for t in tracks
              if t.life >= MIN_LIFE and t.pos[0] - t.first_pos[0] < -16]
    if not flying:
        print("  nothing crossed the screen")
        return
    flying.sort(key=lambda t: t.start)
    at = {f: (x, y) for f, x, y in ref}
    print("  things that crossed the screen leftwards:")
    starts = []
    for t in flying:
        (x0, y0), (x1, y1) = t.first_pos, t.pos
        rx, ry = at.get(t.start, (0, 0))
        tiles = " ".join(f"{v:02X}" for v in sorted(t.tiles))
        gap = f"{t.start - starts[-1]:+4d}" if starts else "   -"
        starts.append(t.start)
        print(f"    frame {t.start:4d} ({gap} since last) from ({x0:3d},{y0:3d}) "
              f"to ({x1:3d},{y1:3d}) over {t.life:3d} frames, tiles {tiles}, "
              f"slot at ({rx:3d},{ry:3d})")
    first = flying[0]
    seq = []
    for frame, x, y, tile in first.history:
        if not seq or seq[-1][3] != tile:
            seq.append((frame, x, y, tile))
    print("  the first one, every frame its tile changed:")
    for frame, x, y, tile in seq[:12]:
        print(f"    frame {frame:4d} ({x:3d},{y:3d}) tile 0x{tile:02X}")
    moves = [(f, y) for i, (f, _x, y) in enumerate(ref)
             if i == 0 or ref[i - 1][2] != y]
    print("  the object's own y, every frame it changed (first 30):")
    print("    " + "  ".join(f"{f}:{y}" for f, y in moves[:30]))
    print("  slots filling and emptying during the run:")
    for frame, k, was, now in events[:20]:
        print(f"    frame {frame:4d} slot {k}: 0x{was:02X} -> 0x{now:02X}")
    if not events:
        print("    none: no slot changed, so it is not an object record")


def run(kind, level, label, deep=False):
    pb, s = approach(kind, level)
    if pb is None:
        print(f"{label}: kind 0x{kind:02X} never came on screen in World {level}")
        return None
    tracks, slots_seen, events, ref = track(pb, s, kind)
    pb.stop()
    grouped = report(label, tracks, slots_seen)
    if deep:
        detail(tracks, events, ref)
    return grouped


def main():
    kind = int(sys.argv[1], 0) if len(sys.argv) > 1 else 0x08
    level = sys.argv[2] if len(sys.argv) > 2 else "1-3"

    control = run(0x3F, level, "control, Gao, an ordinary enemy with no projectile",
                  deep=True)
    if control is None:
        print("\nthe control never ran, so the result below means nothing")
        return 1
    print()
    boss = run(kind, level, f"the kind asked for, 0x{kind:02X}", deep=True)
    if boss is None:
        return 1

    print()
    only_boss = sorted(set(boss) - set(control))
    print("tile groups tracked in the boss run and not in the control:")
    for tiles in only_boss:
        ids = " ".join(f"{t:02X}" for t in tiles)
        print(f"  {ids}  ({len(boss[tiles])} tracks)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
