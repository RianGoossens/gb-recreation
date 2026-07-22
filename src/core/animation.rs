//! Animation state for Mario.
//!
//! The visible pose is derived from movement, not stored as game logic: on the
//! ground and still is idle, on the ground and moving is a walk cycle, and off
//! the ground is the jump pose. Keeping this separate from physics means the
//! renderer can ask "what should Mario look like" without the simulation caring.

use super::entity::Mario;

/// How many frames each walk-cycle sprite is shown before advancing.
pub const WALK_FRAME_TICKS: u8 = 6;
/// Number of frames in the walk cycle.
pub const WALK_FRAMES: u8 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimState {
    Idle,
    Walk,
    Jump,
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
