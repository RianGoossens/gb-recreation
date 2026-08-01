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
/// Each list in the ROM opens with pointers that are not part of the level:
/// three for this level and four for 1-1. The last two of them are the level's
/// bonus rooms, which [`bonus_rooms`] reads.
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

/// World 2's screen lists, in ROM bank 1, reached by playing through all of
/// World 1 (`tools/run_through_levels.py`). 2-1 opens on `0x56CD`, 2-2 on
/// `0x5BA3` and 2-3 on `0x6100`, and each start is six bytes into the run the
/// scan finds, the same three-pointer prefix every World 1 list carries.
///
/// The columns confirm it: the walkthrough's capture of 2-1 matches all 320
/// of the decoded columns and 2-2 matches all 280. The run lost its last life
/// partway through 2-3, so that one is matched for 261 of its 360 columns.
///
/// The three pointers before each of them are the same shape as World 1's:
/// the last two decode to sealed coin rooms (see [`bonus_rooms`]).
pub const LEVEL_2_1_LIST: usize = 0x055C1;
pub const LEVEL_2_2_LIST: usize = 0x055E8;
pub const LEVEL_2_3_LIST: usize = 0x0560B;

/// The three levels of World 2, in order.
pub const WORLD_2: [(&str, usize); 3] = [
    ("2-1", LEVEL_2_1_LIST),
    ("2-2", LEVEL_2_2_LIST),
    ("2-3", LEVEL_2_3_LIST),
];

/// Every level whose screen list has been pinned by playing to it. These are
/// the control for [`level_list`], which derives all twelve from the ROM.
pub const MEASURED_LEVELS: [(&str, usize); 6] = [
    ("1-1", LEVEL_1_1_LIST),
    ("1-2", LEVEL_1_2_LIST),
    ("1-3", LEVEL_1_3_LIST),
    ("2-1", LEVEL_2_1_LIST),
    ("2-2", LEVEL_2_2_LIST),
    ("2-3", LEVEL_2_3_LIST),
];

/// Every bank holding level data opens with a 0x32-byte header, and the tile
/// graphics start right after it. The header is two tables of 16-bit pointers
/// indexed by `world * 3 + level`: thirteen for screen lists, then twelve for
/// object lists at [`OBJECT_TABLE`].
///
/// A bank holds only some of the twelve levels and the slots for the rest
/// repeat a triple rather than sitting empty, so a table read alone does not
/// say which world it belongs to. The six lists already measured by playing
/// pin it: the header reproduces all six exactly, at the index their world and
/// level number give, with the level's own first screen [`LIST_PREFIX`] bytes
/// past the entry every time.
pub const BANK_HEADER: usize = 0x32;
const SCREEN_TABLE: usize = 0x00;
const OBJECT_TABLE: usize = 0x1A;
/// A header entry points three pointers before the level's own first screen.
/// Those three are the world's opening screen and the level's two bonus rooms
/// (see [`bonus_rooms`]).
pub const LIST_PREFIX: usize = 6;

/// Which bank holds each world's data.
///
/// Worlds 1 and 2 are measured by playing to them. Worlds 3 and 4 come from
/// the headers: bank 1 is the only bank whose tables hold two distinct triples,
/// so it carries two worlds, and its screen table names the second at indices 9
/// to 11, which is World 4. Bank 3's table is a single triple repeated, and
/// World 3 is the only world left for it. Neither has been checked by playing
/// there.
pub const WORLD_BANKS: [usize; 4] = [0x08000, 0x04000, 0x0C000, 0x04000];

/// The cartridge's twelve levels, in play order.
pub const LEVEL_NAMES: [&str; 12] = [
    "1-1", "1-2", "1-3", "2-1", "2-2", "2-3", "3-1", "3-2", "3-3", "4-1", "4-2", "4-3",
];

/// The index a level is read from the header tables at.
pub fn level_index(name: &str) -> Option<usize> {
    LEVEL_NAMES.iter().position(|&n| n == name)
}

fn header_entry(rom: &[u8], bank: usize, table: usize, index: usize) -> Option<usize> {
    let at = bank + table + index * 2;
    let pointer = u16::from_le_bytes([*rom.get(at)?, *rom.get(at + 1)?]);
    rom_offset(pointer, bank)
}

/// Where a level's screen list starts, skipping the bonus-room prefix.
pub fn level_list(rom: &[u8], name: &str) -> Option<usize> {
    Some(level_list_head(rom, name)? + LIST_PREFIX)
}

/// Where a level's screen list starts including its prefix, which is what the
/// header points at and what [`bonus_rooms`] measures back from.
pub fn level_list_head(rom: &[u8], name: &str) -> Option<usize> {
    let index = level_index(name)?;
    header_entry(rom, WORLD_BANKS[index / 3], SCREEN_TABLE, index)
}

/// Where a level's object list starts, from the second header table.
pub fn level_objects(rom: &[u8], name: &str) -> Option<usize> {
    let index = level_index(name)?;
    header_entry(rom, WORLD_BANKS[index / 3], OBJECT_TABLE, index)
}

/// The name of the level a screen list belongs to, if it is one of the twelve.
pub fn level_of_list(rom: &[u8], start: usize) -> Option<&'static str> {
    LEVEL_NAMES
        .iter()
        .find(|&&name| level_list_head(rom, name) == Some(start))
        .copied()
}

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
    let open = reachable(columns);
    let at = |c: usize, r: usize| c * ROWS + r;
    (1..columns.len()).rev().find_map(|c| {
        (1..ROWS - 1)
            .rev()
            .find(|&r| standing(c, r) && standing(c - 1, r) && open[at(c, r)])
            .map(|r| (c, r))
    })
}

