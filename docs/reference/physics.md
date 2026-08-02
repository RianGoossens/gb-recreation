# Reference notes: movement physics

Working notes for pinning Mario's movement constants to the cartridge. Values
marked "observed" were read directly from the running, verified ROM. Values
marked "to pin" are still provisional placeholders.

## Method: watch the speed register, don't read the disassembly

Same approach as the title-screen ROM offsets (see `title-screen.md`): observe
the real game and let the data tell you where it lives, rather than starting
from a disassembly address.

`tools/find_mario_speed.py` boots the verified ROM, starts World 1-1, holds
Right, and snapshots all of WRAM (`0xC000`-`0xDFFF`) every frame. A speed
register has a distinctive shape: it starts at 0, climbs by a fixed step each
frame while a direction is held, saturates at a fixed maximum, then falls by a
step each frame once released, holding at 0 rather than going negative. No
other kind of WRAM byte behaves that way, so scanning every address for the
shape finds it without needing to know its name or address ahead of time.

## Observed: horizontal walking speed

Address `0xC20C` is Mario's horizontal speed:

- Starts at 0, climbs by **1 per frame** while Right is held (Left mirrors it
  negatively).
- Saturates at **6** while walking.
- Holding B alongside Right does not change this: checked directly, `0xC20C`
  caps at 6 whether B is held or not, over 400 frames continuously. There is
  no separate run/dash speed to model at the WRAM level; whatever B does in
  the original (shooting a superball, per `faithfulness.md`), it is not a
  horizontal speed modifier.
- Falls by **1 per frame** once released, and holds at 0 rather than
  overshooting negative.

Correlating this against Mario's on-screen X position (`0xC202`, read only
before the camera starts scrolling, so it is still a direct pixel count) shows
one whole pixel of movement for every 6 units the speed register
accumulates: at speed 1-5 the sprite advances roughly every other frame, and
once speed saturates at 6 it advances exactly 1 pixel every frame. So the
original's speed unit is 1/6 pixel per frame, and a saturated speed of 6 is
exactly 1 pixel per frame.

## Converting to our subpixel scale

Our engine uses `SUBPIXEL = 256` (`src/core/entity.rs`), so 1 pixel is 256 of
our units. Mapping the original's 1/6-pixel unit onto that scale:

| Constant | Original | Converted (256 / 6, rounded) |
|----------|----------|-------------------------------|
| `WALK_ACCEL` | 1 unit/frame | 43 |
| `FRICTION` | 1 unit/frame | 43 |
| `MAX_WALK_SPEED` | 6 units (1 px/frame) | 256 |

Accel and friction are equal and symmetric in the original (same 6-frame ramp
up and down), which is why our two constants are now equal too; the earlier
placeholder values had friction weaker than acceleration, which does not
match the observed behavior.

These three live in `src/core/physics.rs`, cited there, and pinned by
`physics_constants_are_pinned` in the same file so a future change is a
deliberate act.

Worth noting given a later discovery in this file's own history (see
`0xC20C` not holding horizontal speed while airborne, in
`level-1-1.md`'s stitching section): `find_mario_speed.py` never presses
jump at all, only holds and releases Right, so these three constants
were measured entirely from grounded behavior and are not affected by
that airborne quirk. Checked directly rather than assumed.

## Still to pin

This section is the working log, kept in the order it was actually found.
Gravity, jump velocity, and jump cut got pinned and implemented further
down ("Implemented, with one more correction along the way"), and stomp
bounce got pinned after that. Nothing in this document is still open.

At the time this section was written, gravity, jump velocity, jump cut, and
stomp bounce were all still provisional placeholders. Forcing a jump (hold A, release, let Mario land) and diffing
WRAM the same way found Mario's vertical state cluster, right next to the
horizontal one:

