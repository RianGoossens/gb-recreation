//! Super Mario Land reproduction: library root.
//!
//! The crate is split along four boundaries so the game logic stays free of
//! rendering and I/O (see CLAUDE.md):
//!
//! - [`core`]: deterministic game state and simulation. No rendering, no I/O.
//! - [`render`]: turns a game state into a pixel framebuffer.
//! - [`input`]: Game Boy button state, fed into the core each frame.
//! - [`assets`]: loads tiles, palettes, and level data into memory.
//!
//! The dependency direction is one way: `render`, `input`, and `assets` know
//! about `core` types, but `core` never depends on them. This keeps the
//! simulation deterministic and testable in isolation.

pub mod assets;
pub mod core;
pub mod input;
pub mod render;
pub mod rom;

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
