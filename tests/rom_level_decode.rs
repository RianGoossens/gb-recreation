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

/// The control for the copy tables. The measured World 2 overlay came from
/// searching video RAM at 2-1's opening; the loader in bank 0 gives the same
/// blocks from the ROM alone, for every world. Each measured span has to sit
/// inside a derived copy at the same ROM-to-VRAM delta.
///
/// The measured `0x09732` span is skipped: it is the shared atlas at that
/// address, so it is not part of the overlay at all.
#[test]
fn the_loaders_own_tables_reproduce_the_measured_world_2_overlay() {
    let Some(data) = rom() else { return };
    let derived = level::tile_overlay(&data, 2).expect("World 2 loads an overlay");

    for (from, to, size) in level::WORLD_2_TILE_BLOCKS {
        if from == level::TILES_ROM_OFFSET + (to - 0x8000) {
            continue;
        }
        let holder = derived
            .iter()
            .find(|&&(_, dest, len)| dest <= to && to + size <= dest + len)
            .unwrap_or_else(|| panic!("no derived copy covers {to:#06X}"));
        let (src, dest, _) = *holder;
        assert_eq!(from, src + (to - dest), "the copy at {to:#06X} reads elsewhere");
    }
    assert!(level::tile_overlay(&data, 1).is_none(), "World 1 loads no overlay");
}

/// Worlds 3 and 4 read their tiles from the same tables, and each level's
/// opening screen has to draw from the spans its world replaces. Without this
/// the overlay could land entirely on tiles nothing uses and still look fine.
#[test]
fn every_world_after_the_first_draws_from_its_own_overlay() {
    let Some(data) = rom() else { return };
    let shared = level::gameplay_tiles(&data).unwrap();

    for name in level::LEVEL_NAMES {
        let world = name[..1].parse::<usize>().unwrap();
        let list = level::level_list(&data, name).expect("header entry");
        let tiles = level::tiles_for_level(&data, list).unwrap();
        if world == 1 {
            assert_eq!(tiles, shared, "World {name} uses the shared atlas unchanged");
            continue;
        }
        assert_ne!(tiles, shared, "World {name} has to differ from World 1's");

        let blocks = level::tile_overlay(&data, world).unwrap();
        let overlaid = |id: u8| {
            // Background tiles use signed addressing: below 128 reads from
            // 0x9000, the rest from 0x8800.
            let addr = if id < 128 {
                0x9000 + (id as usize) * 16
            } else {
                0x8800 + (id as usize - 128) * 16
            };
            blocks.iter().any(|&(_, to, size)| (to..to + size).contains(&addr))
        };
        let used = level::decode_level(&data, list)
            .iter()
            .take(SCREEN_COLUMNS)
            .flatten()
            .filter(|&&t| overlaid(t))
            .count();
        assert!(used > 0, "World {name}'s opening screen draws no overlaid tile");
    }
}

/// Which levels carry the cartridge's exit door. Worlds 1 and 2 showed the
/// pattern (the first two levels of a world have one, the third does not,
/// because it ends the world rather than leading anywhere) and the four levels
/// named from the bank header follow it, all eight of them.
#[test]
fn only_a_worlds_last_level_lacks_an_exit_door() {
    let Some(data) = rom() else { return };

    for name in level::LEVEL_NAMES {
        let list = level::level_list(&data, name).expect("header entry");
        let columns = level::decode_level(&data, list);
        let door = level::exit_door(&columns);
        assert_eq!(
            door.is_some(),
            !name.ends_with("-3"),
            "World {name} {} an exit door",
            if door.is_some() { "has" } else { "has no" }
        );
    }
}

