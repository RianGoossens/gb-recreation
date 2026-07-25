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

  (Solidity is now resolved, see below; kept here as the working log of
  how it got there.) The earlier "two captures of the same cell
  disagreed" finding is superseded:
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
  structure rather than onto it.

  Tried that too: a pure standing jump (Right never pressed during the
  flight at all) has zero horizontal drift and lands exactly where it
  launched, so it can only ever land in front of or already past the
  structure, never on it, confirmed across six launch positions. Adding a
  short Right tap (1-3 frames) at takeoff to get a small controlled hop
  instead jumps discretely between whole columns (a 1-frame tap barely
  moves him at all, a 2-frame tap already carries 1-2 columns), with no
  granularity fine enough caught to land on the narrow step surfaces in
  between, across 72 more combinations of launch position, tap length, and
  jump height.

  **Resolved, and the other direction from the working assumption**: the
  pyramid is not solid. Built `tools/convert_level_1_1_to_level_format.py`
  to turn this opening screen into a real, loadable level for our own
  engine (see the stitching section below for why this exists at all),
  first marking the pyramid tiles solid per the "presumed by level-design
  consistency" note above. Loading that level and holding Right in our
  own engine, Mario does not walk at all: he oscillates in place right at
  his spawn column, immediately blocked, for the entire test. That
  directly contradicts every one of the dozens of real-emulator traces
  taken this session showing him walk freely from spawn (column 6) all
  the way to the camera lock (column 10) and beyond, with zero collision
  event ever observed along the way. Marking the pyramid tiles non-solid
  instead and rerunning the same test, Mario walks at a smooth, steady
  1 px/frame the whole way, matching the real game exactly. The
  level-design-consistency presumption was wrong: these tiles look like a
  solid staircase but do not collide like one, at least not against
  Mario's horizontal movement near the ground. `SOLID_TILES` in the
  conversion tool now only includes `96` and `97`; the pyramid tiles are
  treated as non-solid background, and the resulting level matches
  observed reality far better than the presumption did. This does not
  rule out the pyramid being solid from *above* (a fall onto its top
  surface, never tested), only that it does not act as a wall against
  the horizontal approach every real trace this session actually used.
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

`tools/stitch_level_1_1.py` first walked in a "hold Right for
`WALK_FRAMES`, release for `RELEASE_FRAMES`, repeat" rhythm instead of
holding Right the whole time, as a general survival heuristic.
`WALK_FRAMES=40, RELEASE_FRAMES=100` reached world column 64 before the
next death (up from 48 holding Right continuously), confirmed
reproducible. Still a heuristic, not a fix: the run still eventually
died, and the exact rhythm was picked from a small grid search, not
derived from anything about the hazard itself.

Replaced that with real OAM-based reaction: every frame, scan all 40
sprite slots except Mario's own (slots 3-6, confirmed fixed across every
OAM dump this session), release Right whenever another sprite is within
`DANGER_RADIUS` pixels of Mario's on-screen X, and resume once clear. A
radius/pause grid search found survival kept improving as the radius grew
(16 through 200px), plateauing at world column 77 for radius 120 and
above (further radius stopped helping, an actual hazard-detection
ceiling rather than a tuning artifact). This is real reacting to what is
on screen, not a blind rhythm, and reaches world position 77 versus the
rhythm's 64.

That is a genuine methodology improvement, but the two numbers are not a
strict apples-to-apples win: the *confirmed streamed tile data* the
reactive run actually captured only reached world column 60 (a 61-wide
map), slightly less than the rhythm-based run's 64-wide result. Reaching
a further world position does not automatically mean the ring buffer
streamed further data in by the time of death; the two clocks (Mario's
position and the buffer's own streaming) do not move in lockstep,
especially once the walk pattern isn't a steady rhythm. Both figures are
real and both are recorded so a future session comparing runs is not
misled by only one of them.

### Standing still doesn't save him, and jumping changes everything

Checked what the death at world column 77 actually looked like: Mario
sat stationary (Right released, `x` and `y` both flat) for over ten
frames with the same hazard sprite (tile `144`) persistently within
`DANGER_RADIUS`, and still died. That rules out the "wait for the enemy
to pass" theory from the first hazard: this enemy (or Mario's earlier
one, revisited) walks toward him regardless of what he does, so pausing
only delays contact, it does not avoid it.

