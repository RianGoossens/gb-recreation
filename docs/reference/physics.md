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

That rules out treating this as solved. Two things are still tangled
together and this session did not separate them: the max-hold case's
rise (11 frames) not matching its fall (13 frames) could mean the
cartridge really does use a faster fall than rise (a common Mario
convention, and our own engine's single-constant `GRAVITY` does not
model that), or it could just as easily be a frame or two of lag between
Y actually returning to ground level and the grounded flag catching up,
with no asymmetric physics involved at all. Deriving new `GRAVITY`,
`JUMP_VELOCITY`, or `JUMP_CUT` values from this data was deliberately
not done: the short-hold anomaly above means the underlying model isn't
understood well enough yet to trust a derived number over the existing,
tested placeholder. Recorded as a data point for the next attempt, not
as a fix.
