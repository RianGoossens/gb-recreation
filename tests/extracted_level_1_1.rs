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
    let Some(mut level) = extracted() else { return };
    // Geometry only. Since the enemies stream in with the camera the way the
    // cartridge's read pointer does, the level actually contains them now, and
    // a walker that only knows how to jump gaps dies to the third one. What
    // this test is for is walls that seal the level and pits that are not
    // there, so the enemies come out.
    level.enemy_spawns.clear();
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
        let mut furthest = start;
        for _ in 0..120 {
            game.step(buttons);
            furthest = furthest.max(game.mario.pixel_x());
        }
        // The furthest he reached, not where he ends up: dying puts him back
        // at the spawn, and 1-2 kills him on the way down off its opening
        // ledge whatever the extraction did.
        assert!(
            furthest > start,
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


/// The end trigger has to be somewhere Mario could get to.
///
/// World 1-3 has no exit door, so its trigger is placed by rule, and the rule
/// used to be "two columns from the right edge at the ground row". In 1-3 that
/// lands at column 298 row 13, a one-tile pocket with solid tiles above,
/// below and either side. The level could not be finished. A flood fill over
/// open cells from the spawn catches that, where checking the cell itself
/// does not.
#[test]
fn every_end_trigger_is_reachable_from_the_spawn() {
    // World 4-3 is left out: it is a vehicle stage with no ground under the
    // spawn at all, so walking it is not the question.
    for (i, path) in ["1_1", "1_2", "1_3", "2_1", "2_2", "2_3", "3_1", "3_2", "3_3", "4_1", "4_2"]
        .iter()
        .enumerate()
    {
        let file = format!("assets/extracted/level_{path}.txt");
        if !std::path::Path::new(&file).exists() {
            continue;
        }
        let level = Level::from_file(&file).expect("extracted level parses");
        let end = level.end.expect("every level needs an end trigger");
        let (w, h) = (level.solids.width as i32, level.solids.height as i32);

        let mut seen = vec![false; (w * h) as usize];
        let start = (level.spawn.0 / 8, level.spawn.1 / 8);
        let mut queue = vec![start];
        seen[(start.1 * w + start.0) as usize] = true;
        while let Some((x, y)) = queue.pop() {
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= w || ny >= h {
                    continue;
                }
                let at = (ny * w + nx) as usize;
                if seen[at] || level.solids.is_solid(nx, ny) {
                    continue;
                }
                seen[at] = true;
                queue.push((nx, ny));
            }
        }

        let cell = (end.0 / 8, end.1 / 8);
        assert!(
            seen[(cell.1 * w + cell.0) as usize],
            "World {}-{} walls its end trigger in at column {}, row {}",
            i / 3 + 1,
            i % 3 + 1,
            cell.0,
            cell.1
        );
    }
}


/// World 1-3 used to load with no enemies at all, because both kinds it
/// introduces were unmeasured.
///
/// Eight of its records are fallers and six of them reach the level file. The
/// other two start inside solid tiles, which the cartridge does and the text
/// format cannot represent, so the stamper leaves those cells alone rather
/// than replacing terrain with an enemy.
#[test]
fn world_1_3_carries_its_fallers() {
    let path = "assets/extracted/level_1_3.txt";
    if !std::path::Path::new(path).exists() {
        return;
    }
    let level = Level::from_file(path).expect("extracted level parses");
    let fallers = level
        .enemy_spawns
        .iter()
        .filter(|&&(_, _, kind)| kind == EnemyKind::Faller)
        .count();
    assert_eq!(fallers, 6);
    // Its one ground walker starts inside terrain too, so the fallers are all
    // the level ends up with.
    assert_eq!(level.enemy_spawns.len(), 6);

    // They arrive with the camera, so a fresh Game has none of them live yet:
    // the nearest is well past the first screen. All six are waiting.
    let game = Game::new(level);
    assert!(game.enemies.is_empty());
    assert_eq!(game.pending_enemy_count(), 6);
}

/// World 2-1 and 2-2 carry the cartridge's own exit door, the 2x2 block of
/// `0x13 0x21` over `0x24 0x39`, in the same place World 1's levels do: twice
/// in one column, a raised one for the bonus route and one at ground level.
/// So the door rule that replaced "two columns from the right edge" holds in a
/// world it was not derived from.
///
/// 2-3 has no door, like 1-3, because it ends the world rather than leading to
/// another level, and falls back to the far-end placement.
#[test]
fn world_2_ends_on_the_cartridges_own_door() {
    let door = |name: &str| -> Option<(usize, usize)> {
        let path = format!("assets/extracted/level_{name}.txt");
        let level = Level::from_file(&path).ok()?;
        level.end.map(|(x, y)| (x as usize / 8, y as usize / 8))
    };
    let Some(exit) = door("2_1") else { return };
    assert_eq!(exit, (318, 13), "World 2-1's door is at column 318");
    assert_eq!(door("2_2"), Some((278, 13)), "World 2-2's door is at column 278");
    assert_eq!(door("2_3"), Some((353, 14)), "World 2-3 has no door to find");
}

