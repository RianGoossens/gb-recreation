//! Decoding World 1-1 out of the cartridge.
//!
//! Skips when the ROM is absent (fresh checkout, CI), like every other
//! ROM-gated test. The decode was scored 88/88 against every column the
//! running game reveals, with the runner-up candidate start at 45%; that
//! scoring lives in `docs/reference/level-1-1.md`. What these tests pin is
//! that the shipped decoder still produces that same level.

use sml::assets::level::{self, Column, ROWS, SCREEN_COLUMNS};

const ROM: &str = "super_mario_land.gb";
const COLUMNS: usize = 300;

fn rom() -> Option<Vec<u8>> {
    std::fs::read(ROM).ok()
}

const CAPTURE: &str = "assets/extracted/captured_columns.txt";

/// The capture file is one column per line, 16 tile ids, a blank line between
/// levels. Written by `tools/run_through_levels.py` and gitignored.
fn captured_levels(text: &str) -> Vec<Vec<Vec<u8>>> {
    text.split("\n\n")
        .filter(|block| !block.trim().is_empty())
        .map(|block| {
            block
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(|line| line.split_whitespace().map(|t| t.parse().unwrap()).collect())
                .collect()
        })
        .collect()
}

fn level_1_1() -> Option<Vec<Column>> {
    rom().map(|data| level::decode_level(&data, level::LEVEL_1_1_LIST))
}

#[test]
fn world_1_1_is_fifteen_screens_of_twenty_columns() {
    let Some(data) = rom() else { return };
    let pointers = level::screen_list(&data, level::LEVEL_1_1_LIST);
    assert_eq!(
        pointers,
        vec![
            0x62BE, 0x6200, 0x62BE, 0x6381, 0x645F, 0x62BE, 0x650D, 0x62BE, 0x6200, 0x6381,
            0x6200, 0x62BE, 0x65DE, 0x66B5, 0x67BB
        ]
    );
    let columns = level_1_1().unwrap();
    assert_eq!(columns.len(), pointers.len() * SCREEN_COLUMNS);
    assert_eq!(columns.len(), COLUMNS);
}

/// Six of the fifteen screen pointers repeat, so the columns they draw have to
/// come out byte-identical. This is what an earlier, screen-less reading of the
/// data mistook for the level itself repeating by coincidence.
#[test]
fn a_reused_screen_pointer_draws_identical_columns() {
    let Some(columns) = level_1_1() else { return };
    for i in 0..SCREEN_COLUMNS {
        assert_eq!(
            columns[i],
            columns[40 + i],
            "column {i} and column {} share screen pointer 0x62BE",
            40 + i
        );
    }
}

/// The worked example from the format notes, decoded from the real ROM rather
/// than from a handful of bytes in a unit test.
#[test]
fn world_column_87_matches_its_record() {
    let Some(columns) = level_1_1() else { return };
    let column = columns[87];
    assert_eq!((column[0], column[1]), (83, 64));
    assert_eq!(&column[3..10], &[244; 7]);
    assert_eq!((column[14], column[15]), (96, 97));
    for row in [2, 10, 11, 12, 13] {
        assert_eq!(column[row], level::FILLER);
    }
}

#[test]
fn the_ground_band_runs_under_the_opening_screen() {
    let Some(columns) = level_1_1() else { return };
    for (i, column) in columns.iter().take(SCREEN_COLUMNS).enumerate() {
        assert_eq!((column[14], column[15]), (96, 97), "column {i}");
    }
}

/// Rian played the cartridge and reported holes an earlier reading had paved
/// over. These are the nine columns of real pit.
#[test]
fn the_decoded_level_keeps_its_nine_pits() {
    let Some(columns) = level_1_1() else { return };
    assert_eq!(
        level::pits(&columns),
        vec![89, 90, 138, 139, 247, 248, 249, 261, 262]
    );
}

/// 0xF4 is decoration the game lets Mario walk through. Treating it as solid
/// blocks the route dead at world column 269.
#[test]
fn the_passable_decoration_is_not_treated_as_solid() {
    let Some(columns) = level_1_1() else { return };
    let cells: usize = columns
        .iter()
        .map(|c| c.iter().filter(|&&t| t == 0xF4).count())
        .sum();
    assert_eq!(cells, 18, "0xF4 appears 18 times in World 1-1");
    assert!(!level::is_solid(0xF4));
}

