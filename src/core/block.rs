//! Interactive blocks: the ones Mario bumps from below.
//!
//! A block sits in the solid world like any other tile, but bumping it from
//! underneath does something. A question block gives up its contents once, then
//! is spent. A brick just takes the hit for now (breaking it needs big Mario,
//! which arrives with the power-up).

use crate::core::level::TILE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    /// Gives a coin when bumped.
    Question,
    /// Gives a mushroom power-up when bumped.
    PowerUp,
    Brick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Block {
    /// Top-left pixel of the block's tile.
    pub x: i32,
    pub y: i32,
    pub kind: BlockKind,
    /// A question block becomes used after it gives up its contents.
    pub used: bool,
}

impl Block {
    pub fn new(x: i32, y: i32, kind: BlockKind) -> Self {
        Self {
            x,
            y,
            kind,
            used: false,
        }
    }

    /// Pixel edges (left, top, right, bottom), inclusive.
    pub fn edges(&self) -> (i32, i32, i32, i32) {
        (self.x, self.y, self.x + TILE - 1, self.y + TILE - 1)
    }
}
