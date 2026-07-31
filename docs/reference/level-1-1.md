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

## The level data is in the ROM, and the format is decoded

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

### Finding the second half of the format

The column records were found first, and reading them alone produced two
wrong answers (below). What settled it was looking for the game's own read
position instead of pattern-matching the bytes.

`tools/find_level_pointer.py` boots to gameplay, walks Mario right, and
scans WRAM and HRAM for a 16-bit value inside the banked-ROM window
(0x4000-0x7FFF) that never decreases. Exactly one real candidate came back:

| address | at level start | after 68 columns | bytes per column |
|---------|----------------|------------------|------------------|
| 0xD010 | 0x6002 | 0x601D | 0.4 |

Bank 2 is mapped at CPU 0x4000, so 0x6002 is ROM offset 0x0A002, a few
hundred bytes below the column records. 0.4 bytes per column is far too slow
to be a column read (records average about 9 bytes), so 0xD010 is a
different table, most likely the object or enemy spawn list. Its value was
still the useful part: it said to hexdump around 0x0A000, and that region
holds a block of 16-bit pointers, all landing in 0x62xx-0x6Dxx, which is the
column-record region.

### The format

Two layers. A **column record** is a list of runs terminated by `0xFE`:

    (row << 4) | count        place `count` tiles starting at `row`
    <count tile bytes>        tile ids, top to bottom
    ...                       more runs
    0xFE                      end of column

with two special cases:

    count == 0                a full 16 rows, not an empty run
    0xFD <tile>               in place of the tile bytes, repeat one
                              tile for the whole run

Anything no run covers is the background filler, tile 44. Neither `0xFD` nor
`0xFE` ever appears as a real tile id in any of the 88 observed columns.

Worked example, world column 87:

```
02 53 40 | 37 fd f4 | e2 60 61 | fe
02          row 0, 2 tiles:  83, 64
   53 40
           37    row 3, 7 tiles, all 244
              fd f4
                     e2  row 14, 2 tiles: 96, 97
                        60 61
```

A **level** is a 0xFF-terminated list of 16-bit little-endian pointers to
those records. Each pointer starts one screen, and a screen is exactly 20
columns, the width of the display. World 1-1's list is at **0x0A198**:

    62BE 6200 62BE 6381 645F 62BE 650D 62BE 6200 6381 6200 62BE 65DE 66B5 67BB

15 screens, 300 columns, 2400 pixels, ending on the level's gate. Six of the
fifteen are reused. That reuse is the whole explanation for the "mystery" of
the opening screen: world columns 0-19 and columns 40-59 are byte-identical
because both are the screen at 0x62BE, drawn twice.

### Three bugs found by checking properly

All three looked correct for a while.

**Splitting the stream on every `0xFE` works for 47 columns and then
breaks.** The stated reason at the time (a tile id can itself be `0xFE`) was
wrong. The real cause is the `0xFD` repeat marker: skipping it makes the
parser consume the wrong number of bytes and desynchronise, after which a
`0xFE` shows up mid-run and looks like proof that tiles can be `0xFE`. With
`0xFD` handled, no tile in the level is ever `0xFD` or `0xFE`.

**The start offset was wrong twice, and the level's own repeat is what hid
it.** 0x0A2BD was pinned by finding the one tile run on the opening screen
that is unique in the whole ROM (`36 71 73`) and stepping back two records.
That gave 20 consecutive columns matching the game at spawn exactly, tile
for tile, which felt conclusive. It was not: 1-1's first screen is drawn
again at column 40, so a 20-column window fits in two places, and the fit
landed on the wrong one. A second attempt moved to 0x0A206 with a linear
read, which matched 66 of 67 columns only because 1-1's first five screens
happen to sit in near-linear order in the record pool. It had no way to
express a reused screen and could not produce columns 0-19 at all.

Caught by scoring against far more data. `tools/capture_columns.py` reads
every world column the running game reveals (0 to 87, off the ring buffer as
the camera moves) and `tools/decode_level.py --verify` scores every candidate
list start in the surrounding 192 bytes:

