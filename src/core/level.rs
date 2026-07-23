//! Level geometry: which tiles are solid.
//!
//! Collision works against a grid of solid or empty tiles, one bool per 8x8
//! tile. This is deliberately simple and human editable, which also serves the
//! later moddability goal: a level's solids can be written as rows of text.

use crate::core::block::BlockKind;

/// Tile size in pixels.
pub const TILE: i32 = 8;

/// A grid of solid tiles. Anything outside the grid reads as empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Solids {
    pub width: usize,
    pub height: usize,
    cells: Vec<bool>,
}

impl Solids {
    pub fn new(width: usize, height: usize, cells: Vec<bool>) -> Self {
        assert_eq!(width * height, cells.len(), "cells must be width*height");
        Self {
            width,
            height,
            cells,
        }
    }

    pub fn empty(width: usize, height: usize) -> Self {
        Self::new(width, height, vec![false; width * height])
    }

    /// Build from rows of text. Solid tiles are `#` and the block markers `?`
    /// (question) and `B` (brick), since blocks are part of the solid world.
    /// Anything else is empty. Rows must be equal length.
    pub fn from_rows(rows: &[&str]) -> Self {
        let height = rows.len();
        let width = rows.first().map(|r| r.len()).unwrap_or(0);
        let mut cells = Vec::with_capacity(width * height);
        for row in rows {
            assert_eq!(row.len(), width, "rows must be equal length");
            for ch in row.chars() {
                cells.push(matches!(ch, '#' | '?' | 'B'));
            }
        }
        Self::new(width, height, cells)
    }

    /// Is the tile at (tx, ty) solid? Out-of-range tiles are empty.
    pub fn is_solid(&self, tx: i32, ty: i32) -> bool {
        if tx < 0 || ty < 0 || tx as usize >= self.width || ty as usize >= self.height {
            return false;
        }
        self.cells[ty as usize * self.width + tx as usize]
    }

    /// Is any solid tile touched by the pixel rectangle [left, right] x
    /// [top, bottom] (inclusive, in pixels)?
    pub fn rect_hits_solid(&self, left: i32, top: i32, right: i32, bottom: i32) -> bool {
        let tx0 = left.div_euclid(TILE);
        let tx1 = right.div_euclid(TILE);
        let ty0 = top.div_euclid(TILE);
        let ty1 = bottom.div_euclid(TILE);
        for ty in ty0..=ty1 {
            for tx in tx0..=tx1 {
                if self.is_solid(tx, ty) {
                    return true;
                }
            }
        }
        false
    }
}

/// A playable level: the solid geometry plus where Mario starts. Visuals (the
/// background tile map) are loaded separately; this is the gameplay side.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Level {
    pub solids: Solids,
    /// Mario's spawn, top-left pixel.
    pub spawn: (i32, i32),
    /// Enemy spawn points, top-left pixel. Kind is decided by the caller; for
    /// now every marker is a Goomba.
    pub enemy_spawns: Vec<(i32, i32)>,
    /// Coin positions, top-left pixel.
    pub coins: Vec<(i32, i32)>,
    /// Interactive block spawns: top-left pixel and kind.
    pub blocks: Vec<(i32, i32, BlockKind)>,
}

impl Level {
    /// Build a level from rows of text. `#` is a solid tile, `M` marks Mario's
    /// spawn, `G` a Goomba, `C` a coin, `?` a question block, `B` a brick block.
    /// The block markers are also solid. Anything else is empty. Rows must be
    /// equal length. This is the human-editable format levels are authored in.
    pub fn from_rows(rows: &[&str]) -> Self {
        let solids = Solids::from_rows(rows);
        let mut spawn = (0, 0);
        let mut enemy_spawns = Vec::new();
        let mut coins = Vec::new();
        let mut blocks = Vec::new();
        for (ty, row) in rows.iter().enumerate() {
            for (tx, ch) in row.chars().enumerate() {
                let (px, py) = (tx as i32 * TILE, ty as i32 * TILE);
                match ch {
                    'M' => spawn = (px, py),
                    'G' => enemy_spawns.push((px, py)),
                    'C' => coins.push((px, py)),
                    '?' => blocks.push((px, py, BlockKind::Question)),
                    'B' => blocks.push((px, py, BlockKind::Brick)),
                    _ => {}
                }
            }
        }
        Self {
            solids,
            spawn,
            enemy_spawns,
            coins,
            blocks,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_reads_spawn_and_solids() {
        let level = Level::from_rows(&[
            "........",
            "...M....",
            "........",
            "########",
        ]);
        // Spawn 'M' is at column 3, row 1 -> pixel (24, 8).
        assert_eq!(level.spawn, (24, 8));
        // The 'M' tile is not solid, the floor is.
        assert!(!level.solids.is_solid(3, 1));
        assert!(level.solids.is_solid(0, 3));
    }

    #[test]
    fn level_without_spawn_defaults_to_origin() {
        let level = Level::from_rows(&["....", "####"]);
        assert_eq!(level.spawn, (0, 0));
    }

    #[test]
    fn from_rows_marks_solids() {
        let s = Solids::from_rows(&["....", "####"]);
        assert!(!s.is_solid(0, 0));
        assert!(s.is_solid(0, 1));
        assert!(s.is_solid(3, 1));
    }

    #[test]
    fn out_of_range_is_empty() {
        let s = Solids::from_rows(&["#"]);
        assert!(!s.is_solid(-1, 0));
        assert!(!s.is_solid(0, -1));
        assert!(!s.is_solid(1, 0));
        assert!(!s.is_solid(0, 1));
    }

    #[test]
    fn rect_hits_solid_spans_tiles() {
        // Solid floor along row 2 (pixels y 16..23).
        let s = Solids::from_rows(&["....", "....", "####"]);
        assert!(!s.rect_hits_solid(0, 0, 7, 7)); // top-left tile, empty
        assert!(s.rect_hits_solid(0, 8, 7, 16)); // reaches into the floor row
    }
}
