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
//! cap is `256` (1 px/frame). Gravity and jump values are still provisional;
//! see the plan.

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
/// Downward acceleration per frame.
pub const GRAVITY: i32 = 40;
/// Cap on downward speed, so falling does not tunnel through thin floors.
pub const MAX_FALL_SPEED: i32 = 640;
/// Upward speed given at the start of a jump.
pub const JUMP_VELOCITY: i32 = 700;
/// Releasing the jump button early clamps any remaining rise to this, giving
/// short hops when tapped and full jumps when held.
pub const JUMP_CUT: i32 = 200;
/// Upward speed Mario gets from stomping an enemy.
pub const STOMP_BOUNCE: i32 = 500;

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

    // Gravity only builds up while airborne. Resting on a solid keeps vy at 0,
    // so Mario sits still instead of nudging into the floor every frame.
    if !mario.on_ground {
        mario.vy = (mario.vy + t.gravity).min(t.max_fall_speed);
    }
    mario.y += mario.vy;
    resolve_vertical(mario, solids);

    mario.on_ground = grounded(mario, solids);
    if mario.on_ground && mario.vy > 0 {
        mario.vy = 0;
    }
}

/// Start a jump on the frame the jump button is pressed while grounded. Holding
/// the button does not re-jump (a latch guards that). Releasing early while
/// still rising cuts the jump short, which gives variable jump height.
fn apply_jump(mario: &mut Mario, buttons: Buttons, t: &Tuning) {
    let jump = buttons.is_held(Button::A);

    if mario.on_ground && jump && !mario.jump_latched {
        mario.vy = -t.jump_velocity;
        mario.on_ground = false;
        mario.jump_latched = true;
    }
    if !jump {
        mario.jump_latched = false;
        if mario.vy < -t.jump_cut {
            mario.vy = -t.jump_cut;
        }
    }
}

/// Pixel edges of Mario's bounding box: (left, top, right, bottom), inclusive.
fn edges(mario: &Mario) -> (i32, i32, i32, i32) {
    let (w, h) = mario.size();
    let left = mario.pixel_x();
    let top = mario.pixel_y();
    (left, top, left + w - 1, top + h - 1)
}

fn resolve_horizontal(mario: &mut Mario, solids: &Solids) {
    let (w, _h) = mario.size();
    let (left, top, right, bottom) = edges(mario);
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

fn resolve_vertical(mario: &mut Mario, solids: &Solids) {
    let (_w, h) = mario.size();
    let (left, top, right, bottom) = edges(mario);
    if mario.vy > 0 && solids.rect_hits_solid(left, bottom, right, bottom) {
        let floor_top = bottom.div_euclid(TILE) * TILE;
        mario.y = pixels(floor_top - h);
        mario.vy = 0;
    } else if mario.vy < 0 && solids.rect_hits_solid(left, top, right, top) {
        let ceil_bottom = top.div_euclid(TILE) * TILE + (TILE - 1);
        mario.y = pixels(ceil_bottom + 1);
        mario.vy = 0;
    }
}

/// True when a solid sits directly under Mario's feet.
fn grounded(mario: &Mario, solids: &Solids) -> bool {
    let (left, _top, right, bottom) = edges(mario);
    solids.rect_hits_solid(left, bottom + 1, right, bottom + 1)
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
    use crate::core::entity::Facing;

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
        // Walking is pinned to observed emulator RAM (see the module doc
        // comment); gravity/jump are still provisional. This test is a
        // tripwire: if a constant changes, it is a deliberate act, not an
        // accident. Update the expected values here in the same commit that
        // retunes them.
        assert_eq!(WALK_ACCEL, 43);
        assert_eq!(FRICTION, 43);
        assert_eq!(MAX_WALK_SPEED, 256);
        assert_eq!(GRAVITY, 40);
        assert_eq!(MAX_FALL_SPEED, 640);
        assert_eq!(JUMP_VELOCITY, 700);
        assert_eq!(JUMP_CUT, 200);
        assert_eq!(STOMP_BOUNCE, 500);
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
        let mut m = Mario::new(8, 0); // small (8x8), above the floor
        for _ in 0..200 {
            step_motion(&mut m, Buttons::default(), &solids, &Tuning::default());
        }
        // Floor top is y=24, Mario is 8 tall, so he rests at y=16.
        assert_eq!(m.pixel_y(), 16);
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
        // Wall's left edge is x=56, Mario is 8 wide, so he stops at x=48 and
        // cannot pass it no matter how long he pushes.
        assert_eq!(m.pixel_x(), 48);
        for _ in 0..10 {
            step_motion(&mut m, held(Button::Right), &solids, &Tuning::default());
            assert_eq!(m.pixel_x(), 48);
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

    #[test]
    fn mario_settles_into_a_one_tile_wide_pit() {
        // Walls on columns 2 and 4 make a one-tile-wide slot at column 3, with a
        // floor underneath. Mario dropped in should rest on the floor.
        let solids = Solids::from_rows(&[
            "..#.#...",
            "..#.#...",
            "..#.#...",
            "########",
        ]);
        let mut m = Mario::new(24, 0); // column 3, in the slot
        for _ in 0..200 {
            step_motion(&mut m, Buttons::default(), &solids, &Tuning::default());
        }
        // Pit floor top is row 3 (y=24); small Mario rests at y=16.
        assert_eq!(m.pixel_x(), 24);
        assert_eq!(m.pixel_y(), 16);
        assert!(m.on_ground);
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
