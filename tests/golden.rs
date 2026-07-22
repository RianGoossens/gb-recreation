//! Golden-image test for the render pipeline.
//!
//! Renders our own demo scene to a PNG and compares it byte for byte against a
//! committed reference. The demo scene is our content, not extracted game data,
//! so it is safe to commit and runs in CI.
//!
//! To (re)generate the golden after an intentional render change:
//!   REGEN_GOLDEN=1 cargo test --test golden

use std::fs;

const GOLDEN: &str = "tests/golden/demo.png";

#[test]
fn demo_scene_matches_golden() {
    let fb = sml::scene::demo();
    let png = sml::png::encode_gray(sml::SCREEN_WIDTH, sml::SCREEN_HEIGHT, &fb.to_gray());

    if std::env::var("REGEN_GOLDEN").is_ok() {
        fs::create_dir_all("tests/golden").unwrap();
        fs::write(GOLDEN, &png).unwrap();
        return;
    }

    let golden = fs::read(GOLDEN)
        .expect("missing golden; create it with REGEN_GOLDEN=1 cargo test --test golden");
    assert!(
        png == golden,
        "demo render differs from the golden image. If this change is intended, \
         regenerate with REGEN_GOLDEN=1 cargo test --test golden"
    );
}
