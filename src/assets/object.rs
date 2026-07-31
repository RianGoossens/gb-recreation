//! The cartridge's object lists: where a level's enemies and items start.
//!
//! Terrain comes from the column records in [`super::level`]. Everything that
//! moves comes from a second, much smaller table, one list per level.
//!
//! A record is three bytes and a list ends on `0xFF`:
//!
//! ```text
//! x      position, in units of 16 pixels (two columns)
//! y      low nibble is the row plus 2, high nibble is a pixel offset on x
//! kind   what to create, with the top bit marking a record normal play skips
//! ```
//!
//! Records are sorted by `x`, which is how the game gets away with a single
//! forward read pointer: it walks the list once as the camera moves right and
//! never looks back.
//!
//! Every part of that was measured rather than read off a disassembly, using
//! the game's own read pointer at `0xD010` (`tools/trace_object_spawns.py`).
//! The account is in `docs/reference/objects.md`; in short:
//!
//! - The pointer steps past a record at an exact, repeatable camera position,
//!   and tying that to the column counter puts the object's world pixel at
//!   `16 * x`, with no leftover offset.
//! - Its slot in work RAM gets a Y of `8 * (y & 0x0F) + 16`, an OAM coordinate,
//!   which puts the object in playfield row `(y & 0x0F) - 2`.
//! - The slot's X is `0xBF + (y >> 4)`, so the high nibble shifts the object
//!   within its 16-pixel step. World 1-3 settles this on its own: two records
//!   at `x = 0x76`, one with `y = 0x0D` and one with `y = 0x8D`, spawn in the
//!   same frame exactly 8 pixels apart.
//! - The top bit of `kind` suppresses the record. Clearing it in a scratch copy
//!   of the cartridge makes that record spawn, at the predicted column
//!   (`tools/probe_object_type_flag.py`). What the flag selects for is open.

use std::path::Path;

use crate::rom;
use super::AssetError;

/// World 1-1's object list. This is the value the game's own read pointer at
/// `0xD010` holds the moment the level opens (`0x6002` in bank 2), so it is
/// the start by construction rather than by pattern matching.
pub const LEVEL_1_1_OBJECTS: usize = 0x0A002;

/// World 1-2's object list, read the same way after playing through 1-1 and
/// the bonus game (`tools/find_object_lists.py`). The lists sit back to back:
/// this one starts two bytes past the terminator of 1-1's.
pub const LEVEL_1_2_OBJECTS: usize = 0x0A073;

/// World 1-3's object list, read after playing through 1-2. It starts one byte
/// past 1-2's terminator.
///
/// This one is only partly understood. The position mapping places all 37 of
/// 1-1's records and all 46 of 1-2's on a cell that is not solid, and misses on
/// 17 of 1-3's 48. Every miss is a kind 1-3 introduces (`0x02` and `0x0C`, plus
/// one `0x36`), and the nine `0x02` records carry a `y` byte whose high nibble
/// is 4 or C, which no record in the other two levels uses. Something about
/// those kinds is read differently, and it has not been traced yet.
pub const LEVEL_1_3_OBJECTS: usize = 0x0A0FE;

/// The three levels of World 1 and their object lists, in order.
pub const WORLD_1_OBJECTS: [(&str, usize); 3] = [
    ("1-1", LEVEL_1_1_OBJECTS),
    ("1-2", LEVEL_1_2_OBJECTS),
    ("1-3", LEVEL_1_3_OBJECTS),
];

const RECORD: usize = 3;
const LIST_END: u8 = 0xFF;

/// Marks a record the game does not act on during normal play.
pub const SKIP: u8 = 0x80;
/// A row byte counts from the top of the screen, above the status bar.
pub const ROW_OFFSET: i32 = 2;
/// One step of `x` is 16 pixels, two columns.
pub const PIXELS_PER_STEP: usize = 16;

/// One record of a level's object list, as stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectRecord {
    pub x: u8,
    pub y: u8,
    pub kind: u8,
}

impl ObjectRecord {
    /// The object's world position in pixels, left edge.
    pub fn pixel_x(&self) -> usize {
        self.x as usize * PIXELS_PER_STEP + (self.y >> 4) as usize
    }

    /// The column the object starts in.
    pub fn column(&self) -> usize {
        self.pixel_x() / 8
    }

    /// The playfield row the object occupies. Ground-standing objects sit one
    /// row above the ground they rest on. Can be negative: one skipped record
    /// in World 1-3 has a row byte of 0, which would put it in the status bar.
    pub fn row(&self) -> i32 {
        (self.y & 0x0F) as i32 - ROW_OFFSET
    }

