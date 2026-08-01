//! Mario's graphics, straight out of the cartridge's object atlas.
//!
//! Skips when the ROM is absent, like every other ROM-gated test.

use sml::assets::sprite::{self, Size};

const ROM: &str = "super_mario_land.gb";

fn sheet() -> Option<Vec<sml::tiles::Tile>> {
    world_sheet(1)
}

fn world_sheet(world: usize) -> Option<Vec<sml::tiles::Tile>> {
    let data = std::fs::read(ROM).ok()?;
    sprite::sprite_sheet(&data, world).ok()
}

/// The reason these four tiles are Mario and not some other figure in the
/// atlas. His collision box was measured on the cartridge by walling him into
/// corridors, which has nothing to do with graphics, and came out 11 wide by
/// 12 tall. The first frame draws over 10 by 12 pixels of its block: the same
/// height exactly, and a box a pixel wider than the drawing.
#[test]
fn small_marios_first_frame_is_the_size_he_was_measured_at() {
    let Some(sheet) = sheet() else { return };
    let pixels = sprite::mario_pixels(&sheet, Size::Small, 0);
    let (x, y, w, h) = sprite::ink_box(&pixels).expect("the frame draws something");
    assert_eq!((w, h), (10, 12));
    // He stands on the bottom edge of his block, which is what lines the
    // drawing up with a position given as feet on the ground.
    assert_eq!(y + h, sprite::FRAME_SIZE);
    assert_eq!(x, 3);
}

/// Big Mario is the taller figure, which is the other half of the check: the
/// two sizes are told apart by the drawing rather than by where they sit.
#[test]
fn big_mario_is_taller_than_small_mario() {
    let Some(sheet) = sheet() else { return };
    let small = sprite::ink_box(&sprite::mario_pixels(&sheet, Size::Small, 0)).unwrap();
    let big = sprite::ink_box(&sprite::mario_pixels(&sheet, Size::Big, 0)).unwrap();
    assert_eq!(small.3, 12);
    assert_eq!(big.3, 16);
    assert!(big.3 > small.3);
}

/// The three frames of a size are three different pictures, all Mario-shaped.
/// A layout read wrongly gives blank blocks or repeats, and this catches both.
#[test]
fn each_size_has_three_distinct_frames() {
    let Some(sheet) = sheet() else { return };
    for size in [Size::Small, Size::Big] {
        let frames: Vec<_> = (0..sprite::FRAMES)
            .map(|f| sprite::mario_pixels(&sheet, size, f))
            .collect();
        for (i, frame) in frames.iter().enumerate() {
            let (_, _, w, h) = sprite::ink_box(frame).expect("frame draws something");
            assert!((9..=13).contains(&w), "frame {i} is {w} wide");
            assert!((12..=16).contains(&h), "frame {i} is {h} tall");
        }
        assert_ne!(frames[0], frames[1]);
        assert_ne!(frames[1], frames[2]);
        assert_ne!(frames[0], frames[2]);
    }
}

/// A world loads its own enemies over part of the atlas. Comparing the four
/// worlds' sheets tile for tile leaves exactly the ids in `PER_WORLD`
/// differing, and Mario's frames among the ones that do not: he is the same
/// drawing in every world, the enemies are not.
#[test]
fn a_world_replaces_only_the_enemy_tiles() {
    let Some(first) = world_sheet(1) else { return };
    let sheets: Vec<_> = (1..=4).filter_map(world_sheet).collect();
    assert_eq!(sheets.len(), 4);

    let differing: Vec<usize> = (0..256)
        .filter(|&id| sheets.iter().any(|s| s[id] != first[id]))
        .collect();
    assert_eq!(differing.first().copied(), Some(*sprite::PER_WORLD.start() as usize));
    assert_eq!(differing.last().copied(), Some(*sprite::PER_WORLD.end() as usize));
    assert_eq!(differing.len(), 61);

    for size in [Size::Small, Size::Big] {
        for frame in 0..sprite::FRAMES {
            for id in sprite::mario_frame(size, frame) {
                assert!(
                    !differing.contains(&(id as usize)),
                    "Mario's tile {id} changes between worlds"
                );
            }
        }
    }
}