/// The plain-text level the engine loads is generated from the decode, so its
/// shape has to follow from it.
#[test]
fn the_text_level_has_a_row_per_playfield_row() {
    let Some(columns) = level_1_1() else { return };
    let text = level::to_level_text(&columns);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), ROWS);
    for line in &lines {
        assert_eq!(line.chars().count(), COLUMNS);
    }
    assert!(lines[13].contains('M'), "spawn marker is on the row above the ground");
    assert!(lines[13].contains('E'), "end trigger is on the row above the ground");
}

/// There is no pointer table anywhere in the ROM holding World 1-1's list
/// address, so the lists have to be found by their own structure. Super Mario
/// Land has twelve levels and the scan finds exactly twelve lists, spread over
/// the three switchable banks, one of them containing 1-1's verified start.
#[test]
fn the_scan_finds_all_twelve_screen_lists() {
    let Some(data) = rom() else { return };
    let lists = level::find_screen_lists(&data);
    let starts: Vec<usize> = lists.iter().map(|(start, _)| *start).collect();
    assert_eq!(
        starts,
        vec![
            0x055BB, 0x055E2, 0x05605, 0x05630, 0x05665, 0x056AE, 0x0A190,
            0x0A1B7, 0x0A1DA, 0x0D03F, 0x0D074, 0x0D0A7,
        ]
    );

    let world_1 = &lists[6];
    assert!(
        world_1.0 <= level::LEVEL_1_1_LIST
            && level::LEVEL_1_1_LIST < world_1.0 + 2 * world_1.1.len(),
        "the seventh list has to contain World 1-1's verified start"
    );
}

/// A screen pointer is resolved against the bank its list was found in, so a
/// list in bank 1 or bank 3 decodes against its own columns. Every list the
/// scan returns produces exactly 20 columns per pointer; a bank mix-up would
/// leave short levels where a column record failed to decode.
#[test]
fn every_found_list_decodes_fully_in_its_own_bank() {
    let Some(data) = rom() else { return };
    for (start, pointers) in level::find_screen_lists(&data) {
        let columns = level::decode_level(&data, start);
        assert_eq!(
            columns.len(),
            pointers.len() * level::SCREEN_COLUMNS,
            "list at {start:#07X} decoded short"
        );
    }
}

/// The strongest check there is on the decode: drive the real cartridge
/// through the levels and compare every column it draws.
///
/// The capture file is written by `tools/run_through_levels.py` and is
/// gitignored, so this skips when it is absent. Each captured level is matched
/// against whichever pinned level it starts like, and then has to agree
/// exactly for as far as the run got. Short blocks are Mario dying early and
/// the level restarting, which is still a real comparison, just a shorter one.
/// A block that starts mid-level (World 1-3 restarts from a checkpoint) is
/// skipped rather than mismatched, and so is one longer than the level it
/// matches, which means the run lost a life and the block runs on into a
/// restart (World 2-3 does this).
#[test]
fn the_decode_matches_every_column_the_real_game_draws() {
    let Ok(text) = std::fs::read_to_string(CAPTURE) else { return };
    let Some(data) = rom() else { return };

    let mut checked: Vec<(&str, usize)> = Vec::new();
    for captured in captured_levels(&text) {
        let Some((name, columns)) = level::MEASURED_LEVELS
            .iter()
            .map(|&(name, list)| (name, level::decode_level(&data, list)))
            // A whole screen, not one column: World 2-1 and 2-2 open on the
            // same first column and only part company later in the screen.
            .find(|(_, columns)| {
                let n = level::SCREEN_COLUMNS.min(captured.len());
                (0..n).all(|i| columns[i][..] == captured[i][..])
            })
        else {
            continue;
        };
        if captured.len() > columns.len() {
            continue;
        }
        let reached = captured.len().min(columns.len());
        let mismatches: Vec<usize> = (0..reached)
            .filter(|&i| columns[i][..] != captured[i][..])
            .collect();
        assert!(
            mismatches.is_empty(),
            "World {name} differs from the running game at {mismatches:?}"
        );
        checked.push((name, reached));
    }

    let best = |name: &str| {
        checked
            .iter()
            .filter(|(n, _)| *n == name)
            .map(|(_, r)| *r)
            .max()
            .unwrap_or(0)
    };
    assert_eq!(best("1-1"), 300, "the whole of World 1-1 should be covered");
    assert_eq!(best("1-2"), 280, "the whole of World 1-2 should be covered");
    assert_eq!(best("1-3"), 300, "the whole of World 1-3 should be covered");
    assert_eq!(best("2-1"), 320, "the whole of World 2-1 should be covered");
    assert_eq!(best("2-2"), 280, "the whole of World 2-2 should be covered");
}