| screen list | columns matched | level length |
|-------------|-----------------|--------------|
| **0x0A198** | **88/88 (100%)** | 300 |
| 0x0A1AA | 40/88 (45%) | 120 |
| 0x0A1A6 | 40/88 (45%) | 160 |

One detail matters for that to work: **capture each column the first time it
appears.** Coins live in the background tilemap, so a column re-read after
Mario has walked through it is missing the ones he collected. Keeping later
reads costs about 4 points of match rate, which is enough noise to make a
wrong answer look defensible.

### What is still open

The whole of World 1-1 now comes out of the ROM with no emulator involved,
so the opening-screen gap is closed. Two smaller questions remain:

- **How the game picks a level's list.** No reference to 0x0A198 exists
  anywhere in the ROM, and no RAM byte or word tracks the list pointer or a
  screen index (both were scanned for directly). The game recomputes it. The
  four pointers immediately before 0x0A198 (`5F15 62BE 6817 68C7`) belong to
  something else and are unidentified; 0x5F15 is the only pointer in the
  region that falls outside the 0x62xx-0x6Dxx record pool.
- **Which tiles are solid**, which the format does not encode. That is still
  observation work (see the solidity sections above).

Two further 0xFF-terminated lists sit just after 1-1's, at 0x0A1B7 (17
screens, 340 columns) and 0x0A1DA (18 screens, 360 columns). Every pointer in
both decodes cleanly under the same rules, so the format is not specific to
1-1. Which levels they are has not been checked.

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
- Extend the level conversion (`tools/convert_level_1_1_to_level_format.py`,
  see the section above) past the opening screen to all 300 columns the ROM
  decode gives, and wire the result in behind the existing ROM gating,
  replacing the placeholder demo level. This is now the next real step.
- Identify how the game selects a level's screen list, so the other eleven
  levels can be extracted without hunting each list by hand.
- Classify the solid tiles across the full 300 columns. The level format
  carries tile ids only, so solidity stays an observation job.

The walker's wall at world column 78, and the gaps inside the stitched
range, no longer matter: those were limits of discovering the level by
playing it, and the ROM decode replaces that path entirely. The walker is
still used, but only to capture ground truth for checking a decode.

## Observing which tiles are solid

The level format carries tile ids and nothing else, so solidity is separate
observation work. `tools/classify_solid_tiles.py` runs the game and collects
two kinds of evidence per tile: frames where the tile was directly under
Mario's feet while grounded (support), and frames where Mario's own box was
inside the tile (overlap). A tile that supports him is solid; a tile he rests
inside is not.

Coordinates were calibrated rather than assumed. Mario is four sprites in OAM
slots 3-6, giving a 16x16 box, with OAM y biased by 16 and x by 8. Playfield
row `r` is drawn at screen y `(r + 2) * 8`. At spawn his box bottom is screen
y 128, which is playfield row 14, and row 14 there is tile 96.

### Two systematic errors, both found by disbelieving the first answer

The first run classified tile 99 (a pipe) and tile 232 (the fill inside a
raised platform) as non-solid, which is wrong for both.

**Mario's box laps into the wall he is blocked by.** He is 16 pixels wide, so
a box pressed flush against a wall still covers 1 or 2 pixels of the wall's
own tile column. Every wall that stopped him was being recorded as a wall he
walked through. Testing against a box inset 4 pixels on each side removed it,
and tile 99 dropped out of the classified set entirely.

**Landing clips him into the surface he lands on.** Jumping onto the raised
platform at world columns 61-64 puts him a few frames inside the 232 fill
below its 96 surface. A cell now has to stay occupied for 8 consecutive
frames before it counts. Nothing rests inside a solid.

Both errors had the same shape as the offset mistakes above: a rule that
looked right, checked against data that could not distinguish it from the
alternative.

### What is actually established

Over a run reaching world column 68:

