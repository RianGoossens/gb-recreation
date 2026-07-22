//! Input: the Game Boy's eight buttons.
//!
//! See [`mapping`] for turning keyboard keys into a button snapshot.
//!
//! A frontend (keyboard, gamepad) maps physical keys onto these buttons and
//! hands a [`Buttons`] snapshot to [`crate::core::GameState::step`] each frame.
//! The core only ever sees this snapshot, so gameplay does not care where the
//! input came from. That also lets tests drive the game with scripted input.

pub mod mapping;

/// The eight Game Boy buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Up,
    Down,
    Left,
    Right,
    A,
    B,
    Start,
    Select,
}

/// Which buttons are held this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Buttons {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
    a: bool,
    b: bool,
    start: bool,
    select: bool,
}

impl Buttons {
    pub fn is_held(&self, button: Button) -> bool {
        match button {
            Button::Up => self.up,
            Button::Down => self.down,
            Button::Left => self.left,
            Button::Right => self.right,
            Button::A => self.a,
            Button::B => self.b,
            Button::Start => self.start,
            Button::Select => self.select,
        }
    }

    pub fn set(&mut self, button: Button, held: bool) {
        match button {
            Button::Up => self.up = held,
            Button::Down => self.down = held,
            Button::Left => self.left = held,
            Button::Right => self.right = held,
            Button::A => self.a = held,
            Button::B => self.b = held,
            Button::Start => self.start = held,
            Button::Select => self.select = held,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_holds_nothing() {
        let b = Buttons::default();
        assert!(!b.is_held(Button::A));
        assert!(!b.is_held(Button::Start));
    }

    #[test]
    fn set_toggles_one_button_only() {
        let mut b = Buttons::default();
        b.set(Button::Left, true);
        assert!(b.is_held(Button::Left));
        assert!(!b.is_held(Button::Right));
    }
}