| Address | Behavior observed |
|---------|--------------------|
| `0xC201` | Mario's Y pixel position. Decreases while rising, increases while falling, flat while grounded. |
| `0xC20A` | Grounded flag: `1` while touching solid ground, `0` while airborne. Flips to `0` the frame a jump starts and back to `1` the frame Mario lands. |
| `0xC207` | Vertical phase: `0` grounded, `1` rising, `2` falling. |
| `0xC208` | Climbs from 0 during the rise, peaks at the apex, falls back to 0 during descent. Looks like a height-above-ground counter rather than a raw speed; not yet interpreted with confidence. |

`0xC201`'s per-frame deltas during a jump are not a clean constant step
(unlike the horizontal speed register), so gravity and jump velocity are not
pinned yet from this alone; a byte-level Y position loses the subpixel detail
that would show the true per-frame acceleration. The grounded flag
(`0xC20A`) is clean enough to use directly for the tile-collision
classification subtask in `level-1-1.md` without waiting on gravity to be
pinned.

Checked again for a subpixel companion register (the same relationship
`0xC20C` has to `0xC202` for horizontal movement): a WRAM scan for a byte
whose delta is itself roughly constant during a jump (a true velocity
signature, one derivative up from a position) turned up `0xC00C` and
`0xC010`, but they turned out to be exact mirrors of each other and of
the already-documented `0xC208` shape: pinned at a sentinel value
(`-128`) while grounded, jumping to a peak on takeoff, easing down to a
minimum at the jump's apex, then back up to the peak at landing. That is
the same "height above ground" signal already noted for `0xC208`, not a
subpixel velocity accumulator. No new lead found; gravity and jump
velocity are still unpinned.

Tried a different angle: instead of needing per-frame subpixel velocity,
measure aggregate jump kinematics (whole-pixel peak height, whole-frame
timing) and solve for gravity/initial velocity algebraically, since
those two are measurable precisely even from whole-pixel data. Holding
`A` for 12 or more frames gives an identical, reproducible result every
time: a 24px peak, reached 11 frames after takeoff, landing 24 frames
after takeoff (so 13 frames falling). Shorter holds are not simply a
smaller version of the same arc: holding 8 frames gives a *later* apex
(frame 16) than holding 12+ frames (frame 11) despite a similar peak
height (23px vs 24px), which a simple "hold cuts the rise short" model
does not explain.

