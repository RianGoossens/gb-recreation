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
/// How long a walker holds A. The rise runs for `MAX_RISE_FRAMES` and then,
/// while he is moving, glides upward for `GLIDE_FRAMES` more, so releasing at
/// the cap gives up the top third of the jump. World 1-1's pillar at columns
/// 78 and 79 is 32px above the ground and needs the glide.
const FULL_JUMP: u32 =
    (sml::core::physics::MAX_RISE_FRAMES + sml::core::physics::GLIDE_FRAMES) as u32;
/// The first row a surface can be in and still hold Mario up, given his top
/// edge. A surface in row `r` has its top at `r * 8`, and he can step onto it
/// when that is at or below his feet, which are at `y + 12` since small Mario
/// is 12 tall. Using his lowest *pixel* instead is a row out and counts the
/// ledge he is already standing on top of as somewhere ahead he can walk to.
fn feet_row(y: i32) -> i32 {
    (y + 12).div_euclid(8)
}

/// A walker's jump button, with the one rule the engine imposes on it: the
/// button is latched, so holding A does not start a second jump. A walker that
/// re-arms the moment its hold runs out presses A on every single frame, which
/// means it jumps once and then never again. That is what pinned Mario in
/// World 1-3's pocket at column 11 for nine thousand frames: he was pressing
/// jump the whole time and the latch was never released.
struct Jump {
    held: u32,
    was_held: bool,
}

impl Jump {
    fn new() -> Self {
        Jump { held: 0, was_held: false }
    }

    /// A new jump can only start on a frame after one where A was up.
    fn ready(&self) -> bool {
        self.held == 0 && !self.was_held
    }

    fn start(&mut self) {
        self.held = FULL_JUMP;
    }

    fn press(&mut self, buttons: &mut Buttons) {
        self.was_held = self.held > 0;
        if self.held > 0 {
            buttons.set(Button::A, true);
            self.held -= 1;
        }
    }
}

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
        .filter(|&&(_, _, kind)| kind == EnemyKind::Fly)
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
    use sml::core::lift::{LiftAxis, Motion};
    // World 1-1's last two records, kinds 0x0A and 0x0B, either side of the
    // exit door. Both were confirmed to hold Mario up on the cartridge.
    assert_eq!(level.lifts.len(), 2);
    let mut placed = level.lifts.clone();
    placed.sort_by_key(|&(x, _, _)| x);
    // Their records are 8E 87 0A and 92 84 0B. Both carry a y high nibble of
    // 8, so both sit half a step right of their x byte, at pixels 2280 and
    // 2344. The text format is a tile grid, so each lands on the column that
    // contains it and the sub-tile offset is lost.
    assert_eq!(placed[0], (285 * 8, 5 * 8, Motion::Cycle(LiftAxis::Vertical)));
    assert_eq!(placed[1], (293 * 8, 2 * 8, Motion::Cycle(LiftAxis::Horizontal)));
}

