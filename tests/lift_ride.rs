//! Riding a lift through the real game loop.
//!
//! The unit tests in `core::lift` check the cycle and the one landing. These
//! check the whole thing running: gravity, collision, and the lift all in
//! `Game::step`, which is where a wrong ordering between them shows up.

use sml::core::lift::{DROP_DELAY, HORIZONTAL_HALF_CYCLE, VERTICAL_HALF_CYCLE};
use sml::core::level::Level;
use sml::game::Game;
use sml::input::{Button, Buttons};

/// Mario over a lift with nothing else under him for a long way.
fn over_a_lift(marker: char) -> Level {
    let mut rows = vec![".".repeat(20); 12];
    rows[1].replace_range(2..3, "M");
    rows[3].replace_range(2..3, &marker.to_string());
    rows[11] = "#".repeat(20);
    let refs: Vec<&str> = rows.iter().map(String::as_str).collect();
    Level::from_rows(&refs)
}

#[test]
fn mario_falls_onto_a_lift_and_rides_it_down() {
    let mut game = Game::new(over_a_lift('V'));
    for _ in 0..20 {
        game.step(Buttons::default());
    }
    assert!(game.mario.on_ground, "he should have landed on the lift");
    let landed = game.mario.pixel_y();
    let lift_top = game.lifts[0].y;

    // Stop short of the reversal: the first 20 frames were spent landing.
    for _ in 0..VERTICAL_HALF_CYCLE - 40 {
        game.step(Buttons::default());
    }
    assert_eq!(game.lifts[0].y - lift_top, (VERTICAL_HALF_CYCLE as i32 - 40) / 2);
    assert!(game.mario.on_ground, "still standing on it");
    assert_eq!(
        game.mario.pixel_y() - landed,
        game.lifts[0].y - lift_top,
        "he moved exactly as far as the lift did"
    );
}

#[test]
fn a_horizontal_lift_carries_him_sideways() {
    let mut game = Game::new(over_a_lift('H'));
    for _ in 0..20 {
        game.step(Buttons::default());
    }
    assert!(game.mario.on_ground);
    let landed = game.mario.pixel_x();
    let lift_left = game.lifts[0].x;

    // Not a full half cycle: 20 frames of it are already spent getting him
    // onto the lift, and counting from zero would run past the reversal.
    for _ in 0..HORIZONTAL_HALF_CYCLE - 40 {
        game.step(Buttons::default());
    }
    assert_eq!(
        game.mario.pixel_x() - landed,
        game.lifts[0].x - lift_left,
        "carried exactly as far as the lift went, without pressing anything"
    );
    assert!(game.lifts[0].x > lift_left, "and the lift did move");
}

#[test]
fn he_can_jump_up_through_a_lift_and_land_back_on_it() {
    let mut game = Game::new(over_a_lift('V'));
    for _ in 0..20 {
        game.step(Buttons::default());
    }
    let resting = game.mario.pixel_y();

    let mut jump = Buttons::default();
    jump.set(Button::A, true);
    let mut highest = resting;
    for _ in 0..10 {
        game.step(jump);
        highest = highest.min(game.mario.pixel_y());
    }
    assert!(
        highest < resting - 8,
        "a jump has to clear the lift rather than stopping on it"
    );

    // The lift is still descending while he is in the air, so give him room
    // to catch up with it.
    let mut back_on = false;
    for _ in 0..90 {
        game.step(Buttons::default());
        if game.mario.on_ground {
            back_on = true;
            break;
        }
    }
    assert!(back_on, "and he comes back down onto it");
}

/// The drop block through the whole loop. The shape to match is the traced
/// one: he lands, nothing happens for nine frames, and then both of them go
/// down together a pixel a frame.
#[test]
fn a_drop_block_gives_way_under_him_and_takes_him_down() {
    let mut game = Game::new(over_a_lift('X'));
    // The delay is counted from the touch, so the touch is what to find.
    let mut touched = false;
    for _ in 0..20 {
        game.step(Buttons::default());
        if game.mario.on_ground {
            touched = true;
            break;
        }
    }
    assert!(touched, "he should have landed on the block");
    let landed = game.mario.pixel_y();
    let top = game.lifts[0].y;

    for _ in 0..DROP_DELAY - 1 {
        game.step(Buttons::default());
    }
    assert_eq!(game.lifts[0].y, top, "still put for the nine frames");
    assert_eq!(game.mario.pixel_y(), landed);

    for _ in 0..30 {
        game.step(Buttons::default());
    }
    assert_eq!(game.lifts[0].y - top, 30, "a pixel a frame, no let up");
    assert_eq!(
        game.mario.pixel_y() - landed,
        game.lifts[0].y - top,
        "and he goes down with it"
    );
    assert!(game.mario.on_ground);
}

/// The other half: it is a surface, so it has to hold him before it gives way.
/// Without the block he is in open air and falls to the floor far below.
#[test]
fn without_the_block_he_falls_straight_past() {
    let mut game = Game::new(over_a_lift('.'));
    for _ in 0..20 {
        game.step(Buttons::default());
    }
    assert!(
        !game.mario.on_ground,
        "control: nothing there, so nothing catches him"
    );
}

