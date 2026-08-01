//! Level geometry: which tiles are solid.
//!
//! Collision works against a grid of solid or empty tiles, one bool per 8x8
//! tile. This is deliberately simple and human editable, which also serves the
//! later moddability goal: a level's solids can be written as rows of text.

use crate::core::block::BlockKind;
use crate::core::enemy::EnemyKind;
use crate::core::lift::LiftAxis;
use crate::core::powerup::ItemKind;
use crate::tiles::Tile;

/// Tile size in pixels.
pub const TILE: i32 = 8;

/// A grid of solid tiles. Anything outside the grid reads as empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Solids {
    pub width: usize,
    pub height: usize,
    cells: Vec<bool>,
    /// Tiles that hold Mario up from above but do not block him sideways or
    /// from below. Measured from the cartridge: `tools/probe_solidity.py`
    /// finds four such tile ids, and World 1-2 lays them out as horizontal
    /// runs with distinct end caps, which is the shape of a platform.
    platforms: Vec<bool>,
}

impl Solids {
    pub fn new(width: usize, height: usize, cells: Vec<bool>) -> Self {
        assert_eq!(width * height, cells.len(), "cells must be width*height");
        let platforms = vec![false; cells.len()];
        Self {
            width,
            height,
            cells,
            platforms,
        }
    }

    /// Mark the tile at (tx, ty) as a one-way platform.
    pub fn set_platform(&mut self, tx: usize, ty: usize) {
        if tx < self.width && ty < self.height {
            self.platforms[ty * self.width + tx] = true;
        }
    }

    /// Is the tile at (tx, ty) a one-way platform? Out of range is not.
    pub fn is_platform(&self, tx: i32, ty: i32) -> bool {
        if tx < 0 || ty < 0 || tx as usize >= self.width || ty as usize >= self.height {
            return false;
        }
        self.platforms[ty as usize * self.width + tx as usize]
    }

    /// The topmost platform row touched by the pixel span, if any.
    pub fn platform_under(&self, left: i32, right: i32, bottom: i32) -> Option<i32> {
        let ty = bottom.div_euclid(TILE);
        let tx0 = left.div_euclid(TILE);
        let tx1 = right.div_euclid(TILE);
        (tx0..=tx1).any(|tx| self.is_platform(tx, ty)).then_some(ty)
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
                cells.push(matches!(ch, '#' | '?' | 'B' | 'P'));
            }
        }
        let mut solids = Self::new(width, height, cells);
        for (y, row) in rows.iter().enumerate() {
            for (x, ch) in row.chars().enumerate() {
                if ch == '^' {
                    solids.set_platform(x, y);
                }
            }
        }
        solids
    }

    /// Make the tile at (tx, ty) empty (for example, a broken brick). No effect
    /// if it is out of range.
    pub fn clear(&mut self, tx: i32, ty: i32) {
        if tx >= 0 && ty >= 0 && (tx as usize) < self.width && (ty as usize) < self.height {
            self.cells[ty as usize * self.width + tx as usize] = false;
        }
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
    /// Enemy spawn points: top-left pixel and kind.
    pub enemy_spawns: Vec<(i32, i32, EnemyKind)>,
    /// Coin positions, top-left pixel.
    pub coins: Vec<(i32, i32)>,
    /// Interactive block spawns: top-left pixel and kind.
    pub blocks: Vec<(i32, i32, BlockKind)>,
    /// The level-end trigger, top-left pixel, if the level has one.
    pub end: Option<(i32, i32)>,
    /// Free-standing item spawns: top-left pixel and kind.
    pub items: Vec<(i32, i32, ItemKind)>,
    /// Moving platforms: top-left pixel and the axis they run on.
    pub lifts: Vec<(i32, i32, LiftAxis)>,
    /// The cartridge's own background graphics, when the level came from the
    /// ROM. A hand-written level file has none and renders with placeholders.
    pub graphics: Option<Graphics>,
    /// World and level number, for the status bar, when the level is one of
    /// the cartridge's twelve.
    pub number: Option<(u8, u8)>,
}

/// A level's background as the cartridge draws it: one tile id per cell, row
/// major over the level's own width and height, and the sheet those ids index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Graphics {
    pub cells: Vec<u8>,
    pub tiles: Vec<Tile>,
    pub palette: u8,
    /// The one tile the cartridge animates: its id and its two pictures, in
    /// the order the game shows them. Each is held for [`ANIMATION_HOLD`]
    /// frames.
    pub animated: Option<(u8, Tile, Tile)>,
}

/// How long the cartridge holds each frame of its animated background tile.
/// The routine that swaps it runs every eight frames and alternates on bit 3
/// of the same counter (`docs/reference/level-format.md`).
pub const ANIMATION_HOLD: u32 = 8;

