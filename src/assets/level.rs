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
use crate::tiles::Tile;
use super::title::{tile_vram_addr, VRAM_TILE_BASE};
use super::{AssetError, TileSheet, DEFAULT_BGP};

/// Playfield rows, below the two status bar rows.
pub const ROWS: usize = 16;
/// A screen pointer covers exactly one display width of columns.
pub const SCREEN_COLUMNS: usize = 20;
/// The tile any run leaves uncovered.
pub const FILLER: u8 = 44;
/// The two rows the status bar occupies, above the playfield.
pub const STATUS_ROWS: usize = 2;
/// A full screen: the status bar rows plus the playfield.
pub const SCREEN_ROWS: usize = STATUS_ROWS + ROWS;

const REPEAT: u8 = 0xFD;
const COLUMN_END: u8 = 0xFE;
const LIST_END: u8 = 0xFF;

/// World 1-1's screen list, pinned by capturing all 300 of its columns from
/// the running cartridge.
pub const LEVEL_1_1_LIST: usize = 0x0A198;

/// World 1-2's screen list, pinned by playing through 1-1 and the bonus game
/// that follows it and reading which screen the next level opens on
/// (`tools/capture_next_level_opening.py`). It is `0x69A6`, which only this
/// list points at.
///
/// Each list in the ROM opens with three pointers that are not part of the
/// level: all three known starts sit six bytes into their run. What those
/// three are for is not known yet.
pub const LEVEL_1_2_LIST: usize = 0x0A1BD;

/// World 1-3's screen list, reached the same way: the walkthrough plays
/// through 1-1 and 1-2 and 1-3 opens on `0x6E2F`, which only this list points
/// at, with the columns the game then draws matching the decode.
pub const LEVEL_1_3_LIST: usize = 0x0A1E0;

/// The three levels of World 1, in order.
pub const WORLD_1: [(&str, usize); 3] = [
    ("1-1", LEVEL_1_1_LIST),
    ("1-2", LEVEL_1_2_LIST),
    ("1-3", LEVEL_1_3_LIST),
];

/// A switchable ROM bank is 16 KB, and the CPU sees it at `0x4000`. World 1's
/// data is in bank 2, so a pointer of `0x62BE` is ROM file offset `0xA2BE`.
/// Later worlds live in other banks, which is why the bank is derived from
/// wherever a screen list was found rather than fixed.
const BANK_WINDOW: usize = 0x4000;

/// The base file offset of the bank containing `offset`.
fn bank_base(offset: usize) -> usize {
    offset & !(BANK_WINDOW - 1)
}

/// A decoded column: one tile id per playfield row, top to bottom.
pub type Column = [u8; ROWS];

/// Resolve a screen pointer to a ROM file offset, given the bank it is read
/// from. `None` if it does not point into the bank window at all, which is how
/// a scan tells a real screen list from a run of bytes that merely ends in
/// `0xFF`.
fn rom_offset(pointer: u16, bank: usize) -> Option<usize> {
    let pointer = pointer as usize;
    if !(BANK_WINDOW..BANK_WINDOW * 2).contains(&pointer) {
        return None;
    }
    Some(pointer - BANK_WINDOW + bank)
}

/// Solidity is not stored per column, so it comes from the tile id.
///
/// Measured from the cartridge's own collision test, which reads the
/// background tilemap in video RAM. `tools/probe_solidity.py` writes each of
/// the 256 ids into that tilemap in front of Mario and records the verdict.
/// Everything below `SOLID_FROM` passes through, everything at or above it is
/// solid apart from the ids below.
pub const SOLID_FROM: u8 = 0x60;

/// The one id at or above [`SOLID_FROM`] the game lets Mario through, because
/// it is a coin. Coins live in the background tilemap rather than in the
/// object table, which is why a column re-read after Mario has walked through
/// it is missing the ones he took. Found by playing until the coin counter
/// moved and looking at which tilemap cell changed on that frame
/// (`tools/find_coin_tile.py`). World 1-1 has 18 of them.
pub const COIN: u8 = 0xF4;
pub const PASSABLE: [u8; 1] = [COIN];

pub fn is_coin(tile: u8) -> bool {
    tile == COIN
}

/// Ids that hold Mario up from above but do not block him sideways. The
/// second screen list in the ROM (the candidate World 1-2) lays `0x68`,
/// `0x69` and `0x6A` out as horizontal runs with distinct end caps over
/// hanging supports, which is the shape of a platform. World 1-1 barely uses
/// them.
pub const SEMI_SOLID: [u8; 4] = [0x68, 0x69, 0x6A, 0x7C];

/// Blocks Mario from every direction.
pub fn is_solid(tile: u8) -> bool {
    tile >= SOLID_FROM && !PASSABLE.contains(&tile) && !is_platform(tile)
}

