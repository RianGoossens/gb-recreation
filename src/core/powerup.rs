//! Power-up items: the things Mario collects to change state.
//!
//! An item emerges from a block or sits in the level, then walks and falls like
//! a slow, harmless enemy until Mario touches it. A mushroom makes him big; a
//! star makes him briefly invincible. Both share the same movement, so only the
//! effect on pickup differs (decided by the game).

use crate::core::entity::pixels;
use crate::core::level::{Solids, TILE};
use crate::core::physics::{GRAVITY, MAX_FALL_SPEED};

/// Items are one tile square.
pub const ITEM_SIZE: i32 = 8;
/// Horizontal speed in subpixels per frame. Provisional.
pub const ITEM_SPEED: i32 = 96;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    Mushroom,
    Star,
    /// A flower: makes Mario fire-powered (able to throw superballs).
    Flower,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Item {
    pub x: i32,
    pub y: i32,
    pub vx: i32,
    pub vy: i32,
    pub on_ground: bool,
    pub kind: ItemKind,
}

impl Item {
    fn new(pixel_x: i32, pixel_y: i32, going_left: bool, kind: ItemKind) -> Self {
        let speed = if going_left { -ITEM_SPEED } else { ITEM_SPEED };
        Self {
            x: pixels(pixel_x),
            y: pixels(pixel_y),
            vx: speed,
            vy: 0,
            on_ground: false,
            kind,
        }
    }

    pub fn mushroom(pixel_x: i32, pixel_y: i32, going_left: bool) -> Self {
        Self::new(pixel_x, pixel_y, going_left, ItemKind::Mushroom)
    }

    pub fn star(pixel_x: i32, pixel_y: i32, going_left: bool) -> Self {
        Self::new(pixel_x, pixel_y, going_left, ItemKind::Star)
    }

    pub fn flower(pixel_x: i32, pixel_y: i32, going_left: bool) -> Self {
        Self::new(pixel_x, pixel_y, going_left, ItemKind::Flower)
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
        (l, t, l + ITEM_SIZE - 1, t + ITEM_SIZE - 1)
    }
}

/// Advance an item a frame: walk, reverse at walls, fall, land.
pub fn update_item(m: &mut Item, solids: &Solids) {
    m.x += m.vx;
    let (l, t, r, b) = m.edges();
    if m.vx > 0 && solids.rect_hits_solid(r, t, r, b) {
        let wall_left = r.div_euclid(TILE) * TILE;
        m.x = pixels(wall_left - ITEM_SIZE);
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
        m.y = pixels(floor_top - ITEM_SIZE);
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
    fn item_falls_and_walks() {
        let solids = floor();
        let mut m = Item::mushroom(40, 0, false);
        for _ in 0..200 {
            update_item(&mut m, &solids);
        }
        assert_eq!(m.pixel_y(), 16); // rests on the floor
        assert!(m.on_ground);
    }

    #[test]
    fn item_reverses_at_a_wall() {
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

        let mut m = Item::star(40, 16, false); // walking right into the wall
        let mut reversed = false;
        for _ in 0..200 {
            update_item(&mut m, &solids);
            if m.vx < 0 {
                reversed = true;
            }
            assert!(m.pixel_x() <= 48);
        }
        assert!(reversed);
    }
}
