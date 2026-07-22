//! Super Mario Land reproduction: library root.
//!
//! Module layout is filled in as milestones land. The intent (see CLAUDE.md):
//! keep game logic, rendering, input, and asset loading in separate modules,
//! with a deterministic core that can be stepped and snapshotted for tests
//! and screenshots.

/// The Game Boy display is 160x144 pixels.
pub const SCREEN_WIDTH: u32 = 160;
pub const SCREEN_HEIGHT: u32 = 144;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screen_dimensions_are_game_boy_native() {
        assert_eq!(SCREEN_WIDTH, 160);
        assert_eq!(SCREEN_HEIGHT, 144);
    }
}
