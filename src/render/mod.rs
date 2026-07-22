//! Rendering: turn a game state into pixels.
//!
//! The Game Boy screen is 160x144 with four shades of gray. This module owns a
//! [`Framebuffer`] of that size and, later, the tile and sprite drawing that
//! fills it from a [`crate::core::GameState`]. Rendering only reads game state,
//! it never changes it.
//!
//! Keeping a plain framebuffer here (rather than a window) is what lets the
//! game render headlessly to a PNG for tests and the blog. The windowed
//! frontend is a thin consumer of this buffer.

use crate::{SCREEN_HEIGHT, SCREEN_WIDTH};

/// One of the Game Boy's four display shades, lightest to darkest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Shade {
    #[default]
    White,
    Light,
    Dark,
    Black,
}

/// A 160x144 buffer of shades. The rendering target for one frame.
#[derive(Debug, Clone)]
pub struct Framebuffer {
    pixels: Vec<Shade>,
}

impl Default for Framebuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl Framebuffer {
    pub fn new() -> Self {
        Self {
            pixels: vec![Shade::White; (SCREEN_WIDTH * SCREEN_HEIGHT) as usize],
        }
    }

    pub fn get(&self, x: u32, y: u32) -> Shade {
        self.pixels[Self::index(x, y)]
    }

    pub fn set(&mut self, x: u32, y: u32, shade: Shade) {
        let i = Self::index(x, y);
        self.pixels[i] = shade;
    }

    fn index(x: u32, y: u32) -> usize {
        debug_assert!(x < SCREEN_WIDTH && y < SCREEN_HEIGHT, "pixel out of bounds");
        (y * SCREEN_WIDTH + x) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_framebuffer_is_all_white() {
        let fb = Framebuffer::new();
        assert_eq!(fb.get(0, 0), Shade::White);
        assert_eq!(fb.get(SCREEN_WIDTH - 1, SCREEN_HEIGHT - 1), Shade::White);
    }

    #[test]
    fn set_then_get_round_trips() {
        let mut fb = Framebuffer::new();
        fb.set(10, 20, Shade::Black);
        assert_eq!(fb.get(10, 20), Shade::Black);
        assert_eq!(fb.get(11, 20), Shade::White);
    }
}