| tile | verdict | stood on | walked through |
|------|---------|----------|----------------|
| 96 | solid | 577 | 0 |
| 44 | non-solid | 7 | 694 |
| 94 | non-solid | 1 | 100 |
| 49 | non-solid | 0 | 36 |
| 50 | non-solid | 0 | 12 |
| 51 | non-solid | 0 | 12 |
| 54 | non-solid | 0 | 16 |

Tile 96 separates perfectly: 577 frames of support and not one frame of
sustained overlap. The seven support frames on tile 44 (sky) are Mario at the
edge of a ledge, with half of him over the drop.

49, 50, 51, 54 and 94 are the pyramid tiles, so this reproduces the earlier
pyramid result by a completely different method.

**36 of the level's 43 tile ids remain unresolved.** 27 of them the walker
never reaches at all, stopping at world column 68. The other nine (52, 82,
97, 99, 112, 114, 129, 130, 232) it does reach, and comes back with zero
evidence either way: Mario is beside them repeatedly but neither stands on
them nor rests inside them. That is what a wall he walks up to and jumps over
looks like, and it is also what a decoration behind him looks like, so this
method cannot separate the two.

### A rule that looked like it fixed that, and did not

Tiles Mario is repeatedly up against but never enters are suggestive, so
"pressed against a lot, never entered, therefore solid" was tried. It labels
tiles 112 and 114 solid. Both are pyramid tiles, already shown non-solid by
two independent methods, so the rule is wrong and was removed. It survives as
a reported number only. Being next to a tile says nothing reliable about
whether it would stop him.

### The platform fill, stated precisely

Tile 232 fills rows 12-14 under a 96 surface at row 11, columns 61-64. Logging
which row Mario's feet rest on, per world column:

```
col 58: feet row 14           on tile 96
col 61: feet rows 10-14       (mid-jump)
col 62: feet rows 9-10        on tiles 44, 94
col 65: feet rows 9-11        on tiles 44, 94, 96
```

Past column 61 his feet are only ever on rows 9-11, on top of the platform.
He is never supported by 232 and never rests inside it. That says his route
goes over the top; it does not by itself prove 232 blocks him. Treating it as
non-solid would open a pit at column 61 that the original does not have,
since rows 14 and 15 there are 232 and 97 rather than ground, so the
converter treats fill under a solid surface as solid and that inference is
recorded in `docs/reference/faithfulness.md`.

## Correction: the shipped solidity was wrong

Rian played the cartridge and reported the extracted level had far too few
pipes, far too few blocks to collide with, and none of the holes you can fall
through. All three were right.

The reading being corrected was an allow-list of one solid tile (96) plus two
structural rules invented to cover the gaps: fill propagating downward from a
solid cell, and "a column with no solid cell stands on its lowest non-sky
tile". The second is the worst of it. Applied to a real pit, it invents a
floor. The level it produced had **zero** columns you could fall through.
World 1-1 has nine, at world columns 89-90, 138-139, 247-249 and 261-262.

### What replaced it

One rule: a tile is solid when `0x60 <= id <= 0xE8`.

Every tile the observation settled agrees with it, 7 for 7. The single
observed solid tile is 96, which is `0x60` exactly, and all six observed
non-solid tiles (44, 49, 50, 51, 54, 94) fall below it. Nothing contradicts
it.

It also removes the need for both invented rules, and the reason is worth
recording: each existed only because a solid tile was going unrecognised.
Fill propagation was needed because tile 232, the body of a raised platform,
was not solid; 232 is `0xE8`. The floor rule was needed because the level's
final ground band was not solid; those are tiles 142 and 143, `0x8E` and
`0x8F`. Both are solid under the range, and both hacks disappear.

### Why the upper bound is not just "everything above 0x60"

A plain `>= 0x60` leaves tile 244 (`0xF4`) solid, and the level then cannot be
finished: 244 blocks the route dead at world column 269, where it sits in a
gap in a climbable staircase. Sweeping the upper bound and walking each result
is what isolated it.

| solid range | result |
|-------------|--------|
| 0x60-0x61 | no floor at the exit, falls out of the level |
| 0x60-0x82 | same |
| 0x60-0x8F | walks to the exit gate |
| 0x60-0xE8 | walks to the exit gate |
| 0x60-0xFF | blocked at world column 269 |

