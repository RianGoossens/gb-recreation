//! A tiny grayscale PNG encoder.
//!
//! Just enough to write our 160x144 framebuffer to a real PNG for screenshots
//! and golden-image tests, with no external dependencies. It uses stored (not
//! compressed) deflate blocks, so files are larger than a full encoder would
//! produce, but the output is a valid PNG any viewer opens. Chunk CRCs reuse
//! the CRC32 from the rom module.

use crate::rom::crc32;

/// Encode 8-bit grayscale pixels (row major, `width * height` bytes) as PNG.
pub fn encode_gray(width: u32, height: u32, gray: &[u8]) -> Vec<u8> {
    assert_eq!(gray.len(), (width * height) as usize, "pixel count mismatch");

    let mut out = Vec::new();
    out.extend_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]); // PNG signature

    // IHDR: width, height, bit depth 8, color type 0 (grayscale), rest zero.
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 0, 0, 0, 0]);
    write_chunk(&mut out, b"IHDR", &ihdr);

    // Raw image: each row is a filter byte (0 = none) then the row's pixels.
    let mut raw = Vec::with_capacity((height * (width + 1)) as usize);
    for row in 0..height {
        raw.push(0);
        let start = (row * width) as usize;
        raw.extend_from_slice(&gray[start..start + width as usize]);
    }

    write_chunk(&mut out, b"IDAT", &zlib_store(&raw));
    write_chunk(&mut out, b"IEND", &[]);
    out
}

fn write_chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let mut crc_input = Vec::with_capacity(4 + data.len());
    crc_input.extend_from_slice(kind);
    crc_input.extend_from_slice(data);
    out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
}

/// Wrap data in a zlib stream using stored deflate blocks.
fn zlib_store(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&[0x78, 0x01]); // zlib header, no compression

    let chunks: Vec<&[u8]> = if data.is_empty() {
        vec![&data[0..0]]
    } else {
        data.chunks(0xFFFF).collect()
    };
    for (i, block) in chunks.iter().enumerate() {
        let last = i == chunks.len() - 1;
        out.push(if last { 1 } else { 0 }); // BFINAL, BTYPE=00 (stored)
        let len = block.len() as u16;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes());
        out.extend_from_slice(block);
    }

    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

fn adler32(data: &[u8]) -> u32 {
    const MOD: u32 = 65521;
    let mut a = 1u32;
    let mut b = 0u32;
    for &byte in data {
        a = (a + byte as u32) % MOD;
        b = (b + a) % MOD;
    }
    (b << 16) | a
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Decode a zlib stream made only of stored blocks. Used to prove our
    /// encoder round-trips without pulling in a real inflate implementation.
    fn zlib_store_decode(z: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut i = 2; // skip zlib header
        loop {
            let bfinal = z[i] & 1;
            i += 1;
            let len = u16::from_le_bytes([z[i], z[i + 1]]) as usize;
            i += 4; // len + nlen
            out.extend_from_slice(&z[i..i + len]);
            i += len;
            if bfinal == 1 {
                break;
            }
        }
        out
    }

    #[test]
    fn adler32_known_value() {
        // Adler-32 of "abc" is 0x024D0127.
        assert_eq!(adler32(b"abc"), 0x024D_0127);
    }

    #[test]
    fn png_has_signature_and_iend() {
        let png = encode_gray(2, 2, &[0, 85, 170, 255]);
        assert_eq!(&png[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
        assert_eq!(&png[png.len() - 8..png.len() - 4], b"IEND");
    }

    #[test]
    fn chunk_crcs_are_valid() {
        let png = encode_gray(3, 1, &[10, 20, 30]);
        // Walk chunks after the 8-byte signature and check each CRC.
        let mut i = 8;
        loop {
            let len = u32::from_be_bytes(png[i..i + 4].try_into().unwrap()) as usize;
            let kind_and_data = &png[i + 4..i + 8 + len];
            let stored = u32::from_be_bytes(png[i + 8 + len..i + 12 + len].try_into().unwrap());
            assert_eq!(crc32(kind_and_data), stored, "bad CRC at offset {i}");
            let kind = &png[i + 4..i + 8];
            i += 12 + len;
            if kind == b"IEND" {
                break;
            }
        }
        assert_eq!(i, png.len());
    }

    #[test]
    fn image_data_round_trips() {
        let gray = vec![0u8, 64, 128, 192, 255, 32];
        let png = encode_gray(3, 2, &gray);
        // Pull the IDAT payload out and inflate our stored blocks.
        let mut i = 8;
        let mut idat = Vec::new();
        loop {
            let len = u32::from_be_bytes(png[i..i + 4].try_into().unwrap()) as usize;
            let kind = &png[i + 4..i + 8];
            if kind == b"IDAT" {
                idat.extend_from_slice(&png[i + 8..i + 8 + len]);
            }
            let done = kind == b"IEND";
            i += 12 + len;
            if done {
                break;
            }
        }
        let raw = zlib_store_decode(&idat);
        // Expect filter byte 0 then each row's pixels.
        let expected = [0u8, 0, 64, 128, 0, 192, 255, 32];
        assert_eq!(raw, expected);
    }
}
