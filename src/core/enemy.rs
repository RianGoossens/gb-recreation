//! Enemies: the framework for things that move and can be defeated.
//!
//! An enemy walks, falls under gravity, and collides with the world much like
//! Mario, but with simpler behavior. This module owns the shared parts: the
//! entity, one physics step (walk plus gravity plus collision), and despawning
//! enemies that have scrolled off screen. Per-type quirks (like a Goomba not
//! walking off ledges) build on top of this.

use crate::core::level::{Solids, TILE};
use crate::core::physics::{GRAVITY, MAX_FALL_SPEED};
use crate::SCREEN_WIDTH;

/// Enemies are one tile square.
pub const ENEMY_SIZE: i32 = 8;
/// Horizontal walk speed in subpixels per frame. Provisional.
pub const ENEMY_WALK_SPEED: i32 = 96;
/// How far past the screen edges an enemy may be before it despawns.
pub const DESPAWN_MARGIN: i32 = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnemyKind {
    Goomba,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Enemy {
    /// Top-left, in subpixels (see entity::SUBPIXEL).
    pub x: i32,
    pub y: i32,
    pub vx: i32,
    pub vy: i32,
    pub on_ground: bool,
    pub alive: bool,
    pub kind: EnemyKind,
}

impl Enemy {
    /// A Goomba at a whole-pixel position, walking left or right.
    pub fn goomba(pixel_x: i32, pixel_y: i32, going_left: bool) -> Self {
        use crate::core::entity::pixels;
        let speed = if going_left { -ENEMY_WALK_SPEED } else { ENEMY_WALK_SPEED };
        Self {
            x: pixels(pixel_x),
            y: pixels(pixel_y),
            vx: speed,
            vy: 0,
            on_ground: false,
            alive: true,
            kind: EnemyKind::Goomba,
        }
    }

    pub fn pixel_x(&self) -> i32 {
        self.x.div_euclid(crate::core::entity::SUBPIXEL)
    }

    pub fn pixel_y(&self) -> i32 {
        self.y.div_euclid(crate::core::entity::SUBPIXEL)
    }

    /// Pixel edges (left, top, right, bottom), inclusive.
    pub fn edges(&self) -> (i32, i32, i32, i32) {
        let l = self.pixel_x();
        let t = self.pixel_y();
        (l, t, l + ENEMY_SIZE - 1, t + ENEMY_SIZE - 1)
    }
}

/// Advance one enemy a frame: walk, reverse at walls, fall, land on floors.
pub fn update_enemy(enemy: &mut Enemy, solids: &Solids) {
    use crate::core::entity::pixels;
    if !enemy.alive {
        return;
    }

    // A Goomba turns at a ledge instead of walking off. When grounded, probe the
    // ground just past its leading foot; if nothing is there, reverse first.
    if enemy.on_ground && enemy.kind == EnemyKind::Goomba && enemy.vx != 0 {
        let (l, _t, r, b) = enemy.edges();
        let ahead = if enemy.vx > 0 { r + 1 } else { l - 1 };
        if !solids.rect_hits_solid(ahead, b + 1, ahead, b + 1) {
            enemy.vx = -enemy.vx;
        }
    }

    enemy.x += enemy.vx;
    let (l, t, r, b) = enemy.edges();
    if enemy.vx > 0 && solids.rect_hits_solid(r, t, r, b) {
        let wall_left = r.div_euclid(TILE) * TILE;
        enemy.x = pixels(wall_left - ENEMY_SIZE);
        enemy.vx = -enemy.vx;
    } else if enemy.vx < 0 && solids.rect_hits_solid(l, t, l, b) {
        let wall_right = l.div_euclid(TILE) * TILE + (TILE - 1);
        enemy.x = pixels(wall_right + 1);
        enemy.vx = -enemy.vx;
    }

    // Gravity only builds up while airborne, so a resting enemy sits still
    // instead of creeping into the floor (same rule as Mario).
    if !enemy.on_ground {
        enemy.vy = (enemy.vy + GRAVITY).min(MAX_FALL_SPEED);
    }
    enemy.y += enemy.vy;
    let (l, _t, r, b) = enemy.edges();
    if enemy.vy > 0 && solids.rect_hits_solid(l, b, r, b) {
        let floor_top = b.div_euclid(TILE) * TILE;
        enemy.y = pixels(floor_top - ENEMY_SIZE);
        enemy.vy = 0;
    }

    let (l, _t, r, b) = enemy.edges();
    enemy.on_ground = solids.rect_hits_solid(l, b + 1, r, b + 1);
    if enemy.on_ground && enemy.vy > 0 {
        enemy.vy = 0;
    }
}

/// Remove enemies that are dead or have scrolled off screen. `camera_x` is the
/// left edge of the visible window in pixels.
pub fn despawn_offscreen(enemies: &mut Vec<Enemy>, camera_x: i32) {
    let left_bound = camera_x - DESPAWN_MARGIN;
    let right_bound = camera_x + SCREEN_WIDTH as i32 + DESPAWN_MARGIN;
    enemies.retain(|e| {
        if !e.alive {
            return false;
        }
        let (l, _t, r, _b) = e.edges();
        r >= left_bound && l <= right_bound
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::level::Solids;

    fn floor() -> Solids {
        // 20 wide, floor on the bottom row (row 3, pixels y 24..31).
        Solids::from_rows(&[
            &".".repeat(20),
            &".".repeat(20),
            &".".repeat(20),
            &"#".repeat(20),
        ])
    }

    #[test]
    fn enemy_falls_and_lands_on_the_floor() {
        let solids = floor();
        let mut e = Enemy::goomba(40, 0, true);
        e.vx = 0; // isolate the fall; horizontal walk is tested separately
        for _ in 0..200 {
            update_enemy(&mut e, &solids);
        }
        assert_eq!(e.pixel_y(), 16); // rests on floor top y=24, 8 tall
        assert!(e.on_ground);
    }

    #[test]
    fn enemy_reverses_at_a_wall() {
        // Wall column at x 56..63 (tile 7), floor below.
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

        let mut e = Enemy::goomba(40, 16, false); // walking right toward the wall
        assert!(e.vx > 0);
        let mut reversed = false;
        for _ in 0..200 {
            update_enemy(&mut e, &solids);
            if e.vx < 0 {
                reversed = true;
            }
            assert!(e.pixel_x() <= 48, "should never pass the wall at x=56");
        }
        assert!(reversed, "should have turned around at the wall");
    }

    #[test]
    fn goomba_turns_at_a_ledge_instead_of_walking_off() {
        // A short platform (tiles 5..9 on the floor row) with empty space beyond.
        // Rows are 20 wide, 4 tall; the platform is on the bottom row.
        let mut floor_row = ".".repeat(20);
        floor_row.replace_range(5..10, "#####");
        let solids = Solids::from_rows(&[
            &".".repeat(20),
            &".".repeat(20),
            &".".repeat(20),
            &floor_row,
        ]);

        // Goomba standing on the platform (tile 6, pixel x 48, y 16), walking right.
        let mut e = Enemy::goomba(48, 16, false);
        e.on_ground = true;
        for _ in 0..300 {
            update_enemy(&mut e, &solids);
        }
        // It never leaves the platform: its feet stay over solid tiles 5..9.
        let (l, _t, r, b) = e.edges();
        assert!(solids.rect_hits_solid(l, b + 1, r, b + 1), "should still be on the platform");
    }

    #[test]
    fn dead_enemies_do_not_move() {
        let solids = floor();
        let mut e = Enemy::goomba(40, 16, true);
        e.alive = false;
        let before = (e.x, e.y);
        update_enemy(&mut e, &solids);
        assert_eq!((e.x, e.y), before);
    }

    #[test]
    fn despawn_removes_dead_and_offscreen() {
        let mut enemies = vec![
            Enemy::goomba(100, 16, true),  // on screen
            Enemy::goomba(-100, 16, true), // far left, off screen
            Enemy::goomba(120, 16, true),  // on screen but dead
        ];
        enemies[2].alive = false;
        despawn_offscreen(&mut enemies, 0);
        assert_eq!(enemies.len(), 1);
        assert_eq!(enemies[0].pixel_x(), 100);
    }
}