244 being decoration is independently visible in its shape: 18 cells, as
isolated singles on alternating rows at column 277, and as a seven-tall bar
floating in open sky at column 87 with nothing beneath it.

### How far to trust this

Not very far. It is a fit to 7 observed tiles plus "the level can be
finished", not a read of the cartridge's collision test, which has not been
located in the ROM. 36 of the level's 43 tile ids are decided by the rule
alone. The upper bound is the weakest part: this level contains no tile
between `0xE8` and `0xF4`, so the data pins it only to somewhere inside that
gap.

Finding the game's own collision test is the real fix, and it is now the
open task. The lesson is the same one as the two wrong offsets: a rule that
fits the little data that was gathered is not the same as a rule that is
right, and shipping it inside a level nobody had played is what let it stand.

## The cartridge answers: collision reads the tilemap

The rule above is now replaced by a measured one. The open question was
where the game keeps the data its collision code tests against, and the
answer is that it tests the background tilemap in video RAM directly, the
same bytes that are on screen. There is no separate collision map: a search
of every byte from `0x8000` to `0xFFFF` for a copy of a visible column found
only the tilemap itself.

That is worth more than a located routine, because it makes the game
answerable. Writing a tile id into the tilemap in front of Mario changes
what he can walk through, so every one of the 256 ids can be put in front of
him and the game's own collision code asked about it. `probe_solidity.py`
does exactly that, twice per id:

* a fourteen-row wall four tile columns ahead, ground left untouched. Mario
  is camera locked at screen x 81, so a run ending short of that was stopped
  by the wall.
* the ground band ahead replaced by the id. Mario either walks across it or
  drops through.

Getting the probe to work took one correction. The first version measured
Mario's horizontal speed byte, which stays at its intended value while he is
pressed against a wall, so a blocked run and a free run both read as moving.
His screen x separates them cleanly: walls at ring columns 7, 8 and 9 stop
him at exactly 58, 66 and 74, eight pixels apart, against 81 for a free run.

### The result

| ids | behavior |
|-----|----------|
| `0x00`-`0x5F` | pass through, no support |
| `0x60`-`0xFF` | solid, with the exceptions below |
| `0x68`, `0x69`, `0x6A`, `0x7C` | hold Mario up, do not block him sideways |
| `0xF4` | passes through, no support |

So the rule is `id >= 0x60`, with `0xF4` carved out and four semi-solid ids.
It agrees with all 7 tiles the earlier walk-through observation had settled,
and it agrees with the playability sweep that found `0xF4` had to be
passable. The old `0x60 <= id <= 0xE8` produces the same grid for this level
(1-1 contains no id between `0xE8` and `0xF4`, and none of the four
semi-solid ids), so the extracted level file does not change. The reasoning
behind it did: `0xE8` was never a boundary, it was one exception guessed at
from the wrong end.

None of the semi-solid ids appear in World 1-1, so they are untested against
a real structure and our level format flattens them to solid. They will
matter in a later level.

## Where the extraction lives now

The decoding is Rust, in `src/assets/level.rs`, run by `sml extract-level`.
The Python that found the format (`decode_level.py`,
`convert_level_1_1_to_level_format.py`) is deleted, along with
`classify_solid_tiles.py`, which `probe_solidity.py` supersedes. The port was
checked by diffing the two tools' output on World 1-1: byte-identical, 300
columns, 922 solid cells, the same nine pits.

The solidity rule is three named constants there (`SOLID_FROM`, `PASSABLE`,
`SEMI_SOLID`) rather than a magic range, each cited to the probe.
`tests/rom_level_decode.rs` pins the decode against the ROM: the fifteen
screen pointers, the six that repeat drawing byte-identical columns, world
column 87 against its worked-example record, and the nine pits.

The PyBoy tools stay Python. They observe a running emulator, which is not
something the product does.

## Every column, from the running game

The decode was scored against 88 columns, because that is how far a reactive
walker got before dying. It is now scored against all 300, and matches every
one.

