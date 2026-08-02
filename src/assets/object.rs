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
//! kind   what to create, with the top bit marking an expert-mode-only object
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
//! - The top bit of `kind` holds the record back for expert mode, the harder
//!   replay unlocked by finishing the game. Clearing it in a scratch copy of
//!   the cartridge makes that record spawn at the predicted column
//!   (`tools/probe_object_type_flag.py`), and setting the game's own
//!   expert-mode byte releases all of them at once (see [`EXPERT_ONLY`]).

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
/// The position mapping places all 37 of 1-1's records and all 46 of 1-2's on
/// a cell that is not solid, and misses on 15 of 1-3's 48, all of them kinds
/// 1-3 introduces. Tracing the spawns showed the slot positions match the
/// decode, so those records really do start inside terrain and that is the
/// game's doing rather than a decoding error.
pub const LEVEL_1_3_OBJECTS: usize = 0x0A0FE;

/// The three levels of World 1 and their object lists, in order.
pub const WORLD_1_OBJECTS: [(&str, usize); 3] = [
    ("1-1", LEVEL_1_1_OBJECTS),
    ("1-2", LEVEL_1_2_OBJECTS),
    ("1-3", LEVEL_1_3_OBJECTS),
];

/// World 2-1's and 2-2's object lists, in ROM bank 1 alongside World 2's
/// terrain.
///
/// `0xD010` gives the pointer (`0x5179` and `0x5222`) but not the bank, and
/// the bank cannot be read off the mapped window: doing that answers bank 3
/// for World 1-1, whose list is pinned at `0x0A002` in bank 2, because the
/// bank switched in at that instant is whatever the game last touched.
///
/// `tools/find_object_bank.py` asks the spawns instead. It plays to the level,
/// pairs each step of the read pointer with the slot that fills on the same
/// frame, and checks the kind byte against all three candidate banks. Bank 1
/// predicts every one of 2-1's 37 spawns and every one of 2-2's 29; bank 2
/// gets 2 and 1, bank 3 gets 0 and 1. World 1-1 is the control and comes back
/// bank 2, 16 of 16.
///
/// These are not wired into `sml extract-level` yet. 23 of 2-1's 37 spawns
/// land on a row its record's `y` byte does not give (a `y` of `0x13` reads as
/// row 1 and the slot holds 166, the bottom of the screen), and that includes
/// kinds the engine already implements, so writing them into a level file
/// would place them wrongly. Settling that comes first.
pub const LEVEL_2_1_OBJECTS: usize = 0x05179;
pub const LEVEL_2_2_OBJECTS: usize = 0x05222;

/// World 2-3's list, `0xD010` reading `0x529B` the moment the level opens.
///
/// Its bank went unmeasured for a while: three attempts at the spawn probe
/// killed the machine before the run got going, and bank 1 was only where its
/// two siblings are. The cartridge answers it directly instead. Bank 1's own
/// object table holds `0x529B` at index 5, which is World 2-3's slot, and the
/// same table reproduces every other measured list at its own index, so the
/// pointer is read from the bank the table sits in.
pub const LEVEL_2_3_OBJECTS: usize = 0x0529B;

/// World 2's three object lists, in order.
pub const WORLD_2_OBJECTS: [(&str, usize); 3] = [
    ("2-1", LEVEL_2_1_OBJECTS),
    ("2-2", LEVEL_2_2_OBJECTS),
    ("2-3", LEVEL_2_3_OBJECTS),
];

const RECORD: usize = 3;
const LIST_END: u8 = 0xFF;

/// Marks a record that only appears in expert mode, the harder replay the
/// cartridge unlocks once the game has been finished. The game gates it on
/// `hWinCount` at 0xFF9A, which is zero until then: sweeping every byte of
/// work RAM and high RAM for one that makes a marked record spawn finds that
/// address and no other (`tools/find_skip_flag.py`), and holding it non-zero
/// takes World 1-1 from 16 objects to all 37 (`tools/verify_skip_flag.py`).
/// The kaspermeerts disassembly agrees: `Call_24EF` in bank 0 reads
/// `hWinCount`, and skips the record on bit 7 only when it is zero.
pub const EXPERT_ONLY: u8 = 0x80;
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
    /// row above the ground they rest on. Can be negative: one expert-only
    /// record in World 1-3 has a row byte of 0, above the playfield.
    pub fn row(&self) -> i32 {
        (self.y & 0x0F) as i32 - ROW_OFFSET
    }

    /// Whether this record is one of the extra objects expert mode adds.
    pub fn expert_only(&self) -> bool {
        self.kind & EXPERT_ONLY != 0
    }

    /// The kind byte without the expert flag, so an expert-only record can
    /// still be compared against the kinds that spawn in normal play.
    pub fn kind_id(&self) -> u8 {
        self.kind & !EXPERT_ONLY
    }
}

