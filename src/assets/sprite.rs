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

/// Blocks across one row of a size.
pub const BLOCKS: usize = 8;

/// The blocks a walk cycles through, in order.
///
/// Traced on the running game (`tools/trace_mario_frames.py`): holding right
/// cycles the top left sprite through tiles `0x00`, `0x02`, `0x04` and back,
/// each held about four frames. The still pose is part of the walk rather than
/// separate from it, which is why there are three and not two.
pub const WALK_BLOCKS: [usize; 3] = [0, 1, 2];

/// The block Mario is drawn in while he is off the ground.
///
/// The same trace, jumping from a stand and jumping from a run: the sprite is
/// tile `0x08` for every airborne frame, through both the rising and falling
/// values of the cartridge's own phase byte. So there is one jump pose and it
/// does not change on the way down. This is one of the five blocks per row
/// that reading the atlas alone could not identify.
pub const JUMP_BLOCK: usize = 4;
/// The pose for changing direction at speed, block 5.
///
/// Seen twice in the trace, both times for exactly 7 frames, at the moment the
/// input reversed while he was running: once as tile `0x0A` unflipped after
/// pressing left while moving right, once as `0x0B` mirrored after the
/// opposite. Nothing else in the trace draws it.
pub const SKID_BLOCK: usize = 5;

/// How long a walking block is held, in frames.
///
/// Four, and it does not change with his speed: every stretch of the trace
/// holds each block for exactly four frames, walking and with B held, over 28,
/// 33 and 9 changes of pose.
pub const WALK_HOLD: u32 = 4;

/// The tile ids of one of Mario's frames, in reading order: top left, top
/// right, bottom left, bottom right.
///
/// `block` indexes across a size's row of eight. [`WALK_BLOCKS`] are the three
/// a walk cycles through, [`JUMP_BLOCK`] is the airborne pose and
/// [`SKID_BLOCK`] the reversing one. The last two blocks of each row hold
/// other characters rather than Mario, and the third is a Mario pose nothing
/// in the trace draws.
pub fn mario_frame(size: Size, block: usize) -> [u8; 4] {
    let row = match size {
        Size::Small => 0,
        Size::Big => 2,
    };
    let column = (block % BLOCKS) * FRAME_TILES;
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
pub fn mario_pixels(sheet: &[Tile], size: Size, block: usize) -> [[u8; FRAME_SIZE]; FRAME_SIZE] {
    let ids = mario_frame(size, block);
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

// Which tiles each object kind is drawn from, measured on the cartridge with
// `tools/measure_object_sprites.py`: play through the worlds, and for every
// slot that holds a kind, collect the OAM entries the game placed near it and
// report them as offsets from the slot's own position. Objects with a
// neighbour or Mario inside the window are skipped, since two overlapping
// sets would each be labelled with the other's tiles.
//
// The anchor came out the same for every kind measured: the lowest row of the
// drawing sits at the slot's y and the left column one pixel right of its x.
// The game runs in 8x8 sprite mode, so an OAM tile id is an atlas id with
// nothing to decode, and the Fly's four ids (`0xA0`, `0xA1`, `0xB0`, `0xB1`)
// are the 2x2 block of a 16-wide sheet that Mario's frames are, which
// confirms that layout from the running game rather than from the picture.
//
// The drawings are taller and wider than the 8 by 8 collision box the engine
// gives an enemy. That box has never been measured on the cartridge, so it
// stays as it is and the drawing is anchored to its bottom left corner
// (`docs/reference/faithfulness.md`).

/// One tile of an object's drawing, placed relative to the bottom left corner
/// of the object.
///
/// `dy` counts up from the bottom edge, so the lowest row of a drawing is
/// `-8` and the row above it `-16`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Piece {
    pub dx: i32,
    pub dy: i32,
    pub tile: u8,
}

const fn piece(dx: i32, dy: i32, tile: u8) -> Piece {
    Piece { dx, dy, tile }
}

/// The Chibibo (object kind `0x00`), a single tile.
pub const CHIBIBO: &[Piece] = &[piece(0, -8, 0x90)];

/// The Nokobon (kind `0x04`), one tile wide and two tall.
pub const NOKOBON: &[Piece] = &[piece(0, -16, 0x96), piece(0, -8, 0x97)];

/// The Fly (kind `0x0E`), a 2x2 block out of the per-world band.
pub const FLY: &[Piece] = &[
    piece(0, -16, 0xA0),
    piece(8, -16, 0xA1),
    piece(0, -8, 0xB0),
    piece(8, -8, 0xB1),
];

/// The Falling Slab (kind `0x0C`), two tiles side by side.
pub const FALLING_SLAB: &[Piece] = &[piece(0, -8, 0xDD), piece(8, -8, 0xDE)];

/// A lift (kinds `0x0A` and `0x0B`), the same tile three times over. Both
/// axes draw identically; only their movement differs.
pub const LIFT: &[Piece] = &[
    piece(0, -8, 0xEF),
    piece(8, -8, 0xEF),
    piece(16, -8, 0xEF),
];

/// A drop block (kind `0x36`), one tile, the id next door to the lift's.
///
/// The blocks are placed in rows, so the sprite survey always had a second
/// one inside its window and skipped the kind; this came from a run with a
/// single block isolated (`tools/probe_drop_block_support.py`). Its OAM
/// attribute byte has bit 7 set, so the background covers it, which makes it
/// the third kind measured with the priority bit after `0x02` and `0x10`.
pub const DROP_BLOCK: &[Piece] = &[piece(0, -8, 0xEE)];

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