Changed the reaction from "release Right and wait" to "release Right and
jump" (a stomp attempt, cooldown-limited so it does not spam) whenever a
sprite is within radius and Mario is grounded. The effect was dramatic:
the same run that died at column 77 pausing survived past world column
1880 jumping, over a 15000-frame run, without dying once. `radius=90`
with a stomp reaction was enough; wider radii did not change the
outcome.

That result needs an important caveat, found by actually running the
updated `stitch_level_1_1.py` with a 5000-frame cap: it reported "safely
captured up to world column ~630" (Mario's real dead-reckoned travel
distance) but the tracked tilemap output only reached world column 63,
barely two ring-buffer laps. The transition-watching method (a slot's
world-column identity increments by 32 only when its *value* changes)
has a real blind spot at this scale: long uniform terrain, most likely a
long flat stretch of ground tile `96` repeated across many laps,
produces identical bytes lap after lap, so nothing ever looks like a
change and the tracked identity silently stalls. This is not wrong data,
each entry that does appear is still a genuine direct observation, it is
missing data: the method has no way to know how many laps of unchanging
content it silently sat through. Worse, if the terrain ever did change
again after such a silent stretch, the next detected change would be
attributed to "previous lap + 32", undercounting the true world column
by an unknown multiple of 32. That did not happen in this particular
run (nothing was ever detected past column 63, so nothing got
mislabeled), but it is a real correctness risk for any future run where
a long uniform stretch is followed by real variation.

### Fixing the mislabeling risk

The undercounting risk above is now fixed. `0xC20C` (horizontal speed,
1/6-pixel units) is integrated every single frame into a running
subpixel position counter, real hardware state rather than an assumed
speed, so it keeps advancing correctly through jumps, pauses, and the
screen-position freeze alike. When a slot's tile value does change, its
new world column is now picked as the nearest multiple of 32 to that
precise position estimate, not a blind "+32 from whatever it was
labeled before". A stale label from an undetected repeat can no longer
propagate: each detected change gets its lap number from position
directly.

That fix itself needed a second correction: "nearest multiple of 32"
picked a small negative column (`-5`) for a high buffer index (27)
evaluated very early in the run, before Mario had traveled far, because
`-5` was numerically closer to the position estimate at that point than
the correct `27`. The level cannot have negative columns, and a slot's
world column cannot legitimately regress once established, so the
picker now rejects any candidate below the slot's current value (or 0)
and takes the next viable lap instead of whichever is numerically
closest.

Rerunning the 5000-frame survive-and-stitch pass with both fixes gives a
clean 141-wide map (world columns 0-140) with 45 of those columns
actually confirmed (the rest still blank, not yet streamed in during
this run): columns 0-26 solid ground matching the already-verified
static grid, a gap at 27-31 (still unloaded as of this run, consistent
with earlier findings), then a sparse scatter of confirmed columns
further out (32-33, 59-70, 103-106, 139-140) with everything between
still unobserved. No garbage or inconsistent values anywhere in the
output. The blind spot on genuinely uniform terrain remains by nature
(if a tile's value never changes there is nothing to detect, whichever
method is used), but the fix removes the risk of that blind spot
corrupting data that comes after it.

### Correction: that position estimate was itself wrong

The "141-wide map, 45 confirmed columns" result above is superseded.
`0xC20C` does not hold horizontal speed while airborne at all: checked
directly, it climbs unboundedly, one unit per frame, well past the
walking cap of 6, for as long as a jump lasts, some other counter
reusing the same address mid-flight (confirmed: `0xC20C` read 48 the
frame a jump started and climbed to 87 by 39 frames later, with
`0xC202` frozen at the camera lock the whole time). The 5000-frame run
above used the stomp-reaction from the previous section, which jumps
often, so its position estimate was integrating this bogus climbing
value on every one of those jumps and came out inflated. It reported
"safely captured up to world column ~720"; a sanity check against real
screenshots at four points across that same run showed almost no
visual change between three of them, which is what caught this.

