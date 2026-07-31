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
/// address, so the lists have to be found by their own structure. World 1 has
/// three levels and the scan finds exactly three lists, one of them containing
/// 1-1's verified start.
#[test]
fn the_scan_finds_world_ones_three_screen_lists() {
    let Some(data) = rom() else { return };
    let lists = level::find_screen_lists(&data);
    let starts: Vec<usize> = lists.iter().map(|(start, _)| *start).collect();
    assert_eq!(starts, vec![0x0A190, 0x0A1B7, 0x0A1DA]);

    let (start, pointers) = &lists[0];
    assert!(
        *start <= level::LEVEL_1_1_LIST
            && level::LEVEL_1_1_LIST < start + 2 * pointers.len(),
        "the first list has to contain World 1-1's verified start"
    );
}

/// The strongest check there is on the decode: drive the real cartridge
/// through the whole of World 1-1 and compare every column.
///
/// The capture file is written by `tools/run_through_levels.py` and is
/// gitignored, so this skips when it is absent. When it is there, the decode
/// has to reproduce it exactly. It did, 300 of 300, which replaced the earlier
/// ground truth of 88 columns from a walker that died a quarter of the way in.
#[test]
fn the_decode_matches_every_column_the_real_game_draws() {
    const CAPTURE: &str = "assets/extracted/captured_columns.txt";
    let Ok(text) = std::fs::read_to_string(CAPTURE) else { return };
    let Some(columns) = level_1_1() else { return };

    let captured: Vec<Vec<u8>> = text
        .lines()
        .take_while(|line| !line.trim().is_empty())
        .map(|line| line.split_whitespace().map(|t| t.parse().unwrap()).collect())
        .collect();
    assert_eq!(captured.len(), COLUMNS, "capture should cover the whole level");

    let mismatches: Vec<usize> = (0..COLUMNS)
        .filter(|&i| columns[i][..] != captured[i][..])
        .collect();
    assert!(
        mismatches.is_empty(),
        "columns decoded from the ROM differ from the running game at {mismatches:?}"
    );
}
