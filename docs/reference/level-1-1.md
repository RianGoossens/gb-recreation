# Reference notes: World 1-1

Working notes for extracting World 1-1's real geometry. Values marked
"observed" were read directly from the verified ROM or a headless emulator
run. This file grows as the extraction subtasks land; see the plan for the
breakdown.

## Reaching gameplay (for any tool that needs it)

Boot the ROM, wait for the title screen (600 frames is enough, same as the
title extraction), press Start, then wait for the level to finish loading
(300 more frames is enough to reach a controllable Mario standing at the
spawn of 1-1). `tools/extract_level_1_1.py` does this.

## The opening screen (observed)

Read live from VRAM right after gameplay starts:

| Thing | Value | Meaning |
|-------|-------|---------|
| LCDC | 0xC3 | background on, signed tile addressing, map at 0x9800 |
| BGP | 0xE4 | same palette as the title screen |
| SCX, SCY | 0, 0 | no scroll yet, spawn is the left edge of the level |
| Tile addressing | signed (0x8800 method) | same scheme as the title screen |
| Unique tiles | 39 of 360 cells | the opening 20x18 screen |

## Tile graphics: no new ROM offsets needed

Every one of the 39 unique tiles used on the opening screen was checked
against the ROM (search each tile's live VRAM bytes for its file offset, the
same technique as `tools/find_rom_offset.py`). All of them land inside the
three tile blocks already pinned for the title screen (`title.rs`,
`title-screen.md`):

- Block 1 (`rom 0x9032`, `vram 0x9000`, size `0x2C0`)
- Block 2 (`rom 0xB91A`, `vram 0x9300`, size `0x500`)
- Block 3 (`rom 0xBE1A`, `vram 0x8800`, size `0x170`)

So the title screen and World 1-1 draw from the same shared bank-2 tile
atlas; the level just uses a different slice of it (ground, pyramid blocks,
palm trees, bushes, clouds) alongside the shared HUD/text tiles. No fourth
block was needed for this screen.

A caution from doing this search per tile: several tiles are simple or
repetitive enough (a solid color, a symmetric pattern) that the same 16 bytes
occur more than once in the ROM. Searching a single tile in isolation can
report a coincidental match in the wrong bank before the real one. The three
blocks above were cross-checked against every observed tile address falling
inside their already-verified ranges, not from a fresh single-tile search
taken at face value.

## The opening tilemap

`tools/extract_level_1_1.py` reads the background tile map the same way
`extract_title.py` does, and writes `assets/extracted/level_1_1_opening.tmap`
(our SMLM format) plus a tile sheet and reference screenshot, all gitignored.
This captures only the 20x18 view visible at spawn; the level scrolls well
past it.

## Tile solidity: what is confirmed so far

Method: force Mario to walk and jump through the opening screen (`SCX` stays
`0` here, so the raw tilemap index at `(feet_row, x // 8)` is directly
readable with no scroll math), and watch `0xC20A` (see `physics.md`) flip
between grounded and airborne. A tile a resting Mario stands on is solid; a
tile his jump arc passes through with no effect on his motion is not.

- Tile `96` (the ground surface row, `row 16` for the whole opening screen):
  **solid**. Directly observed: Mario's feet rest on it continuously
  whenever `0xC20A == 1`.
- Tile `44` (the blank sky/background filler, most of rows 0-15):
  **non-solid**. Directly observed: it is the tile at every cell Mario's
  jump arc passes through in open air with no collision effect.
- Tile `97` (directly beneath `96`, the underground fill) is presumed solid
  by level-design consistency (a ground block is solid all the way down),
  but this has not been independently confirmed by a direct collision, since
  Mario never touches it from below or the side on this screen. Flagging
  this rather than asserting it as observed.
