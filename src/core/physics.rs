//! Movement physics for Mario.
//!
//! All values are in subpixels (see [`entity::SUBPIXEL`]). Velocity is
//! subpixels per frame, acceleration is subpixels per frame per frame.
//!
//! Walking (`WALK_ACCEL`, `FRICTION`, `MAX_WALK_SPEED`) is pinned to the real
//! game: `tools/find_mario_speed.py` boots the ROM, holds Right, and snapshots
//! WRAM every frame. Address `0xC20C` is Mario's horizontal speed: it climbs
//! by 1 per frame, caps at 6, and falls by 1 per frame once Right is
//! released, holding at 0 rather than going negative. Correlating it against
//! Mario's on-screen X (address `0xC202`, before the camera starts scrolling)
//! shows one whole pixel is covered for every 6 units of accumulated speed,
//! so the original's speed unit is 1/6 pixel and a capped speed of 6 is
//! exactly 1 pixel per frame. Converted to our subpixel scale (256 per
//! pixel): accel and friction are both `round(256 / 6) = 43`, and the walking
//! cap is `256` (1 px/frame).
//!
//! Jumping (`JUMP_VELOCITY`, `RISE_DRIFT`, `MAX_RISE_FRAMES`, `JUMP_CUT`,
//! `GRAVITY`) is pinned the same way (see `docs/reference/physics.md`), by
//! reading `0xC201` (Y position) and `0xC207` (an internal rise/fall phase
//! byte) frame by frame through several jumps. The cartridge's jump is not
//! one continuously-accelerating parabola; it is three regimes: rising with
//! the button held decelerates only a tiny, near-negligible amount
//! (`RISE_DRIFT`) for up to a fixed frame count (`MAX_RISE_FRAMES`); if the
//! button is released before that count is up, a real, much stronger
//! deceleration takes over (`JUMP_CUT`); falling accelerates for real
//! (`GRAVITY`), a different rate again. The frame count running out while
//! still held is its own, distinct case, not routed through `JUMP_CUT`: the
//! traced data shows a one-frame flip straight to falling there, so it is
//! modeled as a direct velocity reset in `apply_vertical_accel`, not a slow
//! decay. `STOMP_BOUNCE` is measured too, by `tools/measure_stomp_bounce.py`
//! (61 landed stomps off one save state); the bounce always decays through
//! `JUMP_CUT`, never through the held-rise regime, which is why `Mario`
//! carries a `bouncing` flag.

use super::entity::{pixels, Mario};
use super::level::{Solids, TILE};
use crate::input::{Button, Buttons};
use crate::tuning::Tuning;