    /// Whether normal play skips this record.
    pub fn skipped(&self) -> bool {
        self.kind & SKIP != 0
    }

    /// The kind byte without the skip flag, so a skipped record can still be
    /// compared against the kinds that do spawn.
    pub fn kind_id(&self) -> u8 {
        self.kind & !SKIP
    }
}

/// Read a level's object list from `start` until its `0xFF` terminator.
pub fn object_list(rom: &[u8], start: usize) -> Vec<ObjectRecord> {
    let mut out = Vec::new();
    let mut i = start;
    while i + RECORD <= rom.len() && rom[i] != LIST_END {
        out.push(ObjectRecord {
            x: rom[i],
            y: rom[i + 1],
            kind: rom[i + 2],
        });
        i += RECORD;
    }
    out
}

/// The records normal play actually acts on, in the order the game meets them.
pub fn spawning(records: &[ObjectRecord]) -> Vec<ObjectRecord> {
    records.iter().copied().filter(|r| !r.skipped()).collect()
}

/// The ground walker, the one kind whose movement has been measured off the
/// cartridge: one pixel every three frames, off a ledge rather than turning at
/// it, straight down at a pixel a frame (`docs/reference/faithfulness.md`).
/// World 1-1's list uses it ten times, World 1-2 four, World 1-3 not at all.
pub const WALKER: u8 = 0x00;

/// The two lift kinds. World 1-1 uses one of each, at columns 284 and 293,
/// either side of its exit door. Dropping Mario onto both showed they carry
/// him (`tools/probe_lift.py`), and their cycles were measured from the
/// running game (`docs/reference/objects.md`).
pub const LIFT_VERTICAL: u8 = 0x0A;
pub const LIFT_HORIZONTAL: u8 = 0x0B;

/// Where a level puts its lifts, as (column, row, vertical).
pub fn lift_spawns(records: &[ObjectRecord]) -> Vec<(usize, usize, bool)> {
    spawning(records)
        .iter()
        .filter(|r| matches!(r.kind, LIFT_VERTICAL | LIFT_HORIZONTAL))
        .filter_map(|r| {
            usize::try_from(r.row())
                .ok()
                .map(|row| (r.column(), row, r.kind == LIFT_VERTICAL))
        })
        .collect()
}

/// Where a level puts its ground walkers, as (column, row).
///
/// Deliberately only this kind. The other kinds spawn and move, but nothing
/// about how they move has been measured, so a level file is short of enemies
/// rather than carrying invented ones.
pub fn walker_spawns(records: &[ObjectRecord]) -> Vec<(usize, usize)> {
    spawning(records)
        .iter()
        .filter(|r| r.kind == WALKER)
        .filter_map(|r| usize::try_from(r.row()).ok().map(|row| (r.column(), row)))
        .collect()
}

/// Verify the ROM, then read a level's object list out of it.
pub fn extract_objects(
    rom_path: impl AsRef<Path>,
    start: usize,
) -> Result<Vec<ObjectRecord>, AssetError> {
    rom::verify_file(&rom_path).map_err(AssetError::Rom)?;
    let data = std::fs::read(&rom_path).map_err(AssetError::Io)?;
    Ok(object_list(&data, start))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_list_stops_at_its_terminator() {
        let rom = [0x0C, 0x0F, 0x00, 0x21, 0x0C, 0x84, 0xFF, 0x99, 0x99, 0x99];
        let records = object_list(&rom, 0);
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].kind, 0x00);
        assert!(records[1].skipped());
    }

    #[test]
    fn a_record_places_itself_in_the_level() {
        let record = ObjectRecord {
            x: 0x0C,
            y: 0x0F,
            kind: 0x00,
        };
        assert_eq!(record.pixel_x(), 192);
        assert_eq!(record.column(), 24);
        assert_eq!(record.row(), 13);
        assert!(!record.skipped());
    }

    #[test]
    fn the_y_nibbles_are_read_separately() {
        // World 1-3's pair at x 0x76: same step, same row, 8 pixels apart.
        let low = ObjectRecord {
            x: 0x76,
            y: 0x0D,
            kind: 0x36,
        };
        let high = ObjectRecord {
            x: 0x76,
            y: 0x8D,
            kind: 0x36,
        };
        assert_eq!(low.row(), high.row());
        assert_eq!(high.pixel_x() - low.pixel_x(), 8);
        assert_eq!(low.pixel_x(), 0x76 * 16);
    }

    #[test]
    fn a_row_byte_of_zero_reads_as_above_the_playfield() {
        let record = ObjectRecord {
            x: 0x69,
            y: 0x10,
            kind: 0x84,
        };
        assert_eq!(record.row(), -2);
        assert!(record.skipped());
    }
}
