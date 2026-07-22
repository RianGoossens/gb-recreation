//! Built-in scenes for screenshots and tests.
//!
//! These are our own content, not extracted game data, so they are safe to
//! render in CI and commit as golden images.

use crate::render::{render_background, Framebuffer, Palette, TileMap};
use crate::tiles::Tile;

fn solid_tile(color_index: u8) -> Tile {
    Tile {
        pixels: [[color_index; 8]; 8],
    }
}

/// Four solid shades arranged as diagonal bands, drawn through the real tilemap
/// path so the whole render pipeline is exercised.
pub fn demo() -> Framebuffer {
    let tiles: Vec<Tile> = (0..4).map(|i| solid_tile(i as u8)).collect();
    let (w, h) = (20usize, 18usize);
    let cells = (0..w * h)
        .map(|c| (((c % w) + (c / w)) % 4) as u8)
        .collect();
    let map = TileMap::new(w, h, cells);
    let mut fb = Framebuffer::new();
    render_background(&mut fb, &map, &tiles, 0, 0, &Palette::new(0xE4));
    fb
}
