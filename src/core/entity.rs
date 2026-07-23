//! Mario as a moving entity.
//!
//! Positions and velocities are kept in subpixels (fixed point) rather than
//! whole pixels. The Game Boy tracks fractional movement this way, and we need
//! the same so slow acceleration and friction feel right and stay deterministic.

use crate::input::{Button, Buttons};

/// Subpixels per pixel. Position and velocity are integers in these units.
pub const SUBPIXEL: i32 = 256;

/// Convert a whole-pixel value to subpixels.
pub const fn pixels(n: i32) -> i32 {
    n * SUBPIXEL
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Facing {
    Left,
    Right,
}

/// Mario's power level. Drives his height and what a hit does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Power {
    Small,
    Big,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mario {
    /// Position of the top-left of the sprite, in subpixels.
    pub x: i32,
    pub y: i32,
    /// Velocity in subpixels per frame.
    pub vx: i32,
    pub vy: i32,
    pub facing: Facing,
    pub on_ground: bool,
    pub power: Power,
    /// True while the jump button is held after a jump started, so holding it
    /// does not trigger a second jump. Cleared when the button is released.
    pub jump_latched: bool,
    /// Cleared when Mario takes a fatal hit. The game turns this back on when it
    /// respawns him.
    pub alive: bool,
    /// Frames of invulnerability after shrinking, so one touch does not chain
    /// into a second hit. Counts down to zero.
    pub invuln: u32,
}

impl Mario {
    /// Place Mario at a whole-pixel position, standing still, facing right.
    pub fn new(pixel_x: i32, pixel_y: i32) -> Self {
        Self {
            x: pixels(pixel_x),
            y: pixels(pixel_y),
            vx: 0,
            vy: 0,
            facing: Facing::Right,
            on_ground: false,
            power: Power::Small,
            jump_latched: false,
            alive: true,
            invuln: 0,
        }
    }

    /// Top-left pixel position, rounding toward negative infinity so movement
    /// is consistent on both sides of zero.
    pub fn pixel_x(&self) -> i32 {
        self.x.div_euclid(SUBPIXEL)
    }

    pub fn pixel_y(&self) -> i32 {
        self.y.div_euclid(SUBPIXEL)
    }

    /// Sprite size in pixels. Small Mario is one tile, big Mario is two tall.
    pub fn size(&self) -> (i32, i32) {
        match self.power {
            Power::Small => (8, 8),
            Power::Big => (8, 16),
        }
    }

    /// Face toward the horizontal direction requested by the buttons. No held
    /// left/right leaves facing unchanged. Left and right together cancel.
    pub fn face_from_input(&mut self, buttons: Buttons) {
        let left = buttons.is_held(Button::Left);
        let right = buttons.is_held(Button::Right);
        match (left, right) {
            (true, false) => self.facing = Facing::Left,
            (false, true) => self.facing = Facing::Right,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_sets_pixel_position_and_defaults() {
        let m = Mario::new(20, 100);
        assert_eq!(m.pixel_x(), 20);
        assert_eq!(m.pixel_y(), 100);
        assert_eq!(m.vx, 0);
        assert_eq!(m.facing, Facing::Right);
        assert_eq!(m.power, Power::Small);
        assert!(!m.on_ground);
    }

    #[test]
    fn subpixel_movement_shows_up_in_whole_pixels_only_when_crossing() {
        let mut m = Mario::new(0, 0);
        m.x += SUBPIXEL / 2; // half a pixel
        assert_eq!(m.pixel_x(), 0);
        m.x += SUBPIXEL / 2; // now a whole pixel
        assert_eq!(m.pixel_x(), 1);
    }

    #[test]
    fn pixel_position_rounds_toward_negative() {
        let mut m = Mario::new(0, 0);
        m.x = -1;
        assert_eq!(m.pixel_x(), -1);
    }

    #[test]
    fn big_mario_is_taller() {
        let mut m = Mario::new(0, 0);
        assert_eq!(m.size(), (8, 8));
        m.power = Power::Big;
        assert_eq!(m.size(), (8, 16));
    }

    #[test]
    fn facing_follows_input() {
        let mut m = Mario::new(0, 0);
        let mut b = Buttons::default();
        b.set(Button::Left, true);
        m.face_from_input(b);
        assert_eq!(m.facing, Facing::Left);

        let mut b = Buttons::default();
        b.set(Button::Right, true);
        m.face_from_input(b);
        assert_eq!(m.facing, Facing::Right);

        // Both held: facing unchanged from the last set value (Right).
        let mut b = Buttons::default();
        b.set(Button::Left, true);
        b.set(Button::Right, true);
        m.face_from_input(b);
        assert_eq!(m.facing, Facing::Right);
    }
}