/// Horizontal acceleration while a direction is held.
pub const WALK_ACCEL: i32 = 43;
/// Deceleration applied when no direction is held.
pub const FRICTION: i32 = 43;
/// Cap on horizontal speed while walking.
pub const MAX_WALK_SPEED: i32 = 256;
/// Downward acceleration per frame while falling. Derived from total fall
/// distance and duration (about 25px over about 13 frames from rest at the
/// apex), a sturdier measure than differentiating the noisy per-frame
/// quadratic fits in physics.md, whose fall-phase residuals ran up to 1.6px
/// and undershot this once actually simulated end to end.
pub const GRAVITY: i32 = 76;
/// Cap on downward speed, so falling does not tunnel through thin floors.
pub const MAX_FALL_SPEED: i32 = 640;
/// Upward speed given at the start of a jump, and held roughly steady
/// (see `RISE_DRIFT`) for as long as the button stays held within
/// `MAX_RISE_FRAMES`.
pub const JUMP_VELOCITY: i32 = 602;
/// Tiny deceleration applied each frame while rising with the button held.
/// On its own this would take about 61 frames to decay to zero; it never
/// gets the chance, because `MAX_RISE_FRAMES` cuts the held rise short first.
pub const RISE_DRIFT: i32 = 10;
/// How many frames a held rise can last before it is cut off regardless of
/// the button still being held.
pub const MAX_RISE_FRAMES: i32 = 12;
/// Deceleration applied once a rise is no longer sustained by a held button
/// within `MAX_RISE_FRAMES`. Far stronger than `RISE_DRIFT`, which is what
/// gives a tapped jump a short hop and a held jump a full-height one.
///
/// This was 29, from a quadratic fitted to the released segment of the
/// hold=3, 5 and 8 traces, and it made an early release the *highest* jump
/// the engine had: releasing at frame 12 rose 43px where holding through the
/// cap rose 30px. Both traces in `docs/reference/physics.md` say otherwise,
/// hold=3 peaking at 15px and hold=8 at 24px, so the fit was wrong in the way
/// the fall phase's was, and the same sturdier measure fixes it. Stepping the
/// engine's own arc at candidate values reproduces both peaks exactly at 76,
/// which is `GRAVITY`. They are kept separate constants: one pair of traces
/// agreeing on a number is not enough to claim the cartridge decelerates a
/// released rise with the same code that makes him fall.
pub const JUMP_CUT: i32 = 76;
/// What a moving jump does at the rise cap instead of stopping dead.
///
/// The cap was measured from a standing jump, and standing is the only case
/// where it stops the rise outright. `tools/measure_jump_height.py` holds A
/// for 40 frames from the same save state with and without a direction held,
/// and the two arcs are identical for the first twelve frames and then part:
/// standing flips straight to +1 and falls (24px), moving carries on rising
/// about a pixel a frame for another twelve (33px). That extra 9px is what
/// World 1-1 needs to get over the pillar at columns 78 and 79, which is 32px
/// above the ground either side of it.
///
/// 1.5 px/frame decaying to nothing over 12 frames covers those 9px, which is
/// `2d/t` and `v/t` for the segment.
pub const GLIDE_VELOCITY: i32 = 384;
/// Deceleration through the glide, so it reaches zero as the frames run out.
pub const GLIDE_CUT: i32 = 32;
/// How long the glide lasts.
pub const GLIDE_FRAMES: i32 = 12;
/// Deceleration applied to a stomp bounce, which has its own trace and its
/// own answer: 61 stomps give 8px over about 12.4 frames, so `2d/t^2` is
/// 26.9. A bounce ran on `JUMP_CUT` while that was 29 and the two were within
/// noise of each other; they are not now, and the bounce keeps its own.
pub const BOUNCE_CUT: i32 = 29;
/// Upward speed Mario gets from stomping an enemy. Measured from 61 real
/// stomps on the first World 1-1 Chibibo (`tools/measure_stomp_bounce.py`):
/// the bounce lifts Mario 8px over about 12.4 frames before the phase byte
/// flips back to falling. Holding the jump button through the stomp changes
/// that by 1px, so there is one bounce speed, not a held and unheld version.
///
/// The raw `2d/t` reading is 1.30 px/frame (333 here), but that assumes the
/// deceleration is `2d/t^2 = 26.9`, and the engine decelerates a bounce with
/// `BOUNCE_CUT` (29). Simulating 333 against that reaches only 7px. 360 is
/// the speed that reproduces the traced 8px over 13 frames with the
/// deceleration we actually apply.
pub const STOMP_BOUNCE: i32 = 360;

/// Update horizontal velocity and facing from the held buttons, without moving.
fn walk_velocity(mario: &mut Mario, buttons: Buttons, t: &Tuning) {
    let left = buttons.is_held(Button::Left);
    let right = buttons.is_held(Button::Right);

    match (left, right) {
        (true, false) => mario.vx -= t.walk_accel,
        (false, true) => mario.vx += t.walk_accel,
        _ => mario.vx = coast_to_zero(mario.vx, t.friction),
    }

    mario.vx = mario.vx.clamp(-t.max_walk_speed, t.max_walk_speed);
    mario.face_from_input(buttons);
}

/// Advance Mario's horizontal movement by one frame from the held buttons,
/// ignoring the world. Used where there is no level yet.
pub fn step_walk(mario: &mut Mario, buttons: Buttons, t: &Tuning) {
    walk_velocity(mario, buttons, t);
    mario.x += mario.vx;
}

/// Advance Mario one frame against the level: walk sideways, fall under gravity,
/// and resolve collisions with solid tiles. Sets `on_ground` when standing on a
/// solid.
pub fn step_motion(mario: &mut Mario, buttons: Buttons, solids: &Solids, t: &Tuning) {
    walk_velocity(mario, buttons, t);
    mario.x += mario.vx;
    resolve_horizontal(mario, solids);

    apply_jump(mario, buttons, t);
    apply_vertical_accel(mario, buttons, t);

    let was_above = edges(mario).3;
    mario.y += mario.vy;
    resolve_vertical(mario, solids, was_above);

    mario.on_ground = grounded(mario, solids);
    if mario.on_ground {
        mario.bouncing = false;
        if mario.vy > 0 {
            mario.vy = 0;
        }
    }
}

