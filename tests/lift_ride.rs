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

#[test]
fn a_level_without_lifts_has_none() {
    let level = Level::from_rows(&["....", "..M.", "####"]);
    assert!(level.lifts.is_empty());
    let game = Game::new(level);
    assert!(game.lifts.is_empty());
}