- The elevated block staircase above the ground row (rows 9-15, roughly
  columns 0-15) is now precisely mapped from the static spawn tilemap
  (`SCX = 0`, safe to read directly). Full grid, `(column, row): tile`,
  reading down each ascending/descending side from the ground up:

  ```
  row  9:                          col8=54  col9=94
  row 10:                 col7=54           col10=94
  row 11:        col5=129 col6=54                    col11=94
  row 12:        col5=54                                      col12=94
  row 13: col2=54 col3=94 col4=54           col9=50 col10=51           col13=94
  row 14: col1=112 col2=113        col4=94  col7=50 col8=51 col9=49           col14=94
  row 15: col0=54 col1=114 col2=115        col5=94  col7=49    col9=49              col15=94
  row 16: ground (tile 96) under all of it
  ```

  It is a symmetric pyramid: a rising staircase from column 0 up to a peak
  around columns 8-9 (tile `94` tracing the outer diagonal edge on both
  sides, `54`/`112`-`115`/`49`-`51`/`129` filling in the step faces), then
  back down to column 15. This was previously described only vaguely (a
  loose list of tile IDs); the exact per-column shape above is new.

  Solidity is still **not directly confirmed by collision**, and the
  earlier "two captures of the same cell disagreed" finding is superseded:
  that was most likely the same `SCX`-sampling aliasing that caused the
  `x = 81` mystery above, not a sub-pixel column problem. Every practical
  jump tried this session (triggered at various points approaching the
  staircase, held for 1-4 frames, starting from a dead stop or a full run)
  either failed to leave the ground at all or cleared the entire structure
  in one arc without ever registering a landing on it. Concretely, with a
  minimal jump (`A` held 2 frames) from a standing start, Mario's arc peaks
  at only 10px of height right around column 7-8 and is already descending
  again by the time he is back over solid ground past column 9; the
  staircase's own steps range from 8px (column 7) up to 56px (columns 8-9)
  above the ground, and by the frame his height matches a given step's
  surface, his forward speed has already carried him past that column. A
  full running jump only makes this worse (more horizontal distance
  covered per frame of height gained). So this structure cannot be
  collision-tested by jumping into it from a normal rightward run; it
  would need either a slower approach (partial run, not saturated speed)
  or a controlled fall from directly above a specific column. Until one of
  those confirms it, treat it as solid by level-design consistency (it is
  drawn as a stacked-block staircase, the same convention used everywhere
  else in the series), the same caveat already applied to tile `97`.

  Tried the "release Right first" trick that worked for the world-column-48
  hazard (see the stitching section below): released Right for 50-90
  frames to let speed decay to 0, then jumped with only 0-20 frames of
  re-acceleration first. Still landed past the structure every time. The
  reason is different from the hazard case: Right stays held through the
  whole ~30+ frame flight, so Mario's horizontal speed ramps right back up
  to near-saturation *during* the jump regardless of how slow he was at
  takeoff, covering enough distance to clear the structure anyway. A
  standing high jump with Right released throughout the flight (not just
  before it) is the untried next variant, though at that point he is
  jumping mostly straight up and might just come back down in front of the
  structure rather than onto it. Left as still-unconfirmed rather than
  forced.