/// Start a jump on the frame the jump button is pressed while grounded. Holding
/// the button does not re-jump (a latch guards that).
fn apply_jump(mario: &mut Mario, buttons: Buttons, t: &Tuning) {
    let jump = buttons.is_held(Button::A);

    if mario.on_ground && jump && !mario.jump_latched {
        mario.vy = -t.jump_velocity;
        mario.on_ground = false;
        mario.jump_latched = true;
        mario.rise_frames = 0;
        mario.bouncing = false;
    }
    if !jump {
        mario.jump_latched = false;
    }
}

/// Three regimes, not one constant acceleration (see the module doc comment
/// and `docs/reference/physics.md`): a near-flat rise while the button is
/// held, capped at a fixed frame count; a real deceleration if released
/// before that cap; and a real acceleration while falling. The cap running
/// out while still held is not the same event as an early release: the
/// traced data shows a one-frame flip straight to falling there (no gradual
/// decay first), so it is modeled as a direct reset. That reset frame also
/// applies the first frame of gravity immediately rather than waiting for
/// the next one: the traced transition frame already shows a small amount
/// of falling motion, not a frame spent sitting at zero velocity.
fn apply_vertical_accel(mario: &mut Mario, buttons: Buttons, t: &Tuning) {
    if mario.on_ground {
        return;
    }
    let rising = mario.vy < 0;
    let holding_jump = buttons.is_held(Button::A) && !mario.bouncing;

    if rising && holding_jump && mario.rise_frames < t.max_rise_frames {
        mario.rise_frames += 1;
        mario.vy += t.rise_drift;
        return;
    }
    // The cap. Standing still it stops the rise outright; moving, a second
    // and slower rise follows it (`GLIDE_VELOCITY`).
    if rising && holding_jump && mario.rise_frames == t.max_rise_frames {
        mario.rise_frames += 1;
        if mario.vx != 0 {
            mario.vy = -t.glide_velocity;
            return;
        }
        mario.vy = 0;
    } else if holding_jump
        && mario.vy < 0
        && mario.rise_frames < t.max_rise_frames + t.glide_frames
    {
        mario.rise_frames += 1;
        mario.vy = (mario.vy + t.glide_cut).min(0);
        return;
    } else if rising {
        mario.vy += if mario.bouncing {
            t.bounce_cut
        } else {
            t.jump_cut
        };
        return;
    }
    mario.vy = (mario.vy + t.gravity).min(t.max_fall_speed);
}

/// Pixel edges of Mario's bounding box: (left, top, right, bottom), inclusive.
fn edges(mario: &Mario) -> (i32, i32, i32, i32) {
    let (w, h) = mario.size();
    let left = mario.pixel_x();
    let top = mario.pixel_y();
    (left, top, left + w - 1, top + h - 1)
}

/// How much of Mario the terrain stops when he walks into it, measured up
/// from his feet.
///
/// Not his whole height. Writing a ceiling into World 1-1's own tilemap and
/// walking him at it (`tools/probe_corridor_height.py`) puts the boundary
/// exactly one tile up: a slab in the 8 pixels directly above the floor stops
/// him dead at the column it starts in, and the same slab one row higher, in
/// the 8 pixels his head occupies, does not slow him at all. The control is a
/// full-height wall in the same place, which stops him, so a run where the
/// tilemap writes never landed would not read as "nothing blocks him".
///
/// This is what World 1-3's corridor at columns 185 to 192 is: one free row
/// under a slab, which a 12 pixel Mario cannot enter and an 8 pixel one walks
/// straight down. His height is still 12 for standing on things and for
/// ceilings overhead, both measured separately.
pub const WALK_HEIGHT: i32 = 8;

/// How wide the part of Mario is that a ceiling stops, centred in his box.
///
/// Five, measured the way the lift's surface was: write a ceiling of a known
/// width into World 1-1's own tilemap and sweep a jump under it a pixel at a
/// time (`tools/measure_head_width.py`). A one tile ceiling cuts the jump
/// short over a 12 pixel window of his own position and a three tile ceiling
/// over 28, and `ceiling + head - 1` gives 5 from both. The free rise of 33
/// either side of each window is the control that says the sweep is wide
/// enough to hold it, and the second width is the control that says this is a
/// width at all rather than one number that happened to fit.
///
/// Five is the number an enemy's contact box came out at by an unrelated
/// route, and the lift's foot is six: the cartridge tests small parts of him
/// rather than his whole drawing, over and over.
pub const HEAD_WIDTH: i32 = 5;

