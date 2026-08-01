//! Animation state for Mario.
//!
//! The visible pose is derived from movement rather than stored as game logic:
//! on the ground and still is idle, on the ground and moving is a walk cycle,
//! off the ground is the jump pose, and moving one way while pressing the
//! other is the skid. Keeping this separate from physics means the renderer
//! can ask "what should Mario look like" without the simulation caring.
//!
//! Which pose plays when is measured, not chosen: `tools/trace_mario_frames.py`
//! reads the tile of the sprite the cartridge draws at Mario's own position
//! every frame (`docs/reference/sprites.md`).

use super::entity::{Facing, Mario};

fn facing_of(vx: i32) -> Facing {
    if vx < 0 {
        Facing::Left
    } else {
        Facing::Right
    }
}

/// How many frames each walk-cycle sprite is shown before advancing.
///
/// Four, and the same four whether he is walking or running: every stretch of
/// the trace holds each pose for exactly 4 frames, with B held and without.
pub const WALK_FRAME_TICKS: u8 = 4;
/// Number of frames in the walk cycle.
pub const WALK_FRAMES: u8 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimState {
    Idle,
    Walk,
    Jump,
    /// Moving one way with the other direction pressed. The cartridge shows
    /// its own pose for this, drawn facing the way he is still travelling
    /// rather than the way he is now pressing.
    Skid,
}

/// Tracks the current animation state and the walk-cycle position over time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Animator {
    frame: u8,
    ticks: u8,
}

impl Animator {
    pub fn new() -> Self {
        Self::default()
    }

    /// The pose implied by Mario's current movement.
    pub fn state(mario: &Mario) -> AnimState {
        if !mario.on_ground {
            AnimState::Jump
        } else if mario.vx != 0 && mario.facing == facing_of(-mario.vx) {
            AnimState::Skid
        } else if mario.vx != 0 {
            AnimState::Walk
        } else {
            AnimState::Idle
        }
    }

    /// Advance one frame. Walking cycles through the walk frames on a timer,
    /// anything else resets to the first frame.
    pub fn update(&mut self, mario: &Mario) {
        if Self::state(mario) == AnimState::Walk {
            self.ticks += 1;
            if self.ticks >= WALK_FRAME_TICKS {
                self.ticks = 0;
                self.frame = (self.frame + 1) % WALK_FRAMES;
            }
        } else {
            self.frame = 0;
            self.ticks = 0;
        }
    }

    /// Which walk-cycle frame to draw. Meaningful while walking, 0 otherwise.
    pub fn frame(&self) -> u8 {
        self.frame
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::entity::pixels;

    fn walking() -> Mario {
        let mut m = Mario::new(0, 0);
        m.on_ground = true;
        m.vx = pixels(1);
        m
    }

    #[test]
    fn state_reflects_movement() {
        let mut m = Mario::new(0, 0);
        m.on_ground = true;
        assert_eq!(Animator::state(&m), AnimState::Idle);
        m.vx = pixels(1);
        assert_eq!(Animator::state(&m), AnimState::Walk);
        m.on_ground = false;
        assert_eq!(Animator::state(&m), AnimState::Jump);
    }

    /// Pressing the opposite way while still moving is its own pose on the
    /// cartridge, and it is not the walk: the trace draws block 5 there.
    #[test]
    fn pressing_against_the_movement_is_a_skid() {
        let mut m = walking();
        assert_eq!(Animator::state(&m), AnimState::Walk);
        m.facing = Facing::Left;
        assert_eq!(Animator::state(&m), AnimState::Skid);
        // And the other way round, which is the second of the two the trace
        // caught.
        m.vx = -m.vx;
        assert_eq!(Animator::state(&m), AnimState::Walk);
        m.facing = Facing::Right;
        assert_eq!(Animator::state(&m), AnimState::Skid);
    }

    #[test]
    fn a_skid_needs_him_on_the_ground_and_moving() {
        let mut m = walking();
        m.facing = Facing::Left;
        m.on_ground = false;
        assert_eq!(Animator::state(&m), AnimState::Jump, "airborne wins");
        m.on_ground = true;
        m.vx = 0;
        assert_eq!(Animator::state(&m), AnimState::Idle, "standing still cannot skid");
    }

    #[test]
    fn walk_cycle_advances_on_the_timer() {
        let m = walking();
        let mut anim = Animator::new();
        assert_eq!(anim.frame(), 0);
        for _ in 0..WALK_FRAME_TICKS {
            anim.update(&m);
        }
        assert_eq!(anim.frame(), 1);
        for _ in 0..WALK_FRAME_TICKS {
            anim.update(&m);
        }
        assert_eq!(anim.frame(), 2);
    }

    #[test]
    fn walk_cycle_wraps() {
        let m = walking();
        let mut anim = Animator::new();
        for _ in 0..(WALK_FRAME_TICKS as usize * WALK_FRAMES as usize) {
            anim.update(&m);
        }
        assert_eq!(anim.frame(), 0);
    }

    #[test]
    fn idle_resets_the_walk_frame() {
        let walk = walking();
        let mut anim = Animator::new();
        for _ in 0..WALK_FRAME_TICKS {
            anim.update(&walk);
        }
        assert_eq!(anim.frame(), 1);

        let mut idle = Mario::new(0, 0);
        idle.on_ground = true;
        anim.update(&idle);
        assert_eq!(anim.frame(), 0);
    }
}
