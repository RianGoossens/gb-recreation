//! Golden-image test for the render pipeline.
//!
//! Renders our own demo scene to a PNG and compares it byte for byte against a
//! committed reference. The demo scene is our content, not extracted game data,
//! so it is safe to commit and runs in CI.
//!
//! To (re)generate the golden after an intentional render change:
//!   REGEN_GOLDEN=1 cargo test --test golden

use sml::game::Game;
use std::fs;

/// Compare a PNG to a committed golden, or regenerate it when REGEN_GOLDEN is set.
fn check_golden(path: &str, png: &[u8]) {
    if std::env::var("REGEN_GOLDEN").is_ok() {
        fs::create_dir_all("tests/golden").unwrap();
        fs::write(path, png).unwrap();
        return;
    }
    let golden = fs::read(path)
        .unwrap_or_else(|_| panic!("missing golden {path}; create it with REGEN_GOLDEN=1 cargo test --test golden"));
    assert!(
        png == golden.as_slice(),
        "{path} differs from its golden. If intended, regenerate with \
         REGEN_GOLDEN=1 cargo test --test golden"
    );
}

fn encode(fb: &sml::render::Framebuffer) -> Vec<u8> {
    sml::png::encode_gray(sml::SCREEN_WIDTH, sml::SCREEN_HEIGHT, &fb.to_gray())
}

#[test]
fn demo_scene_matches_golden() {
    check_golden("tests/golden/demo.png", &encode(&sml::scene::demo()));
}

#[test]
fn game_start_frame_matches_golden() {
    let game = Game::new(Game::demo_level());
    check_golden("tests/golden/game_start.png", &encode(&game.render()));
}

#[test]
fn game_after_walking_right_matches_golden() {
    let mut game = Game::new(Game::demo_level());
    let mut buttons = sml::input::Buttons::default();
    buttons.set(sml::input::Button::Right, true);
    for _ in 0..90 {
        game.step(buttons);
    }
    check_golden("tests/golden/game_walk_right.png", &encode(&game.render()));
}
