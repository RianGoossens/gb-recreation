//! The bridge from our framebuffer to a window.
//!
//! The pixel conversion lives here, testable on its own, so the windowing code
//! (behind the optional `gui` feature) stays a thin loop. `to_argb` turns a
//! 160x144 shade buffer into a scaled 0xAARRGGBB buffer a window can blit.

use crate::render::Framebuffer;
use crate::{SCREEN_HEIGHT, SCREEN_WIDTH};

/// Convert the framebuffer to 0xAARRGGBB pixels, each source pixel expanded into
/// a `scale` by `scale` block. Fully opaque. Scale is at least 1.
pub fn to_argb(fb: &Framebuffer, scale: usize) -> Vec<u32> {
    let scale = scale.max(1);
    let sw = SCREEN_WIDTH as usize;
    let sh = SCREEN_HEIGHT as usize;
    let w = sw * scale;
    let mut buf = vec![0u32; w * sh * scale];
    for y in 0..sh {
        for x in 0..sw {
            let g = fb.get(x as u32, y as u32).to_gray() as u32;
            let argb = 0xFF00_0000 | (g << 16) | (g << 8) | g;
            for dy in 0..scale {
                let row = (y * scale + dy) * w;
                for dx in 0..scale {
                    buf[row + x * scale + dx] = argb;
                }
            }
        }
    }
    buf
}

/// The pixel size of the scaled buffer: (width, height).
pub fn scaled_size(scale: usize) -> (usize, usize) {
    let scale = scale.max(1);
    (SCREEN_WIDTH as usize * scale, SCREEN_HEIGHT as usize * scale)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::Shade;

    #[test]
    fn buffer_has_scaled_dimensions() {
        let fb = Framebuffer::new();
        let buf = to_argb(&fb, 3);
        let (w, h) = scaled_size(3);
        assert_eq!(w, SCREEN_WIDTH as usize * 3);
        assert_eq!(h, SCREEN_HEIGHT as usize * 3);
        assert_eq!(buf.len(), w * h);
    }

    #[test]
    fn white_is_opaque_white_black_is_opaque_black() {
        let mut fb = Framebuffer::new(); // all white
        fb.set(0, 0, Shade::Black);
        let buf = to_argb(&fb, 1);
        assert_eq!(buf[0], 0xFF00_0000); // black, opaque
        assert_eq!(buf[1], 0xFFFF_FFFF); // white, opaque
    }

    #[test]
    fn scaling_replicates_each_pixel_into_a_block() {
        let mut fb = Framebuffer::new();
        fb.set(0, 0, Shade::Black);
        let scale = 2;
        let buf = to_argb(&fb, scale);
        let (w, _h) = scaled_size(scale);
        // The top-left 2x2 block should all be black.
        assert_eq!(buf[0], 0xFF00_0000);
        assert_eq!(buf[1], 0xFF00_0000);
        assert_eq!(buf[w], 0xFF00_0000);
        assert_eq!(buf[w + 1], 0xFF00_0000);
    }
}