/// Each of World 2's lists starts on the screen the game opens that level
/// with, read off the running cartridge by playing through all of World 1.
#[test]
fn world_2_starts_on_the_screens_the_game_opens_it_with() {
    let Some(data) = rom() else { return };
    let opening = [
        (level::LEVEL_2_1_LIST, 0x56CD, 16),
        (level::LEVEL_2_2_LIST, 0x5BA3, 14),
        (level::LEVEL_2_3_LIST, 0x6100, 18),
    ];
    for (list, first, screens) in opening {
        let pointers = level::screen_list(&data, list);
        assert_eq!(pointers.first(), Some(&first), "list at {list:#07X}");
        assert_eq!(pointers.len(), screens, "list at {list:#07X}");
    }
}

/// World 1-3's list start, reached by playing through 1-1 and 1-2. It opens on
/// 0x6E2F, which only the third list points at.
#[test]
fn world_1_3_starts_on_the_screen_the_game_opens_it_with() {
    let Some(data) = rom() else { return };
    let pointers = level::screen_list(&data, level::LEVEL_1_3_LIST);
    assert_eq!(pointers.first(), Some(&0x6E2F));
    assert_eq!(pointers.len(), 15, "15 screens, 300 columns");

    // Unique to this list, so the match that pinned it is not ambiguous.
    let elsewhere = level::WORLD_1
        .iter()
        .filter(|(name, _)| *name != "1-3")
        .any(|&(_, list)| level::screen_list(&data, list).contains(&0x6E2F));
    assert!(!elsewhere, "0x6E2F should appear in no other level's list");
}

/// All three of World 1's levels decode to a whole number of screens and end
/// on the same exit screen.
#[test]
fn world_1_decodes_end_to_end() {
    let Some(data) = rom() else { return };
    let sizes: Vec<usize> = level::WORLD_1
        .iter()
        .map(|&(_, list)| level::decode_level(&data, list).len())
        .collect();
    assert_eq!(sizes, vec![300, 280, 300]);
    // 1-1 and 1-2 share an exit screen; 1-3 ends the world and has its own.
    let last = |list| *level::screen_list(&data, list).last().unwrap();
    assert_eq!(last(level::LEVEL_1_1_LIST), 0x67BB);
    assert_eq!(last(level::LEVEL_1_2_LIST), 0x67BB);
    assert_eq!(last(level::LEVEL_1_3_LIST), 0x75C6);
}

/// The cartridge's exit door, rather than a guess at where the exit should be.
/// It is a 2x2 block whose top-left tile is 0x13, and it appears exactly twice
/// in 1-1 and twice in 1-2, in the same column each time: a raised one leading
/// to the bonus route and one at ground level.
#[test]
fn the_exit_door_is_where_the_cartridge_puts_it() {
    let Some(data) = rom() else { return };
    for (name, list, column) in [("1-1", level::LEVEL_1_1_LIST, 298), ("1-2", level::LEVEL_1_2_LIST, 278)] {
        let columns = level::decode_level(&data, list);
        let doors: Vec<(usize, usize)> = columns
            .iter()
            .enumerate()
            .flat_map(|(c, col)| {
                col.iter()
                    .enumerate()
                    .filter(|(_, &t)| t == level::EXIT_DOOR)
                    .map(move |(r, _)| (c, r))
            })
            .collect();
        assert_eq!(doors, vec![(column, 0), (column, 13)], "World {name}");
        assert_eq!(level::exit_door(&columns), Some((column, 13)), "World {name}");

        // The full 2x2 block, so the tile is a door and not a lone id.
        assert_eq!(columns[column][13], 0x13);
        assert_eq!(columns[column + 1][13], 0x21);
        assert_eq!(columns[column][14], 0x24);
        assert_eq!(columns[column + 1][14], 0x39);
    }
}

