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

Gravity, jump velocity, jump cut, and stomp bounce are still provisional
placeholders. Forcing a jump (hold A, release, let Mario land) and diffing
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

1. **Rising, A held**: near-constant upward speed (~2.1-2.3 px/frame), no
   measurable deceleration. Looks like the classic "holding jump counters
   gravity" convention used across Mario games.
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

This is a real structural finding. It does not become a shipped
`GRAVITY`/`JUMP_VELOCITY`/`JUMP_CUT` number today: those constants assume a
single continuous-acceleration model, and this data describes the
three-regime state machine above instead. Implementing this properly means
changing `step_motion` to switch behavior on an explicit rise/fall phase
(mirroring `0xC207`) rather than adding one constant every airborne frame
the way it does now, which is an engine change with its own tests, not a
constant tweak to make quietly here. Recorded as a strong, verified lead for
that task, including the exact traces and fitted values above to check any
future implementation against.
