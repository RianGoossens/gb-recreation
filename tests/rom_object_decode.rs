//! Decoding World 1-1's object list out of the cartridge.
//!
//! Skips when the ROM is absent, like every other ROM-gated test.
//!
//! The numbers here are what the running game did, not what the decoder
//! produced. `tools/trace_object_spawns.py` watched the game's own read
//! pointer walk this list across the whole level: 36 moves over 37 records,
//! filling 16 object slots, each at the column the record predicts. What these
//! tests pin is that the shipped decoder still agrees with that trace.

use sml::assets::level;
use sml::assets::object::{self, ObjectRecord};

const ROM: &str = "super_mario_land.gb";

fn rom() -> Option<Vec<u8>> {
    std::fs::read(ROM).ok()
}

fn records() -> Option<Vec<ObjectRecord>> {
    rom().map(|data| object::object_list(&data, object::LEVEL_1_1_OBJECTS))
}

#[test]
fn world_1_1_has_thirty_seven_object_records() {
    let Some(records) = records() else { return };
    assert_eq!(records.len(), 37);
}

#[test]
fn sixteen_of_them_spawn_in_normal_play() {
    let Some(records) = records() else { return };
    // Every slot fill the trace saw, and no more: the other 21 records carry
    // the skip bit and the game walks straight past them.
    assert_eq!(object::spawning(&records).len(), 16);
}

#[test]
fn the_list_is_sorted_by_position() {
    let Some(records) = records() else { return };
    // A single forward read pointer only works if the list never goes back.
    assert!(records.windows(2).all(|w| w[0].x <= w[1].x));
}

#[test]
fn the_first_record_is_the_one_the_pointer_moved_on_first() {
    let Some(records) = records() else { return };
    let first = records[0];
    assert_eq!((first.x, first.y, first.kind), (0x0C, 0x0F, 0x00));
    assert_eq!(first.column(), 24);
    assert_eq!(first.row(), 13);
    assert!(!first.skipped());
}

#[test]
fn the_last_record_sits_near_the_level_end() {
    let Some(records) = records() else { return };
    let last = *records.last().unwrap();
    assert_eq!((last.x, last.y, last.kind), (0x92, 0x84, 0x0B));
    assert_eq!(last.column(), 292);
    // World 1-1 is 300 columns, so the list runs to the far end of it.
    assert!(last.column() < 300);
}

#[test]
fn every_object_stands_in_an_empty_cell() {
    let Some(data) = rom() else { return };
    let columns = level::decode_level(&data, level::LEVEL_1_1_LIST);
    let records = object::object_list(&data, object::LEVEL_1_1_OBJECTS);
    for r in &records {
        let tile = columns[r.column()][r.row()];
        assert!(
            !level::is_solid(tile),
            "record {:02X} {:02X} {:02X} lands inside solid tile {tile} at column {} row {}",
            r.x,
            r.y,
            r.kind,
            r.column(),
            r.row(),
        );
    }
}

#[test]
fn the_ground_level_records_have_ground_under_them() {
    let Some(data) = rom() else { return };
    let columns = level::decode_level(&data, level::LEVEL_1_1_LIST);
    let records = object::object_list(&data, object::LEVEL_1_1_OBJECTS);
    // Row 13 is the row directly above World 1-1's ground. Anything the list
    // puts there has to be standing on something, or the position mapping is
    // off by a row.
    let ground_level: Vec<&ObjectRecord> = records.iter().filter(|r| r.row() == 13).collect();
    assert_eq!(ground_level.len(), 16);
    for r in ground_level {
        let below = columns[r.column()][r.row() + 1];
        assert!(
            level::is_solid(below),
            "record at column {} row {} has nothing to stand on",
            r.column(),
            r.row(),
        );
    }
}

#[test]
fn the_kinds_that_spawn_are_the_ones_the_trace_saw() {
    let Some(records) = records() else { return };
    let mut kinds: Vec<u8> = object::spawning(&records).iter().map(|r| r.kind).collect();
    kinds.sort_unstable();
    kinds.dedup();
    assert_eq!(kinds, vec![0x00, 0x04, 0x0A, 0x0B, 0x0E]);
}
