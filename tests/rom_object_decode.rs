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
    // Every slot fill the trace saw, and no more: the other 21 records are
    // held back for expert mode and the game walks straight past them.
    assert_eq!(object::spawning(&records).len(), 16);
}

#[test]
fn expert_mode_adds_the_rest_rather_than_replacing_them() {
    let Some(records) = records() else { return };
    // Holding 0xFF9A non-zero on the cartridge takes World 1-1 from 16 objects
    // to every record in the list, normal-play ones included
    // (`tools/verify_skip_flag.py`).
    let normal = object::spawning_in(&records, object::Mode::Normal);
    let expert = object::spawning_in(&records, object::Mode::Expert);
    assert_eq!(expert.len(), 37);
    assert!(normal.iter().all(|r| expert.contains(r)));
}

#[test]
fn expert_mode_puts_more_walkers_and_lifts_in_the_level() {
    let Some(records) = records() else { return };
    let normal = object::walker_spawns(&records, object::Mode::Normal);
    let expert = object::walker_spawns(&records, object::Mode::Expert);
    assert_eq!(normal.len(), 11);
    assert_eq!(expert.len(), 26);
    // The lifts either side of the exit are normal-play records, so the count
    // is the same and expert mode does not reach them.
    assert_eq!(object::lift_spawns(&records, object::Mode::Normal).len(), 2);
    assert_eq!(object::lift_spawns(&records, object::Mode::Expert).len(), 2);
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
    assert!(!first.expert_only());
}

#[test]
fn the_last_record_sits_near_the_level_end() {
    let Some(records) = records() else { return };
    let last = *records.last().unwrap();
    assert_eq!((last.x, last.y, last.kind), (0x92, 0x84, 0x0B));
    // The high nibble of 0x84 shifts it half a step right of column 292.
    assert_eq!(last.pixel_x(), 0x92 * 16 + 8);
    assert_eq!(last.column(), 293);
    // World 1-1 is 300 columns, so the list runs to the far end of it.
    assert!(last.column() < 300);
}

/// The tile a record's position lands on, or `None` if it is off the playfield.
fn tile_under(columns: &[level::Column], r: &ObjectRecord) -> Option<u8> {
    let row = usize::try_from(r.row()).ok()?;
    columns.get(r.column())?.get(row).copied()
}

#[test]
fn every_object_stands_in_an_empty_cell() {
    let Some(data) = rom() else { return };
    let columns = level::decode_level(&data, level::LEVEL_1_1_LIST);
    let records = object::object_list(&data, object::LEVEL_1_1_OBJECTS);
    for r in &records {
        let tile = tile_under(&columns, r).expect("1-1 places every record on the playfield");
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
        let below = columns[r.column()][r.row() as usize + 1];
        assert!(
            level::is_solid(below),
            "record at column {} row {} has nothing to stand on",
            r.column(),
            r.row(),
        );
    }
}

#[test]
fn all_three_world_1_levels_have_a_list() {
    let Some(data) = rom() else { return };
    let counts: Vec<usize> = object::WORLD_1_OBJECTS
        .iter()
        .map(|&(_, start)| object::object_list(&data, start).len())
        .collect();
    assert_eq!(counts, vec![37, 46, 48]);
}

#[test]
fn the_lists_sit_back_to_back_in_the_rom() {
    let Some(data) = rom() else { return };
    // Each list is a run of three-byte records ending on 0xFF, and the next
    // level's start was read off the game's pointer rather than computed, so
    // this checks the two agree. World 1-1's list is followed by a second
    // 0xFF, which is why 1-2 starts two bytes on rather than one.
    let ends: Vec<usize> = object::WORLD_1_OBJECTS
        .iter()
        .map(|&(_, start)| start + 3 * object::object_list(&data, start).len())
        .collect();
    assert_eq!(data[ends[0]], 0xFF);
    assert_eq!(data[ends[0] + 1], 0xFF);
    assert_eq!(object::LEVEL_1_2_OBJECTS, ends[0] + 2);
    assert_eq!(data[ends[1]], 0xFF);
    assert_eq!(object::LEVEL_1_3_OBJECTS, ends[1] + 1);
    assert_eq!(data[ends[2]], 0xFF);
}

#[test]
fn world_1_2_places_every_record_the_same_way_1_1_does() {
    let Some(data) = rom() else { return };
    let columns = level::decode_level(&data, level::LEVEL_1_2_LIST);
    for r in object::object_list(&data, object::LEVEL_1_2_OBJECTS) {
        let tile = tile_under(&columns, &r).expect("1-2 places every record on the playfield");
        assert!(
            !level::is_solid(tile),
            "record {:02X} {:02X} {:02X} lands inside a solid tile",
            r.x,
            r.y,
            r.kind,
        );
    }
}

#[test]
fn world_1_3_starts_some_kinds_inside_the_terrain() {
    let Some(data) = rom() else { return };
    let columns = level::decode_level(&data, level::LEVEL_1_3_LIST);
    let records = object::object_list(&data, object::LEVEL_1_3_OBJECTS);
    let inside: Vec<&ObjectRecord> = records
        .iter()
        .filter(|r| tile_under(&columns, r).is_some_and(level::is_solid))
        .collect();
    // Not a decode error. `tools/trace_level_objects.py` watched the game put
    // these in a slot at exactly the position decoded here, so 1-3 really does
    // start these kinds inside solid tiles. The count is pinned so a change to
    // the position mapping cannot move it unnoticed.
    assert_eq!(inside.len(), 15);
    let mut kinds: Vec<u8> = inside.iter().map(|r| r.kind_id()).collect();
    kinds.sort_unstable();
    kinds.dedup();
    assert_eq!(kinds, vec![0x02, 0x0C, 0x36]);
}

