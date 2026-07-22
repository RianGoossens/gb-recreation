//! Movement physics for Mario.
//!
//! All values are in subpixels (see [`entity::SUBPIXEL`]). Velocity is
//! subpixels per frame, acceleration is subpixels per frame per frame.
//!
//! The constants here are PROVISIONAL. They give movement that reads correctly
//! (build up speed, cap out, coast to a stop) but are not yet the original
//! game's exact numbers. Pinning those to the reference is its own plan task,
//! so treat these as placeholders behind named constants, easy to swap.

use super::entity::{pixels, Mario};
use super::level::{Solids, TILE};
use crate::input::{Button, Buttons};

/// Horizontal acceleration while a direction is held.
pub const WALK_ACCEL: i32 = 24;
/// Deceleration applied when no direction is held.
pub const FRICTION: i32 = 16;
/// Cap on horizontal speed while walking.
pub const MAX_WALK_SPEED: i32 = 320;
/// Downward acceleration per frame.
pub const GRAVITY: i32 = 40;
/// Cap on downward speed, so falling does not tunnel through thin floors.
pub const MAX_FALL_SPEED: i32 = 640;

/// Update horizontal velocity and facing from the held buttons, without moving.
fn walk_velocity(mario: &mut Mario, buttons: Buttons) {
    let left = buttons.is_held(Button::Left);
    let right = buttons.is_held(Button::Right);

    match (left, right) {
        (true, false) => mario.vx -= WALK_ACCEL,
        (false, true) => mario.vx += WALK_ACCEL,
        _ => mario.vx = coast_to_zero(mario.vx, FRICTION),
    }

    mario.vx = mario.vx.clamp(-MAX_WALK_SPEED, MAX_WALK_SPEED);
    mario.face_from_input(buttons);
}

/// Advance Mario's horizontal movement by one frame from the held buttons,
/// ignoring the world. Used where there is no level yet.
pub fn step_walk(mario: &mut Mario, buttons: Buttons) {
    walk_velocity(mario, buttons);
    mario.x += mario.vx;
}

/// Advance Mario one frame against the level: walk sideways, fall under gravity,
/// and resolve collisions with solid tiles. Sets `on_ground` when standing on a
/// solid.
pub fn step_motion(mario: &mut Mario, buttons: Buttons, solids: &Solids) {
    walk_velocity(mario, buttons);
    mario.x += mario.vx;
    resolve_horizontal(mario, solids);

    // Gravity only builds up while airborne. Resting on a solid keeps vy at 0,
    // so Mario sits still instead of nudging into the floor every frame.
    if !mario.on_ground {
        mario.vy = (mario.vy + GRAVITY).min(MAX_FALL_SPEED);
    }
    mario.y += mario.vy;
    resolve_vertical(mario, solids);

    mario.on_ground = grounded(mario, solids);
    if mario.on_ground && mario.vy > 0 {
        mario.vy = 0;
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
        step_walk(&mut m, held(Button::Right));
        assert_eq!(m.vx, WALK_ACCEL);
        assert!(m.x > start);
        assert_eq!(m.facing, Facing::Right);
    }

    #[test]
    fn speed_is_capped_at_max_walk() {
        let mut m = Mario::new(0, 0);
        for _ in 0..1000 {
            step_walk(&mut m, held(Button::Right));
        }
        assert_eq!(m.vx, MAX_WALK_SPEED);
    }

    #[test]
    fn releasing_coasts_to_a_stop() {
        let mut m = Mario::new(0, 0);
        for _ in 0..20 {
            step_walk(&mut m, held(Button::Right));
        }
        assert!(m.vx > 0);
        for _ in 0..1000 {
            step_walk(&mut m, Buttons::default());
        }
        assert_eq!(m.vx, 0);
    }

    #[test]
    fn friction_does_not_overshoot_past_zero() {
        let mut m = Mario::new(0, 0);
        m.vx = FRICTION / 2; // less than one friction step
        step_walk(&mut m, Buttons::default());
        assert_eq!(m.vx, 0);
    }

    #[test]
    fn left_and_right_together_coast_like_no_input() {
        let mut m = Mario::new(0, 0);
        m.vx = 100;
        let mut both = Buttons::default();
        both.set(Button::Left, true);
        both.set(Button::Right, true);
        step_walk(&mut m, both);
        assert_eq!(m.vx, 100 - FRICTION);
    }

    #[test]
    fn stepping_is_deterministic() {
        let script = [Button::Right, Button::Right, Button::Left];
        let mut a = Mario::new(10, 10);
        let mut b = Mario::new(10, 10);
        for &button in &script {
            step_walk(&mut a, held(button));
            step_walk(&mut b, held(button));
        }
        assert_eq!(a, b);
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
            step_motion(&mut m, Buttons::default(), &solids);
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
            step_motion(&mut m, held(Button::Right), &solids);
        }
        // Wall's left edge is x=56, Mario is 8 wide, so he stops at x=48 and
        // cannot pass it no matter how long he pushes.
        assert_eq!(m.pixel_x(), 48);
        for _ in 0..10 {
            step_motion(&mut m, held(Button::Right), &solids);
            assert_eq!(m.pixel_x(), 48);
        }
    }

    #[test]
    fn not_grounded_while_in_the_air() {
        let solids = floor_level();
        let mut m = Mario::new(8, 0);
        step_motion(&mut m, Buttons::default(), &solids);
        assert!(!m.on_ground);
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
        step_motion(&mut m, Buttons::default(), &solids);
        // He cannot enter the ceiling: top is pushed back to y=8, vy cleared.
        assert_eq!(m.pixel_y(), 8);
        assert_eq!(m.vy, 0);
    }
}