/// The title screen's background is a screen record like any level's, at index
/// 12 of bank 2's table, which is the "level index 0x0C" the extraction code
/// already knew from the disassembly.
#[test]
fn the_title_screens_background_comes_out_of_the_rom() {
    let Some(data) = rom() else { return };
    let columns = level::title_screen(&data).expect("bank 2 names the title screen");
    assert_eq!(columns.len(), SCREEN_COLUMNS);
    assert!(columns.iter().all(|c| c.len() == ROWS));

    // The screen has to be the title screen rather than some other record: the
    // word START sits on screen row 13, so those cells cannot all be empty.
    let row = 13 - 2;
    assert!(
        columns.iter().filter(|c| c[row] != level::FILLER).count() >= 5,
        "the decoded screen has nothing drawn where START is"
    );
}

/// Every world animates one background tile. Both tile loaders end by saving
/// its high bitplane to `0xC600`, and the routine at `0x02416` alternates
/// between that and a per-world frame in bank 0.
#[test]
fn every_world_animates_one_tile_between_two_frames() {
    let Some(data) = rom() else { return };
    let mut seen = Vec::new();
    for world in 1..=4 {
        let (loaded, alternate) =
            level::animation_frames(&data, world).expect("both frames of the animated tile");
        assert_ne!(loaded, alternate, "world {world}'s two frames are the same picture");
        assert!(loaded.iter().any(|&b| b != 0), "world {world} loads a blank tile");
        assert!(alternate.iter().any(|&b| b != 0), "world {world}'s second frame is blank");
        assert!(!seen.contains(&alternate), "world {world} repeats another world's frame");
        seen.push(alternate);
    }
}

/// The relation that pins the table's base and index. World 2's second frame
/// is its first shifted down a row with every row rotated right two pixels,
/// which is a water surface flowing. A wrong base or a wrong index would put
/// some other world's bytes here and break it.
#[test]
fn world_2s_second_frame_is_its_first_one_shifted() {
    let Some(data) = rom() else { return };
    let (loaded, alternate) = level::animation_frames(&data, 2).expect("world 2's frames");
    assert_eq!(alternate[0], 0x00);
    for row in 1..8 {
        assert_eq!(
            alternate[row],
            loaded[row - 1].rotate_right(2),
            "row {row} does not follow the shift"
        );
    }
}

/// What the animated tile draws in World 2: the water line. It runs along the
/// bottom row of 2-1 and 2-2 from the first column to the last screen, which
/// is dry ground with the exit door on it, and along the top row of 2-3 for
/// the whole level, which plays underwater from end to end.
#[test]
fn the_animated_tile_is_world_2s_water_line() {
    let Some(data) = rom() else { return };
    for (name, row) in [("2-1", ROWS - 1), ("2-2", ROWS - 1), ("2-3", 0)] {
        let list = level::level_list(&data, name).expect("header entry");
        let columns = level::decode_level(&data, list);
        let wet: Vec<bool> =
            columns.iter().map(|c| c[row] == level::ANIMATED_TILE).collect();
        let dry = if name == "2-3" { 0 } else { SCREEN_COLUMNS };
        assert!(wet[..wet.len() - dry].iter().all(|&w| w), "World {name} row {row} has a gap");
        assert!(wet[wet.len() - dry..].iter().all(|&w| !w), "World {name} ends wet");
    }
}

/// Every bonus room draws with its own world's tiles. The rooms are screen
/// records like any other, so a coin in one is the same tile a coin in the
/// level is, and every cell has to name a tile the world actually loaded.
#[test]
fn every_bonus_room_draws_with_its_worlds_tiles() {
    let Some(data) = rom() else { return };
    for (index, &name) in level::LEVEL_NAMES.iter().enumerate() {
        let world = index / 3 + 1;
        let sheet = level::world_tile_sheet(&data, world).expect("the world's tiles");
        assert_eq!(sheet.len(), 256);
        let list = level::level_list(&data, name).expect("header entry");
        let rooms = level::bonus_rooms(&data, list).expect("a run before the list");
        let bank = level::bank_of(list);
        let mut coins = 0;
        for pointer in rooms.iter().skip(1) {
            let columns = level::screen(&data, *pointer, bank).expect("a room decodes");
            assert_eq!(columns.len(), SCREEN_COLUMNS);
            coins += columns.iter().flatten().filter(|&&t| level::is_coin(t)).count();
        }
        assert!(coins > 0, "World {name}'s bonus rooms hold no coins");
    }
}