/// Which pass through the cartridge a level is being played on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// The first play through. Expert-only records stay out.
    #[default]
    Normal,
    /// The replay unlocked by finishing the game, which adds every record.
    Expert,
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
    spawning_in(records, Mode::Normal)
}

/// The records a given mode acts on. Expert mode adds to normal play rather
/// than replacing it: every record spawns.
pub fn spawning_in(records: &[ObjectRecord], mode: Mode) -> Vec<ObjectRecord> {
    records
        .iter()
        .copied()
        .filter(|r| mode == Mode::Expert || !r.expert_only())
        .collect()
}

/// The two ground walkers. Both move one pixel every three frames and turn at
/// a wall; they differ at a ledge, where `WALKER` steps off and falls straight
/// down and `LEDGE_TURNER` turns around. Measured by writing a wall and then a
/// pit into the tilemap in front of each (`tools/probe_walker_turn.py`).
/// World 1-1's list uses `WALKER` ten times and `LEDGE_TURNER` once.
pub const WALKER: u8 = 0x00;
pub const LEDGE_TURNER: u8 = 0x04;

/// The jumper. Still for 54 frames, then a 32 px hop 15 px high over 48, on a
/// fixed table of rises rather than an accumulating speed
/// (`tools/trace_jumper.py`). It turns at a wall and hops off a ledge. World
/// 1-1's list uses it five times in normal play.
pub const HOPPER: u8 = 0x0E;

/// The two lift kinds. World 1-1 uses one of each, at columns 284 and 293,
/// either side of its exit door. Dropping Mario onto both showed they carry
/// him (`tools/probe_lift.py`), and their cycles were measured from the
/// running game (`docs/reference/objects.md`).
pub const LIFT_VERTICAL: u8 = 0x0A;
pub const LIFT_HORIZONTAL: u8 = 0x0B;

/// Where a level puts its lifts, as (column, row, vertical).
pub fn lift_spawns(records: &[ObjectRecord], mode: Mode) -> Vec<(usize, usize, bool)> {
    spawning_in(records, mode)
        .iter()
        .filter(|r| matches!(r.kind_id(), LIFT_VERTICAL | LIFT_HORIZONTAL))
        .filter_map(|r| {
            usize::try_from(r.row())
                .ok()
                .map(|row| (r.column(), row, r.kind_id() == LIFT_VERTICAL))
        })
        .collect()
}

/// The drop block: still until Mario stands on it, then it holds nine frames
/// and descends a pixel a frame, carrying him, without ever coming back
/// (`tools/probe_drop_block_support.py`). The most-used kind in the game, 50
/// records across nine of the twelve levels.
pub const DROP_BLOCK: u8 = 0x36;

/// Where a level puts its drop blocks, as (column, row).
pub fn drop_block_spawns(records: &[ObjectRecord], mode: Mode) -> Vec<(usize, usize)> {
    spawning_in(records, mode)
        .iter()
        .filter(|r| r.kind_id() == DROP_BLOCK)
        .filter_map(|r| usize::try_from(r.row()).ok().map(|row| (r.column(), row)))
        .collect()
}

/// The faller. World 1-3 is the only level of World 1 that uses it. It holds
/// position for 175 frames from spawn and then drops straight down at a pixel
/// a frame (`tools/probe_faller_trigger.py`), and it hurts Mario from every
/// side but a stomp (`tools/probe_object_contact.py`).
pub const FALLER: u8 = 0x0C;

/// Where a level puts its fallers, as (column, row).
pub fn faller_spawns(records: &[ObjectRecord], mode: Mode) -> Vec<(usize, usize)> {
    spawning_in(records, mode)
        .iter()
        .filter(|r| r.kind_id() == FALLER)
        .filter_map(|r| usize::try_from(r.row()).ok().map(|row| (r.column(), row)))
        .collect()
}

/// Bunbun. Flies level and leftwards in bursts, one pixel a frame for 41
/// frames and then 33 frames still, taking no notice of Mario
/// (`tools/measure_flyer.py`). Its contact sweep is a walker's
/// (`tools/probe_object_contact.py`). World 1-2 has 19 of them.
pub const BUNBUN: u8 = 0x42;

/// Where a level puts its Bunbuns, as (column, row).
pub fn bunbun_spawns(records: &[ObjectRecord], mode: Mode) -> Vec<(usize, usize)> {
    spawning_in(records, mode)
        .iter()
        .filter(|r| r.kind_id() == BUNBUN)
        .filter_map(|r| usize::try_from(r.row()).ok().map(|row| (r.column(), row)))
        .collect()
}