Two pokes get the real cartridge through a whole level. Collision reads the
background tilemap, so the terrain can be replaced: every column Mario has
not reached yet becomes tile 0, which is below `0x60` and therefore not
solid. That removes pits, pipes and walls in one go. It does not remove the
enemies, which still take all three lives inside a thousand frames, so his Y
position and the vertical phase byte are pinned each frame and he flies over
them. He reaches the exit of World 1-1 at frame 2299.

Capturing what scrolls past needs no camera tracking. The game writes a
column into the tilemap once, when it scrolls in at the right edge, about
every eight frames, and the ring column it writes to advances by one each
time, so any ring column that changes is fresh level data in world order.
`tools/run_through_levels.py` records it and flattens it 24 frames later.

Three things went wrong before the numbers came out right, all of them
found by logging the ring index rather than by reasoning about the code:

* The top two playfield rows are a skyline the game keeps redrawing.
  Flattening them made every column look freshly written on the next frame,
  and the capture cycled with period 19 instead of advancing.
* Flattening with the level's own background tile made plain columns of
  background-over-ground read as already flat, so they were skipped.
  Tile 0 never appears in level data and has no such ambiguity.
* Rings holding the opening screen were seeded but never flattened, so when
  the game later wrote an identical column into one, nothing changed and the
  column was missed. That cost exactly two columns, at 58 and 59, and showed
  up as a single alignment break with a perfect match either side of it.

The last one is worth keeping in mind: the result before that fix was 102 of
298 columns matching, which reads like a decode that is mostly wrong. It was
two dropped columns and a shifted comparison.

## The other levels

Nothing in the ROM holds World 1-1's screen list address, so there is no
table to follow to the other levels. `sml scan-levels` finds them by
structure instead, and there are exactly three in the level data, matching
World 1's three levels:

```
0x0A190  19 screens (11 unique)   <- contains World 1-1's list
         5F15 62BE 6817 68C7 62BE 6200 62BE 6381 645F 62BE ...
0x0A1B7  17 screens (11 unique)
         62BE 6817 68C7 69A6 6A61 6B23 6A61 6BF5 6CAB 6B23 ...
0x0A1DA  18 screens (13 unique)
         62BE 76CA 779F 6E2F 6F21 6FF2 6FF2 70FD 6F21 6E2F ...
```

World 1-1's verified list starts at 0x0A198, which is six bytes into the
first run, right after `62BE 6817 68C7`. The same three-pointer shape opens
the other two lists, which suggests each level's playable screens start six
bytes in, putting 1-2 at 0x0A1BD and 1-3 at 0x0A1E0. **That is a guess and
is not shipped.** The one list whose start is known was pinned by capturing
the running game, and the other two have not been.

Reaching 1-2 in the emulator is the open piece. The walkthrough above gets
through 1-1 and captures the "WORLD 1-2" title card, then stops producing
columns: Mario's pinned Y is chosen for 1-1, and 1-2 is built on floating
platforms over open sky (only 40% of its columns have anything solid on the
bottom two rows, against 92% for 1-1).


## 0xF4 is a coin

Called decoration in two places before this, on the strength of its shape:
18 cells in World 1-1, isolated singles on alternating rows near the end and
a seven-tall bar floating in open sky with nothing beneath it. Those are
coins. The seven-tall bar is a coin tower.

Coins are drawn into the background tilemap rather than spawned from the
object table, which is the same fact that made `capture_columns.py` record
each column the first time it appeared: a column re-read after Mario walked
through it is missing the ones he took. `tools/find_coin_tile.py` uses that
directly. It flies Mario through the level at a sweep of heights, removing
only the solid tiles from his path so the coins survive, and records which
tilemap cell changed on any frame the coin counter moved. One answer, 13
times over: `244 -> 44`.

The evidence was already sitting in the solidity probe's output and went
unread. Probing tile 244 as a wall printed its four cells afterwards as
`[244, 244, 244, 44]`: Mario had collected one on his way past.

So the level's coins need no object table. They are in the geometry, and
`sml extract-level` writes them as `C`.