/// Holds Mario up from above only.
pub fn is_platform(tile: u8) -> bool {
    SEMI_SOLID.contains(&tile)
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
fn decode_screen(rom: &[u8], pointer: u16, bank: usize) -> Option<Vec<Column>> {
    let mut columns = Vec::with_capacity(SCREEN_COLUMNS);
    let mut i = rom_offset(pointer, bank)?;
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
    let bank = bank_base(start);
    for pointer in screen_list(rom, start) {
        match decode_screen(rom, pointer, bank) {
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

/// Top-left tile of the cartridge's level-exit door, a 2x2 block of
/// `0x13 0x21` over `0x24 0x39`. It appears exactly twice in World 1-1 and
/// twice in World 1-2, both times in the same column: once at row 0, the
/// raised door that leads to the bonus route, and once at row 13, the one at
/// ground level. The lower one is the level's exit.
///
/// World 1-3 has neither, because it ends the world rather than leading to
/// the next level.
pub const EXIT_DOOR: u8 = 0x13;

/// Where to put the end trigger in a level with no exit door: the rightmost
/// cell Mario could actually stand on and reach.
///
/// A stand-in either way, but the previous one (two columns from the right
/// edge, at the ground row) put World 1-3's trigger at column 298 row 13,
/// a one-tile pocket with solid tiles above, below and either side. The level
/// could not be finished at all. Mario is 12 px tall, so standing takes two
/// free rows, and he has to arrive from somewhere: without the check on the
/// column to the left, the rightmost floor in 1-3 is another sealed shaft.
pub fn far_end(columns: &[Column]) -> Option<(usize, usize)> {
    let standing = |c: usize, r: usize| {
        !is_solid(columns[c][r]) && !is_solid(columns[c][r - 1]) && is_solid(columns[c][r + 1])
    };
    (1..columns.len()).rev().find_map(|c| {
        (1..ROWS - 1)
            .rev()
            .find(|&r| standing(c, r) && standing(c - 1, r))
            .map(|r| (c, r))
    })
}

/// The exit door's top-left cell, if the level has one.
pub fn exit_door(columns: &[Column]) -> Option<(usize, usize)> {
    columns
        .iter()
        .enumerate()
        .flat_map(|(c, col)| {
            col.iter()
                .enumerate()
                .filter(|(_, &t)| t == EXIT_DOOR)
                .map(move |(r, _)| (c, r))
        })
        .max_by_key(|&(_, r)| r)
}

/// Render decoded columns as our plain-text level format, which
/// `Level::from_file` loads: `#` solid, `^` a one-way platform, `C` a coin,
/// `.` empty, `M` spawn, `E` end trigger.
pub fn to_level_text(columns: &[Column]) -> String {
    to_level_text_with_objects(columns, &[])
}

/// The same, with object markers stamped in: `G` for a ground walker, `V` and
/// `H` for the two lifts.
///
/// Only kinds whose behaviour has actually been measured are passed in here
/// (see [`super::object`]); the rest of a level's objects are left out rather
/// than guessed at, so a level file is short of them rather than wrong about
/// them.
pub fn to_level_text_with_objects(columns: &[Column], objects: &[(usize, usize, u8)]) -> String {
    let width = columns.len();
    let mut rows: Vec<Vec<u8>> = (0..ROWS)
        .map(|r| {
            columns
                .iter()
                .map(|c| match c[r] {
                    t if is_solid(t) => b'#',
                    t if is_platform(t) => b'^',
                    t if is_coin(t) => b'C',
                    _ => b'.',
                })
                .collect()
        })
        .collect();
    for &(column, row, marker) in objects {
        // The cell has to be free. A record that would land on a coin or on
        // Mario's spawn is dropped rather than allowed to replace it.
        if let Some(cell) = rows.get_mut(row).and_then(|r| r.get_mut(column)) {
            if *cell == b'.' {
                *cell = marker;
            }
        }
    }
    if width >= 2 {
        rows[GROUND_ROW - 1][SPAWN_COLUMN.min(width - 1)] = b'M';
        // The door when the level has one, the far end otherwise.
        let end = exit_door(columns)
            .or_else(|| far_end(columns))
            .unwrap_or((width - 2, GROUND_ROW - 1));
        rows[end.1][end.0] = b'E';
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
        .filter(|(_, c)| !c.iter().any(|&t| is_solid(t) || is_platform(t)))
        .map(|(i, _)| i)
        .collect()
}

/// Every screen list in the ROM, found by structure rather than by a pointer
/// table: nothing in the ROM holds World 1-1's list address, so there is no
/// table to read. A candidate is a run of 16-bit pointers into the bank
/// window, terminated by `0xFF`, where every pointer decodes 20 valid column
/// records and most of the resulting columns contain something solid.
///
/// Returns each list's ROM offset and its pointers, longest run first in each
/// cluster: a real list also decodes from two bytes in, minus its first
/// screen, so overlapping candidates are collapsed.
pub fn find_screen_lists(rom: &[u8]) -> Vec<(usize, Vec<u16>)> {
    const MIN_SCREENS: usize = 6;
    // Deliberately not "has ground under it": World 1-2 is built on floating
    // platforms over open sky and only 40% of its columns have anything solid
    // on the bottom two rows, which a ground test throws away.
    const MIN_SOLID: f32 = 0.7;

    let mut found: Vec<(usize, Vec<u16>)> = Vec::new();
    // Bank 0 is always mapped at 0x0000, so no pointer into the 0x4000 window
    // ever resolves to it. The scan starts at the first switchable bank.
    for start in BANK_WINDOW..rom.len() {
        let pointers = screen_list(rom, start);
        if pointers.len() < MIN_SCREENS {
            continue;
        }
        let columns = decode_level(rom, start);
        if columns.len() != pointers.len() * SCREEN_COLUMNS {
            continue;
        }
        let solid = columns
            .iter()
            .filter(|c| c.iter().any(|&t| is_solid(t) || is_platform(t)))
            .count();
        if (solid as f32) < columns.len() as f32 * MIN_SOLID {
            continue;
        }
        found.push((start, pointers));
    }

    let mut kept: Vec<(usize, Vec<u16>)> = Vec::new();
    for entry in found {
        match kept.last() {
            Some((start, pointers)) if entry.0 < start + 2 * pointers.len() => {}
            _ => kept.push(entry),
        }
    }
    kept
}

/// Where gameplay's tile graphics live in the ROM, and how much.
///
/// One contiguous copy: ROM `0x08032` to VRAM `0x8000`, the whole 0x1800 of
/// tile data. Found by reading video RAM after a level has loaded and locating
/// each chunk of it in the ROM file (`tools/find_gameplay_tile_blocks.py`),
/// which is the same technique that pinned the title screen's blocks.
///
/// The title screen draws from the same bank-2 atlas but copies three slices
/// of it to different VRAM addresses. Reusing its layout for a level renders
/// font glyphs, because a level's tile ids then index the wrong tiles.
pub const TILES_ROM_OFFSET: usize = 0x08032;
pub const TILES_SIZE: usize = 0x1800;

/// Gameplay's VRAM tile data, straight from the ROM.
pub fn gameplay_tiles(rom: &[u8]) -> Result<&[u8], AssetError> {
    let end = TILES_ROM_OFFSET + TILES_SIZE;
    if end > rom.len() {
        return Err(AssetError::OutOfRange { end, len: rom.len() });
    }
    Ok(&rom[TILES_ROM_OFFSET..end])
}

/// A level's own graphics, as the cartridge draws them.
///
/// Returns a deduplicated sheet plus a `SCREEN_COLUMNS` by [`SCREEN_ROWS`] map
/// of indices into it, starting at world column `first_column`. The top two
/// rows are blank: that is where the status bar goes, which the game draws
/// with a mid-frame scanline split rather than from the level.
pub fn extract_screen(
    rom_path: impl AsRef<Path>,
    list: usize,
    first_column: usize,
) -> Result<(TileSheet, Vec<u8>), AssetError> {
    rom::verify_file(&rom_path).map_err(AssetError::Rom)?;
    let rom = std::fs::read(&rom_path).map_err(AssetError::Io)?;
    let columns = decode_level(&rom, list);
    if columns.is_empty() {
        return Err(AssetError::BadFormat);
    }
    let vram = gameplay_tiles(&rom)?;

    let mut tiles: Vec<Tile> = vec![Tile { pixels: [[0; 8]; 8] }];
    let mut seen: std::collections::HashMap<[u8; 16], u8> = std::collections::HashMap::new();
    let mut cells = vec![0u8; SCREEN_COLUMNS * STATUS_ROWS];
    for row in 0..ROWS {
        for i in 0..SCREEN_COLUMNS {
            let column = &columns[(first_column + i) % columns.len()];
            let offset = tile_vram_addr(column[row]) - VRAM_TILE_BASE;
            let raw: [u8; 16] = vram[offset..offset + 16].try_into().unwrap();
            let index = *seen.entry(raw).or_insert_with(|| {
                tiles.push(Tile::decode(&raw));
                (tiles.len() - 1) as u8
            });
            cells.push(index);
        }
    }
    Ok((TileSheet::new(tiles, DEFAULT_BGP), cells))
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
        assert!(!is_solid(COIN), "a coin does not block Mario");
        assert!(is_coin(0xF4));
        for tile in SEMI_SOLID {
            assert!(is_platform(tile), "these hold Mario up from above only");
            assert!(!is_solid(tile), "and do not block him sideways");
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
        // No exit door here, so the trigger goes on the rightmost cell Mario
        // could stand on, which on a flat floor is the last column.
        assert_eq!(lines[13].chars().nth(9), Some('E'));
    }

    #[test]
    fn pits_are_columns_with_nothing_solid() {
        let mut columns = vec![[FILLER; ROWS]; 4];
        columns[0][14] = 96;
        columns[3][14] = 96;
        assert_eq!(pits(&columns), vec![1, 2]);
    }
}
