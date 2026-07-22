//! Assets: extracting and loading game data.
//!
//! Graphics are pulled out of the verified ROM into our own on-disk format by a
//! reproducible command, then loaded here for [`crate::render`] and
//! [`crate::core`] to use. Extracted files are gitignored, only the code lives
//! in the repo. Extraction always verifies the ROM first, so we never build
//! assets from an unknown dump.

use std::io;
use std::path::Path;

use crate::rom;
use crate::tiles::{decode_tiles, Tile};

/// A collection of decoded tiles plus the palette they render with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TileSheet {
    pub tiles: Vec<Tile>,
    /// Game Boy BGP value: four 2-bit shade selections packed into a byte.
    pub palette: u8,
}

/// Default DMG background palette: index 0 lightest, 3 darkest (BGP 0xE4).
pub const DEFAULT_BGP: u8 = 0xE4;

const MAGIC: &[u8; 4] = b"SMLT";
const FORMAT_VERSION: u8 = 1;

impl TileSheet {
    pub fn new(tiles: Vec<Tile>, palette: u8) -> Self {
        Self { tiles, palette }
    }

    /// Our asset format: magic, version, palette, tile count, then 64 index
    /// bytes per tile (row major). Small and dependency free so the loader is
    /// trivial and the format is easy to read.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(11 + self.tiles.len() * 64);
        out.extend_from_slice(MAGIC);
        out.push(FORMAT_VERSION);
        out.push(self.palette);
        out.extend_from_slice(&(self.tiles.len() as u32).to_le_bytes());
        for tile in &self.tiles {
            for row in tile.pixels {
                out.extend_from_slice(&row);
            }
        }
        out
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, AssetError> {
        if data.len() < 10 || &data[0..4] != MAGIC {
            return Err(AssetError::BadFormat);
        }
        if data[4] != FORMAT_VERSION {
            return Err(AssetError::BadFormat);
        }
        let palette = data[5];
        let count = u32::from_le_bytes(data[6..10].try_into().unwrap()) as usize;
        let body = &data[10..];
        if body.len() != count * 64 {
            return Err(AssetError::BadFormat);
        }
        let mut tiles = Vec::with_capacity(count);
        for t in 0..count {
            let mut pixels = [[0u8; 8]; 8];
            for (r, row) in pixels.iter_mut().enumerate() {
                let start = t * 64 + r * 8;
                row.copy_from_slice(&body[start..start + 8]);
            }
            tiles.push(Tile { pixels });
        }
        Ok(Self { tiles, palette })
    }