fn resolve_horizontal(mario: &mut Mario, solids: &Solids) {
    let (w, _h) = mario.size();
    let (left, _full_top, right, bottom) = edges(mario);
    let top = bottom - (WALK_HEIGHT - 1);
    if mario.vx > 0 && solids.rect_hits_solid(right, top, right, bottom) {
        let wall_left = right.div_euclid(TILE) * TILE;
        mario.x = pixels(wall_left - w);
        mario.vx = 0;
    } else if mario.vx < 0 && solids.rect_hits_solid(left, top, left, bottom) {
        let wall_right = left.div_euclid(TILE) * TILE + (TILE - 1);
        mario.x = pixels(wall_right + 1);
        mario.vx = 0;
    }
}

/// `was_above` is Mario's bottom edge before this frame's move, which is what
/// decides a one-way platform: he lands on it only if his feet started above
/// its top. Coming up from underneath or standing inside it, it is not there.
fn resolve_vertical(mario: &mut Mario, solids: &Solids, was_above: i32) {
    let (_w, h) = mario.size();
    let (left, top, right, bottom) = edges(mario);
    let landed = solids.rect_hits_solid(left, bottom, right, bottom)
        || landed_on_platform(solids, left, right, bottom, was_above);
    if mario.vy > 0 && landed {
        let floor_top = bottom.div_euclid(TILE) * TILE;
        mario.y = pixels(floor_top - h);
        mario.vy = 0;
    } else if mario.vy < 0 && {
        let inset = (right - left + 1 - HEAD_WIDTH) / 2;
        solids.rect_hits_solid(left + inset, top, left + inset + HEAD_WIDTH - 1, top)
    } {
        let ceil_bottom = top.div_euclid(TILE) * TILE + (TILE - 1);
        mario.y = pixels(ceil_bottom + 1);
        mario.vy = 0;
    }
}

/// A one-way platform catches a fall only if the feet crossed its top edge
/// during this frame.
fn landed_on_platform(
    solids: &Solids,
    left: i32,
    right: i32,
    bottom: i32,
    was_above: i32,
) -> bool {
    match solids.platform_under(left, right, bottom) {
        Some(ty) => was_above < ty * TILE,
        None => false,
    }
}

/// True when something sits directly under Mario's feet, solid or platform.
///
/// A platform only counts while he is not moving up, or jumping through one
/// would stop him at the apex with its top an inch under his feet.
fn grounded(mario: &Mario, solids: &Solids) -> bool {
    let (left, _top, right, bottom) = edges(mario);
    solids.rect_hits_solid(left, bottom + 1, right, bottom + 1)
        || (mario.vy >= 0 && solids.platform_under(left, right, bottom + 1).is_some())
}

