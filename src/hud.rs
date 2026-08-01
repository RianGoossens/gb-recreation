//! The cartridge's status bar.
//!
//! Two rows of 8x8 background tiles across the top of the screen, drawn with
//! the font that sits at the start of the shared tile atlas. The layout and
//! every id below are read off the emulator capture of World 1-1's opening
//! (`assets/extracted/level_1_1_opening.tmap`), by matching each tile in that
//! capture's sheet against the ROM's own background tiles.

/// Columns and rows the bar covers.
pub const COLUMNS: usize = 20;
pub const ROWS: usize = 2;

/// An empty cell. Four ids in the font's block draw an empty tile, so the
/// capture alone cannot say which one the bar uses; `0x2C` is the only one of
/// them outside the range a world's tile overlay rewrites (`0x31` to `0x6F`),
/// and it is the same filler the level format leaves uncovered columns as.
pub const BLANK: u8 = 0x2C;
/// Zero. The font draws it and the letter O identically, so the capture reads
/// as either `0x00` or `0x18`; `0x00` is the one in the digits' own run.
pub const ZERO: u8 = 0x00;
pub const DASH: u8 = 0x29;
/// The coin symbol in front of the coin count.
pub const COIN: u8 = 0x2A;
/// The multiplication sign in "MARIO x02" and in the coin count.
pub const TIMES: u8 = 0x2B;

/// The tile for a decimal digit.
pub fn digit(value: u8) -> u8 {
    match value % 10 {
        0 => ZERO,
        d => d,
    }
}

/// The tile for an uppercase letter.
pub fn letter(c: char) -> u8 {
    0x0A + (c as u8 - b'A')
}

fn write(row: &mut [u8; COLUMNS], at: usize, text: &str) {
    for (i, c) in text.chars().enumerate() {
        row[at + i] = letter(c);
    }
}

/// Zero-padded digits, most significant first.
fn padded(value: u32, width: usize) -> Vec<u8> {
    let mut out = vec![ZERO; width];
    let mut left = value;
    for cell in out.iter_mut().rev() {
        *cell = digit((left % 10) as u8);
        left /= 10;
    }
    out
}

/// The status bar as tile ids, row major.
pub fn status_bar(score: u32, coins: u32, lives: u32, world: (u8, u8), time: u32) -> [[u8; COLUMNS]; ROWS] {
    let mut top = [BLANK; COLUMNS];
    write(&mut top, 0, "MARIO");
    top[5] = TIMES;
    top[6..8].copy_from_slice(&padded(lives, 2));
    write(&mut top, 10, "WORLD");
    write(&mut top, 16, "TIME");

    let mut bottom = [BLANK; COLUMNS];
    // Score is right aligned in six cells with blanks in front of it, not
    // zeros: the capture shows a score of 0 as one digit at the sixth cell.
    let mut left = score;
    for column in (0..6).rev() {
        bottom[column] = digit((left % 10) as u8);
        left /= 10;
        if left == 0 {
            break;
        }
    }
    bottom[7] = COIN;
    bottom[8] = TIMES;
    bottom[9..11].copy_from_slice(&padded(coins, 2));
    bottom[12] = digit(world.0);
    bottom[13] = DASH;
    bottom[14] = digit(world.1);
    bottom[17..20].copy_from_slice(&padded(time, 3));

    [top, bottom]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The capture this layout was read from: World 1-1's opening, two lives
    /// left, no coins, no score, 393 on the clock. The ids are the capture's
    /// own, with the two ambiguous glyphs resolved as documented above.
    #[test]
    fn reproduces_the_captured_status_bar() {
        let bar = status_bar(0, 0, 2, (1, 1), 393);
        let top: Vec<String> = bar[0].iter().map(|b| format!("{b:02X}")).collect();
        let bottom: Vec<String> = bar[1].iter().map(|b| format!("{b:02X}")).collect();
        assert_eq!(
            top.join(" "),
            "16 0A 1B 12 18 2B 00 02 2C 2C 20 18 1B 15 0D 2C 1D 12 16 0E"
        );
        assert_eq!(
            bottom.join(" "),
            "2C 2C 2C 2C 2C 00 2C 2A 2B 00 00 2C 01 29 01 2C 2C 03 09 03"
        );
    }

    #[test]
    fn a_six_digit_score_fills_its_field() {
        let bar = status_bar(123456, 0, 0, (1, 1), 0);
        assert_eq!(&bar[1][0..6], &[1, 2, 3, 4, 5, 6]);
    }
}
