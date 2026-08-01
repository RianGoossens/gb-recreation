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

/// An empty cell.
pub const BLANK: u8 = 0x6F;
/// The game draws a zero with the same tile it draws the letter O with. Tile
/// `0x00` is a narrower zero the status bar never uses.
pub const ZERO: u8 = 0x18;
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
    /// left, no coins, no score, 393 on the clock.
    #[test]
    fn reproduces_the_captured_status_bar() {
        let bar = status_bar(0, 0, 2, (1, 1), 393);
        let top: Vec<String> = bar[0].iter().map(|b| format!("{b:02X}")).collect();
        let bottom: Vec<String> = bar[1].iter().map(|b| format!("{b:02X}")).collect();
        assert_eq!(
            top.join(" "),
            "16 0A 1B 12 18 2B 18 02 6F 6F 20 18 1B 15 0D 6F 1D 12 16 0E"
        );
        assert_eq!(
            bottom.join(" "),
            "6F 6F 6F 6F 6F 18 6F 2A 2B 18 18 6F 01 29 01 6F 6F 03 09 03"
        );
    }

    #[test]
    fn a_six_digit_score_fills_its_field() {
        let bar = status_bar(123456, 0, 0, (1, 1), 0);
        assert_eq!(&bar[1][0..6], &[1, 2, 3, 4, 5, 6]);
    }
}
