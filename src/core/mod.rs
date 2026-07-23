//! Deterministic game simulation.
//!
//! This is the heart of the reproduction: game state and the rules that advance
//! it. It holds no rendering code and does no I/O. Given the same start state
//! and the same input, stepping produces the same result every time. That
//! determinism is what makes tests and headless screenshots reliable.
//!
//! Filled in as the milestones land: the world, entities (Mario, enemies,
//! items), physics, and collision all live here. For now this defines the
//! outermost shape, a stepping loop over an input snapshot.

pub mod animation;
pub mod block;
pub mod enemy;
pub mod entity;
pub mod level;
pub mod physics;
pub mod powerup;
pub mod superball;

use crate::input::Buttons;

/// The whole game state at one instant. Grows as systems are added.
#[derive(Debug, Default, Clone)]
pub struct GameState {
    /// Frames advanced since the state was created. A placeholder that proves
    /// stepping is deterministic until real state arrives.
    pub frame: u64,
}

impl GameState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Advance the simulation by one frame given the buttons held this frame.
    /// Real systems (physics, collision, entities) hang off this call later.
    pub fn step(&mut self, _buttons: Buttons) {
        self.frame = self.frame.wrapping_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stepping_is_deterministic() {
        let mut a = GameState::new();
        let mut b = GameState::new();
        for _ in 0..10 {
            a.step(Buttons::default());
            b.step(Buttons::default());
        }
        assert_eq!(a.frame, b.frame);
        assert_eq!(a.frame, 10);
    }
}