/// A lift Mario can ride has to survive the loader and the game's own set-up,
/// not just the extractor.
#[test]
fn world_1_2_keeps_its_lifts_through_the_loader() {
    use sml::core::lift::Motion;
    let Some(levels) = sml::session::world_1_levels() else { return };
    let cycling = |level: &Level| {
        level
            .lifts
            .iter()
            .filter(|&&(_, _, motion)| motion != Motion::Drop)
            .count()
    };
    assert_eq!(cycling(&levels[1]), 7);
    assert_eq!(levels[1].lifts.len(), 9, "and its two drop blocks");
    let game = sml::game::Game::new(levels[1].clone());
    assert_eq!(game.lifts.len(), 9);
    assert_eq!(
        game.lifts.iter().filter(|l| l.motion == Motion::Drop).count(),
        2
    );
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
        // From his feet, not his head. A platform level with his chest is 16
        // pixels above the surface he is on, so counting it walks him off the
        // step at column 29 of World 1-2 instead of jumping up it.
        (feet_row(y)..ROWS).any(|row| game.level.solids.is_standable(column, row))
    };

    let mut stalled = 0;
    let mut furthest = i32::MIN;
    let mut jump = Jump::new();

    for _ in 0..20_000 {
        let x = game.mario.pixel_x();
        if x <= furthest {
            stalled += 1;
        } else {
            stalled = 0;
            furthest = x;
        }
        let gap = !ground_ahead(&game, x, game.mario.pixel_y());
        if (stalled > 6 || gap) && game.mario.on_ground && jump.ready() {
            jump.start();
        }

        let mut buttons = Buttons::default();
        buttons.set(Button::Right, true);
        jump.press(&mut buttons);
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
/// understood in full now. 2-1 and 2-2 both stop at column 69 because both
/// lists point at screen `0x5D32` for world columns 60 to 79 and it has no
/// floor, and the crossing is a lift rather than the swimming that was
/// assumed: the ledge ends at column 59 with a single solid tile at row 9,
/// and the nearest lift's surface is 56 pixels further on and 16 pixels
/// above, on a 53 pixel cycle. That is a timed running jump onto a moving
/// platform, which this walker cannot do; it either runs off the edge or
/// waits at it forever, and both were tried. So 69 is a fact about the bot at
/// this gap, and what the cartridge does there is measured separately
/// (`tools/probe_water.py`, and `world_2_1s_water_gap_is_crossed_by_lift` in
/// tests/lift_ride.rs). Worlds 3 and 4 stop earlier still and why is not
/// looked into yet.
///
/// What this pins is the geometry. A decode that breaks these levels open or
/// seals them shut moves the number.
#[test]
fn the_geometry_of_worlds_2_to_4_is_walkable_as_far_as_it_is_recorded() {
    // Every one of these moved when the jump was measured properly. The
    // walker had been jumping 43px, from a `JUMP_CUT` that made releasing the
    // button at frame 12 the highest jump the engine had; the cartridge gives
    // 24px standing and 33 moving, and the engine now gives 26 and 35. A
    // shorter jump reaches less far, so these numbers went down: 2-1 from 69,
    // 2-2 from 69, 3-1 from 98, 4-1 from 66. 3-3 went the other way, 12 to 23,
    // because the glide that replaced the old cliff-edge cap climbs a step it
    // could not.
    for (name, expected) in [
        ("2_1", 19), ("2_2", 63), ("2_3", 178),
        ("3_1", 90), ("3_2", 34), ("3_3", 23),
        ("4_1", 65), ("4_2", 106), ("4_3", 7),
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
            (feet_row(y)..ROWS).any(|row| game.level.solids.is_standable(column, row))
        };

        let mut stalled = 0;
        let mut furthest = i32::MIN;
        let mut jump = Jump::new();
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
            if (stalled > 6 || gap) && game.mario.on_ground && jump.ready() {
                jump.start();
            }
            let mut buttons = Buttons::default();
            buttons.set(Button::Right, true);
            jump.press(&mut buttons);
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

/// The cartridge animates one background tile. In World 2-1 it is the water
/// line along the level's bottom row, which is the last eight pixel rows of
/// the screen: they change every eight frames and come back on the sixteenth.
/// The right half is read so no sprite is in the way.
#[test]
fn the_water_line_animates_on_the_cartridges_cadence() {
    use sml::assets::level as assets;
    let Ok(level) = assets::extracted_level("2-1") else { return };
    if level.graphics.as_ref().and_then(|g| g.animated).is_none() {
        return;
    }
    let mut game = Game::new(level);
    let strip = |game: &Game| {
        let pixels = game.render().to_gray();
        (136..144)
            .flat_map(|y| (80..160).map(move |x| (y, x)))
            .map(|(y, x)| pixels[y * 160 + x])
            .collect::<Vec<u8>>()
    };
    let mut seen = Vec::new();
    for _ in 0..3 {
        for _ in 0..sml::core::level::ANIMATION_HOLD {
            game.step(Buttons::default());
        }
        seen.push(strip(&game));
    }
    assert_ne!(seen[0], seen[1], "the water never changed");
    assert_eq!(seen[0], seen[2], "the water did not come back");
}

/// World 1-1's opening frame against the emulator capture of the same frame.
/// The capture is a tilemap plus its own sheet, so this is our whole screen
/// against the real game's: status bar, background, and where they sit.
///
/// Every 8x8 cell has to match except the ones Mario stands in. He is drawn as
/// a placeholder block, and the cartridge draws his sprite there.
#[test]
fn the_opening_frame_matches_the_emulator_capture() {
    use sml::assets::level as assets;
    use sml::render::{render_background, Framebuffer, Palette};

    let (Ok(map), Ok(sheet)) = (
        sml::assets::load_tilemap("assets/extracted/level_1_1_opening.tmap"),
        sml::assets::TileSheet::load("assets/extracted/level_1_1_opening.tiles"),
    ) else {
        return;
    };
    let Ok(level) = assets::extracted_level("1-1") else { return };
    if level.graphics.is_none() {
        return;
    }

    let mut captured = Framebuffer::new();
    render_background(&mut captured, &map, &sheet.tiles, 0, 0, &Palette::new(sheet.palette));

    // The values the capture was taken at: two lives left, no coins, no score,
    // 393 on the clock.
    let mut game = Game::new(level);
    game.lives = 2;
    game.timer = 393;
    // One frame, because the capture is of a game that has run one. A Game
    // that has never stepped has Mario off the ground, and he is drawn in the
    // airborne pose, which the cartridge does not do at a level's opening.
    game.step(Buttons::default());

    let (mw, mh) = game.mario.size();
    let mario = (
        game.mario.pixel_x() - game.camera.x,
        game.mario.pixel_y() - game.camera.y + sml::game::PLAYFIELD_TOP,
        mw,
        mh,
    );

    let (ours, theirs) = (game.render().to_gray(), captured.to_gray());
    let mut wrong = Vec::new();
    for ty in 0..18 {
        for tx in 0..20 {
            let (x, y) = (tx as i32 * 8, ty as i32 * 8);
            let over_mario = x < mario.0 + mario.2 && x + 8 > mario.0 && y < mario.1 + mario.3 && y + 8 > mario.1;
            if over_mario {
                continue;
            }
            let same = (0..8).all(|r| {
                let at = (ty * 8 + r) * 160 + tx * 8;
                ours[at..at + 8] == theirs[at..at + 8]
            });
            if !same {
                wrong.push((tx, ty));
            }
        }
    }
    assert!(wrong.is_empty(), "cells that do not match the capture: {wrong:?}");
}

/// A coin on screen is the cartridge's coin tile, drawn where the level says.
/// The engine draws coins itself rather than leaving them in the background,
/// so this checks the picture it draws is the cartridge's own.
#[test]
fn coins_draw_with_the_cartridges_coin_tile() {
    use sml::assets::level as assets;
    use sml::render::{Framebuffer, Palette};

    let Ok(level) = assets::extracted_level("1-1") else { return };
    let Some(graphics) = level.graphics.clone() else { return };
    let coin = graphics.tiles[assets::COIN as usize];
    // World 1-1's first coins hang high over world column 87, so the camera
    // has to walk out to them. They are out of Mario's reach on the ground,
    // so they are all still there when it arrives.
    let &(cx, cy) = level.coins.iter().min().expect("World 1-1 has coins");

    let mut expected = Framebuffer::new();
    expected.draw_tile(&coin, 0, 0, &Palette::new(graphics.palette));
    let expected = expected.to_gray();

    // Put the camera on the coin rather than walking there: this is a check
    // on what gets drawn, and holding right through 1-1 costs Mario his lives.
    let mut game = Game::new(level);
    game.camera.x = cx - 80;
    assert!(game.coins.contains(&(cx, cy)), "the coin is still there");

    let drawn = game.render().to_gray();
    let (sx, sy) = (
        (cx - game.camera.x) as usize,
        (cy - game.camera.y + sml::game::PLAYFIELD_TOP) as usize,
    );
    for row in 0..8 {
        let at = (sy + row) * 160 + sx;
        assert_eq!(
            &drawn[at..at + 8],
            &expected[row * 160..row * 160 + 8],
            "row {row} of the coin at ({cx}, {cy})"
        );
    }
}

/// The cartridge's exit door is drawn by the level's own background, so the
/// engine's end marker stays off. Without the level's graphics it stays on,
/// since a hand-written level has no door drawn for it.
#[test]
fn a_cartridge_level_does_not_draw_its_own_end_marker() {
    use sml::assets::level as assets;
    let Ok(level) = assets::extracted_level("1-1") else { return };
    let Some((ex, ey)) = level.end else { panic!("1-1 has an end trigger") };
    if level.graphics.is_none() {
        return;
    }

    let mut plain = level.clone();
    plain.graphics = None;

    let at = |game: &Game| {
        let pixels = game.render().to_gray();
        (0..8)
            .map(|row| {
                let i = (ey - game.camera.y + sml::game::PLAYFIELD_TOP) as usize * 160
                    + (ex - game.camera.x) as usize
                    + 3;
                pixels[i + row * 160]
            })
            .collect::<Vec<u8>>()
    };
    let mut cartridge = Game::new(level);
    cartridge.camera.x = ex - 80;
    let mut blocks = Game::new(plain);
    blocks.camera.x = ex - 80;

    // The marker is a black pole down the middle of its tile, so without the
    // cartridge's graphics that column is black top to bottom.
    let marker = at(&blocks);
    assert!(marker.iter().all(|&p| p == 0), "the placeholder marker is not a black pole");
    assert!(at(&cartridge).iter().any(|&p| p != 0), "the marker is still drawn over the door");
}

/// The status bar follows the session from level to level. A session rebuilds
/// its game from a cloned level on every transition and restart, so the
/// cartridge graphics and the level number have to survive the clone.
#[test]
fn the_status_bar_names_each_level_of_a_world() {
    use sml::session::Session;
    let Some(levels) = sml::session::world_levels(1) else { return };
    if levels[0].graphics.is_none() {
        return;
    }
    assert_eq!(
        levels.iter().map(|l| l.number).collect::<Vec<_>>(),
        vec![Some((1, 1)), Some((1, 2)), Some((1, 3))]
    );

    let mut session = Session::new(levels.clone());
    let mut start = Buttons::default();
    start.set(Button::Start, true);
    for _ in 0..4 {
        session.step(start);
        session.step(Buttons::default());
    }
    for (index, level) in levels.iter().enumerate() {
        assert_eq!(session.current_level(), index, "the session is on the wrong level");
        let bar = sml::hud::status_bar(
            session.game.score,
            session.game.coins_collected,
            session.game.lives,
            level.number.expect("a cartridge level knows its number"),
            session.game.timer,
        );
        assert_eq!(bar[1][12], sml::hud::digit(1), "world number on level {index}");
        assert_eq!(bar[1][14], sml::hud::digit(index as u8 + 1));
        assert!(session.game.level.graphics.is_some(), "level {index} lost its graphics");
        // What the session actually renders has to be the same two rows.
        let drawn = session.render().to_gray();
        let mut expected = sml::render::Framebuffer::new();
        let graphics = level.graphics.as_ref().unwrap();
        for (row, cells) in bar.iter().enumerate() {
            for (column, &id) in cells.iter().enumerate() {
                expected.draw_tile(
                    &graphics.tiles[id as usize],
                    column as i32 * 8,
                    row as i32 * 8,
                    &sml::render::Palette::new(graphics.palette),
                );
            }
        }
        let expected = expected.to_gray();
        assert_eq!(&drawn[..16 * 160], &expected[..16 * 160], "bar on level {index}");

        // Finish the level the way the session does, so the next pass is
        // checking a game the session built rather than one the test did.
        session.game.completed = true;
        for _ in 0..8 {
            session.step(start);
            session.step(Buttons::default());
        }
    }
}

/// Mario is drawn from the cartridge's own atlas, not as a black block. Every
/// pixel his frame draws has to be on screen in the right shade, and every
/// pixel it leaves transparent has to show what was behind him.
#[test]
fn mario_draws_with_the_cartridges_sprite() {
    use sml::assets::level as assets;
    use sml::assets::sprite::{self, Size, FRAME_SIZE};
    use sml::render::Palette;

    let Ok(level) = assets::extracted_level("1-1") else { return };
    let Some(graphics) = level.graphics.clone() else { return };

    let mut game = Game::new(level);
    // A frame of nothing held: still on the ground, so the still frame.
    game.step(Buttons::default());
    let drawn = game.render().to_gray();

    let pixels = sprite::mario_pixels(&graphics.sprites, Size::Small, 0);
    let (_, mh) = game.mario.size();
    let left = game.mario.pixel_x() - game.camera.x - 3;
    let top = game.mario.pixel_y() - game.camera.y + sml::game::PLAYFIELD_TOP + mh
        - FRAME_SIZE as i32;
    let palette = Palette::new(sml::assets::DEFAULT_BGP);

    let mut ink = 0;
    for (dy, row) in pixels.iter().enumerate() {
        for (dx, &index) in row.iter().enumerate() {
            if index == 0 {
                continue;
            }
            ink += 1;
            let (x, y) = (left + dx as i32, top + dy as i32);
            assert_eq!(
                drawn[(y as usize) * 160 + x as usize],
                palette.shade(index).to_gray(),
                "sprite pixel {dx},{dy}"
            );
        }
    }
    assert_eq!(ink, 84, "the still frame draws 84 pixels");
}

/// Facing left mirrors the frame rather than drawing a second set of tiles,
/// which is what the hardware's flip bit does and why the atlas holds only
/// right-facing Mario. The control is the unmirrored comparison, which has to
/// fail: Mario is not symmetric, and a test that passes either way is measuring
/// nothing.
#[test]
fn walking_left_mirrors_him() {
    use sml::assets::level as assets;
    use sml::assets::sprite::{self, Size, FRAME_SIZE};
    use sml::render::Palette;

    let Ok(level) = assets::extracted_level("1-1") else { return };
    let Some(graphics) = level.graphics.clone() else { return };
    let mut game = Game::new(level);

    let mut left = Buttons::default();
    left.set(Button::Left, true);
    game.step(left);
    let drawn = game.render().to_gray();

    let pixels = sprite::mario_pixels(&graphics.sprites, Size::Small, 0);
    let (_, mh) = game.mario.size();
    let block_left = game.mario.pixel_x() - game.camera.x - 3;
    let top = game.mario.pixel_y() - game.camera.y + sml::game::PLAYFIELD_TOP + mh
        - FRAME_SIZE as i32;
    let palette = Palette::new(sml::assets::DEFAULT_BGP);

    let matches = |mirror: bool| {
        pixels.iter().enumerate().all(|(dy, row)| {
            row.iter().enumerate().all(|(dx, &index)| {
                if index == 0 {
                    return true;
                }
                let sx = if mirror { FRAME_SIZE - 1 - dx } else { dx };
                let (x, y) = (block_left + sx as i32, top + dy as i32);
                drawn[(y as usize) * 160 + x as usize] == palette.shade(index).to_gray()
            })
        })
    };
    assert!(matches(true), "facing left did not draw the mirrored frame");
    assert!(!matches(false), "the unmirrored frame also matched, so this proves nothing");
}

/// A hand-written level has no atlas to draw from, so it keeps the block. A
/// custom level still renders.
#[test]
fn a_hand_written_level_keeps_the_placeholder_mario() {
    let level = Game::demo_level();
    assert!(level.graphics.is_none());
    let game = Game::new(level);
    let drawn = game.render().to_gray();
    let x = (game.mario.pixel_x() - game.camera.x) as usize;
    let y = (game.mario.pixel_y() - game.camera.y + sml::game::PLAYFIELD_TOP) as usize;
    // The placeholder is solid black across its whole width.
    assert!((0..8).all(|dx| drawn[(y + 6) * 160 + x + dx] == 0));
}

/// Any world can be played as a campaign, not just World 1. This drives a real
/// session through World 3's three levels: each one has to arrive with its own
/// graphics, its own number on the status bar, and Mario standing in it.
#[test]
fn a_session_plays_a_world_other_than_the_first() {
    let Some(levels) = sml::session::world_levels(3) else { return };
    assert_eq!(levels.len(), 3);
    if levels[0].graphics.is_none() {
        return;
    }

    let mut session = sml::session::Session::new(levels);
    let mut start = Buttons::default();
    start.set(Button::Start, true);
    session.step(start);
    session.step(Buttons::default());

    for level in 1..=3u8 {
        assert_eq!(session.current_level(), level as usize - 1);
        assert_eq!(session.game.level.number, Some((3, level)));
        assert!(session.game.level.graphics.is_some(), "3-{level} lost its graphics");

        let bar = sml::hud::status_bar(
            session.game.score,
            session.game.coins_collected,
            session.game.lives,
            (3, level),
            session.game.timer,
        );
        assert_eq!(bar[1][12], sml::hud::digit(3));
        assert_eq!(bar[1][14], sml::hud::digit(level));
        // The frame renders without panicking and is not blank.
        let drawn = session.render().to_gray();
        assert!(drawn.iter().any(|&p| p != drawn[0]), "3-{level} renders as one flat shade");

        session.game.completed = true;
        for _ in 0..8 {
            session.step(start);
            session.step(Buttons::default());
        }
    }
}

/// The enemies draw from the cartridge's atlas too, at the tiles measured off
/// the running game (`tools/measure_object_sprites.py`). World 1-1's list is
/// mostly Chibibos, so this walks until one is on screen and checks every ink
/// pixel of tile 0x90 against the shade it should be.
///
/// The control is the same comparison against a different tile, which has to
/// fail: a test that passes for any tile would only be showing that something
/// was drawn.
#[test]
fn a_chibibo_draws_with_the_cartridges_tile() {
    use sml::assets::level as assets;
    use sml::core::enemy::{EnemyKind, ENEMY_SIZE};
    use sml::render::Palette;

    let Ok(level) = assets::extracted_level("1-1") else { return };
    let Some(graphics) = level.graphics.clone() else { return };

    let mut game = Game::new(level);
    let mut held = Buttons::default();
    held.set(Button::Right, true);
    let mut found = None;
    for _ in 0..1200 {
        game.step(held);
        let on_screen = game.enemies.iter().find(|e| {
            e.alive
                && e.kind == EnemyKind::Goomba
                && (16..140).contains(&(e.pixel_x() - game.camera.x))
        });
        if let Some(enemy) = on_screen {
            found = Some((enemy.pixel_x(), enemy.pixel_y()));
            break;
        }
    }
    let Some((ex, ey)) = found else {
        panic!("no Chibibo came on screen in 1200 frames");
    };

    let drawn = game.render().to_gray();
    let left = ex - game.camera.x;
    let top = ey - game.camera.y + sml::game::PLAYFIELD_TOP + ENEMY_SIZE - 8;
    let palette = Palette::new(sml::assets::DEFAULT_BGP);

    let matches = |tile: &sml::tiles::Tile| {
        let mut ink = 0;
        for (dy, row) in tile.pixels.iter().enumerate() {
            for (dx, &index) in row.iter().enumerate() {
                if index == 0 {
                    continue;
                }
                let (x, y) = (left + dx as i32, top + dy as i32);
                if drawn[(y as usize) * 160 + x as usize] != palette.shade(index).to_gray() {
                    return None;
                }
                ink += 1;
            }
        }
        Some(ink)
    };

    let ink = matches(&graphics.sprites[0x90]).expect("the Chibibo draws tile 0x90");
    assert!(ink > 20, "tile 0x90 is a drawing rather than a blank, {ink} pixels");
    assert!(
        matches(&graphics.sprites[0x96]).is_none(),
        "control: the Nokobon's tile must not match where the Chibibo is"
    );
}

/// The lifts were never drawn at all before their tiles were measured, so
/// Mario rode an invisible platform. The cartridge draws one as the same tile
/// three times over, which is 24 pixels against the 16 the collision box uses
/// (`docs/reference/objects.md` records that disagreement).
#[test]
fn a_lift_draws_three_tiles_wide() {
    use sml::assets::level as assets;
    use sml::render::Palette;

    let Ok(level) = assets::extracted_level("1-1") else { return };
    let Some(graphics) = level.graphics.clone() else { return };
    assert!(!level.lifts.is_empty(), "1-1's list has lifts in it");

    let mut game = Game::new(level);
    // Put the camera on the first lift rather than walking the whole level.
    let lift = game.lifts[0];
    game.camera.x = (lift.x - 60).max(0);
    let drawn = game.render().to_gray();

    let palette = Palette::new(sml::assets::DEFAULT_BGP);
    let tile = &graphics.sprites[0xEF];
    let left = lift.x - game.camera.x;
    let top = lift.y - game.camera.y + sml::game::PLAYFIELD_TOP;

    for column in 0..3 {
        for (dy, row) in tile.pixels.iter().enumerate() {
            for (dx, &index) in row.iter().enumerate() {
                if index == 0 {
                    continue;
                }
                let x = left + column * 8 + dx as i32;
                let y = top + dy as i32;
                assert_eq!(
                    drawn[(y as usize) * 160 + x as usize],
                    palette.shade(index).to_gray(),
                    "lift column {column} pixel {dx},{dy}"
                );
            }
        }
    }
}

/// The drop block draws as a single `0xEE`, the tile next to the lift's, which
/// is the whole difference between the two on screen.
#[test]
fn a_drop_block_draws_one_tile_wide() {
    use sml::assets::level as assets;
    use sml::core::lift::Motion;
    use sml::render::Palette;

    let Ok(level) = assets::extracted_level("1-2") else { return };
    let Some(graphics) = level.graphics.clone() else { return };

    let mut game = Game::new(level);
    let block = *game
        .lifts
        .iter()
        .find(|l| l.motion == Motion::Drop)
        .expect("1-2's list has drop blocks in it");
    game.camera.x = (block.x - 60).max(0);
    let drawn = game.render().to_gray();

    let palette = Palette::new(sml::assets::DEFAULT_BGP);
    let tile = &graphics.sprites[0xEE];
    let left = block.x - game.camera.x;
    let top = block.y - game.camera.y + sml::game::PLAYFIELD_TOP;
    let mut ink = 0;
    for (dy, row) in tile.pixels.iter().enumerate() {
        for (dx, &index) in row.iter().enumerate() {
            if index == 0 {
                continue;
            }
            ink += 1;
            let (x, y) = (left + dx as i32, top + dy as i32);
            assert_eq!(
                drawn[(y as usize) * 160 + x as usize],
                palette.shade(index).to_gray(),
                "drop block pixel {dx},{dy}"
            );
        }
    }
    assert!(ink > 0, "control: tile 0xEE has to draw something");
}

/// A hand-written level brings no cartridge graphics, so its enemies keep the
/// placeholder block. Custom levels have to keep working.
#[test]
fn a_hand_written_level_keeps_the_placeholder_enemy() {
    let level = sml::core::level::Level::from_rows(&["M...G.", "######", "######"]);
    let mut game = Game::new(level);
    game.step(Buttons::default());
    assert!(game.render().to_gray().iter().any(|&p| p != 0xFF));
}


/// World 1 played end to end, through the session rather than a level at a
/// time. These three levels are the default campaign, and until now nothing
/// walked them in one run: a flood fill proves an end trigger is reachable,
/// and a reachable trigger is not the same as a game that gets to it.
///
/// The walker runs right and jumps when it stalls or runs out of ground, with
/// one addition. Where a gap has a lift in it the crossing is a timing rather
/// than a reflex, so those edges take a wait from a plan, and the plan is
/// searched for: 240 candidates per decision, which is a whole lift cycle
/// there and back, keeping whichever wait carried him furthest. Half a cycle
/// is not enough, and 1-2's second gap is what showed it: its lifts only come
/// down to his level on the return leg. World 1-2's pit needs it,
/// where columns 100 to 116 hold nothing solid and two vertical lifts are the
/// only way across.
///
/// Enemies are cleared, the same as the single-level walkthrough. What is
/// checked is the geometry, the transitions, and that nothing added since
/// seals a route that used to be open.
#[test]
fn world_1_can_be_walked_from_its_first_level_to_its_last() {
    use sml::session::{Phase, Session};
    let Some(levels) = sml::session::world_levels(1) else { return };

    // One run of the whole world with a given plan. Returns how many levels
    // finished and how far he got in the one he stopped in.
    let attempt = |waits: &[u32]| -> (usize, i32) {
        let mut levels = levels.clone();
        for level in &mut levels {
            level.enemy_spawns.clear();
        }
        let mut session = Session::new(levels);
        let mut start = Buttons::default();
        start.set(Button::Start, true);
        session.step(start);
        session.step(Buttons::default());

        let (mut finished, mut on_level) = (0usize, 0usize);
        let (mut furthest, mut stalled) = (0, 0);
        let mut jump = Jump::new();
        let (mut decisions, mut retreat, mut armed) = (0usize, 0u32, false);
        for _ in 0..24_000 {
            if session.current_level() != on_level {
                on_level = session.current_level();
                finished += 1;
                furthest = 0;
                stalled = 0;
            }
            if session.phase() == Phase::Win {
                finished += 1;
                break;
            }
            // A crossing that costs a life is not a crossing, and ending the
            // attempt there keeps the search from replaying every failure to
            // the frame budget.
            if session.game.deaths > 0 {
                break;
            }
            let game = &session.game;
            let x = game.mario.pixel_x();
            if game.mario.on_ground {
                if x <= furthest {
                    stalled += 1;
                } else {
                    stalled = 0;
                    furthest = x;
                }
            }
            // Two look-aheads, and they are for different questions. One
            // column says when to jump, which is at the edge. Two columns say
            // when to think about waiting, because by the time the nearer one
            // is empty he is already airborne with no decision left to make,
            // and jumping on the further one alone falls short of an ordinary
            // pit.
            // From his own row downwards. A platform above him is not somewhere
            // he can fall onto, and counting it walks him off column 15 of 1-2,
            // where one hangs well below the ledge he is standing on.
            let solid_at = |c: i32| {
                (feet_row(game.mario.pixel_y())..16)
                    .any(|row| game.level.solids.is_standable(c, row))
            };
            let ground_ahead = solid_at((x + 12) / 8);
            let ground_soon = solid_at((x + 20) / 8);
            let mut buttons = Buttons::default();
            let feet = game.mario.pixel_y() + 12;
            let on_lift = game.lifts.iter().any(|l| {
                (l.y - feet).abs() <= 2 && x + 11 >= l.x && x <= l.x + l.width()
            });
            // Back up only where there is ground behind him. The wait doubles
            // as a run-up on open terrain, and there are two places it must
            // not: on a lift, where behind him is the gap, and on the three
            // columns before 1-2's third crossing, where the ledge has a
            // one-column hole he reversed straight into.
            let room_behind = solid_at((x - 4) / 8) && solid_at((x - 12) / 8);
            // The wait runs to completion once started, rather than a frame
            // per decision. Both are the same on flat ground and they are not
            // on a ledge: backing up puts ground ahead again, which stops the
            // decision firing, so the wait only advanced on the frames he was
            // back at the edge. What that produced was a delay quantised to
            // the length of that oscillation instead of a wait that sweeps the
            // lift's whole cycle, which is what a timed crossing needs.
            if retreat > 0 {
                retreat -= 1;
                buttons.set(Button::Left, !on_lift && room_behind);
                session.step(buttons);
                continue;
            }
            if game.mario.on_ground && !ground_soon && jump.ready() {
                // The lift he is standing on counts. Riding one to the far end
                // of its travel and jumping from there is how 1-2's third gap
                // is crossed, and a test for a lift strictly ahead of him made
                // that not a decision at all: he jumped off the near end the
                // frame he landed, every time.
                let lift_near = game
                    .lifts
                    .iter()
                    .any(|l| l.x + l.width() > x && l.x < x + 160);
                if lift_near {
                    if armed {
                        armed = false;
                    } else {
                        let want = waits.get(decisions).copied().unwrap_or(0);
                        decisions += 1;
                        if want > 0 {
                            retreat = want;
                            armed = true;
                            buttons.set(Button::Left, !on_lift && room_behind);
                            session.step(buttons);
                            continue;
                        }
                    }
                    jump.start();
                }
            }
            if (stalled > 6 || !ground_ahead) && game.mario.on_ground && jump.ready() {
                jump.start();
            }
            buttons.set(Button::Right, true);
            jump.press(&mut buttons);
            session.step(buttons);
        }
        (finished, furthest)
    };

    let mut plan: Vec<u32> = Vec::new();
    let mut best = attempt(&plan);
    for _round in 0..8 {
        if best.0 >= 3 {
            break;
        }
        let mut improved = (best, None);
        for w in 0..240 {
            plan.push(w);
            let got = attempt(&plan);
            plan.pop();
            if got > improved.0 {
                improved = (got, Some(w));
            }
        }
        // A flat round still moves on: the decision that matters may be the
        // next one.
        plan.push(improved.1.unwrap_or(0));
        best = improved.0;
    }
    // Where this gets to today: World 1-1 and World 1-2 both finish, all three
    // of 1-2's lift crossings included, and he stops early in 1-3.
    //
    // The last of those crossings took a measurement rather than a better
    // search. Every lift in the engine set off in the +1 direction because
    // nobody had looked, and the cartridge sends the horizontal one left
    // (`tools/measure_lift_phase.py`). Going right it never comes within 72
    // pixels of the ledge at column 222, so no plan could have crossed it.
    // He then walks into World 1-3's pocket at column 11 and stops there. That
    // is geometry rather than a plan: see
    // `world_1_3s_pocket_at_column_11_is_a_dead_end_for_this_collision_model`.
    assert_eq!(best.0, 2, "1-1 and 1-2 both have to finish");
    assert!(
        best.1 / 8 >= 10,
        "with plan {plan:?} he only reached column {} of World 1-3",
        best.1 / 8
    );
}

/// What stops the World 1 walk, pinned so a change to the collision model
/// shows up here rather than as a mystery further along.
///
/// World 1-3 opens with a raised block at row 10 spanning columns 7 to 11, and
/// a two-tile wall at columns 13 and 14 whose top is row 12. Walking right
/// along the floor at row 14 takes Mario under the block and up against the
/// wall, and from there he cannot get out: the wall's top is 16 pixels above
/// his feet, and the block above him stops his rise 4 pixels short of that.
/// The route the level means him to take is over the block rather than under
/// it, which the walker has no rule for.
///
/// Ours is the only collision model this has ever been checked against. The
/// rise is capped because *any* part of his 11 pixel width is under the block,
/// and he is only under it by one column here; a cartridge that tests a
/// narrower head would let him through. Nothing has measured that.
#[test]
fn world_1_3s_pocket_at_column_11_is_a_dead_end_for_this_collision_model() {
    let Ok(mut level) = sml::assets::level::extracted_level("1-3") else {
        return;
    };
    for row in 12..=13 {
        assert!(level.solids.is_solid(13, row), "the wall at column 13");
    }
    assert!(level.solids.is_solid(11, 10), "the block overhead");
    assert!(!level.solids.is_solid(12, 10), "which ends at column 11");

    level.enemy_spawns.clear();
    level.spawn = (11 * 8, 14 * 8 - 12);
    let mut game = Game::new(level);
    let mut jump = Jump::new();
    let mut highest = i32::MAX;
    let mut furthest = 0;
    for _ in 0..600 {
        if game.mario.on_ground && jump.ready() {
            jump.start();
        }
        let mut buttons = Buttons::default();
        buttons.set(Button::Right, true);
        jump.press(&mut buttons);
        game.step(buttons);
        highest = highest.min(game.mario.pixel_y());
        furthest = furthest.max(game.mario.pixel_x());
    }
    // His feet at the top of the jump, against the top of the wall at row 12.
    assert_eq!(highest + 12, 100, "the ceiling caps his feet here");
    assert_eq!(furthest, 13 * 8 - 11, "and he never gets past the wall");
}
