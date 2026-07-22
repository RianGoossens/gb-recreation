//! Mapping keyboard keys to Game Boy buttons.
//!
//! This stays independent of any windowing library. A frontend translates its
//! own key codes into our [`Key`] enum, and we decide which [`Button`] each key
//! drives. Keeping it here means the mapping is testable without a window and
//! is easy to remap later for custom controls.

use super::{Button, Buttons};

/// The physical keys we bind. A frontend maps its library's key codes to these.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    Z,
    X,
    Enter,
    Backspace,
}

/// The default binding, matching common Game Boy emulator controls: arrows for
/// the d-pad, Z for B, X for A, Enter for Start, Backspace for Select.
pub fn default_button(key: Key) -> Option<Button> {
    Some(match key {
        Key::Up => Button::Up,
        Key::Down => Button::Down,
        Key::Left => Button::Left,
        Key::Right => Button::Right,
        Key::Z => Button::B,
        Key::X => Button::A,
        Key::Enter => Button::Start,
        Key::Backspace => Button::Select,
    })
}

/// Build a button snapshot from the keys currently held, using the default
/// binding. Keys with no binding are ignored.
pub fn buttons_from_held<I>(held: I) -> Buttons
where
    I: IntoIterator<Item = Key>,
{
    let mut buttons = Buttons::default();
    for key in held {
        if let Some(button) = default_button(key) {
            buttons.set(button, true);
        }
    }
    buttons
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrows_map_to_the_dpad() {
        assert_eq!(default_button(Key::Left), Some(Button::Left));
        assert_eq!(default_button(Key::Up), Some(Button::Up));
    }

    #[test]
    fn action_keys_map_to_ab_and_start_select() {
        assert_eq!(default_button(Key::Z), Some(Button::B));
        assert_eq!(default_button(Key::X), Some(Button::A));
        assert_eq!(default_button(Key::Enter), Some(Button::Start));
        assert_eq!(default_button(Key::Backspace), Some(Button::Select));
    }

    #[test]
    fn held_keys_combine_into_one_snapshot() {
        let buttons = buttons_from_held([Key::Right, Key::X]);
        assert!(buttons.is_held(Button::Right));
        assert!(buttons.is_held(Button::A));
        assert!(!buttons.is_held(Button::B));
        assert!(!buttons.is_held(Button::Left));
    }

    #[test]
    fn empty_input_holds_nothing() {
        let buttons = buttons_from_held([]);
        assert_eq!(buttons, Buttons::default());
    }
}