/// Every extracted level has to put Mario somewhere he can stand. A spawn in
/// mid-air or inside terrain is what a mis-decoded opening screen looks like,
/// and the levels of worlds 2 to 4 are decoded from banks and from a header
/// the extractor had never read before.
///
/// World 4-3 is the one level with no ground under its spawn. It is a vehicle
/// stage: nothing in it is walked.
#[test]
fn mario_can_stand_at_every_spawn_outside_the_vehicle_stage() {
    for name in ["2_1", "2_2", "2_3", "3_1", "3_2", "3_3", "4_1", "4_2"] {
        let path = format!("assets/extracted/level_{name}.txt");
        if !std::path::Path::new(&path).exists() {
            continue;
        }
        let level = Level::from_file(&path).expect("extracted level parses");
        let column = level.spawn.0 / 8;
        assert!(
            (0..ROWS).any(|row| level.solids.is_solid(column, row)),
            "World {name} has nothing under Mario's spawn"
        );
        // Holding right is not enough on its own: World 3-2 spawns Mario a
        // step away from a two-tile block, which he has to jump. Our spawn
        // column is a stand-in rather than the cartridge's own, so this asks
        // whether he can leave the spot at all, not whether he can walk it.
        let mut game = Game::new(level.clone());
        let start = game.mario.pixel_x();
        let mut furthest = start;
        let mut stuck = 0;
        for _ in 0..240 {
            let mut buttons = Buttons::default();
            buttons.set(Button::Right, true);
            buttons.set(Button::A, stuck > 8);
            game.step(buttons);
            if game.mario.pixel_x() > furthest {
                furthest = game.mario.pixel_x();
                stuck = 0;
            } else {
                stuck += 1;
            }
        }
        assert!(
            furthest > start + 16,
            "World {name} never lets Mario get away from his spawn"
        );
    }
}

/// How far the geometry walker gets through each level of worlds 2 to 4.
///
/// Not a pass/fail on the levels. The walker only knows how to run right and
/// jump when it stalls, so these numbers measure it as much as they measure
/// the geometry, and none of the nine is a level finished. World 2's are
/// understood: 2-1 and 2-2 both stop at column 69 because both lists point at
/// screen `0x5D32` for world columns 60 to 79 and it has no floor, which is
/// the water the cartridge expects Mario to swim. Worlds 3 and 4 stop earlier
/// still and why is not looked into yet.
///
/// What this pins is the geometry. A decode that breaks these levels open or
/// seals them shut moves the number.
#[test]
fn the_geometry_of_worlds_2_to_4_is_walkable_as_far_as_it_is_recorded() {
    for (name, expected) in [
        ("2_1", 69), ("2_2", 69), ("2_3", 178),
        ("3_1", 53), ("3_2", 34), ("3_3", 12),
        ("4_1", 66), ("4_2", 106), ("4_3", 7),
    ] {
        let path = format!("assets/extracted/level_{name}.txt");
        if !std::path::Path::new(&path).exists() {
            continue;
        }
        let mut level = Level::from_file(&path).expect("extracted level parses");
        level.enemy_spawns.clear();
        let mut game = Game::new(level);
        let ground_ahead = |game: &Game, x: i32, y: i32| {
            let column = (x + 12) / 8;
            (y / 8..ROWS).any(|row| game.level.solids.is_solid(column, row))
        };

        let mut stalled = 0;
        let mut furthest = i32::MIN;
        let mut hold_jump = 0;
        let mut reached = 0;
        for _ in 0..20_000 {
            let x = game.mario.pixel_x();
            if x <= furthest {
                stalled += 1;
            } else {
                stalled = 0;
                furthest = x;
            }
            reached = reached.max(furthest / 8);
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
                reached = game.level.solids.width as i32;
                break;
            }
        }
        assert_eq!(reached, expected, "World {name}: the walker reached a different column");
    }
}


/// The loader takes any of the cartridge's four worlds, not just World 1.
/// Only World 1 is finishable, and only World 1 is what `default_levels`
/// returns; this is about the files loading and carrying their geometry.
#[test]
fn every_extracted_world_loads_as_three_levels() {
    for world in 1..=4 {
        let Some(levels) = sml::session::world_levels(world) else {
            continue;
        };
        assert_eq!(levels.len(), 3, "World {world} should load three levels");
        for (i, level) in levels.iter().enumerate() {
            assert!(
                level.solids.width >= 240,
                "World {world}-{} loaded {} columns",
                i + 1,
                level.solids.width
            );
        }
    }
    assert!(sml::session::world_levels(0).is_none());
    assert!(sml::session::world_levels(5).is_none());
}

/// A level out of the cartridge carries the cartridge's own background, so the
/// game draws World 1-1's pyramid and palms rather than blocks. A level file
/// written by hand carries none and keeps the placeholders.
#[test]
fn every_cartridge_level_carries_its_own_background() {
    use sml::assets::level as assets;
    if std::fs::read(assets::DEFAULT_ROM).is_err() {
        return;
    }
    for &name in assets::LEVEL_NAMES.iter() {
        let Ok(level) = assets::extracted_level(name) else { return };
        let graphics = level.graphics.expect("the ROM is here, so the background is too");
        let (w, h) = (level.solids.width, level.solids.height);
        assert_eq!(graphics.cells.len(), w * h, "World {name} background is the wrong size");
        assert_eq!(graphics.tiles.len(), 256);

        let distinct: std::collections::HashSet<u8> = graphics.cells.iter().copied().collect();
        assert!(distinct.len() > 10, "World {name} draws with only {} tiles", distinct.len());

        // Coins are drawn into the cartridge's background, and the game draws
        // them itself so they can disappear when taken. Leaving both would
        // paint a coin that cannot be picked up.
        for &(x, y) in &level.coins {
            let cell = (y as usize / 8) * w + x as usize / 8;
            assert_ne!(graphics.cells[cell], assets::COIN, "World {name} leaves a painted coin");
        }
    }
}

/// The background reaches the screen: the same level with and without it does
/// not render the same frame.
#[test]
fn the_cartridges_background_changes_what_is_drawn() {
    use sml::assets::level as assets;
    let Ok(level) = assets::extracted_level("1-1") else { return };
    if level.graphics.is_none() {
        return;
    }
    let mut plain = level.clone();
    plain.graphics = None;
    let drawn = Game::new(level).render().to_gray();
    let blocks = Game::new(plain).render().to_gray();
    assert_ne!(drawn, blocks);
}