/// What the first of a level's three prefix pointers is. It had been explained
/// for worlds 1 and 2, where it is the world's own first screen, and left open
/// for the rest. Worlds 3 and 4 answer it differently: there it repeats one of
/// the level's two coin rooms. Either way it is never a third distinct room,
/// which is what `prefix_rooms` relies on.
#[test]
fn the_first_prefix_pointer_is_never_a_third_room() {
    let Some(data) = rom() else { return };

    for name in level::LEVEL_NAMES {
        let head = level::level_list_head(&data, name).expect("header entry");
        let at = |i: usize| u16::from_le_bytes([data[head + i * 2], data[head + i * 2 + 1]]);
        let world = &name[..1];
        let opening = level::level_list(&data, &format!("{world}-1")).expect("first level");
        let opening = u16::from_le_bytes([data[opening], data[opening + 1]]);

        let first = at(0);
        let repeats_a_room = first == at(1) || first == at(2);
        assert!(
            first == opening || repeats_a_room,
            "World {name}'s first prefix pointer 0x{first:04X} is neither the world's \
             opening screen nor one of its rooms"
        );
        assert!(level::prefix_rooms(&data, head + level::LIST_PREFIX).len() <= 2);
    }
}

/// How many distinct coin rooms each level has. World 2-3 stores the same
/// pointer twice and has one; every other level has two. Worlds 3 and 4 share
/// one prefix across all three of a world's levels, so their levels have the
/// same rooms as each other.
#[test]
fn every_level_has_one_or_two_coin_rooms() {
    let Some(data) = rom() else { return };

    let counts: Vec<usize> = level::LEVEL_NAMES
        .iter()
        .map(|name| {
            let list = level::level_list(&data, name).expect("header entry");
            level::prefix_rooms(&data, list).len()
        })
        .collect();
    assert_eq!(counts, vec![2, 2, 2, 2, 2, 1, 2, 2, 2, 2, 2, 2]);
}

/// Each bank's screen table has a thirteenth entry, one past the twelve
/// levels. Bank 2's is the title screen, which is where the title screen's
/// tilemap comes from. The other two had never been explained: bank 1's is
/// its own first level's entry over again, the same filler its unused level
/// slots use, and bank 3's is a list of four pointers to one screen, a brick
/// corridor open across the middle that no level list mentions anywhere.
#[test]
fn the_thirteenth_entry_of_each_bank() {
    let Some(data) = rom() else { return };

    let entry = |bank: usize| {
        let at = bank + level::TITLE_SCREEN_INDEX * 2;
        u16::from_le_bytes([data[at], data[at + 1]])
    };
    // Bank 1 holds worlds 2 and 4; its first level is 2-1.
    let head = level::level_list_head(&data, "2-1").expect("header entry");
    assert_eq!(level::bank_of(head) + entry(0x04000) as usize % 0x4000, head);

    // Bank 3's corridor: four pointers to one screen, and no level uses it.
    let corridor = entry(0x0C000) as usize % 0x4000 + 0x0C000;
    let pointers: Vec<u16> = (0..4)
        .map(|i| u16::from_le_bytes([data[corridor + i * 2], data[corridor + i * 2 + 1]]))
        .collect();
    assert_eq!(pointers.iter().collect::<std::collections::BTreeSet<_>>().len(), 1);
    let screen = level::screen(&data, pointers[0], 0x0C000).expect("it decodes");
    assert!(!level::is_bonus_room(&screen), "it has no coins, so it is not a room");
    // Solid across the top two rows and the bottom three, open between them.
    assert!(screen.iter().all(|c| level::is_solid(c[0]) && level::is_solid(c[1])));
    assert!(screen.iter().all(|c| (2..13).all(|r| !level::is_solid(c[r]))));
}