Fixed by only trusting `0xC20C` while grounded (`0xC20A == 1`) and,
while airborne, continuing to add whatever speed was last read on the
ground instead (horizontal motion is not affected by jumping, per
`physics.md`, and was already directly observed holding at a steady 1
px/frame through a jump when Mario was already at max speed before
leaving the ground). Rerunning with this fix gives world column ~626
for the same 5000-frame budget, only a modest reduction from the buggy
720, not the large drop expected if most of that distance had been
jump-inflated noise.

That leaves a genuine open question, stated plainly rather than
resolved by assumption: is 626 close to Mario's real travel distance, or
is the level's terrain here uniform enough for a long stretch that a
screenshot taken at two different real positions would look nearly
identical anyway (which the tile data so far is consistent with: row 16
reads solid ground tile `96` almost everywhere it has any confirmed
value at all)? Both are plausible. The airborne-speed bug is confirmed
and fixed either way; whether the resulting distance figure is itself
trustworthy at this scale has not been independently cross-checked, and
should not be treated as settled.

### An attempted cross-check, and a confound it ran into

Tried settling this by recording the exact per-frame button sequence
the reactive walker produced over 1000 frames, then replaying that exact
sequence through our own engine on a flat, obstacle-free test level,
which has independently-pinned walking physics (see `physics.md`).
Comparing the two after those 1000 frames: the PyBoy position estimate
said world column 126, our engine's replay landed at column 105, a real
17% gap, not a rounding difference.

That gap is not clean evidence against the position estimate, though,
because the comparison has a confound: our engine's jump timing does not
match the cartridge's, so a jump in our engine does not necessarily last
the same number of frames as a jump in the real cartridge. Replaying a
button schedule recorded against the real game's jump timing through an
engine with different jump timing will diverge for that reason alone,
with no bug in the position estimate required to explain it.

Isolated a jump-free stretch to sidestep the confound entirely: frames
300-1000 of the same recorded run involved no danger and no jumping at
all (right held continuously, grounded the whole time), the pure case
`0xC20C` integration was already known to handle correctly. The position
estimate advanced from column 39 to column 126 over those 700 frames,
87 columns, which is 696 pixels, almost exactly 1 px/frame at saturated
speed, matching known physics precisely with no jump involved to
confound it. So the grounded case is well confirmed.

### Retried after the jump physics redesign: the gap narrowed