/// Which cells of the level Mario could get to from the spawn, ignoring
/// gravity: a flood fill over everything that is not solid.
///
/// The end-trigger placement needs this. Picking the rightmost cell that
/// merely looks like a floor put World 1-3's trigger in a one-tile pocket
/// sealed on four sides and World 2-3's in another, and in both cases the
/// level could not be finished. A cell the fill never reaches is no good
/// however much it looks like somewhere to stand.
fn reachable(columns: &[Column]) -> Vec<bool> {
    let width = columns.len();
    let mut seen = vec![false; width * ROWS];
    let at = |c: usize, r: usize| c * ROWS + r;
    let start = (SPAWN_COLUMN.min(width - 1), GROUND_ROW - 1);
    let mut queue = vec![start];
    seen[at(start.0, start.1)] = true;
    while let Some((c, r)) = queue.pop() {
        for (dc, dr) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
            let (nc, nr) = (c as i32 + dc, r as i32 + dr);
            if nc < 0 || nr < 0 || nc >= width as i32 || nr >= ROWS as i32 {
                continue;
            }
            let (nc, nr) = (nc as usize, nr as usize);
            if seen[at(nc, nr)] || is_solid(columns[nc][nr]) {
                continue;
            }
            seen[at(nc, nr)] = true;
            queue.push((nc, nr));
        }
    }
    seen
}

/// The two screens sitting immediately before a level's own first screen in
/// its pointer run.
///
/// Every list the scan finds opens with pointers that are not part of the
/// level: three for five of the six pinned levels and four for World 1-1.
/// The last two of them are the level's bonus rooms, the coin chambers the
/// raised exit door leads to. Decoding them shows what they are: enclosed
/// boxes with a solid floor and a solid left wall, filled with coins, where
/// the level's own screens are open terrain.
///
/// A level with only one bonus room stores the same pointer twice (World 2-3
/// has `0x6327` in both places).
pub fn bonus_rooms(rom: &[u8], list_start: usize) -> Option<[u16; 2]> {
    let first = list_start.checked_sub(4)?;
    Some([
        u16::from_le_bytes([*rom.get(first)?, *rom.get(first + 1)?]),
        u16::from_le_bytes([*rom.get(first + 2)?, *rom.get(first + 3)?]),
    ])
}

/// The 20 columns a single screen pointer draws, resolved in `bank`.
pub fn screen(rom: &[u8], pointer: u16, bank: usize) -> Option<Vec<Column>> {
    decode_screen(rom, pointer, bank)
}

/// The bank a ROM offset falls in, for resolving that offset's own pointers.
pub fn bank_of(offset: usize) -> usize {
    bank_base(offset)
}

/// Whether a screen is one of the coin rooms rather than open terrain: a solid
/// floor all the way across, and at least one coin.
///
/// That pair separates all twelve of the pinned levels' bonus rooms from all
/// six of their opening screens. A left wall is not part of the rule: most of
/// the rooms have one, and World 2-3's is an underwater chamber walled on both
/// sides but open across the top two rows.
pub fn is_bonus_room(columns: &[Column]) -> bool {
    let floor = columns.iter().all(|c| is_solid(c[ROWS - 1]));
    let coins = columns.iter().flatten().any(|&t| is_coin(t));
    floor && coins
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

/// The blocks World 2 loads over the shared atlas, as `(rom, vram, size)`.
///
/// A level's tile ids index whatever is in video RAM while it is playing, and
/// that is not one atlas for the whole cartridge. World 2's geometry decodes
/// exactly (all 320 of 2-1's columns match the running game) and renders as
/// garbage through World 1's tiles, which is the same failure that caught the
/// title screen once: data that decodes without error and draws the wrong
/// picture.
///
/// Read from video RAM at 2-1's opening (`tools/find_gameplay_tile_blocks.py
/// 2-1`). Most of the atlas is shared with World 1; four spans are replaced,
/// three of them from `0x04032` onward. That is bank 1 plus `0x32`, the same
/// offset into its own bank that World 1's `0x08032` is into bank 2, so a
/// world's own tiles sit at the start of the bank holding its levels.
///
/// Measured on 2-1 and 2-2, which load byte-identical block layouts, so the
/// overlay is per world rather than per level. 2-3 is not measured, but it
/// renders as a coherent underwater scene through the same spans.
pub const WORLD_2_TILE_BLOCKS: [(usize, usize, usize); 4] = [
    (0x04032, 0x8A00, 0x03C0),
    (0x04432, 0x9340, 0x0100),
    (0x04572, 0x9480, 0x0280),
    (0x09732, 0x9700, 0x0100),
];

/// The tile data in video RAM while the level whose list is at `list` plays.
///
/// World 1's levels use the shared atlas unchanged. World 2's overlay
/// [`WORLD_2_TILE_BLOCKS`] on top of it.
pub fn tiles_for_level(rom: &[u8], list: usize) -> Result<Vec<u8>, AssetError> {
    let mut vram = gameplay_tiles(rom)?.to_vec();
    if WORLD_2.iter().any(|&(_, start)| start == list) {
        for (from, to, size) in WORLD_2_TILE_BLOCKS {
            if from + size > rom.len() {
                return Err(AssetError::OutOfRange { end: from + size, len: rom.len() });
            }
            let at = to - VRAM_TILE_BASE;
            vram[at..at + size].copy_from_slice(&rom[from..from + size]);
        }
    }
    Ok(vram)
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
    let vram = tiles_for_level(&rom, list)?;

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
