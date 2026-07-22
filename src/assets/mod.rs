//! Assets: loading tiles, palettes, and level data.
//!
//! Game data (tile graphics, palettes, tilemaps, level layouts) is extracted
//! from the verified ROM into our own format by a reproducible pipeline, then
//! loaded here into memory the [`crate::render`] and [`crate::core`] modules can
//! use. The extracted files are gitignored; only the loading code lives in the
//! repo.
//!
//! Empty for now. The extraction command and the first real loader (title
//! screen tiles) arrive with Milestone 1, gated behind the ROM hash check.

/// Placeholder for a loaded, decoded tile sheet. Replaced with the real type
/// when the asset pipeline lands.
#[derive(Debug, Default)]
pub struct TileSheet;