impl Level {
    /// Build a level from rows of text. `#` is a solid tile, `^` a one-way
    /// platform, `M` marks Mario's spawn, `G` a Goomba, `F` a Fly, `C` a coin,
    /// `T` a walker that turns at ledges,
    /// `S` a star, `?` a question block, `P` a power block, `B` a brick block,
    /// `E` the level-end trigger, `V` and `H` a lift running up and down or
    /// side to side.
    /// The block markers are also solid; `E`, `S`, and coins are not. Anything
    /// else is empty. Rows must be equal length. This is the human-editable
    /// format levels are authored in.
    pub fn from_rows(rows: &[&str]) -> Self {
        let solids = Solids::from_rows(rows);
        let mut spawn = (0, 0);
        let mut enemy_spawns = Vec::new();
        let mut coins = Vec::new();
        let mut blocks = Vec::new();
        let mut end = None;
        let mut items = Vec::new();
        let mut lifts = Vec::new();
        for (ty, row) in rows.iter().enumerate() {
            for (tx, ch) in row.chars().enumerate() {
                let (px, py) = (tx as i32 * TILE, ty as i32 * TILE);
                match ch {
                    'M' => spawn = (px, py),
                    'G' => enemy_spawns.push((px, py, EnemyKind::Goomba)),
                    'T' => enemy_spawns.push((px, py, EnemyKind::LedgeTurner)),
                    'F' => enemy_spawns.push((px, py, EnemyKind::Fly)),
                    'J' => enemy_spawns.push((px, py, EnemyKind::Hopper)),
                    'D' => enemy_spawns.push((px, py, EnemyKind::Faller)),
                    'C' => coins.push((px, py)),
                    '?' => blocks.push((px, py, BlockKind::Question)),
                    'P' => blocks.push((px, py, BlockKind::PowerUp)),
                    'B' => blocks.push((px, py, BlockKind::Brick)),
                    'S' => items.push((px, py, ItemKind::Star)),
                    'W' => items.push((px, py, ItemKind::Flower)),
                    'E' => end = Some((px, py)),
                    'V' => lifts.push((px, py, LiftAxis::Vertical)),
                    'H' => lifts.push((px, py, LiftAxis::Horizontal)),
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
            end,
            items,
            lifts,
            graphics: None,
            number: None,
        }
    }

    /// Drop the spawns for content Super Mario Land does not have.
    ///
    /// Two things qualify, both tracked in `docs/reference/faithfulness.md`:
    /// the invincibility star (the cartridge has no star at all) and the Fly,
    /// a generic hopper standing in for an SML enemy that has not been pinned.
    /// The end goal is a faithful recreation, so the default build runs this
    /// over every level it plays and a caller has to opt in to keep them.
    ///
    /// The level format itself still parses both markers. A level file is
    /// data, and rejecting it outright would make custom levels that use them
    /// fail to load rather than simply play faithfully.
    pub fn without_non_canonical(mut self) -> Self {
        self.items.retain(|&(_, _, kind)| kind != ItemKind::Star);
        self.enemy_spawns.retain(|&(_, _, kind)| kind != EnemyKind::Fly);
        self
    }

    /// Parse a level from a block of text, one row per line. Trailing blank
    /// lines are ignored. Every remaining row must be the same width, otherwise
    /// this returns an error rather than panicking, so a bad file is reported.
    pub fn from_text(text: &str) -> Result<Self, String> {
        let lines: Vec<&str> = text.trim_end_matches(['\n', '\r']).lines().collect();
        if lines.is_empty() {
            return Err("level is empty".to_string());
        }
        let width = lines[0].len();
        if let Some(bad) = lines.iter().position(|l| l.len() != width) {
            return Err(format!(
                "row {bad} is {} wide but the level is {width} wide",
                lines[bad].len()
            ));
        }
        Ok(Self::from_rows(&lines))
    }

    /// Load a level from a text file.
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("could not read level: {e}"))?;
        Self::from_text(&text)
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
    fn from_str_parses_a_level() {
        let text = "..?..\n..M..\n#####\n";
        let level = Level::from_text(text).unwrap();
        assert_eq!(level.spawn, (2 * TILE, TILE)); // 'M' at col 2, row 1
        assert_eq!(level.blocks.len(), 1); // the '?'
        assert!(level.solids.is_solid(0, 2)); // floor row
    }

    #[test]
    fn from_str_rejects_ragged_rows() {
        let err = Level::from_text("####\n###\n").unwrap_err();
        assert!(err.contains("wide"), "error explains the width mismatch: {err}");
    }

    #[test]
    fn from_str_ignores_trailing_blank_lines() {
        let level = Level::from_text("M.\n##\n\n").unwrap();
        assert_eq!(level.solids.height, 2);
    }

    #[test]
    fn the_shipped_example_level_loads() {
        // Guards the committed example file against typos or ragged rows.
        let level = Level::from_file("levels/example.txt").expect("example level should load");
        assert!(level.end.is_some(), "the example has an end trigger");
        assert!(!level.enemy_spawns.is_empty(), "and some enemies");
        assert!(!level.blocks.is_empty(), "and some blocks");
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
    fn without_non_canonical_drops_the_star_and_the_fly() {
        let level = Level::from_rows(&["M.S.F.", "..G.C.", "######"]);
        assert_eq!(level.items.len(), 1, "the star parses in the first place");
        assert_eq!(level.enemy_spawns.len(), 2);

        let faithful = level.without_non_canonical();
        assert!(faithful.items.is_empty(), "no star survives");
        assert_eq!(
            faithful.enemy_spawns.len(),
            1,
            "the Goomba stays, the Fly goes"
        );
        assert_eq!(faithful.enemy_spawns[0].2, EnemyKind::Goomba);
        assert_eq!(faithful.coins.len(), 1, "coins are canonical");
    }

    #[test]
    fn without_non_canonical_keeps_the_flower() {
        // The superball flower is SML's own power-up, so it must survive.
        let level = Level::from_rows(&["M.W.", "####"]).without_non_canonical();
        assert_eq!(level.items.len(), 1);
        assert_eq!(level.items[0].2, ItemKind::Flower);
    }

    #[test]
    fn rect_hits_solid_spans_tiles() {
        // Solid floor along row 2 (pixels y 16..23).
        let s = Solids::from_rows(&["....", "....", "####"]);
        assert!(!s.rect_hits_solid(0, 0, 7, 7)); // top-left tile, empty
        assert!(s.rect_hits_solid(0, 8, 7, 16)); // reaches into the floor row
    }
}
