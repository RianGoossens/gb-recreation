//! Game Boy tiles: the 8x8 building block of everything on screen.
//!
//! A tile is 16 bytes, two per row. In a row, the first byte holds the low bit
//! of each of the eight pixels and the second byte holds the high bit, so each
//! pixel is a 0 to 3 color index (most significant pixel first). The index is
//! not a shade yet: a palette maps the four indices to four display shades.

/// One decoded 8x8 tile as color indices in 0..=3, row major.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tile {
    pub pixels: [[u8; 8]; 8],
}

impl Tile {
    pub const BYTES: usize = 16;

    /// Decode one tile from its 16 bytes of 2bpp data.
    pub fn decode(bytes: &[u8; 16]) -> Self {
        let mut pixels = [[0u8; 8]; 8];
        for row in 0..8 {
            let low = bytes[row * 2];
            let high = bytes[row * 2 + 1];
            for (x, pixel) in pixels[row].iter_mut().enumerate() {
                let bit = 7 - x;
                let lo = (low >> bit) & 1;
                let hi = (high >> bit) & 1;
                *pixel = (hi << 1) | lo;
            }
        }
        Self { pixels }
    }
}

/// Decode a run of tiles from a byte slice. Extra bytes that do not fill a whole
/// tile are ignored.
pub fn decode_tiles(bytes: &[u8]) -> Vec<Tile> {
    bytes
        .chunks_exact(Tile::BYTES)
        .map(|chunk| Tile::decode(chunk.try_into().unwrap()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_zero_bytes_decode_to_index_zero() {
        let tile = Tile::decode(&[0u8; 16]);
        assert_eq!(tile.pixels, [[0u8; 8]; 8]);
    }

    #[test]
    fn plane_bits_combine_into_indices() {
        // Row 0: low plane 0xFF, high plane 0x00 -> every pixel index 1.
        // Row 1: low plane 0x00, high plane 0xFF -> every pixel index 2.
        // Row 2: both 0xFF -> every pixel index 3.
        let mut bytes = [0u8; 16];
        bytes[0] = 0xFF;
        bytes[1] = 0x00;
        bytes[2] = 0x00;
        bytes[3] = 0xFF;
        bytes[4] = 0xFF;
        bytes[5] = 0xFF;
        let tile = Tile::decode(&bytes);
        assert_eq!(tile.pixels[0], [1; 8]);
        assert_eq!(tile.pixels[1], [2; 8]);
        assert_eq!(tile.pixels[2], [3; 8]);
        assert_eq!(tile.pixels[3], [0; 8]);
    }

    #[test]
    fn pixel_order_is_most_significant_bit_first() {
        // low = 0b1000_0000, high = 0. Only the leftmost pixel is index 1.
        let mut bytes = [0u8; 16];
        bytes[0] = 0b1000_0000;
        let tile = Tile::decode(&bytes);
        assert_eq!(tile.pixels[0][0], 1);
        assert_eq!(tile.pixels[0][1], 0);
        assert_eq!(tile.pixels[0][7], 0);
    }

    #[test]
    fn decode_tiles_counts_whole_tiles_only() {
        let bytes = vec![0u8; Tile::BYTES * 3 + 5];
        assert_eq!(decode_tiles(&bytes).len(), 3);
    }
}