/// World 1-3 ends the world rather than leading to another level, and has no
/// exit door at all. Its trigger is placed at the far end as a stand-in.
#[test]
fn world_1_3_has_no_exit_door() {
    let Some(data) = rom() else { return };
    let columns = level::decode_level(&data, level::LEVEL_1_3_LIST);
    assert_eq!(level::exit_door(&columns), None);
}

/// A level screen renders from the cartridge's own tile graphics. Scored
/// against the emulator's frame by `tools/compare_level_render.py` at 99.60%,
/// where the only differing pixels are Mario's sprite, which the background
/// renderer does not draw.
#[test]
fn a_level_screen_extracts_with_the_cartridges_tiles() {
    if rom().is_none() {
        return;
    }
    let (sheet, cells) =
        level::extract_screen(ROM, level::LEVEL_1_1_LIST, 0).expect("screen extracts");
    assert_eq!(cells.len(), level::SCREEN_COLUMNS * level::SCREEN_ROWS);

    // The status bar rows are blank and the playfield is not.
    let status = level::SCREEN_COLUMNS * level::STATUS_ROWS;
    assert!(cells[..status].iter().all(|&c| c == 0));
    assert!(cells[status..].iter().any(|&c| c != 0));

    // Enough distinct tiles to be a real screen, few enough to be one screen.
    assert!(
        (10..60).contains(&sheet.tiles.len()),
        "got {} unique tiles",
        sheet.tiles.len()
    );
    for &cell in &cells {
        assert!((cell as usize) < sheet.tiles.len());
    }
}

/// Gameplay copies one contiguous run of tile data, ROM 0x08032 to VRAM
/// 0x8000. The title screen draws from the same bank-2 atlas but copies three
/// slices of it elsewhere, and using its layout for a level draws font glyphs.
#[test]
fn the_gameplay_tile_block_is_one_contiguous_copy() {
    let Some(data) = rom() else { return };
    let tiles = level::gameplay_tiles(&data).expect("tile data is in range");
    assert_eq!(tiles.len(), level::TILES_SIZE);
    assert_eq!(level::TILES_ROM_OFFSET, 0x08032);
    assert!(
        tiles.iter().filter(|&&b| b != 0).count() > level::TILES_SIZE / 4,
        "the block should be mostly real tile data"
    );
}

/// Every list opens with pointers that are not part of the level, which was an
/// open question for as long as only World 1 was decoded. The last two of them
/// are the level's bonus rooms: closed boxes with a solid floor across their
/// whole width, filled with coins, where a level's own screens have gaps.
///
/// World 2-3 stores the same pointer twice, so a level with one bonus room
/// repeats it rather than leaving a gap.
#[test]
fn the_pointers_before_a_level_are_its_bonus_rooms() {
    let Some(data) = rom() else { return };
    for (name, list) in level::MEASURED_LEVELS {
        let rooms = level::bonus_rooms(&data, list).expect("a list to have a run before it");
        let bank = level::bank_of(list);
        for pointer in rooms {
            let columns = level::screen(&data, pointer, bank)
                .unwrap_or_else(|| panic!("World {name}'s 0x{pointer:04X} should decode"));
            assert!(
                level::is_bonus_room(&columns),
                "World {name}'s 0x{pointer:04X} should be a coin room"
            );
        }
    }
    let rooms = level::bonus_rooms(&data, level::LEVEL_2_3_LIST).unwrap();
    assert_eq!(rooms[0], rooms[1], "World 2-3 has one bonus room, stored twice");
}

/// A level's own opening screen is not a coin room, which is what makes the test
/// above a real distinction rather than something every screen passes.
#[test]
fn a_levels_own_opening_screen_is_not_a_sealed_room() {
    let Some(data) = rom() else { return };
    for (name, list) in level::MEASURED_LEVELS {
        let pointer = level::screen_list(&data, list)[0];
        let columns = level::screen(&data, pointer, level::bank_of(list)).unwrap();
        assert!(
            !level::is_bonus_room(&columns),
            "World {name} opens on a coin room, so the test proves nothing"
        );
    }
}