That rules out treating this as solved, but one of the two tangled
possibilities is now settled: the grounded flag does not lag Y position.
The full per-frame trace for the max-hold case shows the trajectory
overshoot the ground by 1px at frame 23 (`y = 135`, one past the
ground's `134`), then land cleanly at frame 24 (`y = 134`,
`grounded = 1`) the very next frame, exactly the ordinary discretization
of a falling object crossing a threshold between two frames, not a
multi-frame detection delay. So the 11-frame rise versus 13-frame fall
is real timing, not a measurement artifact: this cartridge's jump
genuinely takes longer to fall than it took to rise, for whatever reason
(likely a faster-fall convention, but not confirmed as such). That still
leaves the short-hold anomaly (an 8-frame hold's apex arriving later
than a 12-frame hold's) unexplained, and deriving new `GRAVITY`,
`JUMP_VELOCITY`, or `JUMP_CUT` values from partial data was deliberately
not done: the underlying model isn't understood well enough yet to trust
a derived number over the existing, tested placeholder. Recorded as
narrowed data for the next attempt, not as a fix.

### A real lead: this looks like two constant-velocity phases, not one accelerating curve

The previous attempts assumed the jump is a single continuously-accelerating
parabola (constant gravity, like our engine models it) and tried to fit that
shape to the max-hold trace. It does not fit: a least-squares parabola over
the whole 24-frame trace leaves residuals up to 4px with a systematic pattern
(the fit under-curves in the middle), meaning the real curve has less
curvature there than a single parabola allows.

Splitting the same trace at the apex (frame 11) and fitting each half
separately tells a different story. The rise (frames 0-11) fits a **straight
line** far better than a parabola: slope -2.13 px/frame, max residual 1.39px,
versus a quadratic fit whose curvature term is nearly zero (`g = 2a ~= 0.04`,
indistinguishable from no acceleration at all). The fall (frames 11-24) is
the same shape the other way: slope +1.99 px/frame, residuals dominated by
what looks like subpixel rounding noise rather than curvature.

This was not just a fit artifact. Recording `0xC207` (the already-documented
vertical phase byte: 0 grounded, 1 rising, 2 falling) alongside Y in the same
run shows it flips from 1 to 2 at exactly frame 12, the same frame the Y
delta changes sign. The game's own internal state agrees with what the
curve-fitting found independently:

```
frame  y    dy  grounded  phase
   10  112  -2  0         1
   11  110  -2  0         1
   12  111  +1  0         2   <- phase flips here, delta flips sign here
   13  113  +2  0         2
```

So the model is closer to two constant vertical speeds (roughly 2.2 px/frame
up, 2.1 px/frame down) with a hard switch at the apex, than to one continuous
`GRAVITY` acceleration applied every frame the way `src/core/physics.rs`
currently does it. That would also explain why the rise/fall frame counts
differ (11 vs 13) despite similar total distance: the two phases have
slightly different constant speeds, not the same acceleration integrated
over an asymmetric arc.

### Refinement: the held rise is not velocity-triggered at all

Checked directly whether "constant" really means constant: held A for a full
50 frames without ever releasing, well past the usual apex. The apex still
lands at frame 12, identical to every other max-hold trial. That rules out
one reading of "held sustains the rise": it does not sustain it
indefinitely, something ends it at a fixed frame regardless of the button.

Fitting the held-rise segment as a quadratic (excluding the frame that has
already turned) gives a small nonzero curvature, `accel(2a) = 0.0385
px/frame^2`, consistent with the very flat linear fit found earlier, not a
new number. But following that acceleration's own implied velocity to zero
(`-b / 2a` from the fit) predicts roughly **61 frames** to naturally reach
zero speed, nowhere near the observed 12-frame apex. So whatever ends the
held rise at frame 12, it is not this small deceleration finally catching
up to the velocity. It looks like a fixed maximum rise duration (a frame
counter, not a velocity threshold), independent of the near-zero
deceleration also present during the hold. Releasing early still cuts in
before that counter runs out, triggering the separate, much stronger
regime-2 deceleration documented above. Any future engine implementation
needs both pieces: a small drift during the held rise, and a hard duration
cap on top of it, not the fixed-duration cap alone and not the drift alone.

### The short-hold anomaly resolves: releasing A early starts real deceleration, it does not cut the rise short

The previous attempt's open question was why an 8-frame hold gives a *later*
apex (frame 16) than a 12+ frame hold (frame 11), which a simple "holding
cuts the rise short" model cannot explain. Recording the full per-frame
trace with early releases (hold 3, 5, and 8 frames, same method as above)
answers it:

```
hold=3:  frame  0  1  2  3  4  5  6  7  8  9 10 11 12 13 14 15(phase->2)
         dy    +0 -3 -3 -2 +0 -2 -1 -1 -1 +0 -1 +0 -1 +0 +0 +0
hold=8:  frame  0  1  2  3  4  5  6  7  8  9 10 11 12 13 14 15 16(phase->2)
         dy    +0 -3 -3 -2 +0 -4 -2 -2 -2 -1 -1 -1 -1 -1 +0 -1 +1
```

While A is held and Mario is rising, upward speed holds roughly steady
(matching the max-hold trace's constant ~2.1-2.3 px/frame). The moment A is
released, the speed does not stay put and does not simply stop: it decays
step by step toward zero over several more frames (`-2, -1, -1, -1, 0, -1,
0, -1, 0, 0` for the hold=3 case), and only once it actually reaches zero
does `0xC207` flip to falling (phase 2). Releasing earlier means more frames
of decay before reaching zero, hence a later apex, exactly the anomaly the
previous attempt found and could not explain. This is a real deceleration
being applied once the button is not held, not a fixed-length rise being
truncated.

The fall phase (`0xC207 == 2`) also is not the constant speed the single
max-hold trace suggested; that trace's linear fit was flattered by having
only 12 noisy samples. All four traces (hold 3, 5, 8, 15) show the same
shape once falling: speed starts near 0 right after the phase flip and
climbs step by step to roughly 3 px/frame by landing, a real accelerating
fall, consistent across every hold length tried.

### Combined picture

Three distinct regimes make up the jump, instead of a single `GRAVITY`
constant:

1. **Rising, A held**: near-constant upward speed (~2.1-2.3 px/frame), with
   only a tiny residual deceleration (~0.04 px/frame², see the refinement
   above). That drift is too small to be what ends the rise: this phase
   ends at a fixed ~12-frame cap regardless of how long A is held, not when
   the drift finally decays velocity to zero (which its own fit puts at
   roughly 61 frames, never reached in practice).
2. **Rising, A released (or otherwise not held)**: upward speed decays
   toward zero over several frames, a real deceleration.
3. **Falling**: downward speed accelerates from ~0 at the apex to ~3
   px/frame by landing, consistently across every hold length tested.

Checked directly whether regimes 2 and 3 share one acceleration magnitude
(fitting a quadratic separately to each phase's segment, for the hold=3,
5, and 8 traces): they do not. Regime 2's deceleration fits consistently
around **0.11-0.12 px/frame²** across all three trials (max residual under
0.8px), while regime 3's fall acceleration fits consistently around
**0.14-0.17 px/frame²** (max residual under 1.6px, noisier but still a
clean upward trend). The fall is reliably faster to accelerate than the
released rise is to decelerate, by roughly 30-50% in every trial. Two
separate constants, not one shared `GRAVITY` reused in both directions.

This is a real structural finding.

### Implemented, with one more correction along the way

`step_motion` (`src/core/physics.rs`) now models the three regimes directly,
via `apply_vertical_accel`, instead of one `GRAVITY` added every airborne
frame. Simulating the new model end to end (a scripted max-hold jump over a
flat floor, checked outside the test suite before committing) caught one
more thing this document had wrong: routing the frame-cap expiry through
`JUMP_CUT`, the same as an early release, produced a jump that rose to about
40px and took roughly 17 extra frames to decelerate before falling, nothing
like the real 24px/24-frame arc. The real trace already showed why, and it
had been read past the first time: at the cap (frame 12 in every held-past
trial), the delta flips from `-2` straight to `+1` in one frame, with no
intermediate values the way an early release shows. That is an abrupt
reset, not a slow decay reaching zero. The engine now models the
cap-expiry-while-held case as a direct `vy = 0`, separately from the
early-release case (which still decelerates gradually via `JUMP_CUT`), and
simulating that reproduces the real arc closely: about 26px peak (real:
24-25px) landing around frame 26 (real: 24).

A second look at that same reset caught one more frame to save. The reset
frame (frame 12) was implemented as `vy = 0` and nothing else for that
frame, meaning that frame contributes zero movement, gravity only starting
to apply from frame 13 onward. But the traced transition frame already
shows `+1`, a small amount of real falling motion, not a frame spent
sitting still at zero velocity. Fixed by applying the first frame of
`GRAVITY` in the same step as the reset instead of waiting a frame: the
simulated landing moved from frame 26 to frame 25, one frame closer to the
real 24. The remaining single-frame gap was left alone rather than chased
further with numbers this data cannot cleanly support.

`GRAVITY` itself also changed from the value first written here. The
per-trial quadratic fits for the fall phase had residuals up to 1.6px, the
noisiest of the fits in this document, and simulating with that value
undershot the real fall considerably (an 18-frame fall instead of ~13).
Total distance over total duration (about 25px over about 13 frames from
rest at the apex) is a sturdier measure for a noisy short segment than
differentiating it twice, and solving `d = 0.5 g t^2` gives **0.296
px/frame²** (76 in our subpixel units), which is what shipped. `JUMP_CUT`
kept its quadratic-fit value (consistent within 0.008 px/frame² across
three trials, a tighter fit than the fall phase, more trustworthy as is).

None of this is pixel-perfect: it is close to the traced shape, checked by
actually running it, not derived and shipped untested. `MAX_FALL_SPEED`
(the tunneling-prevention cap, separate from any of this) was not
re-examined and is still the old placeholder.

## Stomp bounce: pinned on the third attempt

Measured. `tools/measure_stomp_bounce.py` lands 61 real stomps on the first
World 1-1 Chibibo and reads the arc off each one. The two failed attempts
that came before it are kept below, because what fixed them is the useful
part.

Two changes made it work:

**Ground truth is Mario's own phase byte, not an OAM count.** A stomp is the
only thing in this stretch of 1-1 that turns a fall back into a rise without
touching the ground, so `0xC207` going 2 -> 1 while `0xC20A` still says
airborne is the signature. Both earlier attempts keyed off enemies leaving
OAM, which is why both kept "finding" stomps that were really off-screen
despawns.

An enemy-vanish cross-check was added on top of the phase flip at first, and
it rejected every single hit. That looked like the phase flips were false
too. They were not: rendering the frames around one hit
(`tools/stomp_frames.py`) shows Mario landing on the Chibibo, the Chibibo
disappearing, the 100-point popup appearing, and the score going 0 to 100.
The cross-check was wrong, not the signal: the popup adds two sprites, so
the on-screen sprite count goes *up* by one across a stomp. The cross-check
was dropped and the rendered frames stand as the confirmation instead.

**The approach is a save-state sweep, not a live reaction.** Walk right until
an enemy is actually on screen ahead of Mario, snapshot there, then replay
that one frame with every (wait, hold) combination. 61 of 280 combinations
connect.

That sweep also turned up a trap worth writing down: **PyBoy's button state
is not part of a save state**, so it survives `load_state` and leaks from one
trial into the next. The first version of the sweep found a hit at
`wait=10 hold=16`, and re-running exactly that one combination on its own
produced an ordinary jump with no stomp at all, because in the sweep the
previous trial had left Right held across the restore and in the single run
it had not. Releasing every button before restoring is what made trials
reproducible. `sml_boot.restore` already documents a related trap (a button
pressed with no tick after loading does not register); this is a second one
on the same seam.

### The measured arc

Consistent across all 61 stomps, grouped by how long A was held:

| A still held at the stomp | rise | frames to apex | 2d/t |
|---------------------------|------|----------------|------|
| no (6 hold lengths, n=53) | 8px | ~12.4 | 1.30 px/frame |
| yes (n=8) | 9px | 13.5 | 1.34 px/frame |

Holding the jump button through a stomp is worth 1px. That is the important
structural finding, and it is not what the engine did: a stomp bounce is
**not** the held-rise regime. A normal jump held to its cap gets a near-flat
12-frame rise; a stomp bounce decays right away, the same way a released
jump does, whether or not A is down. `Mario` now carries a `bouncing` flag
so `apply_vertical_accel` routes a bounce through `JUMP_CUT` in both cases.
Without it the held case collapsed to nothing, since `rise_frames` is
already spent by the jump that got Mario airborne, so the held branch would
have reset `vy` to 0 on the next frame.

### The value that shipped, and why it is not 2d/t

Same lesson as `GRAVITY` further up: simulating before committing changed
the number. `2d/t` reads 1.30 px/frame (333 in subpixels), but that value
carries its own implied deceleration, `2d/t² = 26.9`. The engine decelerates
a bounce with `JUMP_CUT` (29), measured separately and from tighter data.
Stepping 333 against 29 reaches only 7px, not 8. **360** is the speed that
reproduces the traced 8px arc with the deceleration actually applied, and
that is what shipped. Checked end to end through `Game`'s real stomp path
(a scripted drop onto a Goomba, run and read, then deleted): 8px rise, with
and without A held, matching the cartridge's 8px and 9px.

## Stomp bounce: the two attempts that failed first

Tried the same observation approach on `STOMP_BOUNCE`: script Mario to jump
near the first World 1-1 enemy and read `0xC201`/`0xC207` around the moment
of a kill, the same way the jump traces above were read. Detecting a kill by
counting on-screen OAM slots outside Mario's own (confirmed fixed at slots
3-6) works once filtered to on-screen Y values (`0 < y < 160`; an
unfiltered count picks up stale data in unused slots and reports a constant
20 "enemies" the whole run). But every kill this produced, across several
trigger-distance and jump-hold combinations, sits inside a completely smooth
jump arc with no anomaly in Y at the kill frame: no sudden reversal, no
break in the descent, just an ordinary jump landing normally a dozen-plus
frames later. That means the script is not actually landing a stomp, it is
coincidentally jumping near a moment an enemy leaves OAM for some other
reason (an off-screen despawn, most likely). Scripting a jump that reliably
lands **on top of** a specific enemy, rather than just near one, needs
tighter control over the horizontal approach distance and jump timing than
this attempt used. Left open rather than forcing a bounce number out of
data that does not show one.

A second attempt tried much closer trigger distances (4-14px instead of
10-30px) with short taps (2-6 frames) instead of long holds, on the theory
that the earlier attempts' jumps were arcing clear over the enemy instead
of landing on it, plus an explicit check for the real signature of a
stomp (Y reversing from falling to rising within a frame or two of the
kill) rather than just an OAM count dropping. Every kill across 24
combinations still shows no reversal: most sit at `y=134, grounded`, Y not
moving at all around the kill frame, meaning these are not even mid-air
events, let alone stomps. Two attempts now agree the reactive
jump-when-near approach is not landing real contact with the enemy at
all, in two different ways (arcing over it; not really being airborne at
the moment of the "kill"). A third approach would need a ground-truth
signal independent of OAM presence, such as the score counter (not yet
pinned, see `faithfulness.md`) incrementing on a real stomp, rather than
another tuning pass on jump distance and timing.

## Correction: every jump trace above was a standing jump

Two things in this document were wrong together, and World 1-1's own
geometry is what showed it. The pillar at columns 78 and 79 stands four
tiles, 32 pixels, above the ground on either side of it, and the level is
verified column for column against the running cartridge, so getting past it
is something Mario can do. A 24 pixel jump cannot.

`tools/measure_jump_height.py` takes one save state and jumps from it with
and without a direction held, reading the peak off `0xC201`. The standing
column reproduces this document exactly, which is what makes the rest of the
table worth reading:

| held during the flight | peak rise |
| --- | --- |
| nothing, from a standstill | 24 px |
| a direction | 33 px |
| a direction and B, at speed | 41 px |

A one-frame run-up is enough for 33, with the speed register `0xC20C` still
reading 0 at takeoff, so this is not jump velocity scaling with speed the
way Super Mario Bros does it. The per-frame deltas say what it is. The
standing and moving arcs are identical for twelve frames and then part:

```
standing: 0 -3 -3  0 -4 -2 -2 -2 -2 -2 -2 -2 +1 +2 +2 +2 +2  0 +2 +4
moving:   0 -3 -3  0 -4 -2 -2 -2 -2 -2 -2 -2 -1 -1 -1 -1 -1  0 -1 -1
```

The abrupt reset at the cap is what a *standing* jump does. Moving, the rise
carries on at about a pixel a frame for another twelve frames, 9 pixels more,
which is `GLIDE_VELOCITY` and `GLIDE_FRAMES` in the engine. B adds a further
8 pixels on top at speed, which the engine does not reproduce: `0xC20C` caps
at 6 with or without B, so there is no run speed here to hang it on, and what
B changes about a jump is not measured.

`JUMP_CUT` was wrong in the other direction. At 29, from a quadratic fitted
to the released segment, a 3-frame hold rose 28 px and an 8-frame hold 24,
so pressing the button longer got Mario *less* height, and releasing at
frame 12 gave 43 px, the highest jump the engine had. Stepping the engine's
own arc at candidate values reproduces both traced peaks (15 px and 24 px)
exactly at **76**. That is `GRAVITY`, and they are still separate constants:
one pair of traces agreeing on a number is not enough to claim the cartridge
runs a released rise through the same code that makes him fall. The stomp
bounce kept the old value as `BOUNCE_CUT`, since its own trace measured it.

The lesson is the one already in CLAUDE.md, arriving from a new direction: a
fit whose residuals are systematic should be simulated end to end before it
is trusted, and the level geometry is a measurement too.

## Ceilings, and what a jump under one hits

Our jump is capped when any part of Mario's 11 pixel width is under a solid
tile. Nothing had measured that, and World 1-3's opening is where it bites:
a block at row 10 spans columns 7 to 11, a two-tile wall stands at columns 13
and 14, and walking the floor at row 14 takes Mario under the block and up
against the wall with our engine capping his rise 4 pixels short of the
wall's top. The pocket is a dead end for us.

`tools/probe_ceiling_cap.py` reaches World 1-3, stops the flatten-and-fly the
frame the level opens, and runs the same thing on the cartridge. It prints the
tilemap it reads back, so a flattened opening screen would be visible rather
than silent; this one matches the decode column for column. Two controls: walking right without jumping stays at x 88 forever,
and a free jump at column 3 rises 33 pixels.

**The cartridge gets him out.** Holding right at x 88 and tapping A, his Y byte
runs 134 to 101, a rise of 33 with his feet ending at 95, above the block's own
top at 96. He does not land on it; he comes back down through it.

That does not settle it as "the ceiling test is narrower than he is". A
standing jump from the same spot is capped. Sweeping a
standing jump a pixel at a time across the block, from x 40 to 111:

```
x   40..61  62..69  70..77  78..85  86..93  94..105  106..109
rise    33      16      12      16      12       16        33
```

The block is columns 7 to 11, x 56 to 95. The free rise is 33 either side of
it, so the sweep is wide enough to hold the whole window and both edges are
inside it. Under the block the rise alternates between 12 and 16 on an 8 pixel
cadence offset by 6, and the 16s carry on 10 pixels past the block's right
edge. Neither number is 33, so a standing jump at x 88 is stopped at the same
place ours is.

Two readings fitted that: a head test that follows Mario's leading edge, and a
rise fast enough to cross the tile between checks. Pressing each direction at
the same spot with his x pinned separates them, since only the first can care
which way he is pressing:

```
                                     still  left  right
open sky at x 40 (control)              33    33     33
x 88, under the block's right end       12    12     33
x 48, under the block's left end        33    33     33
```

Left and right give the same speed and opposite answers, so speed is not what
does it and the rise is not crossing the tile between checks. Which direction
he is pressing is.

What the direction selects is still open, because no single tested point fits
both rows. At x 88 the right-pressed point has to be past the block's right
edge at x 95, so at least 8 pixels ahead of him; the same offset at x 48 would
land on the block's left column and cap him there, and it does not. The
standing sweep does not fit a single point either: it starts capping 6 pixels
into the block, alternates 12 and 16 on an 8 pixel cadence, and carries on 10
pixels past the far edge. Something about this is being read wrong, and the
part that is settled is that the pocket is a dead end for us and not for the
cartridge, and that direction is the variable.
