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

/// The blocks a walk cycles through are three different pictures, all
/// Mario-shaped, and so is the jump pose. A layout read wrongly gives blank
/// blocks or repeats, and this catches both.
#[test]
fn each_size_has_three_distinct_walk_frames_and_a_jump() {
    let Some(sheet) = sheet() else { return };
    for size in [Size::Small, Size::Big] {
        let blocks: Vec<usize> = sprite::WALK_BLOCKS
            .iter()
            .copied()
            .chain([sprite::JUMP_BLOCK])
            .collect();
        let frames: Vec<_> = blocks
            .iter()
            .map(|&b| sprite::mario_pixels(&sheet, size, b))
            .collect();
        for (i, frame) in frames.iter().enumerate() {
            let (_, _, w, h) = sprite::ink_box(frame).expect("frame draws something");
            // The jump pose has his arms out, so it is wider than a walk
            // frame. Keeping the walk bound tight is what makes this test
            // catch a misread layout at all.
            let widest = if blocks[i] == sprite::JUMP_BLOCK { 15 } else { 13 };
            assert!((9..=widest).contains(&w), "frame {i} is {w} wide");
            assert!((12..=16).contains(&h), "frame {i} is {h} tall");
        }
        for a in 0..frames.len() {
            for b in a + 1..frames.len() {
                assert_ne!(frames[a], frames[b], "blocks {a} and {b} draw the same");
            }
        }
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
        for block in sprite::WALK_BLOCKS.iter().copied().chain([sprite::JUMP_BLOCK]) {
            for id in sprite::mario_frame(size, block) {
                assert!(
                    !differing.contains(&(id as usize)),
                    "Mario's tile {id} changes between worlds"
                );
            }
        }
    }
}

/// The object sprite tables were transcribed by hand from the measurement
/// tool's output, so nothing but a typo stands between a correct table and a
/// wrong one. Every tile a table names has to be a drawing in the atlas.
///
/// The control is that a blank tile exists to be caught: the atlas has plenty,
/// and the test asserts one so a change that makes everything look non-blank
/// cannot pass silently.
#[test]
fn every_object_sprite_names_a_tile_that_draws_something() {
    use sml::assets::sprite::Piece;

    let Some(sheet) = sheet() else { return };
    let ink = |id: u8| -> usize {
        sheet[id as usize]
            .pixels
            .iter()
            .flatten()
            .filter(|&&p| p != 0)
            .count()
    };

    let tables: [(&str, &[Piece]); 8] = [
        ("Chibibo", sprite::CHIBIBO),
        ("Gao", sprite::GAO),
        ("Bunbun", sprite::BUNBUN),
        ("Nokobon", sprite::NOKOBON),
        ("Fly", sprite::FLY),
        ("Falling Slab", sprite::FALLING_SLAB),
        ("lift", sprite::LIFT),
        ("drop block", sprite::DROP_BLOCK),
    ];
    for (name, pieces) in tables {
        assert!(!pieces.is_empty(), "{name} has no tiles");
        for piece in pieces {
            assert!(
                ink(piece.tile) > 0,
                "{name}'s tile {:#04X} is blank in the atlas",
                piece.tile
            );
        }
        // A 2x2 block reads left to right, top to bottom, so no two pieces may
        // land on the same spot.
        for (i, a) in pieces.iter().enumerate() {
            for b in &pieces[i + 1..] {
                assert!(
                    (a.dx, a.dy) != (b.dx, b.dy),
                    "{name} puts two tiles at the same offset"
                );
            }
        }
    }

    // Control: `ink` has to be able to return zero, or the checks above pass
    // for any tile id at all.
    assert!(
        (0..=255u8).any(|id| ink(id) == 0),
        "control: no tile in the atlas is blank, so a blank one cannot be caught"
    );
}

/// Every 16x16 kind is four ids of one 2x2 block: `n`, `n + 1`, `n + 16`,
/// `n + 17`. Reading them as four consecutive ids draws a scramble, which is
/// what makes the arithmetic worth pinning rather than the ids alone.
#[test]
fn the_sixteen_by_sixteen_kinds_are_all_one_block() {
    for (name, pieces) in [
        ("Fly", sprite::FLY),
        ("Bunbun", sprite::BUNBUN),
        ("Gao", sprite::GAO),
    ] {
        let ids: Vec<u8> = pieces.iter().map(|p| p.tile).collect();
        let n = ids[0];
        assert_eq!(ids, vec![n, n + 1, n + 16, n + 17], "{name}");
        let offsets: Vec<(i32, i32)> = pieces.iter().map(|p| (p.dx, p.dy)).collect();
        assert_eq!(offsets, vec![(0, -16), (8, -16), (0, -8), (8, -8)], "{name}");
    }
}

/// The Fly is drawn from the per-world band, so it looks different in each
/// world, while the Chibibo is drawn from the shared part and does not. That
/// is what the band means, checked against the tables actually in use rather
/// than against the range constant.
#[test]
fn only_the_per_world_kinds_change_between_worlds() {
    let sheets: Vec<_> = (1..=4).filter_map(world_sheet).collect();
    if sheets.len() != 4 {
        return;
    }
    let differs = |id: u8| sheets.iter().any(|s| s[id as usize] != sheets[0][id as usize]);

    for piece in sprite::FLY.iter().chain(sprite::GAO) {
        assert!(differs(piece.tile), "{:#04X} should be per world", piece.tile);
    }
    for piece in sprite::CHIBIBO {
        assert!(!differs(piece.tile), "the Chibibo's {:#04X} should be shared", piece.tile);
    }
}
