//! Moving platforms, measured from the cartridge.
//!
//! World 1-1's last two objects sit either side of the exit door and carry
//! Mario, which is how its raised exit is reachable. Everything here comes
//! from watching them: `tools/probe_lift.py` for the fact that they hold him
//! up at all, `tools/measure_lift.py` for the surface, and
//! `tools/measure_enemy_walk.py` for the cycles. See
//! `docs/reference/objects.md`.
//!
//! A lift moves on one axis only, one pixel every two frames, and reverses on
//! a fixed frame count. It supports Mario from above the way a one-way
//! platform does, and carries him while he stands on it.

use crate::core::entity::{pixels, Mario, SUBPIXEL};

/// A lift is 16 pixels across, drawn from two 8-pixel sprites.
pub const LIFT_WIDTH: i32 = 16;
/// Only its top edge matters for collision, but it needs some depth so a rect
/// test has something to hit.
pub const LIFT_HEIGHT: i32 = 8;
/// One pixel every two frames, on whichever axis the lift runs.
pub const LIFT_STEP_FRAMES: u32 = 2;
/// Frames between reversals for the vertical lift: 120, which is 60 pixels.
pub const VERTICAL_HALF_CYCLE: u32 = 120;
/// Frames between reversals for the horizontal lift: 106, which is 53 pixels.
pub const HORIZONTAL_HALF_CYCLE: u32 = 106;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiftAxis {
    Vertical,
    Horizontal,
}

