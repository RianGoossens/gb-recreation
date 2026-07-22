//! Movement physics for Mario.
//!
//! All values are in subpixels (see [`entity::SUBPIXEL`]). Velocity is
//! subpixels per frame, acceleration is subpixels per frame per frame.
//!
//! The constants here are PROVISIONAL. They give movement that reads correctly
//! (build up speed, cap out, coast to a stop) but are not yet the original
//! game's exact numbers. Pinning those to the reference is its own plan task,
//! so treat these as placeholders behind named constants, easy to swap.

use super::entity::Mario;
use crate::input::{Button, Buttons};

/// Horizontal acceleration while a direction is held.
pub const WALK_ACCEL: i32 = 24;
/// Deceleration applied when no direction is held.
pub const FRICTION: i32 = 16;
/// Cap on horizontal speed while walking.
pub const MAX_WALK_SPEED: i32 = 320;

/// Advance Mario's horizontal movement by one frame from the held buttons:
/// accelerate toward a held direction, otherwise coast to a stop, clamp to the
/// walk speed, then move.
pub fn step_walk(mario: &mut Mario, buttons: Buttons) {
    let left = buttons.is_held(Button::Left);
    let right = buttons.is_held(Button::Right);

    match (left, right) {
        (true, false) => mario.vx -= WALK_ACCEL,
        (false, true) => mario.vx += WALK_ACCEL,
        _ => mario.vx = coast_to_zero(mario.vx, FRICTION),
    }

    mario.vx = mario.vx.clamp(-MAX_WALK_SPEED, MAX_WALK_SPEED);
    mario.x += mario.vx;
    mario.face_from_input(buttons);
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
}
