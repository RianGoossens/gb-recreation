//! Rendering: turn a game state into pixels.
//!
//! The Game Boy screen is 160x144 with four shades of gray. This module owns a
//! [`Framebuffer`] of that size plus the pieces that fill it: a [`Palette`] that
//! maps tile color indices to shades, and background rendering from a
//! [`TileMap`] of tile indices. Rendering only reads game state, it never
//! changes it.
//!
//! Keeping a plain framebuffer here (rather than a window) is what lets the
//! game render headlessly to an image for tests and the blog. The windowed
//! frontend is a thin consumer of this buffer.

use crate::tiles::Tile;
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

impl Shade {
    /// Build a shade from a 2-bit value (0 lightest, 3 darkest).
    pub fn from_u2(v: u8) -> Self {
        match v & 0b11 {
            0 => Shade::White,
            1 => Shade::Light,
            2 => Shade::Dark,
            _ => Shade::Black,
        }
    }

    /// 8-bit grayscale value for image output.
    pub fn to_gray(self) -> u8 {
        match self {
            Shade::White => 255,
            Shade::Light => 170,
            Shade::Dark => 85,
            Shade::Black => 0,
        }
    }
}

/// A Game Boy palette: a BGP-style byte mapping the four color indices to four
/// shades, two bits per index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    pub bgp: u8,
}

impl Palette {
    pub fn new(bgp: u8) -> Self {
        Self { bgp }
    }

    /// Resolve a 0..=3 color index to its shade through this palette.
    pub fn shade(&self, index: u8) -> Shade {
        Shade::from_u2(self.bgp >> ((index & 0b11) * 2))
    }
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

    /// All shades as 8-bit grayscale, row major. Handy for image output.
    pub fn to_gray(&self) -> Vec<u8> {
        self.pixels.iter().map(|s| s.to_gray()).collect()
    }

    /// Draw one tile with its top-left at (ox, oy). Offsets may be negative or
    /// push the tile partly offscreen: pixels outside the screen are clipped.
    /// Every index is drawn (opaque), which is what background tiles want.
    pub fn draw_tile(&mut self, tile: &Tile, ox: i32, oy: i32, palette: &Palette) {
        for ty in 0..8 {
            let y = oy + ty as i32;
            if y < 0 || y >= SCREEN_HEIGHT as i32 {
                continue;
            }
            for tx in 0..8 {
                let x = ox + tx as i32;
                if x < 0 || x >= SCREEN_WIDTH as i32 {
                    continue;
                }
                let shade = palette.shade(tile.pixels[ty][tx]);
                self.set(x as u32, y as u32, shade);
            }
        }
    }

    fn index(x: u32, y: u32) -> usize {
        debug_assert!(x < SCREEN_WIDTH && y < SCREEN_HEIGHT, "pixel out of bounds");
        (y * SCREEN_WIDTH + x) as usize
    }
}

/// A grid of tile indices, like a Game Boy background map. Each cell names a
/// tile in an accompanying tile set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TileMap {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<u8>,
}

impl TileMap {
    pub fn new(width: usize, height: usize, cells: Vec<u8>) -> Self {
        assert_eq!(width * height, cells.len(), "cells must be width*height");
        Self {
            width,
            height,
            cells,
        }
    }

    /// Tile index at a cell, wrapping like the Game Boy background.
    pub fn cell(&self, tx: usize, ty: usize) -> u8 {
        self.cells[(ty % self.height) * self.width + (tx % self.width)]
    }
}

/// Fill the framebuffer from a background map. For each screen pixel we sample
/// the map at (screen + scroll), wrapping over the map's pixel size, exactly as
/// the Game Boy scrolls its background. Missing tile indices draw as blank.
pub fn render_background(
    fb: &mut Framebuffer,
    map: &TileMap,
    tiles: &[Tile],
    scroll_x: i32,
    scroll_y: i32,
    palette: &Palette,
) {
    let map_w = (map.width * 8) as i32;
    let map_h = (map.height * 8) as i32;
    for sy in 0..SCREEN_HEIGHT as i32 {
        let wy = (sy + scroll_y).rem_euclid(map_h);
        let ty = (wy / 8) as usize;
        let py = (wy % 8) as usize;
        for sx in 0..SCREEN_WIDTH as i32 {
            let wx = (sx + scroll_x).rem_euclid(map_w);
            let tx = (wx / 8) as usize;
            let px = (wx % 8) as usize;
            let tile_index = map.cell(tx, ty) as usize;
            if let Some(tile) = tiles.get(tile_index) {
                let shade = palette.shade(tile.pixels[py][px]);
                fb.set(sx as u32, sy as u32, shade);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid_tile(index: u8) -> Tile {
        Tile {
            pixels: [[index; 8]; 8],
        }
    }

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

    #[test]
    fn default_palette_maps_index_to_same_shade() {
        // BGP 0xE4 maps index i to shade i.
        let p = Palette::new(0xE4);
        assert_eq!(p.shade(0), Shade::White);
        assert_eq!(p.shade(1), Shade::Light);
        assert_eq!(p.shade(2), Shade::Dark);
        assert_eq!(p.shade(3), Shade::Black);
    }

    #[test]
    fn inverted_palette_swaps_shades() {
        // BGP 0x1B maps 0->3, 1->2, 2->1, 3->0.
        let p = Palette::new(0x1B);
        assert_eq!(p.shade(0), Shade::Black);
        assert_eq!(p.shade(3), Shade::White);
    }

    #[test]
    fn draw_tile_clips_negative_offsets() {
        let mut fb = Framebuffer::new();
        let p = Palette::new(0xE4);
        // Tile of index 3 (black), placed so only its bottom-right corner shows.
        fb.draw_tile(&solid_tile(3), -7, -7, &p);
        assert_eq!(fb.get(0, 0), Shade::Black);
        assert_eq!(fb.get(1, 1), Shade::White); // outside the visible corner
    }

    #[test]
    fn render_background_tiles_across_screen() {
        let mut fb = Framebuffer::new();
        let p = Palette::new(0xE4);
        // Map cell points at tile 0, which is solid color index 1 (light).
        // The whole screen should become light.
        let map = TileMap::new(1, 1, vec![0]);
        let tiles = vec![solid_tile(1)];
        render_background(&mut fb, &map, &tiles, 0, 0, &p);
        assert_eq!(fb.get(0, 0), Shade::Light);
        assert_eq!(fb.get(100, 100), Shade::Light);
        assert_eq!(fb.get(SCREEN_WIDTH - 1, SCREEN_HEIGHT - 1), Shade::Light);
    }

    #[test]
    fn render_background_respects_scroll_and_wrapping() {
        let mut fb = Framebuffer::new();
        let p = Palette::new(0xE4);
        // Tileset: tile 0 is white (index 0), tile 1 is black (index 3).
        // Map places tile 0 on the left cell, tile 1 on the right.
        let map = TileMap::new(2, 1, vec![0, 1]);
        let tiles = vec![solid_tile(0), solid_tile(3)];
        // No scroll: left 8 px white, next 8 px black.
        render_background(&mut fb, &map, &tiles, 0, 0, &p);
        assert_eq!(fb.get(0, 0), Shade::White);
        assert_eq!(fb.get(8, 0), Shade::Black);
        // Scroll right by 8: the black tile now starts at x=0.
        render_background(&mut fb, &map, &tiles, 8, 0, &p);
        assert_eq!(fb.get(0, 0), Shade::Black);
    }
}
