//! Checks on World 1-1 as extracted from the cartridge.
//!
//! The level file is generated on demand by `sml extract-level` and is
//! gitignored, so these tests skip when it is absent (no ROM, fresh checkout,
//! CI). `tests/rom_level_decode.rs` checks the decode itself; these check that
//! the level it produces is playable, which is the part a wrong solidity rule
//! breaks without breaking anything else.

use sml::core::enemy::EnemyKind;
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

#[test]
fn the_extracted_level_carries_the_cartridge_s_walkers() {
    let Some(level) = extracted() else { return };
    // The ten kind 0x00 records in World 1-1's object list, its one kind 0x04,
    // and its three kind 0x0E. Only those three kinds are written out, being
    // the only ones whose movement has been measured.
    let mut placed: Vec<(i32, i32)> =
        level.enemy_spawns.iter().map(|&(x, y, _)| (x, y)).collect();
    placed.sort_unstable();
    let turners = level
        .enemy_spawns
        .iter()
        .filter(|&&(_, _, kind)| kind == EnemyKind::LedgeTurner)
        .count();
    assert_eq!(turners, 1, "World 1-1 has exactly one of the ledge-turning kind");
    let hoppers = level
        .enemy_spawns
        .iter()
        .filter(|&&(_, _, kind)| kind == EnemyKind::Hopper)
        .count();
    assert_eq!(hoppers, 3, "and three jumpers");
    // Every one of them, at 16 * x across and 8 * row down. The one at pixel
    // 944 is the kind 0x04 record, `3B 0F 04`; 1320, 1872 and 1952 are the
    // kind 0x0E ones.
    assert_eq!(
        placed,
        vec![
            (192, 104),
            (528, 80),
            (640, 16),
            (656, 16),
            (944, 104),
            (1264, 104),
            (1320, 104),
            (1360, 104),
            (1488, 80),
            (1568, 80),
            (1600, 104),
            (1680, 104),
            (1872, 104),
            (1952, 104),
        ]
    );
    for &(x, y, _) in &level.enemy_spawns {
        let (column, row) = (x / 8, y / 8);
        assert!(
            !level.solids.is_solid(column, row),
            "an enemy was placed inside a solid tile at {column},{row}"
        );
    }
}

#[test]
fn the_extracted_level_carries_the_cartridge_s_lifts() {
    let Some(level) = extracted() else { return };
    use sml::core::lift::LiftAxis;
    // World 1-1's last two records, kinds 0x0A and 0x0B, either side of the
    // exit door. Both were confirmed to hold Mario up on the cartridge.
    assert_eq!(level.lifts.len(), 2);
    let mut placed = level.lifts.clone();
    placed.sort_by_key(|&(x, _, _)| x);
    // Their records are 8E 87 0A and 92 84 0B. Both carry a y high nibble of
    // 8, so both sit half a step right of their x byte, at pixels 2280 and
    // 2344. The text format is a tile grid, so each lands on the column that
    // contains it and the sub-tile offset is lost.
    assert_eq!(placed[0], (285 * 8, 5 * 8, LiftAxis::Vertical));
    assert_eq!(placed[1], (293 * 8, 2 * 8, LiftAxis::Horizontal));
}

/// A lift Mario can ride has to survive the loader and the game's own set-up,
/// not just the extractor.
#[test]
fn world_1_2_keeps_its_lifts_through_the_loader() {
    let Some(levels) = sml::session::world_1_levels() else { return };
    assert_eq!(levels[1].lifts.len(), 7);
    let game = sml::game::Game::new(levels[1].clone());
    assert_eq!(game.lifts.len(), 7);
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

/// The three extracted levels load and each has what a level needs.
#[test]
fn world_1_loads_when_it_has_been_extracted() {
    let Some(levels) = sml::session::world_1_levels() else { return };
    assert_eq!(levels.len(), 3);
    let widths: Vec<usize> = levels.iter().map(|l| l.solids.width).collect();
    assert_eq!(widths, vec![300, 280, 300]);
    for (i, level) in levels.iter().enumerate() {
        assert!(level.end.is_some(), "level {i} needs its end trigger");
        let column = level.spawn.0 / 8;
        assert!(
            (0..ROWS).any(|row| level.solids.is_solid(column, row)),
            "level {i} has nothing under Mario's spawn"
        );
    }
}

/// World 1-2 is the level the one-way platforms came from, so its geometry has
/// to carry them through the loader and not just the extractor.
#[test]
fn world_1_2_keeps_its_platforms_through_the_loader() {
    let Some(levels) = sml::session::world_1_levels() else { return };
    let solids = &levels[1].solids;
    let platforms = (0..solids.width as i32)
        .flat_map(|c| (0..ROWS).map(move |r| (c, r)))
        .filter(|&(c, r)| solids.is_platform(c, r))
        .count();
    assert_eq!(platforms, 183);
}

/// A smoke test for the two levels the walker cannot finish. World 1-2 and
/// 1-3 are built over open sky and need real platforming, and our end trigger
/// is placed by a rule (two columns from the right) rather than at the
/// cartridge's own exit, so the walker runs off the end of the geometry. What
/// this does check is that each spawn is somewhere Mario can stand and move
/// from, which is the part a wrong spawn or a mis-decoded opening breaks.
#[test]
fn mario_can_stand_and_move_at_every_world_1_spawn() {
    let Some(levels) = sml::session::world_1_levels() else { return };
    for (i, level) in levels.iter().enumerate() {
        let mut game = Game::new(level.clone());
        let start = game.mario.pixel_x();
        let mut buttons = Buttons::default();
        buttons.set(Button::Right, true);
        for _ in 0..120 {
            game.step(buttons);
        }
        assert!(
            game.mario.pixel_x() > start,
            "level {} left Mario stuck at the spawn",
            i + 1
        );
        // No check that he is still alive: World 1-2's opening ledge ends at
        // column 8, so holding right walks straight off it. That is the
        // level, not a fault in the extraction.
    }
}


/// The cartridge's own numbers, checked against the cartridge's own level.
///
/// The corridor measurement was taken on World 1-1's opening screen, so the
/// extracted level is where it has to hold: an 11 px Mario standing on the
/// ground has to fit between the scenery there, and a 12 px Mario has to fit
/// under everything the level expects him to walk under.
#[test]
fn mario_fits_through_the_real_level_at_his_measured_size() {
    let Some(level) = extracted() else { return };
    let mut game = Game::new(level);
    assert_eq!(game.mario.size(), (11, 12));

    let mut right = Buttons::default();
    right.set(Button::Right, true);
    // A couple of frames to settle onto the ground from the spawn position.
    for _ in 0..4 {
        game.step(Buttons::default());
    }
    let start = game.mario.pixel_x();
    let ground = game.mario.pixel_y();
    for _ in 0..100 {
        game.step(right);
        assert_eq!(game.mario.pixel_y(), ground, "he should stay on the ground");
    }
    // Past the two columns of scenery the level opens with, and clear of the
    // first walker, which is what stops a plain hold-right run.
    assert!(
        game.mario.pixel_x() > start + 80,
        "wedged at column {}",
        game.mario.pixel_x() / 8
    );
}