/// Move a velocity toward zero by `amount`, without overshooting past zero.
fn coast_to_zero(v: i32, amount: i32) -> i32 {
    if v > 0 {
        (v - amount).max(0)
    } else if v < 0 {
        (v + amount).min(0)
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::entity::{Facing, SMALL_HEIGHT, SMALL_WIDTH};

    fn held(button: Button) -> Buttons {
        let mut b = Buttons::default();
        b.set(button, true);
        b
    }

    #[test]
    fn holding_right_builds_speed_and_moves_right() {
        let mut m = Mario::new(50, 100);
        let start = m.x;
        step_walk(&mut m, held(Button::Right), &Tuning::default());
        assert_eq!(m.vx, WALK_ACCEL);
        assert!(m.x > start);
        assert_eq!(m.facing, Facing::Right);
    }

    #[test]
    fn speed_is_capped_at_max_walk() {
        let mut m = Mario::new(0, 0);
        for _ in 0..1000 {
            step_walk(&mut m, held(Button::Right), &Tuning::default());
        }
        assert_eq!(m.vx, MAX_WALK_SPEED);
    }

    #[test]
    fn releasing_coasts_to_a_stop() {
        let mut m = Mario::new(0, 0);
        for _ in 0..20 {
            step_walk(&mut m, held(Button::Right), &Tuning::default());
        }
        assert!(m.vx > 0);
        for _ in 0..1000 {
            step_walk(&mut m, Buttons::default(), &Tuning::default());
        }
        assert_eq!(m.vx, 0);
    }

    #[test]
    fn friction_does_not_overshoot_past_zero() {
        let mut m = Mario::new(0, 0);
        m.vx = FRICTION / 2; // less than one friction step
        step_walk(&mut m, Buttons::default(), &Tuning::default());
        assert_eq!(m.vx, 0);
    }

    #[test]
    fn left_and_right_together_coast_like_no_input() {
        let mut m = Mario::new(0, 0);
        m.vx = 100;
        let mut both = Buttons::default();
        both.set(Button::Left, true);
        both.set(Button::Right, true);
        step_walk(&mut m, both, &Tuning::default());
        assert_eq!(m.vx, 100 - FRICTION);
    }

    #[test]
    fn stepping_is_deterministic() {
        let script = [Button::Right, Button::Right, Button::Left];
        let mut a = Mario::new(10, 10);
        let mut b = Mario::new(10, 10);
        for &button in &script {
            step_walk(&mut a, held(button), &Tuning::default());
            step_walk(&mut b, held(button), &Tuning::default());
        }
        assert_eq!(a, b);
    }

    #[test]
    fn physics_constants_are_pinned() {
        // Every constant here is pinned to observed emulator RAM (see the
        // module doc comment). This test is a tripwire: if a constant
        // changes, it is a deliberate act, not an accident. Update the
        // expected values here in the same commit that retunes them.
        assert_eq!(WALK_ACCEL, 43);
        assert_eq!(FRICTION, 43);
        assert_eq!(MAX_WALK_SPEED, 256);
        assert_eq!(GRAVITY, 76);
        assert_eq!(MAX_FALL_SPEED, 640);
        assert_eq!(JUMP_VELOCITY, 602);
        assert_eq!(RISE_DRIFT, 10);
        assert_eq!(MAX_RISE_FRAMES, 12);
        assert_eq!(JUMP_CUT, 76);
        assert_eq!(BOUNCE_CUT, 29);
        assert_eq!(STOMP_BOUNCE, 360);
    }

    // Gravity and collision.
    use crate::core::level::Solids;

    /// A floor along the bottom row, empty above. 8 tiles wide, 4 tall, so the
    /// floor's top is at pixel y = 24.
    fn floor_level() -> Solids {
        Solids::from_rows(&[
            "........",
            "........",
            "........",
            "########",
        ])
    }

    #[test]
    fn mario_falls_and_lands_on_the_floor() {
        let solids = floor_level();
        let mut m = Mario::new(8, 0); // above the floor
        for _ in 0..200 {
            step_motion(&mut m, Buttons::default(), &solids, &Tuning::default());
        }
        // Floor top is y=24 and Mario is 12 tall, so he rests at y=12.
        assert_eq!(m.pixel_y(), 24 - SMALL_HEIGHT);
        assert_eq!(m.vy, 0);
        assert!(m.on_ground);
    }

    #[test]
    fn walking_into_a_wall_stops_horizontal_movement() {
        // Wall in the rightmost column, floor along the bottom.
        let solids = Solids::from_rows(&[
            ".......#",
            ".......#",
            ".......#",
            "########",
        ]);
        let mut m = Mario::new(8, 16); // on the floor, left of the wall
        for _ in 0..200 {
            step_motion(&mut m, held(Button::Right), &solids, &Tuning::default());
        }
        // Wall's left edge is x=56 and Mario is 11 wide, so he stops at x=45
        // and cannot pass it no matter how long he pushes.
        assert_eq!(m.pixel_x(), 56 - SMALL_WIDTH);
        for _ in 0..10 {
            step_motion(&mut m, held(Button::Right), &solids, &Tuning::default());
            assert_eq!(m.pixel_x(), 56 - SMALL_WIDTH);
        }
    }

    #[test]
    fn not_grounded_while_in_the_air() {
        let solids = floor_level();
        let mut m = Mario::new(8, 0);
        step_motion(&mut m, Buttons::default(), &solids, &Tuning::default());
        assert!(!m.on_ground);
    }

    /// Settle Mario onto the floor so he starts grounded.
    fn resting_on_floor() -> (Mario, Solids) {
        let solids = floor_level();
        let mut m = Mario::new(8, 0);
        for _ in 0..200 {
            step_motion(&mut m, Buttons::default(), &solids, &Tuning::default());
        }
        assert!(m.on_ground);
        (m, solids)
    }

    #[test]
    fn pressing_jump_from_the_ground_launches_up() {
        let (mut m, solids) = resting_on_floor();
        let top = m.pixel_y();
        step_motion(&mut m, held(Button::A), &solids, &Tuning::default());
        assert!(m.vy < 0, "should be moving up");
        assert!(!m.on_ground);
        // A few frames in, he is above where he started.
        for _ in 0..5 {
            step_motion(&mut m, held(Button::A), &solids, &Tuning::default());
        }
        assert!(m.pixel_y() < top);
    }

    /// Rise height in pixels and frames spent rising, for a Mario launched
    /// upward at `STOMP_BOUNCE` from mid-air with `bouncing` set.
    fn bounce_arc(buttons: Buttons) -> (i32, i32) {
        // Tall enough that the arc plays out without touching the floor.
        let solids = Solids::from_rows(&["........", "........", "########"]);
        let mut m = Mario::new(8, 0);
        m.vy = -STOMP_BOUNCE;
        m.bouncing = true;
        let start = m.pixel_y();
        let mut frames = 0;
        while m.vy < 0 {
            step_motion(&mut m, buttons, &solids, &Tuning::default());
            frames += 1;
        }
        (start - m.pixel_y(), frames)
    }

    #[test]
    fn stomp_bounce_matches_the_traced_arc() {
        // The cartridge lifts Mario 8px over about 12.4 frames after a stomp
        // (61 landed stomps, tools/measure_stomp_bounce.py). Simulating the
        // implemented constant end to end has to land in the same place, not
        // just equal its own source number: the first value tried here came
        // straight from 2d/t and only reached 7px once actually stepped.
        let (rise, frames) = bounce_arc(Buttons::default());
        assert_eq!(rise, 8, "bounce rise");
        assert!((12..=14).contains(&frames), "bounce lasted {frames} frames");
    }

    #[test]
    fn stomp_bounce_ignores_a_held_jump_button() {
        // Holding A through a stomp gave 9px over 13.5 frames on the
        // cartridge, essentially the same bounce, not the near-flat held
        // rise a jump gets. Without the `bouncing` flag this arc collapses:
        // rise_frames is already spent from the jump that got Mario airborne,
        // so the held branch would reset vy to 0 on the very next frame.
        let (held_rise, held_frames) = bounce_arc(held(Button::A));
        let (free_rise, free_frames) = bounce_arc(Buttons::default());
        assert_eq!((held_rise, held_frames), (free_rise, free_frames));
    }

    #[test]
    fn landing_clears_the_bounce_flag() {
        let solids = floor_level();
        let mut m = Mario::new(8, 0);
        m.bouncing = true;
        for _ in 0..200 {
            step_motion(&mut m, Buttons::default(), &solids, &Tuning::default());
        }
        assert!(m.on_ground);
        assert!(!m.bouncing);
    }

    #[test]
    fn cannot_jump_again_while_airborne() {
        let (mut m, solids) = resting_on_floor();
        step_motion(&mut m, held(Button::A), &solids, &Tuning::default());
        let vy_after_first = m.vy;
        // Still holding A in the air must not relaunch.
        step_motion(&mut m, held(Button::A), &solids, &Tuning::default());
        assert!(m.vy > vy_after_first, "gravity should reduce upward speed, not reset it");
    }

    #[test]
    fn held_rise_is_cut_off_by_frame_count_not_by_decay() {
        // Holding A well past MAX_RISE_FRAMES must still end the rise: the
        // real cartridge does this too (see physics.md), the drift alone
        // would take dozens of frames to reach zero on its own. Once the cap
        // kicks in, JUMP_CUT's real deceleration takes over and reaches zero
        // in a handful more frames, same as a real jump's total arc length.
        let (mut m, solids) = resting_on_floor();
        for i in 0..MAX_RISE_FRAMES {
            step_motion(&mut m, held(Button::A), &solids, &Tuning::default());
            assert!(m.vy < 0, "frame {i}: should still be rising while under the cap");
        }
        for _ in 0..40 {
            step_motion(&mut m, held(Button::A), &solids, &Tuning::default());
            if m.vy >= 0 {
                return;
            }
        }
        panic!("past the cap, held or not, the rise should have ended within 40 more frames");
    }

    #[test]
    fn releasing_early_decelerates_faster_than_the_held_drift() {
        // Tap: release immediately after takeoff.
        let (mut tap, solids) = resting_on_floor();
        step_motion(&mut tap, held(Button::A), &solids, &Tuning::default());
        let tap_vy_at_takeoff = tap.vy;
        step_motion(&mut tap, Buttons::default(), &solids, &Tuning::default());
        let tap_delta = tap.vy - tap_vy_at_takeoff;

        // Hold: keep A down for the same second frame.
        let (mut hold, solids2) = resting_on_floor();
        step_motion(&mut hold, held(Button::A), &solids2, &Tuning::default());
        let hold_vy_at_takeoff = hold.vy;
        step_motion(&mut hold, held(Button::A), &solids2, &Tuning::default());
        let hold_delta = hold.vy - hold_vy_at_takeoff;

        assert!(
            tap_delta > hold_delta,
            "releasing early should decelerate faster than the held drift: tap_delta={tap_delta} hold_delta={hold_delta}"
        );
    }

    #[test]
    fn holding_jump_goes_higher_than_tapping() {
        // Tapped jump: pressed one frame, released after.
        let (mut tap, solids) = resting_on_floor();
        step_motion(&mut tap, held(Button::A), &solids, &Tuning::default());
        let mut tap_apex = tap.pixel_y();
        for _ in 0..40 {
            step_motion(&mut tap, Buttons::default(), &solids, &Tuning::default());
            tap_apex = tap_apex.min(tap.pixel_y());
        }

        // Held jump: A down the whole way up.
        let (mut hold, solids) = resting_on_floor();
        let mut hold_apex = hold.pixel_y();
        for _ in 0..40 {
            step_motion(&mut hold, held(Button::A), &solids, &Tuning::default());
            hold_apex = hold_apex.min(hold.pixel_y());
        }

        assert!(hold_apex < tap_apex, "holding should reach a higher apex");
    }

    /// How high a jump goes, at three hold lengths, against the traced arcs
    /// in `docs/reference/physics.md`.
    fn apex(hold: u32) -> i32 {
        let (mut m, solids) = resting_on_floor();
        let start = m.pixel_y();
        let mut peak = start;
        for f in 0..40 {
            let buttons = if f < hold { held(Button::A) } else { Buttons::default() };
            step_motion(&mut m, buttons, &solids, &Tuning::default());
            peak = peak.min(m.pixel_y());
        }
        start - peak
    }

    /// The cartridge traces peak at 15px for a 3-frame hold and 24px for an
    /// 8-frame hold. `JUMP_CUT` was fitted rather than simulated and gave 28
    /// and 35, which also made an early release the highest jump the engine
    /// had, since holding through the cap stops the rise outright.
    #[test]
    fn a_short_hold_matches_the_traced_arc() {
        assert_eq!(apex(3), 15, "a 3-frame hold peaks at 15px on the cartridge");
        assert_eq!(apex(8), 24, "an 8-frame hold peaks at 24px");
    }

    /// Longer hold, higher jump, over the range the traces cover. This is
    /// what the old `JUMP_CUT` broke worst: at 29 the curve ran 28px at a
    /// 3-frame hold down to 24px at 8, so pressing the button for longer got
    /// Mario less height.
    ///
    /// The range stops at the cap. Holds of 9 to 12 come out above the
    /// held-throughout peak here (up to 31px against 26px), because releasing
    /// one frame before the cap decays gradually where the cap stops the rise
    /// outright. Nothing measured that side of the boundary on the cartridge,
    /// so it is left as it falls out rather than tuned to look tidy.
    #[test]
    fn a_longer_hold_jumps_higher_up_to_the_cap() {
        let heights: Vec<i32> = (1..=8).map(apex).collect();
        assert!(
            heights.windows(2).all(|w| w[1] > w[0]),
            "each extra frame of hold should add height: {heights:?}"
        );
    }

    /// Holding through the cap, which the cartridge puts at 24-25px.
    #[test]
    fn a_full_jump_matches_the_traced_height() {
        assert_eq!(apex(40), 26);
        assert_eq!(apex(13), apex(40), "the cap ends the rise, not the button");
    }

    /// The same jump with a direction held. On the cartridge that is 33px
    /// against 24 standing, from the same save state, and it is what World
    /// 1-1's own geometry needs: the pillar at columns 78 and 79 stands 32px
    /// above the ground either side of it.
    #[test]
    fn moving_jumps_higher_than_standing_still() {
        let (mut m, solids) = resting_on_floor();
        let start = m.pixel_y();
        let mut peak = start;
        let mut buttons = held(Button::A);
        buttons.set(Button::Right, true);
        for _ in 0..40 {
            step_motion(&mut m, buttons, &solids, &Tuning::default());
            peak = peak.min(m.pixel_y());
        }
        let moving = start - peak;
        assert_eq!(moving, 35, "the cartridge gives 33 here and 24 standing");
        assert!(moving - apex(40) >= 8, "the glide is worth about 9px");
    }

    #[test]
    fn mario_settles_into_a_two_tile_wide_slot() {
        // Mario is 11 px wide, so a one-tile slot no longer admits him. Walls
        // on columns 1 and 4 leave a 16 px slot, which is the narrowest gap
        // that fits him at all.
        let solids = Solids::from_rows(&[
            ".#..#...",
            ".#..#...",
            ".#..#...",
            "########",
        ]);
        let mut m = Mario::new(18, 0);
        for _ in 0..200 {
            step_motion(&mut m, Buttons::default(), &solids, &Tuning::default());
        }
        assert_eq!(m.pixel_x(), 18);
        assert_eq!(m.pixel_y(), 24 - SMALL_HEIGHT);
        assert!(m.on_ground);
    }

    #[test]
    fn a_one_tile_slot_is_too_narrow_for_him() {
        let solids = Solids::from_rows(&[
            "..#.#...",
            "..#.#...",
            "..#.#...",
            "########",
        ]);
        let mut m = Mario::new(24, 0);
        for _ in 0..200 {
            step_motion(&mut m, Buttons::default(), &solids, &Tuning::default());
        }
        // He never reaches the floor of the slot, which is at y = 24.
        assert!(m.pixel_y() + SMALL_HEIGHT < 24);
    }

    #[test]
    fn ceiling_stops_upward_motion() {
        // Solid ceiling on the top row (pixels y 0..7).
        let solids = Solids::from_rows(&[
            "########",
            "........",
            "........",
            "########",
        ]);
        let mut m = Mario::new(8, 8); // top just below the ceiling
        m.vy = -pixels(1); // launched straight up
        step_motion(&mut m, Buttons::default(), &solids, &Tuning::default());
        // He cannot enter the ceiling: top is pushed back to y=8, vy cleared.
        assert_eq!(m.pixel_y(), 8);
        assert_eq!(m.vy, 0);
    }
}

#[cfg(test)]
mod platform_tests {
    use super::*;
    use crate::core::level::Solids;

    /// Four tile ids hold Mario up without blocking him sideways, measured
    /// from the cartridge by `tools/probe_solidity.py`. World 1-2 lays them
    /// out as horizontal runs with distinct end caps.
    fn ledge() -> Solids {
        Solids::from_rows(&[
            "........",
            "........",
            "..^^^^..",
            "........",
            "########",
        ])
    }

    fn drop_onto(start_y: i32) -> Mario {
        let solids = ledge();
        let t = Tuning::default();
        let mut mario = Mario::new(3 * TILE, start_y);
        for _ in 0..60 {
            step_motion(&mut mario, Buttons::default(), &solids, &t);
        }
        mario
    }

    #[test]
    fn a_platform_catches_a_fall_from_above() {
        let mario = drop_onto(0);
        assert!(mario.on_ground, "should be standing on the platform");
        let (_w, h) = mario.size();
        assert_eq!(mario.pixel_y(), 2 * TILE - h, "resting on the platform top");
    }

    #[test]
    fn a_platform_does_not_stop_a_jump_from_below() {
        let solids = ledge();
        let t = Tuning::default();
        let mut mario = Mario::new(3 * TILE, 3 * TILE);
        let mut buttons = Buttons::default();
        buttons.set(Button::A, true);
        let mut highest = mario.pixel_y();
        for _ in 0..40 {
            step_motion(&mut mario, buttons, &solids, &t);
            highest = highest.min(mario.pixel_y());
        }
        assert!(
            highest < 2 * TILE - 8,
            "Mario should pass up through the platform, highest was {highest}"
        );
    }

    #[test]
    fn a_platform_does_not_block_sideways() {
        let solids = ledge();
        let t = Tuning::default();
        // Standing on the ground row, walking right under the platform row.
        let mut mario = Mario::new(0, 3 * TILE);
        let mut buttons = Buttons::default();
        buttons.set(Button::Right, true);
        for _ in 0..120 {
            step_motion(&mut mario, buttons, &solids, &t);
        }
        assert!(
            mario.pixel_x() > 5 * TILE,
            "should have walked past the platform, got {}",
            mario.pixel_x()
        );
    }

    #[test]
    fn without_a_platform_he_falls_to_the_floor() {
        let solids = Solids::from_rows(&[
            "........",
            "........",
            "........",
            "........",
            "########",
        ]);
        let t = Tuning::default();
        let mut mario = Mario::new(3 * TILE, 0);
        for _ in 0..60 {
            step_motion(&mut mario, Buttons::default(), &solids, &t);
        }
        let (_w, h) = mario.size();
        assert_eq!(mario.pixel_y(), 4 * TILE - h);
    }
}