- **Resolved**: the on-screen freeze at `x = 81` is not a blockage. It is
  the standard mid-screen camera lock, the same behavior as the NES Mario
  games: once Mario reaches roughly the horizontal center of the screen,
  the game stops moving his sprite and scrolls the world past him instead.
  Direct proof: holding Right for 1200 frames with no jumping at all keeps
  `0xC202` (Mario's screen X) pinned at `81` the entire time, but a
  frame-by-frame screenshot diff shows the background visibly and
  continuously changing from frame 150 onward (`diff bbox` covers nearly
  the full 160x136 playfield below the status bar, at every one of frames
  150/300/450/600/900/1200 against a frame-50 baseline). The world is
  genuinely scrolling; Mario's sprite just does not move on screen anymore
  once the lock engages.
- The earlier "the level never actually scrolls, `SCX` stays at `0`" claim
  from an earlier pass was a measurement artifact, not a real freeze.
  Sampling `0xFF43` (SCX) once per frame right after `pb.tick()` mostly
  reads `0`, because SML splits the screen with a mid-scanline STAT
  interrupt: the status bar rows render with `SCX = 0` and the playfield
  rows render with the real scroll value, and the register gets reset back
  to `0` for the next frame's status bar before a once-per-frame VBlank
  sample sees it. Sampling more frames caught the real value leaking
  through on some frames: cross-checking many single-frame reads of
  `0xFF43` during this run showed it briefly reading small then steadily
  larger nonzero values (`2`, `8`, `16`, `24`, ... up to `224` over several
  hundred frames) on the frames where the sample raced ahead of the reset,
  climbing at roughly the same 1 pixel/frame rate as Mario's saturated walk
  speed. That is consistent with real, continuous scrolling, not a stuck
  register.
- What this means for extraction: reading `SCX` once per frame right after
  `tick()` is not reliable for driving the tilemap-read formula once
  scrolling starts, since it is usually reading the HUD-row value, not the
  playfield value. `0xC20B` was checked as a possible cleaner source (it
  climbs by 1 every frame while walking right, with no resets) but it is
  **ruled out**: it also climbs by 1 every frame while Mario stands still
  holding Right against nothing (spawn, no movement at all), and it does
  not move at all if Right is never pressed. That is an input-hold-duration
  counter, unrelated to world position, not a scroll accumulator. The real
  per-frame scroll value still needs a way to read `SCX` at the moment the
  playfield rows are drawn rather than at VBlank, for example a `pyboy`
  `hook_register` breakpoint on the game's own SCX-write routine, or
  computing world position by dead reckoning from Mario's known,
  deterministic walk-speed curve (spawn position plus accumulated speed
  per frame) instead of reading scroll hardware at all. The dead-reckoning
  approach is what let this session read the pyramid's tile grid correctly
  past the point where sprite position freezes (see above); it should
  carry over to full-level stitching too, as long as the level's
  underlying tilemap buffer already holds the columns being dead-reckoned
  into (still unverified beyond column 19, the edge of the initially
  loaded screen).
- The old "an enemy might be blocking him" theory is also fully retired:
  OAM at the stuck screen position only ever showed Mario's own sprite
  (four entries, `x` 66-81), and there is no blockage to explain since
  nothing is actually blocked.

## First attempt at stitching: what breaks

The background tilemap buffer at `0x9800` is 32 columns wide (a ring
buffer), and at spawn it is not just the visible 20-column screen: reading
the full 32 columns shows real level data out to column 26 (ground, the
mountain silhouette, and a stray `129` tile past the pyramid), with columns
27-31 still blank (`44`, not streamed in yet). So the game preloads about
7 columns beyond what's on screen, not the whole level.

A first stitching attempt combined dead reckoning (`world_x = 81 + frames
since the camera lock engaged`, the same trick that worked for the pyramid
grid) with periodic full-32-column reads of the ring buffer, converting
each buffer column to a world column by picking whichever wraparound
(`buffer_col + 32*k`) landed closest to the dead-reckoned estimate. This
produced a plausible-looking combined map extending to world column 266,
but it is **wrong** past roughly column 48: Mario silently dies and
respawns at spawn partway through the run (confirmed directly: `0xC202`
snaps from `81` back to `50` at frame 338, and again at frame 678).

The cause is a hazard, not a pit. `0xC201` (Mario's Y position) stays
completely flat (`134`, unchanged) through the whole reset; falling into
a pit would show Y climbing for a while first, as `0xC208`/`0xC201` do
during a normal jump's descent (see `physics.md`). Instead, an OAM dump
one frame before the reset (`f = 336`) shows a sprite (OAM slot 20, tile
`144`, X-flip attribute set) sitting at `x = 83`, directly inside Mario's
own sprite bounding box (his four OAM entries span roughly `x = 73` to
`89` at that moment). That is consistent with an enemy walking into him
from the level's own scripted geometry, not a hole in the ground. The
script never presses jump or reacts to anything, so it walks into
whatever the first enemy on the path is.

Whatever the exact cause, dead reckoning has no way to detect the
respawn and keeps counting world position upward regardless, so the
restarted level's tilemap (the same real column 0-26 content) gets
stamped into the combined map under increasingly wrong, ever-larger
world-column numbers. The repeating pattern this produces (identical
32-column blocks recurring every ~32 columns out to 266) is exactly what
that bug looks like, not real level content.

What this means for the real stitching task: it needs either (a) a script
that actually plays past hazards (jumps over enemies and any real pits,
not just holds Right), or (b) explicit detection of the spawn-reset
signature (`0xC202` dropping back near its spawn value, or Y staying flat
while grounded drops) so a naive dead-reckoning run can at least discard
corrupted data after a death instead of silently mislabeling it.
Recording scroll/position per screen the way the plan describes needs one
of these; walking into the first hazard and extrapolating past it does
not work.

## Stitching: a working approach

`tools/stitch_level_1_1.py` replaces the dead-reckoning-plus-guessing
attempt above with something that does not need to guess at all. Every
frame, it directly compares each of the 32 ring-buffer columns against its
own value from the previous frame. The buffer only ever streams forward
(never rewritten with older data), so a slot's world-column identity
starts at its raw buffer index (true at spawn, confirmed against the
static opening-screen tilemap) and increases by exactly 32 every time
that slot's value actually changes. This needs no scroll register and no
position estimate for correctness, only continuous per-frame observation;
dead reckoning is kept only to report readable progress and to detect the
death/respawn from the earlier section (stops capturing the moment
`0xC202` leaves `81` after the camera lock, same signature as before).

Running it (holding Right only, no jump, so it dies at the same enemy as
before around frame 338) produces a confirmed map out to world column 63,
not just the ~26 columns available from the static spawn snapshot. Every
cell in that output is a real, directly observed value, either the spawn
snapshot itself or an actually-witnessed transition, never a guess. The
result is patchy past column 32: columns 32-58 still read blank (`44`) at
the point capture stopped, while columns 59-63 already show real ground
(tile `96`). That is not a claim about level geometry (a 27-tile gap
would be an unusual level design); it just means the game had streamed
those five particular buffer slots (aliasing to 59-63) with new content
by frame 338, but had not yet refreshed the other columns in that stretch
(they still held their spawn-time content, most of it blank/unloaded).
Reading further, real level geometry there needs a run that survives
longer, most likely a script that can react to the enemy that ends this
one around world column 48.

### Trying to jump past the enemy at column 48

Used `pyboy`'s `save_state`/`load_state` to snapshot right before the
hazard (frame 300) and replay many different jump timings from the same
point without re-simulating the whole run each time. First attempt looked
like a clean sweep across delay and hold length, but every single trial
died identically, including ones that should obviously have cleared a
normal Goomba-sized enemy. Checked the actual Y trajectory: `dy` stayed at
exactly `0` for the entire run in every trial, meaning Mario never left
the ground at all. The bug: pressing a button immediately after
`load_state()`, with no `tick()` in between, does not register. Fixed
structurally rather than left as a footnote to rediscover: `sml_boot.py`
now has `snapshot(pb)`/`restore(pb, state_bytes)` helpers, and `restore`
always ticks once before returning, so this cannot bite a future tool
built on top of it the way it bit this session.

With that fixed (confirmed via `grounded` actually leaving `1`), a sweep
of 15 delays x 10 hold lengths (150 combinations, jump triggered anywhere
from immediately to 42 frames after the snapshot, held 2 to 20 frames)
still died in every single case. This matches the same physics problem
found with the pyramid: at Mario's saturated running speed, a jump's
horizontal travel covers more ground per frame of height gained than a
one-tile-wide hazard allows, so there may be no jump at any height that
clears it while approaching at full speed.

Tried slowing down instead: releasing Right for a while before reaching
the hazard, then resuming (with or without a jump). This survived, and
**no jump was even needed**: releasing Right for at least ~50 frames and
then just continuing was enough on its own. That means the hazard is
almost certainly a moving enemy, not a fixed obstacle: slowing down
changes which frame Mario arrives at its position, so he just needs to
not be there at the same moment as the enemy, not clear it physically.

`tools/stitch_level_1_1.py` now walks in a "hold Right for `WALK_FRAMES`,
release for `RELEASE_FRAMES`, repeat" rhythm instead of holding Right the
whole time, as a general survival heuristic. `WALK_FRAMES=40,
RELEASE_FRAMES=100` reaches world column 64 before the next death (up
from 48 holding Right continuously), confirmed reproducible. This is
still a heuristic, not a fix: the run still eventually dies (just later),
and the exact rhythm was picked from a small grid search, not derived
from anything about the hazard itself. A script that actually detects
enemies (via OAM, the same way this session found the one at column 48)
and reacts to them specifically would be more robust than tuning a
fixed walk/pause rhythm further.

## Open work

- Pin the step/pyramid structure's solid tiles precisely (needs the
  sub-column-accurate probe described above; still not confirmed by
  direct collision, see the pyramid section).
- Extend `tools/stitch_level_1_1.py`'s reach further: the walk/pause
  rhythm is a heuristic that delays the next death, not a fix. A script
  that detects enemies via OAM and reacts to them specifically (or that
  tries many independent runs from further save-stated starting points
  and merges their confirmed transitions) would go further and more
  reliably than tuning `WALK_FRAMES`/`RELEASE_FRAMES` further.
- Once stitching covers enough of the level, convert the confirmed grid
  into `Level`/`Solids` and wire it in behind the existing ROM gating,
  replacing the placeholder demo level.
