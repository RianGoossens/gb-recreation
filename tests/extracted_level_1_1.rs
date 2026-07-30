//! Checks on World 1-1 as extracted from the cartridge.
//!
//! The level file is generated on demand by
//! `tools/convert_level_1_1_to_level_format.py` and is gitignored, so these
//! tests skip when it is absent (no ROM, fresh checkout, CI). When it is
//! there they are the regression test for the extraction: the geometry is
//! decoded from the ROM and the solidity rules are partly inference, so a
//! change to either has to keep the level finishable.

use sml::core::level::Level;
use sml::game::Game;
use sml::input::{Button, Buttons};

const PATH: &str = "assets/extracted/level_1_1.txt";
const COLUMNS: usize = 300;
const ROWS: i32 = 16;

fn extracted() -> Option<Level> {
    if std::path::Path::new(PATH).exists() {
        Some(Level::from_file(PATH).expect("extracted level parses"))
    } else {
        None
    }
}

#[test]
fn extracted_level_has_the_shape_the_rom_decodes_to() {
    let Some(level) = extracted() else { return };
    assert_eq!(level.solids.width, COLUMNS);
    assert_eq!(level.solids.height, ROWS as usize);
    assert!(level.end.is_some(), "the level needs its end trigger");
}

/// Walks the level: hold right, jump when blocked, and jump when the ground
/// ahead runs out. Mario has to reach the end trigger, which fails if the
/// solidity rule seals a wall shut or opens a pit that is not there.
///
/// The gap lookahead is not decoration. World 1-1 has nine columns you can
/// fall through, and a walker that only jumps when it is already stuck walks
/// straight into the first one.
#[test]
fn extracted_level_can_be_walked_from_spawn_to_the_end() {
    let Some(level) = extracted() else { return };
    let mut game = Game::new(level);
    let ground_ahead = |game: &Game, x: i32, y: i32| {
        let column = (x + 12) / 8;
        (y / 8..ROWS).any(|row| game.level.solids.is_solid(column, row))
    };

    let mut stalled = 0;
    let mut furthest = i32::MIN;
    let mut hold_jump = 0;

    for _ in 0..20_000 {
        let x = game.mario.pixel_x();
        if x <= furthest {
            stalled += 1;
        } else {
            stalled = 0;
            furthest = x;
        }
        let gap = !ground_ahead(&game, x, game.mario.pixel_y());
        if (stalled > 6 || gap) && game.mario.on_ground {
            hold_jump = 12;
        }

        let mut buttons = Buttons::default();
        buttons.set(Button::Right, true);
        if hold_jump > 0 {
            buttons.set(Button::A, true);
            hold_jump -= 1;
        }
        game.step(buttons);

        if game.completed {
            return;
        }
    }
    panic!("never reached the end trigger; furthest column was {}", furthest / 8);
}

/// The raised platform at world columns 61-64 is built from tile 232, and an
/// earlier reading did not treat it as solid, which turned its footprint into
/// a pit that is not in the original.
#[test]
fn the_raised_platform_is_not_a_pit() {
    let Some(level) = extracted() else { return };
    for column in 61..=64 {
        assert!(
            (11..ROWS).any(|row| level.solids.is_solid(column, row)),
            "column {column} under the raised platform is empty all the way down"
        );
    }
}

/// World 1-1 has holes you can fall through, verified by Rian playing the
/// cartridge. An earlier reading invented a floor under every one of them and
/// produced a level with no pits at all, so this pins that they are back.
#[test]
fn the_level_has_pits_you_can_fall_through() {
    let Some(level) = extracted() else { return };
    let pits: Vec<i32> = (0..level.solids.width as i32)
        .filter(|&c| !(0..ROWS).any(|r| level.solids.is_solid(c, r)))
        .collect();
    assert_eq!(pits, vec![89, 90, 138, 139, 247, 248, 249, 261, 262]);
}