`step_motion` now implements the measured three-regime jump model instead
of a placeholder (see `physics.md` and `GRAND_MASTER_PLAN.md`'s backlog),
which shrinks the confound above without eliminating it. Recorded a fresh
500-frame button log the same way (react to nearby sprites, jump, same
`DANGER_RADIUS`/cooldown as `stitch_level_1_1.py`) and replayed it through
the updated engine on a flat test level.

The PyBoy position estimate reached column 58 (471.5px) after 500 frames.
Our engine's replay of the identical button sequence reached column 67
(541px), a 14.7% gap. Smaller than the original 1000-frame attempt's 17%,
but still real, consistent with the implemented jump being close to the
traced shape without matching it exactly (about 26 frames per jump versus
the cartridge's about 24, see `physics.md`): 170 of the 500 frames in this
run were spent airborne, and a few extra frames of continued rightward
walking per jump adds up over that many jump events. The airborne-freeze
position estimate itself was not re-examined this round; narrowing the
remaining ~15% gap further needs the jump timing itself narrowed first,
not another look at the position-tracking side.

## Measuring the scroll instead of estimating it, and what that overturned

Everything above this heading that reports a world position past roughly
column 78 is wrong. The corrections in the sections above were real
corrections, but they were all corrections to an estimate, and the estimate
itself had no ground truth to be checked against. This section replaces it
with a measurement and reports what that measurement says about the
earlier numbers.

### Two direct sources, both dead ends

`SCX`, the hardware register that shifts the background sideways, reads `0`
at every VBlank sample, for the reason already documented above (the status
bar split rewrites it mid-frame). Confirmed again here: 16 samples spread
across a 320-frame run, all `0`.

`tools/find_scroll_position.py` scans every byte and every 16-bit pair
(both byte orders) in WRAM, OAM and HRAM for something tracking the
expected scroll, using the same technique that found the speed register.
Three shapes were tried: a monotonic 16-bit counter, an 8-bit value whose
wrapped deltas accumulate to the expected scroll, and a tile-column counter
advancing once per 8 pixels. Nothing matches. There is no readable scroll
variable to find.

### The screen itself is the measurement

`tools/sml_scroll.py` cross-correlates consecutive playfield captures over
candidate shifts of 0 to 8 pixels and sums the winner. No model of Mario's
speed is involved, so nothing about jumps, pauses or the camera lock can
skew it.

Validated in `tools/measure_scroll.py` against two things pinned
independently:

- Between the camera lock and the first death, holding Right gives **91
  pixels of scroll over 92 frames**, which is the saturated 1 px/frame walk
  from `physics.md` to within one frame.
- Across a full run the per-frame shift is only ever 0 or 1, never 2 or
  more, and the worst frame-to-frame match score is 3.3 out of a 0-255
  scale. No scene ever cuts, so nothing is being counted as scroll that is
  really a level reload.

### What the measurement says about the old numbers

The reactive walker's claimed distances were fiction, and this is the part
worth remembering. Measured with the scroll tracker, the walker described
in the sections above covers **137 pixels in 5000 frames** and then sits
still. Screenshots at frame 500 and frame 2999 of that run are
byte-identical apart from the timer: Mario is jammed against a pipe around
world column 17 for the entire rest of the run. The reported figures of
world column 626, 720, and 1880 never happened.

Two separate faults produced them:

1. **`DANGER_RADIUS = 90` is over half the screen width.** Something was
   almost always within it, so Right was released almost permanently and
   Mario never built the speed to clear anything.
2. **Mario's WRAM bytes go stale on death rather than blank.** The death
   sequence freezes the level for about 150 frames, and through all of it
   `0xC202` keeps reading 81, `0xC20A` keeps reading grounded, and `0xC20C`
   keeps reading a saturated 6. So the death check (screen X leaving 81)
   does not fire, and the position integrated from those bytes keeps
   accumulating distance through a death that never moved anyone. Any tool
   reading Mario's state after a death is reading a corpse's last frame.

The frozen-screen counter in `ScrollTracker` is the honest signal for the
second one, and it is what `stitch_level_1_1.py` now stops on.

### The walker that actually travels

`tools/sml_walker.py` holds Right permanently and jumps on two triggers: a
sprite within a much smaller `DANGER_RADIUS` (28px, since a stomp needs
Mario nearly on top of the enemy anyway), and the measured scroll not
advancing for 12 frames while grounded, which is what walking into a pipe
looks like. The stuck trigger is only possible because the scroll is
measured; Mario's own screen X pins at 81 whether he is walking or jammed.

That walker reaches **world column 78 (545px of scroll) at frame 746**
before its first death, reproducibly, and the figure barely moves across a
grid of radius, stuck-threshold and cooldown values (53 to 78, with 78
hit by four of five settings). Past that death the level reloads, so
capture stops there.

### The stitched map, bounded by what the run supports

`tools/stitch_level_1_1.py` now takes its position from the tracker and
stops at the first death. `nearest_world_col` also refuses to label a slot
further ahead than `PRELOAD_MARGIN` (20 tiles) past Mario, since the game
streams a slot in shortly before he reaches it and a label beyond that
cannot be supported by the run. Before that bound the tool happily wrote
136 columns from a run that reached 78.

The result is a 96-column map (world columns 0-95, Mario's 78 plus the
preload margin), 786 cells directly observed and 542 inferred as repeats.

One thing in it looked like a bug and is not. Columns 40-66 reproduce
columns 0-26 tile for tile: the same pyramid, pipe, palm trees and question
block. Independent confirmation that this is real level content and not a
mislabeled lap: screenshots taken at camera column 0 and camera column 40
have a mean absolute pixel difference of 1.9 out of 255 and align best at a
shift of exactly 0. Two separate observations agree the level repeats there.
A screenshot at camera column 60 shows entirely new content (coins, brick
and question blocks, hills), so the level does progress; this stretch just
repeats first. Chunk-based level data reusing a block is the obvious
explanation, though the level format itself has not been pinned.

## The wall at world column 78

What kills the run there, seen by dumping the frames: Mario is on top of a
tall pillar with a flying enemy hovering at his own height, right at the
edge of a gap. Jumping into an enemy at your own height does not stomp it.
The scroll stops at 545px at frame 654, Mario's Y freezes at 83 while the
phase byte reads falling, and about 90 frames later the screen goes fully
static (the level reload).

No fixed policy gets past it. Reactive jumping, constant hopping at five
hold/cooldown combinations, and a grid over danger radius, stuck threshold
and cooldown all die in the same place.

### Searching instead of tuning

`stitch_level_1_1.py` now checkpoints and rewinds rather than relying on
one policy being good enough. Every 120 frames of real progress it saves a
`Checkpoint`: the emulator state, the scroll tracker, and the stitched map
together. All three have to move as one, or a rewind would leave the map
holding tiles from a future that no longer happens. On a death it restores
the last checkpoint and reseeds the walker (jump hold, cooldown, danger
radius, a chance of a spontaneous hop, and a chance of pausing, which is
the only action that can beat an enemy hovering at Mario's height).

Three things this needed that were not obvious:

- **Surviving is not progress.** A walker that stands still survives every
  segment forever. A segment only counts when the measured scroll also
  advances by at least 24 pixels.
- **Right has to be pressed again after a rewind.** PyBoy's button state is
  not part of a save state, so it has to be cleared before restoring, and
  then re-pressed. Forgetting the second half left every retry standing
  perfectly still, which the search scored as no progress, over 156 rewinds
  that all looked like a hazard nothing could get past.
- **Backing up one checkpoint is not enough**, because the segment leading
  in succeeds and puts Mario back in the same spot. The backup now doubles
  each time the same depth fails again, so the search genuinely explores
  different routes rather than retrying one.

With all three, the trace shows the search working as intended: it backs
off 1, then 2, then 4 checkpoints, finds genuinely different paths through
the earlier stretch (checkpoint 5 landing at 409px on one route and 415px
on another), and reaches the wall again by a different sequence.

### What the search says about the wall

Every route converges on scroll 545 and stops. That is a stronger statement
than a single fixed policy failing there: many different approaches, from
several different earlier states, all arrive and none continues. So it is
probably not a timing problem to be searched around.

The likely explanation, from the screenshots, is that the upper route
simply ends there and the level continues below, through the gap. That
would need a walker that can deliberately drop into a gap rather than one
whose only ideas are Right, jump and wait. Untested, and stated as the
leading hypothesis rather than a finding.

## The level data is in the ROM, and the format is mostly decoded

Prompted by Rian (issue 5): playing the game to discover the level caps at
however far a scripted run survives, and the geometry should be in the
cartridge. It is.

### Ruling out the easy possibilities first

`tools/find_level_data.py` searched the 64K ROM for the live tilemap in
every direct form. No row of it appears. No column appears. None of the 19
distinct 2x2 tile blocks on the opening screen appears. A sweep of all 256
constant offsets applied to the tile indices changes none of that. So the
level is not stored as tiles laid out the way they are drawn.

A density scan for the 21 non-filler tile IDs the opening screen uses put
the busiest 64-byte windows in bank 2 (0x0A9B0 at 47/64 bytes, 0x0ACC0 at
37/64). Hexdumping there showed an obvious repeating delimiter: `fe`, then
`53 40`, which are tiles 83 and 64, the two at the top of every column on
screen.

### The format

    0xFE                      start of a column
    (row << 4) | count        place `count` tiles starting at `row`
    <count tile bytes>        tile ids, top to bottom
    ...                       more runs
                              (until the next 0xFE)

Anything no run covers is the background filler, tile 44. `0xFE` cannot be
confused with a run header, since row 15 with a count of 14 would run off
the bottom of a 16-row column.

Worked example, the level's third column:

```
fe 02 53 40 b1 5e e2 60 61
   02          row 0, 2 tiles:  83, 64
      53 40
         b1    row 11, 1 tile:  94
            5e
               e2  row 14, 2 tiles: 96, 97
                  60 61
```

### Two bugs found by checking properly

Both are worth recording, because both looked correct for a while.

**Splitting the stream on every `0xFE` works for 47 columns and then
breaks**, since a tile id can itself be `0xFE`, and a `0xFE` inside a run is
data. Parsing strictly forward, consuming exactly what each run header asks
for, is correct.

**The start offset was wrong, and the level's own repeat is what hid it.**
0x0A2BD was pinned by finding the one tile run on the opening screen that
is unique in the whole ROM (`36 71 73`) and stepping back two records. That
gave 20 consecutive columns matching the game at spawn exactly, tile for
tile, which felt conclusive. It was not: this stretch of 1-1 repeats with a
period of exactly 40 columns, so a 20-column window fits in two places, and
the fit landed on the wrong one.

Caught by scoring against far more data. `tools/decode_level.py --verify`
now reads every world column the running game reveals (0 to 87, sampled off
the ring buffer as the camera moves) and scores every candidate offset:

| alignment | columns matched |
|-----------|-----------------|
| record k = column k + 40 | 44/48 (92%) |
| record k = column k | 21/88 (24%) |
| record k = column k + 41 | 7/47 (15%) |

So 0x0A2BD is world column **40**, not column 0. A 20-column agreement was
never enough evidence on a level that repeats every 40.

### What is still missing

World 1-1's first 40 columns are not immediately before 0x0A2BD (only 9
records chain back cleanly as a valid parse) and do not decode from anywhere
else in banks 2 or 3 either. So the level is assembled from segments, and
whatever points at them has not been found. That is the remaining work, and
it is worth far more than any further improvement to the walker.

### The scroll measurement, confirmed a third way

Rian's other point on issue 5 was that the level might not have repeated at
all, and that the camera simply had not scrolled yet. Checked directly, by
counting how many times the game rewrites a column of its own tilemap ring
buffer (it writes one column per column of scroll, and that is the game's
own bookkeeping, entirely independent of the pixel measurement):

| measured scroll | buffer column rewrites |
|-----------------|------------------------|
| 5 columns | 6 |
| 10 columns | 11 |
| 20 columns | 21 |
| 40 columns | 39 |

The camera genuinely scrolled, so the repeat at 40 columns is real level
content. The original evidence for it was weak though, and that criticism
stands: it rested on two screenshots looking alike, with none of the frames
in between examined. The intermediate frames (`tools/measure_scroll.py`
captures them) show the scene diverging to a mean absolute pixel difference
of about 20 out of 255 and coming back to 1.9 at exactly 320px. That, plus
the rewrite counts, is what the claim should have rested on.

## A real, loadable level from the opening screen

`tools/convert_level_1_1_to_level_format.py` turns the opening 20x18
screen into a plain-text level our own engine already knows how to load
(`Level::from_file`, see `docs/reference/level-format.md`): solid tiles
become `#`, everything else `.`, Mario's spawn becomes `M`. It writes to
`assets/extracted/level_1_1_opening.txt`, gitignored like the rest of
`assets/extracted/`, generated on demand from the verified ROM rather
than committed, since this data is both partial (one screen) and mixes
directly-confirmed and presumed tile classifications.

This is what caught the pyramid's solidity being wrong (see the pyramid
section above): loading the converted level in our own engine and
actually walking it surfaced a contradiction with real gameplay that
reading tile IDs and reasoning about them never would have. Building the
level and playing it, even headlessly, is now part of how this kind of
presumption gets checked, not just an eventual output of the extraction
work.

## Open work

- Confirm whether the pyramid is solid from above (a fall onto its top
  surface was never tested, only the horizontal approach every trace
  this session used; see the pyramid section for what already ruled out
  "solid against a horizontal approach").
- Get past world column 78. A flying enemy on a pillar at the edge of a
  gap, and a checkpoint-and-rewind search that explores many routes and has
  every one of them converge there (see the section above). The leading
  hypothesis is that the route continues below through the gap, which needs
  a walker that can deliberately drop rather than only go right and jump.
- Fill the gaps inside the captured range. Columns 27-31 still read blank
  in the stitched output even though Mario walked across them, so that is
  missing data rather than a pit (a 5-column pit would have killed him).
  The transition-watching method cannot distinguish "never streamed in"
  from "streamed in identical" for those.
- Reading the level out of ROM directly would sidestep the whole
  play-through-and-watch approach. The repeat between columns 0-26 and
  40-66 suggests chunk-based level data; finding that format would give
  the entire level at once instead of however far one run survives.
- Extend the level conversion (`tools/convert_level_1_1_to_level_format.py`,
  see the section above) past the opening screen, once the stitched
  width beyond it is trustworthy enough to be worth converting, and wire
  the result in behind the existing ROM gating, replacing the
  placeholder demo level.
