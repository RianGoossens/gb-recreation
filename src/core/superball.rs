//! The superball: fire Mario's bouncing projectile.
//!
//! A thrown superball travels forward, falls under gravity, bounces off floors,
//! and reverses off walls. It defeats an enemy it touches. It lives for a fixed
//! time and then fizzles, so the screen never fills up with them.

use crate::core::entity::pixels;
use crate::core::level::{Solids, TILE};
use crate::core::physics::{GRAVITY, MAX_FALL_SPEED};

/// A superball is a small square.
pub const SUPERBALL_SIZE: i32 = 6;
/// Horizontal speed in subpixels per frame.
pub const SUPERBALL_SPEED: i32 = 300;
/// Upward speed given by a floor bounce.
pub const SUPERBALL_BOUNCE: i32 = 400;
/// Frames a superball lives before fizzling.
pub const SUPERBALL_LIFE: u32 = 150;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Superball {
    pub x: i32,
    pub y: i32,
    pub vx: i32,
    pub vy: i32,
    pub life: u32,
}

impl Superball {
    pub fn new(pixel_x: i32, pixel_y: i32, going_left: bool) -> Self {
        let speed = if going_left { -SUPERBALL_SPEED } else { SUPERBALL_SPEED };
        Self {
            x: pixels(pixel_x),
            y: pixels(pixel_y),
            vx: speed,
            vy: 0,
            life: SUPERBALL_LIFE,
        }
    }

    pub fn pixel_x(&self) -> i32 {
        self.x.div_euclid(crate::core::entity::SUBPIXEL)
    }

    pub fn pixel_y(&self) -> i32 {
        self.y.div_euclid(crate::core::entity::SUBPIXEL)
    }

    pub fn edges(&self) -> (i32, i32, i32, i32) {
        let l = self.pixel_x();
        let t = self.pixel_y();
        (l, t, l + SUPERBALL_SIZE - 1, t + SUPERBALL_SIZE - 1)
    }
}

/// Advance a superball a frame. Returns false when it has fizzled and should be
/// removed.
pub fn update_superball(s: &mut Superball, solids: &Solids) -> bool {
    if s.life == 0 {
        return false;
    }
    s.life -= 1;

    s.x += s.vx;
    let (l, t, r, b) = s.edges();
    if s.vx > 0 && solids.rect_hits_solid(r, t, r, b) {
        let wall_left = r.div_euclid(TILE) * TILE;
        s.x = pixels(wall_left - SUPERBALL_SIZE);
        s.vx = -s.vx;
    } else if s.vx < 0 && solids.rect_hits_solid(l, t, l, b) {
        let wall_right = l.div_euclid(TILE) * TILE + (TILE - 1);
        s.x = pixels(wall_right + 1);
        s.vx = -s.vx;
    }

    s.vy = (s.vy + GRAVITY).min(MAX_FALL_SPEED);
    s.y += s.vy;
    let (l, _t, r, b) = s.edges();
    if s.vy > 0 && solids.rect_hits_solid(l, b, r, b) {
        let floor_top = b.div_euclid(TILE) * TILE;
        s.y = pixels(floor_top - SUPERBALL_SIZE);
        s.vy = -SUPERBALL_BOUNCE;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::level::Solids;

    fn floor() -> Solids {
        Solids::from_rows(&[
            &".".repeat(20),
            &".".repeat(20),
            &".".repeat(20),
            &"#".repeat(20),
        ])
    }

    #[test]
    fn superball_bounces_off_the_floor() {
        let solids = floor();
        let mut s = Superball::new(40, 8, false);
        // Let it fall and hit the floor at least once.
        let mut bounced = false;
        for _ in 0..30 {
            update_superball(&mut s, &solids);
            if s.vy < 0 {
                bounced = true;
                break;
            }
        }
        assert!(bounced, "it should bounce up off the floor");
    }

    #[test]
    fn superball_fizzles_after_its_life() {
        let solids = floor();
        let mut s = Superball::new(40, 8, false);
        let mut alive = true;
        for _ in 0..(SUPERBALL_LIFE + 2) {
            alive = update_superball(&mut s, &solids);
        }
        assert!(!alive, "it should fizzle eventually");
    }
}
