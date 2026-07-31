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

/// Small Mario's collision width, measured off the cartridge by walling him
/// into corridors of seven different widths and taking the room he had minus
/// the room he used (`tools/measure_mario_box.py`). All seven agree on 11. The
/// cartridge draws him 16 px across, from two sprites, so the box is well
/// inside what is on screen.
pub const SMALL_WIDTH: i32 = 11;

/// Small Mario's collision height, from the same subtraction with a ceiling
/// over him instead of walls beside him. Three headrooms agree on 12.
pub const SMALL_HEIGHT: i32 = 12;

/// Big Mario's collision height. Unmeasured: no run has reached a mushroom on
/// the cartridge yet, so this stays at twice the small sprite's tile height.
/// Labelled as a stand-in in `docs/reference/faithfulness.md`.
pub const BIG_HEIGHT: i32 = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Facing {
    Left,
    Right,
}

/// Mario's power level. Drives his height, what a hit does, and whether he can
/// throw a superball.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Power {
    Small,
    Big,
    /// Big and able to throw superballs (from a flower).
    Fire,
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
    /// Frames spent rising since the current jump started, while the button
    /// is still held. The held rise is capped at a fixed frame count rather
    /// than by its own (much smaller) deceleration ever reaching zero; see
    /// `docs/reference/physics.md`. Reset to 0 on takeoff.
    pub rise_frames: i32,
    /// True while rising from a stomp rather than from a jump. A stomp bounce
    /// decays the same way a released jump does even when the jump button is
    /// held, which the cartridge shows directly (see `STOMP_BOUNCE`), so it
    /// must skip the held-rise regimes. Cleared on landing and on a new jump.
    pub bouncing: bool,
    /// Cleared when Mario takes a fatal hit. The game turns this back on when it
    /// respawns him.
    pub alive: bool,
    /// Frames of invulnerability after shrinking, so one touch does not chain
    /// into a second hit. Counts down to zero.
    pub invuln: u32,
    /// Frames of star invincibility remaining. While positive, touching an enemy
    /// defeats it instead of hurting Mario.
    pub invincible: u32,
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
            rise_frames: 0,
            bouncing: false,
            alive: true,
            invuln: 0,
            invincible: 0,
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

    /// Collision box in pixels, which is smaller than what gets drawn: the
    /// cartridge draws Mario from two 8x16 sprites and collides a narrower box
    /// inside them.
    pub fn size(&self) -> (i32, i32) {
        match self.power {
            Power::Small => (SMALL_WIDTH, SMALL_HEIGHT),
            Power::Big | Power::Fire => (SMALL_WIDTH, BIG_HEIGHT),
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
        assert_eq!(m.size(), (SMALL_WIDTH, SMALL_HEIGHT));
        m.power = Power::Big;
        assert_eq!(m.size(), (SMALL_WIDTH, BIG_HEIGHT));
        const { assert!(BIG_HEIGHT > SMALL_HEIGHT) };
    }

    #[test]
    fn the_collision_box_is_narrower_than_the_sprite() {
        // The cartridge draws Mario 16 px across, from two sprites, and
        // collides 11 (`tools/measure_mario_box.py`).
        assert_eq!(Mario::new(0, 0).size(), (11, 12));
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
