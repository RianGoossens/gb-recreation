//! Power-ups: the mushroom that makes Mario big.
//!
//! A mushroom emerges from a power block, then walks the ground like a slow
//! enemy: it moves in one direction, falls under gravity, reverses at walls, and
//! rides off ledges. Mario grows when he touches it. Its movement mirrors the
//! enemy walker minus the ledge caution, since a mushroom is happy to fall.

use crate::core::entity::pixels;
use crate::core::level::{Solids, TILE};
use crate::core::physics::{GRAVITY, MAX_FALL_SPEED};

/// Mushrooms are one tile square.
pub const MUSHROOM_SIZE: i32 = 8;
/// Horizontal speed in subpixels per frame. Provisional.
pub const MUSHROOM_SPEED: i32 = 96;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mushroom {
    pub x: i32,
    pub y: i32,
    pub vx: i32,
    pub vy: i32,
    pub on_ground: bool,
}

impl Mushroom {
    pub fn new(pixel_x: i32, pixel_y: i32, going_left: bool) -> Self {
        let speed = if going_left { -MUSHROOM_SPEED } else { MUSHROOM_SPEED };
        Self {
            x: pixels(pixel_x),
            y: pixels(pixel_y),
            vx: speed,
            vy: 0,
            on_ground: false,
        }
    }

    pub fn pixel_x(&self) -> i32 {
        self.x.div_euclid(crate::core::entity::SUBPIXEL)
    }

    pub fn pixel_y(&self) -> i32 {
        self.y.div_euclid(crate::core::entity::SUBPIXEL)
    }

    pub fn edges(&self) -> (i32, i32, i32, i32) {
        let l = self.pixel_x();
        let t = self.pixel_y();
        (l, t, l + MUSHROOM_SIZE - 1, t + MUSHROOM_SIZE - 1)
    }
}

/// Advance a mushroom a frame: walk, reverse at walls, fall, land.
pub fn update_mushroom(m: &mut Mushroom, solids: &Solids) {
    m.x += m.vx;
    let (l, t, r, b) = m.edges();
    if m.vx > 0 && solids.rect_hits_solid(r, t, r, b) {
        let wall_left = r.div_euclid(TILE) * TILE;
        m.x = pixels(wall_left - MUSHROOM_SIZE);
        m.vx = -m.vx;
    } else if m.vx < 0 && solids.rect_hits_solid(l, t, l, b) {
        let wall_right = l.div_euclid(TILE) * TILE + (TILE - 1);
        m.x = pixels(wall_right + 1);
        m.vx = -m.vx;
    }

    if !m.on_ground {
        m.vy = (m.vy + GRAVITY).min(MAX_FALL_SPEED);
    }
    m.y += m.vy;
    let (l, _t, r, b) = m.edges();
    if m.vy > 0 && solids.rect_hits_solid(l, b, r, b) {
        let floor_top = b.div_euclid(TILE) * TILE;
        m.y = pixels(floor_top - MUSHROOM_SIZE);
        m.vy = 0;
    }

    let (l, _t, r, b) = m.edges();
    m.on_ground = solids.rect_hits_solid(l, b + 1, r, b + 1);
    if m.on_ground && m.vy > 0 {
        m.vy = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::level::Solids;

    fn floor() -> Solids {
        Solids::from_rows(&[
            &".".repeat(20),
            &".".repeat(20),
            &".".repeat(20),
            &"#".repeat(20),
        ])
    }

    #[test]
    fn mushroom_falls_and_walks() {
        let solids = floor();
        let mut m = Mushroom::new(40, 0, false);
        for _ in 0..200 {
            update_mushroom(&mut m, &solids);
        }
        assert_eq!(m.pixel_y(), 16); // rests on the floor
        assert!(m.on_ground);
    }

    #[test]
    fn mushroom_reverses_at_a_wall() {
        let mut rows = [
            "....................".to_string(),
            "....................".to_string(),
            "....................".to_string(),
            "####################".to_string(),
        ];
        for row in rows.iter_mut().take(3) {
            row.replace_range(7..8, "#");
        }
        let refs: Vec<&str> = rows.iter().map(String::as_str).collect();
        let solids = Solids::from_rows(&refs);

        let mut m = Mushroom::new(40, 16, false); // walking right into the wall
        let mut reversed = false;
        for _ in 0..200 {
            update_mushroom(&mut m, &solids);
            if m.vx < 0 {
                reversed = true;
            }
            assert!(m.pixel_x() <= 48);
        }
        assert!(reversed);
    }
}
