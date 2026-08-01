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

/// Can World 1-2's pit be crossed at all?
///
/// Columns 100 to 116 hold nothing solid and the crossing is two vertical
/// lifts, at columns 105 and 111, each moving 60 pixels on a 120 frame cycle.
/// Three heuristic walkers failed at it, and a heuristic is the wrong tool: a
/// jump onto a moving platform is a timing, and the engine is deterministic,
/// so the timing can be searched for instead of guessed at.
///
/// The walker here has one decision to make and makes it the same way every
/// time: whenever he is standing with nothing solid ahead, he waits `w`
/// frames and then jumps right, taking `w` from a list. The search fills that
/// list one entry at a time, keeping whichever wait got him furthest. A cycle
/// is 120 frames, so 120 candidates covers every phase a lift can be in.
#[test]
fn world_1_2s_pit_can_be_crossed_by_riding_its_lifts() {
    use sml::assets::level as assets;

    let Ok(level) = assets::extracted_level("1-2") else { return };
    let target = 117 * 8;

    // One run of the level with a given plan, returning how far right he got.
    let attempt = |waits: &[u32]| -> i32 {
        let mut level = level.clone();
        level.enemy_spawns.clear();
        let mut game = Game::new(level);
        let (mut furthest, mut decisions, mut waited, mut hold_jump) = (0, 0usize, 0, 0);
        for _ in 0..6000 {
            let x = game.mario.pixel_x();
            // Standing only. Drifting rightwards while falling into the pit
            // covers ground too, and counting that would score the fall as
            // progress; the engine kills him at the bottom and puts him back,
            // so a death has to end the attempt as well.
            if game.mario.on_ground {
                furthest = furthest.max(x);
            }
            if furthest >= target || game.deaths > 0 {
                break;
            }
            // Two columns of look-ahead. Checking the column under his front
            // foot is too late: by the time it is empty he has already walked
            // off the edge and is in the air, where he has no decision left to
            // make.
            let column = (x + 20) / 8;
            let ground_ahead = (0..16).any(|row| game.level.solids.is_solid(column, row));
            let mut buttons = Buttons::default();
            if game.mario.on_ground && !ground_ahead && hold_jump == 0 {
                // Only the pit's own edges are decisions. 1-2 has small holes
                // earlier that a plain jump clears, and letting those consume
                // plan entries tunes the wrong gap.
                if x > 95 * 8 {
                    let want = waits.get(decisions).copied().unwrap_or(0);
                    if waited < want {
                        // Backing up rather than standing still. The jump has
                        // to cover 48 pixels to the lift's near edge, and a
                        // standing jump does not, so the wait doubles as the
                        // run-up.
                        waited += 1;
                        // Back up only when he is on terrain. On a lift there
                        // is nothing behind him but the pit, so waiting there
                        // means standing still.
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
                hold_jump = 14;
            }
            buttons.set(Button::Right, true);
            if hold_jump > 0 {
                buttons.set(Button::A, true);
                hold_jump -= 1;
            }
            game.step(buttons);
        }
        furthest
    };

    let mut plan: Vec<u32> = Vec::new();
    let mut best = attempt(&plan);
    for _round in 0..6 {
        let mut improved = (best, None);
        for w in 0..120 {
            plan.push(w);
            let got = attempt(&plan);
            plan.pop();
            if got > improved.0 {
                improved = (got, Some(w));
            }
        }
        // A round that improves nothing still moves on: the next decision is
        // the one that matters, and stopping here would leave it untried.
        plan.push(improved.1.unwrap_or(0));
        best = improved.0;
        if best >= target {
            break;
        }
    }
    assert!(
        best >= target,
        "the best plan {plan:?} only reached column {}, short of the far side \
at 117",
        best / 8
    );
    // What it finds, with the search as it stands: back up 11 frames at the
    // ledge and jump, hop straight off the first lift, then wait 24 frames on
    // the second for it to rise before the last jump.
    assert_eq!(plan.len(), 3, "three jumps, one per edge: {plan:?}");
}