/// World 2-1's gap, the one the geometry walker stops at, is crossed by lift
/// rather than by swimming. On the cartridge, dropping Mario into it leaves
/// him alive on a lift at its own resting offset, carried back and forth
/// (`tools/probe_water.py`). This is the same thing in the engine.
#[test]
fn world_2_1s_water_gap_is_crossed_by_lift() {
    use sml::assets::level as assets;
    use sml::core::lift::Motion;

    let Ok(level) = assets::extracted_level("2-1") else { return };
    let mut game = Game::new(level);
    let lift = *game
        .lifts
        .iter()
        .filter(|l| l.motion != Motion::Drop)
        .min_by_key(|l| (l.x - 66 * 8).abs())
        .expect("2-1 has lifts in the gap");
    assert!(
        (lift.x - 66 * 8).abs() < 8 * 8,
        "the nearest lift to the gap is at {}",
        lift.x
    );

    // Over it, well above, with nothing else under him for the width of the
    // gap. The camera has to be there too or the lift is never stepped.
    game.mario.x = sml::core::entity::pixels(lift.x + 8);
    game.mario.y = sml::core::entity::pixels(lift.y - 60);
    game.mario.vx = 0;
    game.mario.vy = 0;
    game.camera.x = lift.x - 80;
    let mut landed = None;
    for frame in 0..200 {
        game.step(Buttons::default());
        if game.mario.on_ground {
            landed = Some(frame);
            break;
        }
    }
    assert!(landed.is_some(), "he fell through the gap instead of onto a lift");

    let carried = game.mario.pixel_x();
    for _ in 0..40 {
        game.step(Buttons::default());
    }
    assert!(game.mario.on_ground, "and he stays on it");
    assert_ne!(
        game.mario.pixel_x(),
        carried,
        "carried sideways without pressing anything"
    );
}

#[test]
fn a_level_without_lifts_has_none() {
    let level = Level::from_rows(&["....", "..M.", "####"]);
    assert!(level.lifts.is_empty());
    let game = Game::new(level);
    assert!(game.lifts.is_empty());
}

/// How long to hold A for the highest jump the engine gives. Holding past
/// `MAX_RISE_FRAMES` ends the rise on the spot, so a walker that holds longer
/// jumps lower, which is how the climb out of 1-2's first platform run was
/// being missed.
const FULL_JUMP: u32 = sml::core::physics::MAX_RISE_FRAMES as u32;

/// One run over a lift crossing with a given plan, returning how far right
/// Mario stood.
///
/// He makes one decision and makes it the same way every time: standing with
/// nothing to stand on two columns ahead, he waits `w` frames and then jumps
/// right, taking each `w` from the plan in turn. Everything else is holding
/// right. A jump onto a moving platform is a timing, and the engine is
/// deterministic, so the timing is searched for rather than guessed at; three
/// heuristic walkers failed at the first of these crossings before that.
fn run_plan(level: &Level, first_edge: i32, target: i32, waits: &[u32]) -> i32 {
    let mut level = level.clone();
    level.enemy_spawns.clear();
    let mut game = Game::new(level);
    let (mut furthest, mut decisions, mut waited, mut hold_jump) = (0, 0usize, 0, 0);
    for _ in 0..6000 {
        let x = game.mario.pixel_x();
        // Standing only. Drifting rightwards while falling into the pit covers
        // ground too, and counting that would score the fall as progress; the
        // engine kills him at the bottom and puts him back, so a death has to
        // end the attempt as well.
        if game.mario.on_ground {
            furthest = furthest.max(x);
        }
        if furthest >= target || game.deaths > 0 {
            break;
        }
        // Two columns of look-ahead. Checking the column under his front foot
        // is too late: by the time it is empty he has already walked off the
        // edge and is in the air, where he has no decision left to make. And
        // the question is whether he can *stand* there, so a one-way platform
        // counts: 1-2's second gap ends on four of them.
        let column = (x + 20) / 8;
        // From his own row downwards. Anything above him he cannot fall onto,
        // and counting it walks him off the edge at column 15 of this level,
        // where a platform hangs well below the ledge he is on.
        // From his feet, not his head. A platform level with his chest sits 16
        // pixels above the surface he is standing on, so counting it walks him
        // off the step at column 29 rather than jumping up it.
        // The first row whose surface can hold him: a surface in row `r` has
        // its top at `r * 8`, and his feet are at `y + 12`. His lowest pixel
        // is a row out and counts the ledge he is standing on as somewhere
        // ahead he can walk to.
        let feet_row = (game.mario.pixel_y() + 12).div_euclid(8);
        let ground_ahead =
            (feet_row..16).any(|row| game.level.solids.is_standable(column, row));
        let mut buttons = Buttons::default();
        if game.mario.on_ground && !ground_ahead && hold_jump == 0 {
            // Only this crossing's own edges are decisions. 1-2 has small
            // holes elsewhere that a plain jump clears, and letting those
            // consume plan entries tunes the wrong gap.
            if x > first_edge {
                let want = waits.get(decisions).copied().unwrap_or(0);
                if waited < want {
                    // Backing up rather than standing still. The jump has to
                    // cover 48 pixels to the first lift's near edge, and a
                    // standing jump does not, so the wait doubles as the
                    // run-up.
                    waited += 1;
                    // Back up only when he is on terrain. On a lift there is
                    // nothing behind him but the pit, so waiting there means
                    // standing still.
                    let feet = game.mario.pixel_y() + 12;
                    let on_lift = game.lifts.iter().any(|l| {
                        (l.y - feet).abs() <= 2 && x + 11 >= l.x && x <= l.x + l.width()
                    });
                    buttons.set(Button::Left, !on_lift);
                    game.step(buttons);
                    continue;
                }
                waited = 0;
                decisions += 1;
            }
            hold_jump = FULL_JUMP;
        }
        buttons.set(Button::Right, true);
        if hold_jump > 0 {
            buttons.set(Button::A, true);
            hold_jump -= 1;
        }
        game.step(buttons);
    }
    furthest
}

