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

/// A lift is 24 pixels across, drawn as the same tile three times over.
///
/// This was 16 for as long as the drawing was unknown, and 16 never explained
/// the measurement: Mario is held over a 29 pixel window of his own position
/// (`tools/measure_lift.py`, swept a pixel at a time), which a 16 pixel
/// surface cannot produce for any foot width and a 24 pixel one produces for
/// a foot of 6. Reading the sprites off the running game
/// (`tools/measure_object_sprites.py`) gave the 24 independently.
pub const LIFT_WIDTH: i32 = 24;
/// How much of Mario the surface test actually uses, centred in his box. See
/// [`ride_lifts`].
pub const FOOT_WIDTH: i32 = 6;
/// Only its top edge matters for collision, but it needs some depth so a rect
/// test has something to hit.
pub const LIFT_HEIGHT: i32 = 8;
/// One pixel every two frames, on whichever axis the lift runs.
pub const LIFT_STEP_FRAMES: u32 = 2;
/// Frames between reversals for the vertical lift: 120, which is 60 pixels.
pub const VERTICAL_HALF_CYCLE: u32 = 120;
/// Frames between reversals for the horizontal lift: 106, which is 53 pixels.
pub const HORIZONTAL_HALF_CYCLE: u32 = 106;

/// A drop block (kind `0x36`) is one tile across, drawn as a single `0xEE`.
///
/// Measured as a support window of 13 pixels of Mario's own x
/// (`tools/probe_drop_block_support.py`), and 8 + FOOT_WIDTH - 1 is 13. That
/// is the same foot the lift's 29 gave over a 24 pixel surface, from a
/// different surface width, so the two agree without being fitted to each
/// other. The blocks come in rows, which is why the sprite survey never
/// caught this kind: its neighbour eight pixels right was always inside the
/// window.
pub const DROP_WIDTH: i32 = 8;
/// Frames a drop block stays put after Mario stands on it, before it starts
/// down. Nine in World 1-2 and nine again in World 1-3.
pub const DROP_DELAY: u32 = 9;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motion {
    /// A lift, running back and forth on one axis forever.
    Cycle(LiftAxis),
    /// A drop block: still until stood on, then it falls a pixel a frame and
    /// does not stop or come back.
    Drop,
}

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

    /// Which way a lift sets off from the position its record decodes to.
    ///
    /// Both directions were +1 here for as long as nobody had looked, and one
    /// of them was wrong. `tools/measure_lift_phase.py` catches the slot on
    /// the frame the game fills it and then freezes the camera, so every pixel
    /// that follows is the object's own: the vertical lift goes down and the
    /// horizontal one goes left, in World 1-1 and again in World 1-2. Each
    /// runs a full half cycle before reversing (119 and 105 frames traced
    /// against 120 and 106), so a lift is created at the end of its travel
    /// rather than partway along it.
    ///
    /// World 1-2's third gap is what made this worth measuring. Its lift sets
    /// off from column 232 and the ledge ends at 223: going right it never
    /// comes nearer than 72 pixels, which no jump in the engine covers, and
    /// going left it comes back to column 225.
    pub fn first_direction(self) -> i32 {
        match self {
            LiftAxis::Vertical => 1,
            LiftAxis::Horizontal => -1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lift {
    /// Top-left in pixels. Lifts move in whole pixels, so no subpixel here.
    pub x: i32,
    pub y: i32,
    pub motion: Motion,
    /// +1 or -1 along the axis.
    pub direction: i32,
    /// Frames since the last reversal, or since a drop block was stood on.
    pub phase: u32,
    /// Frames since the last pixel of movement.
    pub tick: u32,
    /// Set once Mario has stood on a drop block. It never clears.
    pub triggered: bool,
}

impl Lift {
    pub fn new(x: i32, y: i32, axis: LiftAxis) -> Self {
        Self::with_motion_at(x, y, Motion::Cycle(axis))
    }

    pub fn drop_block(x: i32, y: i32) -> Self {
        Self::with_motion_at(x, y, Motion::Drop)
    }

    pub fn with_motion_at(x: i32, y: i32, motion: Motion) -> Self {
        Self {
            x,
            y,
            motion,
            direction: match motion {
                Motion::Cycle(axis) => axis.first_direction(),
                Motion::Drop => 1,
            },
            phase: 0,
            tick: 0,
            triggered: false,
        }
    }

    pub fn width(&self) -> i32 {
        match self.motion {
            Motion::Cycle(_) => LIFT_WIDTH,
            Motion::Drop => DROP_WIDTH,
        }
    }

    /// Pixel edges (left, top, right, bottom), inclusive.
    pub fn edges(&self) -> (i32, i32, i32, i32) {
        (
            self.x,
            self.y,
            self.x + self.width() - 1,
            self.y + LIFT_HEIGHT - 1,
        )
    }

    /// Advance a frame. Returns how far it moved, as (dx, dy) in pixels, so a
    /// rider can be carried by exactly the same amount.
    pub fn step(&mut self) -> (i32, i32) {
        let axis = match self.motion {
            Motion::Cycle(axis) => axis,
            Motion::Drop => return self.drop_step(),
        };
        // The move comes before the reversal. Turning first spends the frame's
        // own pixel going back the way it came, and a half cycle then covers
        // 58 pixels instead of the 60 that was measured.
        self.tick += 1;
        let mut step = (0, 0);
        if self.tick >= LIFT_STEP_FRAMES {
            self.tick = 0;
            step = match axis {
                LiftAxis::Vertical => (0, self.direction),
                LiftAxis::Horizontal => (self.direction, 0),
            };
            self.x += step.0;
            self.y += step.1;
        }
        self.phase += 1;
        if self.phase >= axis.half_cycle() {
            self.phase = 0;
            self.direction = -self.direction;
        }
        step
    }

    /// A drop block waits DROP_DELAY frames after being stood on, then
    /// descends a pixel every frame for as long as it exists. Traced in World
    /// 1-2 and again in 1-3: the slot's y is unchanged for nine frames and
    /// then rises by exactly one per frame, with the rider's gap fixed at 10
    /// the whole way down.
    fn drop_step(&mut self) -> (i32, i32) {
        if !self.triggered {
            return (0, 0);
        }
        self.phase += 1;
        if self.phase < DROP_DELAY {
            return (0, 0);
        }
        self.y += 1;
        (0, 1)
    }
}

/// Land Mario on any lift he came down onto this frame, and carry him along
/// with the one he is standing on.
///
/// `was_bottom` is where his feet were before the frame's movement, which is
/// what makes the support one-way: he lands only when he crosses the top edge
/// from above, and jumps up through it otherwise.
pub fn ride_lifts(mario: &mut Mario, lifts: &mut [Lift], moves: &[(i32, i32)], was_bottom: i32) {
    let (w, h) = mario.size();
    // The cartridge does not test his whole body against the surface. Sweeping
    // his position across a lift one pixel at a time holds him over a window
    // of 29 (`tools/measure_lift.py`), and the lift is 24 wide, so what is
    // being tested is FOOT_WIDTH pixels centred in his box: 24 + 6 - 1 = 29.
    // Both edges of the measured window agree on the same centring.
    let inset = (w - FOOT_WIDTH) / 2;
    let left = mario.pixel_x() + inset;
    let right = left + FOOT_WIDTH - 1;

    for (lift, &(dx, dy)) in lifts.iter_mut().zip(moves) {
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
        lift.triggered = true;
        return;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The cartridge holds Mario over a 29 pixel window of his own position,
    /// swept a pixel at a time in `tools/measure_lift.py`. That number is the
    /// whole reason the lift is 24 wide and the foot test is 6: no other pair
    /// of a tile-multiple surface and a sensible foot produces 29.
    #[test]
    fn mario_is_held_across_the_twenty_nine_pixels_the_cartridge_holds_him() {
        let lift = Lift::new(100, 100, LiftAxis::Vertical);
        let held: Vec<i32> = (60..160)
            .filter(|&x| {
                let mut mario = Mario::new(x, 100 - 12);
                mario.vy = 1;
                ride_lifts(&mut mario, &mut [lift], &[(0, 0)], 100 - 1);
                mario.on_ground
            })
            .collect();
        assert_eq!(
            held.len(),
            29,
            "held from {:?} to {:?}",
            held.first(),
            held.last()
        );
    }

    /// The other half of the same measurement. The drop block's surface is 8
    /// wide against the lift's 24, and the window it holds Mario over is 13
    /// against 29. Both are `surface + FOOT_WIDTH - 1` for the same foot, from
    /// two runs that shared no number.
    #[test]
    fn a_drop_block_holds_him_across_the_thirteen_the_cartridge_holds_him() {
        let held: Vec<i32> = (60..160)
            .filter(|&x| {
                let mut mario = Mario::new(x, 100 - 12);
                mario.vy = 1;
                let mut blocks = [Lift::drop_block(100, 100)];
                ride_lifts(&mut mario, &mut blocks, &[(0, 0)], 100 - 1);
                mario.on_ground
            })
            .collect();
        assert_eq!(held.len(), 13, "held from {:?}", held.first());
    }

    #[test]
    fn a_drop_block_stays_put_until_it_is_stood_on() {
        let mut block = Lift::drop_block(0, 40);
        for _ in 0..600 {
            assert_eq!(block.step(), (0, 0));
        }
        assert_eq!(block.y, 40, "the 600 frame trace never saw it move");
    }

    /// The traced arc: nine frames of nothing, then a pixel every frame with
    /// no let up. `tools/probe_drop_block_support.py`, twice.
    #[test]
    fn a_drop_block_waits_nine_frames_and_then_falls_a_pixel_a_frame() {
        let mut mario = Mario::new(0, 40 - 12);
        let mut blocks = [Lift::drop_block(0, 40)];
        mario.vy = 1;
        ride_lifts(&mut mario, &mut blocks, &[(0, 0)], 40 - 1);
        assert!(mario.on_ground);
        assert!(blocks[0].triggered);

        for frame in 0..DROP_DELAY - 1 {
            assert_eq!(blocks[0].step(), (0, 0), "frame {frame} after the touch");
        }
        assert_eq!(blocks[0].y, 40);
        for frame in 0..50 {
            assert_eq!(blocks[0].step(), (0, 1), "frame {frame} of the fall");
        }
        assert_eq!(blocks[0].y, 90);
    }

    /// It carries him down, the way the trace's fixed gap of 10 does.
    #[test]
    fn a_drop_block_takes_mario_with_it() {
        let mut mario = Mario::new(0, 40 - 12);
        let mut blocks = [Lift::drop_block(0, 40)];
        mario.vy = 1;
        ride_lifts(&mut mario, &mut blocks, &[(0, 0)], 40 - 1);
        let (_w, h) = mario.size();
        for _ in 0..DROP_DELAY + 20 {
            let was_bottom = mario.pixel_y() + h - 1;
            let moves = [blocks[0].step()];
            ride_lifts(&mut mario, &mut blocks, &moves, was_bottom);
        }
        assert_eq!(
            mario.pixel_y() + h,
            blocks[0].y,
            "his feet stay on its top edge all the way down"
        );
        assert_eq!(blocks[0].y, 40 + 20 + 1);
    }

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
        assert_eq!(lift.x, -53);
        assert_eq!(lift.direction, 1);
    }

    /// A lift sets off away from its record's position and comes back to it.
    /// Traced on the cartridge in two levels: down for the vertical one, left
    /// for the horizontal one, a full half cycle each way.
    #[test]
    fn a_lift_sets_off_the_way_the_cartridge_sends_it() {
        for (axis, expect) in [
            (LiftAxis::Vertical, (0, 60)),
            (LiftAxis::Horizontal, (-53, 0)),
        ] {
            let mut lift = Lift::new(0, 0, axis);
            let half = axis.half_cycle();
            for _ in 0..half {
                lift.step();
            }
            assert_eq!((lift.x, lift.y), expect, "half a cycle of {axis:?}");
            for _ in 0..half {
                lift.step();
            }
            assert_eq!((lift.x, lift.y), (0, 0), "and back to where it started");
        }
    }

    #[test]
    fn mario_lands_on_a_lift_and_is_carried_by_it() {
        let mut mario = Mario::new(0, 0);
        let lift = Lift::new(0, 40, LiftAxis::Horizontal);
        let (_w, h) = mario.size();
        // Feet one pixel above the lift's top, coming down.
        mario.y = pixels(40 - h + 1);
        mario.vy = SUBPIXEL;
        ride_lifts(&mut mario, &mut [lift], &[(1, 0)], 40 - 2);
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
        ride_lifts(&mut mario, &mut [lift], &[(0, 0)], 60);
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
        ride_lifts(&mut mario, &mut [lift], &[(0, 0)], 40 - 2);
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