/// Gao. Never moves: 700 frames with the camera held still and not a pixel
/// on either axis (`tools/measure_level_kind.py`). What it does instead is
/// spit: a kind `0x23` fireball appears 4 pixels to its left every 137
/// frames, lives 117 of them, and travels up and to the left
/// (`tools/watch_kind_neighbours.py`).
pub const GAO: u8 = 0x3F;

/// Where a level puts its Gaos, as (column, row).
pub fn gao_spawns(records: &[ObjectRecord], mode: Mode) -> Vec<(usize, usize)> {
    spawning_in(records, mode)
        .iter()
        .filter(|r| r.kind_id() == GAO)
        .filter_map(|r| usize::try_from(r.row()).ok().map(|row| (r.column(), row)))
        .collect()
}

/// Honen, World 2's leaping fish. Its x never moves and its y runs a
/// symmetric arc of 114 pixels on a 161-frame cycle
/// (`tools/measure_level_kind.py 2-1 0x10`). 24 records across 2-1 and 2-3,
/// more than any other kind in World 2.
pub const HONEN: u8 = 0x10;

/// Where a level puts its Honen, as (column, row).
pub fn honen_spawns(records: &[ObjectRecord], mode: Mode) -> Vec<(usize, usize)> {
    spawning_in(records, mode)
        .iter()
        .filter(|r| r.kind_id() == HONEN)
        .filter_map(|r| usize::try_from(r.row()).ok().map(|row| (r.column(), row)))
        .collect()
}

/// King Totomesu, World 1-3's boss. Leaps 20 pixels straight up at a pixel
/// every two frames and back down on a 162-frame cycle, with its x fixed
/// (`tools/measure_level_kind.py`). It is the one kind measured that costs
/// Mario a life when he lands on it (`tools/probe_object_contact.py`).
pub const KING_TOTOMESU: u8 = 0x08;

/// Where a level puts King Totomesu, as (column, row). Only World 1-3 has
/// one, eight columns from the level's right edge.
pub fn king_totomesu_spawns(records: &[ObjectRecord], mode: Mode) -> Vec<(usize, usize)> {
    spawning_in(records, mode)
        .iter()
        .filter(|r| r.kind_id() == KING_TOTOMESU)
        .filter_map(|r| usize::try_from(r.row()).ok().map(|row| (r.column(), row)))
        .collect()
}

/// Where a level puts its jumpers, as (column, row).
pub fn fly_spawns(records: &[ObjectRecord], mode: Mode) -> Vec<(usize, usize)> {
    spawning_in(records, mode)
        .iter()
        .filter(|r| r.kind_id() == HOPPER)
        .filter_map(|r| usize::try_from(r.row()).ok().map(|row| (r.column(), row)))
        .collect()
}

/// Where a level puts its ground walkers, as (column, row, turns at ledges).
///
/// Deliberately only these two kinds. The remaining kinds spawn and move in the
/// cartridge, and nothing about how they move has been measured, so a level
/// file is short of enemies rather than carrying invented ones.
pub fn walker_spawns(records: &[ObjectRecord], mode: Mode) -> Vec<(usize, usize, bool)> {
    spawning_in(records, mode)
        .iter()
        .filter(|r| matches!(r.kind_id(), WALKER | LEDGE_TURNER))
        .filter_map(|r| {
            usize::try_from(r.row())
                .ok()
                .map(|row| (r.column(), row, r.kind_id() == LEDGE_TURNER))
        })
        .collect()
}