/// Search for a plan that gets Mario to `target`.
///
/// A beam rather than a hill climb. Keeping only the single best plan tunes
/// one decision at a time, which is enough for a two-lift crossing and not
/// enough for a three-lift one: the wait that gets furthest on the second lift
/// can be the one that leaves the third out of reach, and a single-plan search
/// has already thrown away the alternative by the time it finds out. `WIDTH`
/// plans carried forward keeps those alternatives alive.
fn search_plan(level: &Level, first_edge: i32, target: i32, rounds: usize) -> (i32, Vec<u32>) {
    const WIDTH: usize = 4;
    // A vertical lift's full cycle is 240 frames, so 240 candidate waits cover
    // every phase it can be in when he decides to jump.
    const PHASES: u32 = 240;

    let mut beam: Vec<(i32, Vec<u32>)> = vec![(run_plan(level, first_edge, target, &[]), Vec::new())];
    for _ in 0..rounds {
        let mut next: Vec<(i32, Vec<u32>)> = Vec::new();
        for (_, plan) in &beam {
            for w in 0..PHASES {
                let mut candidate = plan.clone();
                candidate.push(w);
                next.push((run_plan(level, first_edge, target, &candidate), candidate));
            }
        }
        next.sort_by_key(|(reach, _)| std::cmp::Reverse(*reach));
        next.dedup_by_key(|(reach, _)| *reach);
        next.truncate(WIDTH);
        beam = next;
        if beam[0].0 >= target {
            break;
        }
    }
    beam.into_iter().next().unwrap()
}

/// Can World 1-2's first pit be crossed at all?
///
/// Columns 103 to 116 have nothing to stand on and the crossing is two
/// vertical lifts, at columns 105 and 111, each moving 60 pixels on a 120
/// frame cycle. The far side is solid ground at column 117.
///
/// He is placed on the ledge rather than walked to it, so that what fails
/// when this fails is the crossing and not the platform maze before it.
#[test]
fn world_1_2s_pit_can_be_crossed_by_riding_its_lifts() {
    use sml::assets::level as assets;

    let Ok(mut level) = assets::extracted_level("1-2") else { return };
    // The ledge is the one-way platform at row 7, columns 100 to 102.
    level.spawn = (100 * 8, 7 * 8 - 12);
    let target = 117 * 8;
    let (best, plan) = search_plan(&level, 99 * 8, target, 4);
    assert!(
        best >= target,
        "the best plan {plan:?} only reached column {}, short of the far side \
at 117",
        best / 8
    );
}

/// And the second one, which is three lifts.
///
/// Columns 187 to 202 have nothing to stand on, with vertical lifts at 187,
/// 192 and 197, and the far side is a run of one-way platforms starting at
/// column 203. This gap read as 25 columns wide and impossible for a while,
/// because the walker asked whether a column was solid and platforms are not:
/// on that reading the nearest thing to stand on was 96 pixels past the last
/// lift, well beyond any jump.
///
/// He is placed on the ledge rather than walked to it, so what this measures
/// is the gap and not everything before it. The camera has to come along or
/// the lifts are never stepped.
#[test]
fn world_1_2s_second_gap_is_crossed_by_riding_three_lifts() {
    use sml::assets::level as assets;
    use sml::core::entity::pixels;

    let Ok(mut level) = assets::extracted_level("1-2") else { return };
    // Column 184 is the last full step of the ledge, its top at row 13.
    level.spawn = (184 * 8, 13 * 8 - 12);
    let target = 203 * 8;
    let (best, plan) = search_plan(&level, 183 * 8, target, 5);
    assert!(
        best >= target,
        "the best plan {plan:?} only reached column {}, short of the platform \
at 203",
        best / 8
    );

    // And the plan replays: the search is over a deterministic engine, so the
    // same waits have to produce the same crossing a second time.
    let mut replay = level.clone();
    replay.enemy_spawns.clear();
    let mut game = Game::new(replay);
    assert_eq!(game.mario.x, pixels(184 * 8));
    assert_eq!(run_plan(&level, 183 * 8, target, &plan), best);
    game.step(Buttons::default());
}
