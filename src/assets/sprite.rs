//! The cartridge's sprite graphics: the half of the tile atlas the background
//! never touches.
//!
//! A level's tile data covers VRAM `0x8000` to `0x9800` and the background
//! reads from `0x8800` up (see [`super::level`]). Everything below `0x9000` is
//! the object atlas, and the whole of it has been in the tree since the tile
//! copy was pinned; what was missing was which tiles are which object.
//!
//! Part of the atlas is per world. A world's tile copy overwrites ids `0xA0`
//! through `0xDC`, which is where its own enemies are drawn from; the rest,
//! Mario included, is the same in all four.
//!
//! Mario is at the start of it. The atlas is stored as a picture 16 tiles
//! wide, so a frame is a block of tiles rather than a run of consecutive ids:
//! two columns wide and two rows tall, 16 by 16 pixels. Reading it as
//! consecutive ids instead produces a scramble, which is how the arrangement
//! was settled.
//!
//! Which figure is Mario is not a guess either. Small Mario's first frame
//! draws ink over 10 by 12 pixels of its block, and Mario's collision box was
//! measured on the cartridge, by a route with nothing to do with graphics, at
//! 11 by 12 (`tools/measure_mario_box.py`). The heights agree exactly and a
//! box a pixel wider than the drawing is ordinary.

use crate::tiles::Tile;
use super::level::tiles_for_world;
use super::AssetError;

/// Tiles across the atlas as stored.
pub const SHEET_COLUMNS: usize = 16;
/// A character frame is two tiles by two.
pub const FRAME_TILES: usize = 2;
/// A frame's size in pixels.
pub const FRAME_SIZE: usize = FRAME_TILES * 8;

/// The ids a world's own tile copy replaces in the object atlas, which is
/// where its enemies are drawn from. Everything else, Mario included, is the
/// same in all four worlds: comparing the four sheets byte for byte leaves
/// exactly these 61 ids differing and no others.
pub const PER_WORLD: std::ops::RangeInclusive<u8> = 0xA0..=0xDC;

/// Every tile of the object atlas as a world loads it, by id.
pub fn sprite_sheet(rom: &[u8], world: usize) -> Result<Vec<Tile>, AssetError> {
    let vram = tiles_for_world(rom, world)?;
    Ok((0..256)
        .map(|id| Tile::decode(vram[id * 16..id * 16 + 16].try_into().unwrap()))
        .collect())
}

/// Which of Mario's two sizes a frame belongs to. Small Mario's frames are the
/// atlas's first two rows and big Mario's are the next two.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Size {
    Small,
    Big,
}

/// The frames of a size, in the order they sit in the atlas. Only the first
/// three of the eight blocks in a row are Mario standing and walking; what the
/// rest are has not been settled, so they are not offered here.
pub const FRAMES: usize = 3;

/// The tile ids of one of Mario's frames, in reading order: top left, top
/// right, bottom left, bottom right.
///
/// `frame` 0 is the still pose, 1 and 2 are the two walking poses. Which frame
/// the cartridge shows at which moment is a separate question that needs the
/// running game; this only says where the pictures are.
pub fn mario_frame(size: Size, frame: usize) -> [u8; 4] {
    let row = match size {
        Size::Small => 0,
        Size::Big => 2,
    };
    let column = (frame % FRAMES) * FRAME_TILES;
    let top = row * SHEET_COLUMNS + column;
    [
        top as u8,
        (top + 1) as u8,
        (top + SHEET_COLUMNS) as u8,
        (top + SHEET_COLUMNS + 1) as u8,
    ]
}

/// The pixels of one of Mario's frames, 16 by 16, as palette indices with 0
/// meaning transparent.
pub fn mario_pixels(sheet: &[Tile], size: Size, frame: usize) -> [[u8; FRAME_SIZE]; FRAME_SIZE] {
    let ids = mario_frame(size, frame);
    let mut out = [[0u8; FRAME_SIZE]; FRAME_SIZE];
    for (i, id) in ids.iter().enumerate() {
        let (ox, oy) = ((i % 2) * 8, (i / 2) * 8);
        let tile = &sheet[*id as usize];
        for y in 0..8 {
            for x in 0..8 {
                out[oy + y][ox + x] = tile.pixels[y][x];
            }
        }
    }
    out
}

/// The box a frame actually draws in, as `(x, y, width, height)` inside its
/// 16 by 16 block.
pub fn ink_box(pixels: &[[u8; FRAME_SIZE]; FRAME_SIZE]) -> Option<(usize, usize, usize, usize)> {
    let (mut x0, mut x1, mut y0, mut y1) = (FRAME_SIZE, 0usize, FRAME_SIZE, 0usize);
    let mut any = false;
    for (y, row) in pixels.iter().enumerate() {
        for (x, &p) in row.iter().enumerate() {
            if p != 0 {
                any = true;
                x0 = x0.min(x);
                x1 = x1.max(x);
                y0 = y0.min(y);
                y1 = y1.max(y);
            }
        }
    }
    any.then(|| (x0, y0, x1 - x0 + 1, y1 - y0 + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_is_a_block_of_the_sheet_rather_than_consecutive_ids() {
        assert_eq!(mario_frame(Size::Small, 0), [0, 1, 16, 17]);
        assert_eq!(mario_frame(Size::Small, 1), [2, 3, 18, 19]);
        assert_eq!(mario_frame(Size::Big, 0), [32, 33, 48, 49]);
    }
}
