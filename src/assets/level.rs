//! Level extraction from the ROM.
//!
//! Reads a level's geometry straight out of the cartridge, with no emulator
//! involved. The format was found by watching the game's own banked-ROM read
//! pointer in work RAM rather than by reading a disassembly; the full account
//! is in `docs/reference/level-1-1.md`.
//!
//! Two layers.
//!
//! A **column record** is a list of runs, terminated by `0xFE`:
//!
//! ```text
//! (row << 4) | count    place `count` tiles starting at `row`
//! <count tile bytes>    the tile ids, top to bottom
//! ...                   more runs
//! 0xFE                  end of column
//! ```
//!
//! with two special cases: a `count` of 0 means a full 16 rows rather than an
//! empty run, and `0xFD <tile>` in place of the tile bytes repeats one tile for
//! the whole run. Anything no run covers is background filler (tile 44).
//!
//! A **level** is a `0xFF`-terminated list of 16-bit little-endian pointers to
//! those records, one per screen, where a screen is 20 columns (the width of
//! the display). Pointers repeat: World 1-1 reuses six of its fifteen screens,
//! which is why world columns 0-19 are byte-identical to columns 40-59.

use std::path::Path;

use crate::rom;
use super::AssetError;

/// Playfield rows, below the two status bar rows.
pub const ROWS: usize = 16;
/// A screen pointer covers exactly one display width of columns.
pub const SCREEN_COLUMNS: usize = 20;
/// The tile any run leaves uncovered.
pub const FILLER: u8 = 44;

const REPEAT: u8 = 0xFD;
const COLUMN_END: u8 = 0xFE;
const LIST_END: u8 = 0xFF;

/// World 1-1's screen list.
pub const LEVEL_1_1_LIST: usize = 0x0A198;

/// Level data lives in ROM bank 2, which the CPU sees at `0x4000`, so a
/// pointer of `0x62BE` is ROM file offset `0xA2BE`.
const BANK_BASE: usize = 0x8000;
const BANK_WINDOW: usize = 0x4000;

/// A decoded column: one tile id per playfield row, top to bottom.
pub type Column = [u8; ROWS];

fn rom_offset(pointer: u16) -> usize {
    pointer as usize - BANK_WINDOW + BANK_BASE
}

/// Solidity is not stored per column, so it comes from the tile id.
///
/// Measured from the cartridge's own collision test, which reads the
/// background tilemap in video RAM. `tools/probe_solidity.py` writes each of
/// the 256 ids into that tilemap in front of Mario and records the verdict.
/// Everything below `SOLID_FROM` passes through, everything at or above it is
/// solid apart from the ids below.
pub const SOLID_FROM: u8 = 0x60;

/// The one id at or above [`SOLID_FROM`] the game lets Mario through. In World
/// 1-1 it is decoration: isolated single cells, and a seven-tall bar floating
/// in open sky with nothing beneath it.
pub const PASSABLE: [u8; 1] = [0xF4];

/// Ids that hold Mario up but do not block him sideways. None of them appear
/// in World 1-1 and our level format carries one notion of solid, so
/// [`is_solid`] treats them as solid until a level needs the distinction.
pub const SEMI_SOLID: [u8; 4] = [0x68, 0x69, 0x6A, 0x7C];

pub fn is_solid(tile: u8) -> bool {
    tile >= SOLID_FROM && !PASSABLE.contains(&tile)
}

/// One column record starting at `rom[i]`. Returns the column and the offset
/// just past it, or `None` if a run does not fit, which is what tells level
/// data apart from whatever else is nearby in the ROM.
fn decode_column(rom: &[u8], mut i: usize) -> Option<(Column, usize)> {
    let mut column = [FILLER; ROWS];
    while i < rom.len() {
        let header = rom[i];
        if header == COLUMN_END {
            return Some((column, i + 1));
        }
        let row = (header >> 4) as usize;
        let count = match (header & 0x0F) as usize {
            0 => ROWS,
            n => n,
        };
        i += 1;
        if row + count > ROWS || i >= rom.len() {
            return None;
        }
        if rom[i] == REPEAT {
            let tile = *rom.get(i + 1)?;
            i += 2;
            column[row..row + count].fill(tile);
        } else {
            if i + count > rom.len() {
                return None;
            }
            column[row..row + count].copy_from_slice(&rom[i..i + count]);
            i += count;
        }
    }
    None
}

/// The 20 columns a single screen pointer draws.
fn decode_screen(rom: &[u8], pointer: u16) -> Option<Vec<Column>> {
    let mut columns = Vec::with_capacity(SCREEN_COLUMNS);
    let mut i = rom_offset(pointer);
    for _ in 0..SCREEN_COLUMNS {
        let (column, next) = decode_column(rom, i)?;
        columns.push(column);
        i = next;
    }
    Some(columns)
}

/// The `0xFF`-terminated pointer list at `start`.
pub fn screen_list(rom: &[u8], start: usize) -> Vec<u16> {
    let mut pointers = Vec::new();
    let mut i = start;
    while i + 1 < rom.len() && rom[i] != LIST_END {
        pointers.push(u16::from_le_bytes([rom[i], rom[i + 1]]));
        i += 2;
    }
    pointers
}

/// Every column of the level whose screen list starts at `start`. Stops at the
/// first screen that does not decode, so a bad start yields a short level
/// rather than garbage.
pub fn decode_level(rom: &[u8], start: usize) -> Vec<Column> {
    let mut columns = Vec::new();
    for pointer in screen_list(rom, start) {
        match decode_screen(rom, pointer) {
            Some(screen) => columns.extend(screen),
            None => break,
        }
    }
    columns
}

