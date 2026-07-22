//! A tiny 3x5 bitmap font for the HUD.
//!
//! Just the ten digits, our own drawing. Each glyph is five rows of three bits,
//! most significant bit on the left. Enough to show score, coins, lives, and the
//! timer as numbers over the framebuffer.

use crate::render::{Framebuffer, Shade};

pub const GLYPH_W: i32 = 3;
pub const GLYPH_H: i32 = 5;
/// One blank column between digits.
pub const ADVANCE: i32 = GLYPH_W + 1;

const DIGITS: [[u8; 5]; 10] = [
    [0b111, 0b101, 0b101, 0b101, 0b111], // 0
    [0b010, 0b110, 0b010, 0b010, 0b111], // 1
    [0b111, 0b001, 0b111, 0b100, 0b111], // 2
    [0b111, 0b001, 0b111, 0b001, 0b111], // 3
    [0b101, 0b101, 0b111, 0b001, 0b001], // 4
    [0b111, 0b100, 0b111, 0b001, 0b111], // 5
    [0b111, 0b100, 0b111, 0b101, 0b111], // 6
    [0b111, 0b001, 0b001, 0b010, 0b010], // 7
    [0b111, 0b101, 0b111, 0b101, 0b111], // 8
    [0b111, 0b101, 0b111, 0b001, 0b111], // 9
];

fn draw_digit(fb: &mut Framebuffer, digit: u8, x: i32, y: i32, shade: Shade) {
    let glyph = &DIGITS[(digit % 10) as usize];
    for (row, bits) in glyph.iter().enumerate() {
        for col in 0..GLYPH_W {
            if (bits >> (GLYPH_W - 1 - col)) & 1 == 1 {
                let (px, py) = (x + col, y + row as i32);
                if px >= 0 && py >= 0 && (px as u32) < crate::SCREEN_WIDTH && (py as u32) < crate::SCREEN_HEIGHT {
                    fb.set(px as u32, py as u32, shade);
                }
            }
        }
    }
}

/// Draw a number left to right starting at (x, y). Returns the x just past it.
pub fn draw_number(fb: &mut Framebuffer, mut x: i32, y: i32, value: u32, shade: Shade) -> i32 {
    for ch in value.to_string().bytes() {
        draw_digit(fb, ch - b'0', x, y, shade);
        x += ADVANCE;
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drawing_a_number_marks_pixels() {
        let mut fb = Framebuffer::new(); // all white
        let end = draw_number(&mut fb, 2, 2, 8, Shade::Black);
        // "8" fills its whole top row, so the first glyph pixel is black.
        assert_eq!(fb.get(2, 2), Shade::Black);
        assert_eq!(end, 2 + ADVANCE);
    }

    #[test]
    fn multi_digit_numbers_advance() {
        let mut fb = Framebuffer::new();
        let end = draw_number(&mut fb, 0, 0, 100, Shade::Black);
        assert_eq!(end, 3 * ADVANCE); // three digits
    }

    #[test]
    fn zero_renders_and_is_not_blank() {
        let mut fb = Framebuffer::new();
        draw_number(&mut fb, 0, 0, 0, Shade::Black);
        let any_black = (0..GLYPH_W).any(|x| (0..GLYPH_H).any(|y| fb.get(x as u32, y as u32) == Shade::Black));
        assert!(any_black);
    }
}