/// A level's tile ids index whatever atlas is in video RAM while it plays, and
/// that is not one atlas for the whole cartridge. World 2's geometry decodes
/// exactly and draws garbage through World 1's tiles.
///
/// World 1's levels use the shared atlas unchanged; World 2's overlay four
/// spans on top of it, three from bank 1 plus 0x32, which is the same offset
/// into its own bank that World 1's tiles are into bank 2.
#[test]
fn world_2_loads_its_own_tiles_over_the_shared_atlas() {
    let Some(data) = rom() else { return };
    let shared = level::gameplay_tiles(&data).unwrap();

    for (name, list) in level::WORLD_1 {
        let tiles = level::tiles_for_level(&data, list).unwrap();
        assert_eq!(tiles, shared, "World {name} uses the shared atlas unchanged");
    }
    for (name, list) in level::WORLD_2 {
        let tiles = level::tiles_for_level(&data, list).unwrap();
        assert_ne!(tiles, shared, "World {name} has to differ from World 1's");
        for (from, to, size) in level::WORLD_2_TILE_BLOCKS {
            let at = to - 0x8000;
            assert_eq!(
                &tiles[at..at + size],
                &data[from..from + size],
                "World {name}'s block at {to:#06X} should come from {from:#07X}"
            );
        }
    }
}

/// The overlay only matters if World 2 actually draws from the spans it
/// replaces. Without this, the test above would pass on an overlay that lands
/// entirely on tiles no level uses.
#[test]
fn world_2_1s_opening_draws_from_the_overlaid_tiles() {
    let Some(data) = rom() else { return };
    let columns = level::decode_level(&data, level::LEVEL_2_1_LIST);
    // Background tiles use the signed addressing mode, so an id below 128
    // reads from 0x9000 and the rest from 0x8800. Using 0x8000 + id * 16 here
    // looks right and points at the wrong half of video RAM.
    let replaced = |id: u8| {
        let addr = if id < 128 {
            0x9000 + (id as usize) * 16
        } else {
            0x8800 + (id as usize - 128) * 16
        };
        level::WORLD_2_TILE_BLOCKS
            .iter()
            .any(|&(_, to, size)| (to..to + size).contains(&addr))
    };
    let used = columns
        .iter()
        .take(level::SCREEN_COLUMNS)
        .flatten()
        .filter(|&&t| replaced(t))
        .count();
    assert!(used > 0, "World 2-1's opening screen has to use an overlaid tile");
}

/// The control for the header tables. Six of the twelve screen lists were
/// pinned by playing to them, one at a time, over several sessions. The
/// twelve-entry table at the start of each bank has to reproduce all six at
/// the index `world * 3 + level` gives, or it is not what it looks like.
#[test]
fn the_bank_headers_reproduce_every_measured_screen_list() {
    let Some(rom) = rom() else { return };

    for (name, measured) in level::MEASURED_LEVELS {
        assert_eq!(
            level::level_list(&rom, name),
            Some(measured),
            "header disagrees with the measured list for {name}"
        );
    }
}

/// Every level the headers name decodes in the bank the world lives in: a
/// whole number of screens, none of them empty. A bank mix-up does not
/// survive this, since a screen pointer read against the wrong bank lands in
/// unrelated bytes.
#[test]
fn every_level_the_headers_name_decodes_in_its_own_bank() {
    let Some(rom) = rom() else { return };

    for name in level::LEVEL_NAMES {
        let list = level::level_list(&rom, name).expect("header entry");
        let columns = level::decode_level(&rom, list);
        assert!(columns.len() >= 240, "{name} decoded {} columns", columns.len());
        assert_eq!(columns.len() % SCREEN_COLUMNS, 0, "{name} is not whole screens");
    }
}

/// Each header entry points three pointers before the level's own first
/// screen, so the prefix is uniform across all twelve. World 1-1 looked like
/// the exception (its list appeared to start four pointers early) because the
/// scan finds lists by structure and one extra pointer before 1-1 happens to
/// decode.
#[test]
fn every_level_carries_the_same_three_pointer_prefix() {
    let Some(rom) = rom() else { return };

    for name in level::LEVEL_NAMES {
        let head = level::level_list_head(&rom, name).expect("header entry");
        let list = level::level_list(&rom, name).expect("header entry");
        assert_eq!(list - head, level::LIST_PREFIX, "{name}");
        assert!(level::bonus_rooms(&rom, list).is_some(), "{name} has no bonus rooms");
    }
}