impl LiftAxis {
    pub fn half_cycle(self) -> u32 {
        match self {
            LiftAxis::Vertical => VERTICAL_HALF_CYCLE,
            LiftAxis::Horizontal => HORIZONTAL_HALF_CYCLE,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lift {
    /// Top-left in pixels. Lifts move in whole pixels, so no subpixel here.
    pub x: i32,
    pub y: i32,
    pub axis: LiftAxis,
    /// +1 or -1 along the axis.
    pub direction: i32,
    /// Frames since the last reversal.
    pub phase: u32,
    /// Frames since the last pixel of movement.
    pub tick: u32,
}

impl Lift {
    pub fn new(x: i32, y: i32, axis: LiftAxis) -> Self {
        Self {
            x,
            y,
            axis,
            direction: 1,
            phase: 0,
            tick: 0,
        }
    }

    /// Pixel edges (left, top, right, bottom), inclusive.
    pub fn edges(&self) -> (i32, i32, i32, i32) {
        (
            self.x,
            self.y,
            self.x + LIFT_WIDTH - 1,
            self.y + LIFT_HEIGHT - 1,
        )
    }

    /// Advance a frame. Returns how far it moved, as (dx, dy) in pixels, so a
    /// rider can be carried by exactly the same amount.
    pub fn step(&mut self) -> (i32, i32) {
        // The move comes before the reversal. Turning first spends the frame's
        // own pixel going back the way it came, and a half cycle then covers
        // 58 pixels instead of the 60 that was measured.
        self.tick += 1;
        let mut step = (0, 0);
        if self.tick >= LIFT_STEP_FRAMES {
            self.tick = 0;
            step = match self.axis {
                LiftAxis::Vertical => (0, self.direction),
                LiftAxis::Horizontal => (self.direction, 0),
            };
            self.x += step.0;
            self.y += step.1;
        }
        self.phase += 1;
        if self.phase >= self.axis.half_cycle() {
            self.phase = 0;
            self.direction = -self.direction;
        }
        step
    }
}

/// Land Mario on any lift he came down onto this frame, and carry him along
/// with the one he is standing on.
///
/// `was_bottom` is where his feet were before the frame's movement, which is
/// what makes the support one-way: he lands only when he crosses the top edge
/// from above, and jumps up through it otherwise.
pub fn ride_lifts(mario: &mut Mario, lifts: &[Lift], moves: &[(i32, i32)], was_bottom: i32) {
    let (w, h) = mario.size();
    let left = mario.pixel_x();
    let right = left + w - 1;

    for (lift, &(dx, dy)) in lifts.iter().zip(moves) {
        let (ll, lt, lr, _lb) = lift.edges();
        if right < ll || left > lr {
            continue;
        }
        // Two ways to be on it, and both are needed. Landing means his feet
        // swept across the surface this frame. Staying means they were resting
        // on it before it moved: without that, a lift on its way down leaves
        // him a pixel behind every single frame and he never rides it at all.
        let feet = mario.pixel_y() + h;
        let was_feet = was_bottom + 1;
        let was_top = lt - dy;
        let landed = was_feet <= was_top && feet >= lt;
        let stayed = was_feet == was_top;
        if mario.vy < 0 || !(landed || stayed) {
            continue;
        }
        mario.y = pixels(lt - h);
        mario.vy = 0;
        mario.on_ground = true;
        mario.x += dx * SUBPIXEL;
        return;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lift_moves_a_pixel_every_two_frames() {
        let mut lift = Lift::new(0, 100, LiftAxis::Vertical);
        assert_eq!(lift.step(), (0, 0));
        assert_eq!(lift.step(), (0, 1));
        assert_eq!(lift.y, 101);
    }

    #[test]
    fn the_vertical_lift_covers_sixty_pixels_before_turning() {
        let mut lift = Lift::new(0, 0, LiftAxis::Vertical);
        for _ in 0..VERTICAL_HALF_CYCLE {
            lift.step();
        }
        assert_eq!(lift.y, 60);
        assert_eq!(lift.direction, -1);
        for _ in 0..VERTICAL_HALF_CYCLE {
            lift.step();
        }
        assert_eq!(lift.y, 0);
    }

    #[test]
    fn the_horizontal_lift_covers_fifty_three_pixels_before_turning() {
        let mut lift = Lift::new(0, 0, LiftAxis::Horizontal);
        for _ in 0..HORIZONTAL_HALF_CYCLE {
            lift.step();
        }
        assert_eq!(lift.x, 53);
        assert_eq!(lift.direction, -1);
    }

    #[test]
    fn mario_lands_on_a_lift_and_is_carried_by_it() {
        let mut mario = Mario::new(0, 0);
        let lift = Lift::new(0, 40, LiftAxis::Horizontal);
        let (_w, h) = mario.size();
        // Feet one pixel above the lift's top, coming down.
        mario.y = pixels(40 - h + 1);
        mario.vy = SUBPIXEL;
        ride_lifts(&mut mario, &[lift], &[(1, 0)], 40 - 2);
        assert_eq!(mario.pixel_y() + h, 40, "his feet rest on the top edge");
        assert!(mario.on_ground);
        assert_eq!(mario.vy, 0);
        assert_eq!(mario.pixel_x(), 1, "carried by the lift's own pixel");
    }

    #[test]
    fn a_rising_mario_passes_through_a_lift() {
        let mut mario = Mario::new(0, 0);
        let lift = Lift::new(0, 40, LiftAxis::Vertical);
        let (_w, h) = mario.size();
        mario.y = pixels(40 - h + 1);
        mario.vy = -SUBPIXEL;
        ride_lifts(&mut mario, &[lift], &[(0, 0)], 60);
        assert!(!mario.on_ground, "a jump goes up through it");
        assert_eq!(mario.vy, -SUBPIXEL);
    }

    #[test]
    fn a_lift_to_the_side_does_not_catch_him() {
        let mut mario = Mario::new(0, 0);
        let lift = Lift::new(64, 40, LiftAxis::Vertical);
        let (_w, h) = mario.size();
        mario.y = pixels(40 - h + 1);
        mario.vy = SUBPIXEL;
        ride_lifts(&mut mario, &[lift], &[(0, 0)], 40 - 2);
        assert!(!mario.on_ground);
    }

    #[test]
    fn a_lift_only_runs_on_its_own_axis() {
        let mut vertical = Lift::new(10, 10, LiftAxis::Vertical);
        let mut horizontal = Lift::new(10, 10, LiftAxis::Horizontal);
        for _ in 0..40 {
            vertical.step();
            horizontal.step();
        }
        assert_eq!(vertical.x, 10);
        assert_eq!(horizontal.y, 10);
    }
}