/// What the cartridge's own developers called each object type, from the
/// `enemies.asm` equate list of the `kaspermeerts/supermarioland`
/// disassembly. Nothing observable on the running game carries a name, so this
/// is the one thing here that cannot be measured.
///
/// It is not taken on trust. Seven of these ids had already been measured on
/// the cartridge with no reference to any name, and every one of them lands on
/// a name that matches what was measured:
///
/// | id | measured | name |
/// |---|---|---|
/// | `0x00` | walks, falls off ledges | Chibibo |
/// | `0x02` | oscillates 16 px down and back, never moves sideways | Pakkun Flower |
/// | `0x04` | walks, turns at ledges | Nokobon |
/// | `0x0A` `0x0B` | carry Mario on one axis each | platforms |
/// | `0x0C` | waits 175 frames, then falls at a pixel a frame | Falling Slab |
/// | `0x0E` | 54 frames still, then a 32 px hop | Fly |
///
/// An off-by-one anywhere in the table would break all seven at once, so the
/// alignment is checked rather than assumed.
///
/// Two entries disagree with what was measured and keep the measurement.
/// `0x0A` is named the horizontal platform there and `0x0B` the vertical one;
/// on the cartridge `0x0A` moves only on Y and `0x0B` only on X, watched twice
/// (`tools/measure_enemy_walk.py` and `tools/probe_lift.py`), so this table
/// says only "platform" for both. And `0x02`, named for a piranha plant, did
/// not hurt Mario at any vertical overlap through two full cycles
/// (`tools/probe_object_contact.py`), which is still unexplained.
///
/// Ids the disassembly leaves as `ENEMY_xx`, or names with a question mark
/// against them, are left unnamed here rather than guessed at.
pub fn kind_name(kind: u8) -> Option<&'static str> {
    Some(match kind & !EXPERT_ONLY {
        0x00 => "Chibibo",
        0x01 => "Chibibo, stomped",
        0x02 => "Pakkun Flower",
        0x03 => "Ganchan, spawning",
        0x04 => "Nokobon",
        0x05 => "Nokobon's bomb",
        0x06 => "Genkotsu",
        0x08 => "King Totomesu",
        0x09 => "Pompon Flower",
        0x0A | 0x0B => "platform",
        0x0C => "Falling Slab",
        0x0E => "Fly",
        0x0F => "Fly, stomped",
        0x10 => "Honen",
        0x13 => "lift",
        0x14 => "lift, ascending",
        0x16 => "Mekabon",
        0x17 => "Mekabon's head",
        0x18 => "Mekabon's body",
        0x1A => "Dragonzamasu",
        0x1C => "Mekabon, stomped",
        0x1D => "Yurarin",
        0x1E => "fire breath",
        0x20 => "Gunion",
        0x21 => "Gunion, exploding",
        0x22 => "Gunion's fireball",
        0x23 => "fireball",
        0x24 => "Yurarin Boo",
        0x25 => "Suu",
        0x27 => "explosion",
        0x28 => "mushroom in flight",
        0x29 => "mushroom",
        0x2A => "heart in flight",
        0x2B => "heart",
        0x2D => "flower, growing",
        0x2E => "flower",
        0x30 => "Torion",
        0x31 => "Tokotoko",
        0x32 => "Hiyoihoi",
        0x34 => "star",
        0x35 => "falling spike",
        0x36 => "drop block",
        0x37 => "drop block, falling",
        0x38 => "diagonal platform, north east",
        0x39 => "diagonal platform, north west",
        0x3A => "small vertical platform",
        0x3B => "small horizontal platform",
        0x3C => "Batadon",
        0x3D => "Batadon, stomped",
        0x3F => "Gao",
        0x40 => "Gao, stomped",
        0x42 => "Bunbun",
        0x43 => "Bunbun, stomped",
        0x45 => "arrow",
        0x47 => "Ganchan",
        0x48 => "Tamao",
        0x49 => "pipe cannon",
        0x4B => "Gira",
        0x51 => "Pompon spore",
        0x52 => "Roketon",
        0x53 => "chicken",
        0x54 => "Roto Disc",
        0x55 => "Pakkun Flower, upside down",
        0x56 => "Pionpi",
        0x57 => "Pionpi, stomped",
        0x59 => "Chikako",
        0x5A => "Gira, diagonal",
        0x5C => "cannonball",
        0x5D => "small cannonball, top",
        0x5E => "small cannonball, middle",
        0x5F => "small cannonball, bottom",
        0x60 => "Tatanga",
        0x61 => "Biokinton",
        _ => return None,
    })
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
        assert!(records[1].expert_only());
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
        assert!(!record.expert_only());
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
    fn every_measured_kind_lands_on_a_name_that_matches_it() {
        // Each of these was measured on the cartridge before any name was
        // read. They span 0x00 to 0x0E, so a table shifted by any amount
        // breaks at least one.
        assert_eq!(kind_name(WALKER), Some("Chibibo"));
        assert_eq!(kind_name(LEDGE_TURNER), Some("Nokobon"));
        assert_eq!(kind_name(HOPPER), Some("Fly"));
        assert_eq!(kind_name(FALLER), Some("Falling Slab"));
        assert_eq!(kind_name(LIFT_VERTICAL), Some("platform"));
        assert_eq!(kind_name(LIFT_HORIZONTAL), Some("platform"));
        assert_eq!(kind_name(0x02), Some("Pakkun Flower"));
    }

    #[test]
    fn an_expert_only_record_keeps_its_name() {
        assert_eq!(kind_name(WALKER | EXPERT_ONLY), Some("Chibibo"));
    }

    #[test]
    fn an_unnamed_kind_stays_unnamed() {
        // The disassembly leaves 0x07 as ENEMY_07, so we do too.
        assert_eq!(kind_name(0x07), None);
    }

    #[test]
    fn a_row_byte_of_zero_reads_as_above_the_playfield() {
        let record = ObjectRecord {
            x: 0x69,
            y: 0x10,
            kind: 0x84,
        };
        assert_eq!(record.row(), -2);
        assert!(record.expert_only());
    }
}