#[test]
fn only_one_record_in_world_1_falls_off_the_playfield() {
    let Some(data) = rom() else { return };
    let mut off = Vec::new();
    for (name, start) in object::WORLD_1_OBJECTS {
        let list = level::WORLD_1.iter().find(|(n, _)| *n == name).unwrap().1;
        let columns = level::decode_level(&data, list);
        for r in object::object_list(&data, start) {
            if tile_under(&columns, &r).is_none() {
                off.push((name, r));
            }
        }
    }
    // World 1-3's `69 10 84`: a row byte of 0 puts it two rows above the
    // playfield. It is an expert-mode record, so normal play never reads it.
    assert_eq!(off.len(), 1);
    assert_eq!(off[0].0, "1-3");
    assert!(off[0].1.expert_only());
}

#[test]
fn the_kinds_that_spawn_are_the_ones_the_trace_saw() {
    let Some(records) = records() else { return };
    let mut kinds: Vec<u8> = object::spawning(&records).iter().map(|r| r.kind).collect();
    kinds.sort_unstable();
    kinds.dedup();
    assert_eq!(kinds, vec![0x00, 0x04, 0x0A, 0x0B, 0x0E]);
}

/// World 2-1's and 2-2's lists, pinned by playing to each level and checking
/// the kind of every spawn against all three candidate banks
/// (`tools/find_object_bank.py`). A list has to be sorted by `x`, since the
/// game walks it with a single forward read pointer, and it has to terminate.
#[test]
fn world_2s_pinned_lists_are_sorted_and_terminated() {
    let Some(data) = rom() else { return };
    for (name, start, count) in [
        ("2-1", object::LEVEL_2_1_OBJECTS, 56),
        ("2-2", object::LEVEL_2_2_OBJECTS, 40),
        ("2-3", object::LEVEL_2_3_OBJECTS, 39),
    ] {
        let records = object::object_list(&data, start);
        assert_eq!(records.len(), count, "World {name}");
        assert!(
            records.windows(2).all(|w| w[0].x <= w[1].x),
            "World {name}'s records have to be sorted by x"
        );
    }
}

/// The same three bytes read in the wrong bank are a different list. Neither
/// of the banks the probe ruled out gives World 2-1 a sorted one, which is the
/// cheap structural check behind the measured answer.
#[test]
fn world_2_1s_list_only_sorts_in_the_bank_the_probe_found() {
    let Some(data) = rom() else { return };
    let sorted_at = |start: usize| {
        let records = object::object_list(&data, start);
        records.windows(2).all(|w| w[0].x <= w[1].x)
    };
    assert!(sorted_at(object::LEVEL_2_1_OBJECTS));
    assert!(!sorted_at(object::LEVEL_2_1_OBJECTS + 0x4000));
    assert!(!sorted_at(object::LEVEL_2_1_OBJECTS + 0x8000));
}

/// The control for the second header table. Five object lists were pinned by
/// reading the game's own read pointer at level open, three of them in a bank
/// that had to be measured from the spawns. The table at bank + 0x1A has to
/// reproduce all five at the same index the screen list sits at.
#[test]
fn the_bank_headers_reproduce_every_measured_object_list() {
    let Some(data) = rom() else { return };

    let measured = [
        ("1-1", object::LEVEL_1_1_OBJECTS),
        ("1-2", object::LEVEL_1_2_OBJECTS),
        ("1-3", object::LEVEL_1_3_OBJECTS),
        ("2-1", object::LEVEL_2_1_OBJECTS),
        ("2-2", object::LEVEL_2_2_OBJECTS),
    ];
    for (name, start) in measured {
        assert_eq!(
            level::level_objects(&data, name),
            Some(start),
            "header disagrees with the measured object list for {name}"
        );
    }
}

/// Every list the header names is a run of three-byte records ending on 0xFF,
/// sorted by x, and inside its own bank. Reading a list against the wrong bank
/// gives an unsorted run, which is what the probe used to tell World 2's bank.
#[test]
fn every_object_list_the_headers_name_is_sorted_and_terminated() {
    let Some(data) = rom() else { return };

    for name in level::LEVEL_NAMES {
        let start = level::level_objects(&data, name).expect("header entry");
        let records = object::object_list(&data, start);
        assert!(!records.is_empty(), "{name} has no records");
        let end = start + 3 * records.len();
        assert_eq!(data[end], 0xFF, "{name} is not terminated");
        assert_eq!(level::bank_of(start), level::bank_of(end), "{name} crosses a bank");
        let xs: Vec<u8> = records.iter().map(|r| r.x).collect();
        let mut sorted = xs.clone();
        sorted.sort_unstable();
        assert_eq!(xs, sorted, "{name} is not sorted by x");
    }
}

/// Bank 1's six object lists sit back to back in index order, each ending on
/// the 0xFF the next one starts after. World 2-3's list was the one the spawn
/// probe never got to measure, and this is the corroboration for taking it
/// from the header: it fills exactly the gap between 2-2's end and World 4-1's
/// start, with no bytes left over.
#[test]
fn bank_1s_object_lists_run_back_to_back() {
    let Some(data) = rom() else { return };

    let mut expected = object::LEVEL_2_1_OBJECTS;
    for name in ["2-1", "2-2", "2-3", "4-1", "4-2", "4-3"] {
        let start = level::level_objects(&data, name).expect("header entry");
        assert_eq!(start, expected, "World {name} does not follow its neighbour");
        let end = start + 3 * object::object_list(&data, start).len();
        assert_eq!(data[end], 0xFF);
        expected = end + 1;
    }
}