/// Decode a level from the verified ROM at `rom_path`.
pub fn extract_level(rom_path: impl AsRef<Path>, start: usize) -> Result<Vec<Column>, AssetError> {
    rom::verify_file(&rom_path).map_err(AssetError::Rom)?;
    let data = std::fs::read(&rom_path).map_err(AssetError::Io)?;
    let columns = decode_level(&data, start);
    if columns.is_empty() {
        return Err(AssetError::BadFormat);
    }
    Ok(columns)
}

/// Where Mario starts, and the row his feet rest on.
const SPAWN_COLUMN: usize = 6;
const GROUND_ROW: usize = 14;

/// Render decoded columns as our plain-text level format, which
/// `Level::from_file` loads: `#` solid, `.` empty, `M` spawn, `E` end trigger.
pub fn to_level_text(columns: &[Column]) -> String {
    let width = columns.len();
    let mut rows: Vec<Vec<u8>> = (0..ROWS)
        .map(|r| {
            columns
                .iter()
                .map(|c| if is_solid(c[r]) { b'#' } else { b'.' })
                .collect()
        })
        .collect();
    if width >= 2 {
        rows[GROUND_ROW - 1][SPAWN_COLUMN.min(width - 1)] = b'M';
        rows[GROUND_ROW - 1][width - 2] = b'E';
    }
    let mut out = String::with_capacity((width + 1) * ROWS);
    for row in rows {
        out.push_str(std::str::from_utf8(&row).unwrap());
        out.push('\n');
    }
    out
}

/// Columns with no solid cell anywhere: the holes Mario can fall through.
pub fn pits(columns: &[Column]) -> Vec<usize> {
    columns
        .iter()
        .enumerate()
        .filter(|(_, c)| !c.iter().any(|&t| is_solid(t)))
        .map(|(i, _)| i)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The worked example from the format notes: world column 87.
    ///
    /// ```text
    /// 02 53 40 | 37 fd f4 | e2 60 61 | fe
    /// 02       -> row 0, 2 tiles: 83, 64
    /// 37 fd f4 -> row 3, 7 tiles, all 244
    /// e2       -> row 14, 2 tiles: 96, 97
    /// ```
    #[test]
    fn decodes_the_worked_example_column() {
        let bytes = [0x02, 0x53, 0x40, 0x37, 0xFD, 0xF4, 0xE2, 0x60, 0x61, 0xFE];
        let (column, next) = decode_column(&bytes, 0).unwrap();
        assert_eq!(next, bytes.len());
        assert_eq!(column[0], 83);
        assert_eq!(column[1], 64);
        assert_eq!(&column[3..10], &[244; 7]);
        assert_eq!(column[2], FILLER, "rows no run covers are filler");
        assert_eq!(column[10], FILLER);
        assert_eq!((column[14], column[15]), (96, 97));
    }

    #[test]
    fn a_count_nibble_of_zero_means_a_full_column() {
        let bytes = [0x00, 0xFD, 0x63, 0xFE];
        let (column, _) = decode_column(&bytes, 0).unwrap();
        assert_eq!(column, [0x63; ROWS]);
    }

    #[test]
    fn a_run_that_overflows_the_column_is_rejected() {
        // row 14, 8 tiles: runs off the bottom, so this is not level data.
        let bytes = [0xE8, 1, 2, 3, 4, 5, 6, 7, 8, 0xFE];
        assert!(decode_column(&bytes, 0).is_none());
    }

    #[test]
    fn an_unterminated_record_is_rejected() {
        let bytes = [0x02, 0x53, 0x40];
        assert!(decode_column(&bytes, 0).is_none());
    }

    #[test]
    fn the_screen_list_stops_at_its_terminator() {
        let bytes = [0xBE, 0x62, 0x00, 0x62, 0xFF, 0x99, 0x99];
        assert_eq!(screen_list(&bytes, 0), vec![0x62BE, 0x6200]);
    }

    /// Measured from the cartridge, see `tools/probe_solidity.py`.
    #[test]
    fn solidity_matches_the_probed_rule() {
        assert!(!is_solid(0x00));
        assert!(!is_solid(0x5F));
        assert!(is_solid(0x60), "tile 96 is the ground");
        assert!(is_solid(0xE8), "tile 232 is the raised platform fill");
        assert!(is_solid(0xFF));
        assert!(!is_solid(0xF4), "0xF4 is decoration the game lets Mario through");
        for tile in SEMI_SOLID {
            assert!(is_solid(tile), "semi-solid ids are flattened to solid");
        }
    }

    #[test]
    fn level_text_marks_spawn_and_end() {
        let mut columns = vec![[FILLER; ROWS]; 10];
        for column in columns.iter_mut() {
            column[14] = 96;
        }
        let text = to_level_text(&columns);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), ROWS);
        assert_eq!(lines[14], "##########");
        assert_eq!(lines[13].chars().nth(SPAWN_COLUMN), Some('M'));
        assert_eq!(lines[13].chars().nth(8), Some('E'));
    }

    #[test]
    fn pits_are_columns_with_nothing_solid() {
        let mut columns = vec![[FILLER; ROWS]; 4];
        columns[0][14] = 96;
        columns[3][14] = 96;
        assert_eq!(pits(&columns), vec![1, 2]);
    }
}