    pub fn save(&self, path: impl AsRef<Path>) -> io::Result<()> {
        std::fs::write(path, self.to_bytes())
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, AssetError> {
        let data = std::fs::read(path).map_err(AssetError::Io)?;
        Self::from_bytes(&data)
    }

    /// A binary PGM (P5) preview, tiles laid out in a grid `cols` wide. Useful
    /// for eyeballing an extraction. Palette is applied so shades look right.
    pub fn to_pgm(&self, cols: usize) -> Vec<u8> {
        let cols = cols.max(1);
        let rows = self.tiles.len().div_ceil(cols);
        let w = cols * 8;
        let h = rows * 8;
        let palette = crate::render::Palette::new(self.palette);
        let mut pixels = vec![0xFFu8; w * h];
        for (i, tile) in self.tiles.iter().enumerate() {
            let tx = (i % cols) * 8;
            let ty = (i / cols) * 8;
            for y in 0..8 {
                for x in 0..8 {
                    pixels[(ty + y) * w + (tx + x)] =
                        palette.shade(tile.pixels[y][x]).to_gray();
                }
            }
        }
        let mut out = format!("P5\n{w} {h}\n255\n").into_bytes();
        out.extend_from_slice(&pixels);
        out
    }
}

/// Extract `count` tiles starting at `offset` in the ROM at `rom_path`, after
/// verifying the ROM is the expected dump. Returns them as a [`TileSheet`] with
/// the default palette; callers can override the palette once it is known.
pub fn extract_tiles(
    rom_path: impl AsRef<Path>,
    offset: usize,
    count: usize,
) -> Result<TileSheet, AssetError> {
    rom::verify_file(&rom_path).map_err(AssetError::Rom)?;
    let data = std::fs::read(&rom_path).map_err(AssetError::Io)?;
    let end = offset + count * Tile::BYTES;
    if end > data.len() {
        return Err(AssetError::OutOfRange {
            end,
            len: data.len(),
        });
    }
    let tiles = decode_tiles(&data[offset..end]);
    Ok(TileSheet::new(tiles, DEFAULT_BGP))
}

const MAP_MAGIC: &[u8; 4] = b"SMLM";

/// Load a tile map in our SMLM format: magic, version, u16 width, u16 height,
/// then width*height tile indices. Produced by the extraction tool.
pub fn load_tilemap(path: impl AsRef<Path>) -> Result<crate::render::TileMap, AssetError> {
    let data = std::fs::read(path).map_err(AssetError::Io)?;
    tilemap_from_bytes(&data)
}

fn tilemap_from_bytes(data: &[u8]) -> Result<crate::render::TileMap, AssetError> {
    if data.len() < 9 || &data[0..4] != MAP_MAGIC || data[4] != FORMAT_VERSION {
        return Err(AssetError::BadFormat);
    }
    let width = u16::from_le_bytes(data[5..7].try_into().unwrap()) as usize;
    let height = u16::from_le_bytes(data[7..9].try_into().unwrap()) as usize;
    let cells = &data[9..];
    if cells.len() != width * height {
        return Err(AssetError::BadFormat);
    }
    Ok(crate::render::TileMap::new(width, height, cells.to_vec()))
}

#[derive(Debug)]
pub enum AssetError {
    Io(io::Error),
    Rom(rom::VerifyError),
    OutOfRange { end: usize, len: usize },
    BadFormat,
}

impl std::fmt::Display for AssetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetError::Io(e) => write!(f, "io error: {e}"),
            AssetError::Rom(e) => write!(f, "{e}"),
            AssetError::OutOfRange { end, len } => {
                write!(f, "requested bytes up to {end} but ROM is only {len} long")
            }
            AssetError::BadFormat => write!(f, "not a valid tile asset file"),
        }
    }
}

impl std::error::Error for AssetError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_sheet() -> TileSheet {
        let mut a = [[0u8; 8]; 8];
        a[0][0] = 3;
        a[7][7] = 1;
        let mut b = [[2u8; 8]; 8];
        b[3][3] = 0;
        TileSheet::new(
            vec![Tile { pixels: a }, Tile { pixels: b }],
            DEFAULT_BGP,
        )
    }

    #[test]
    fn asset_bytes_round_trip() {
        let sheet = sample_sheet();
        let bytes = sheet.to_bytes();
        let back = TileSheet::from_bytes(&bytes).unwrap();
        assert_eq!(sheet, back);
    }

    #[test]
    fn bad_magic_is_rejected() {
        assert!(matches!(
            TileSheet::from_bytes(b"nope and some more bytes here"),
            Err(AssetError::BadFormat)
        ));
    }

    #[test]
    fn tilemap_bytes_parse() {
        // SMLM: magic, version 1, width 2, height 1, cells [3, 7].
        let mut bytes = b"SMLM".to_vec();
        bytes.push(1);
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&[3, 7]);
        let map = tilemap_from_bytes(&bytes).unwrap();
        assert_eq!((map.width, map.height), (2, 1));
        assert_eq!(map.cell(0, 0), 3);
        assert_eq!(map.cell(1, 0), 7);
    }

    #[test]
    fn tilemap_bad_size_is_rejected() {
        let mut bytes = b"SMLM".to_vec();
        bytes.push(1);
        bytes.extend_from_slice(&4u16.to_le_bytes());
        bytes.extend_from_slice(&4u16.to_le_bytes());
        bytes.extend_from_slice(&[0, 1]); // too few cells
        assert!(matches!(tilemap_from_bytes(&bytes), Err(AssetError::BadFormat)));
    }

    #[test]
    fn pgm_has_expected_dimensions() {
        let sheet = sample_sheet();
        let pgm = sheet.to_pgm(1); // 2 tiles, 1 column -> 8x16 image
        let header = std::str::from_utf8(&pgm[..12]).unwrap();
        assert!(header.starts_with("P5\n8 16\n255\n"), "header was {header:?}");
    }
}
