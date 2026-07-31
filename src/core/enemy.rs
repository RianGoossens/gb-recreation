//! Enemies: the framework for things that move and can be defeated.
//!
//! An enemy walks, falls under gravity, and collides with the world much like
//! Mario, but with simpler behavior. This module owns the shared parts: the
//! entity, one physics step (walk plus falling plus collision), and despawning
//! enemies that have scrolled off screen. Per-type quirks (like a Fly's hop)
//! build on top of this.

use crate::core::level::{Solids, TILE};
use crate::core::physics::GRAVITY;
use crate::SCREEN_WIDTH;

/// Enemies are one tile square.
pub const ENEMY_SIZE: i32 = 8;
/// Horizontal walk speed in subpixels per frame.
///
/// Measured from the cartridge with `tools/measure_enemy_walk.py`, on the
/// object kind World 1-1's list uses ten times. The camera in Super Mario
/// Land only moves while Mario does, so letting go of right freezes it and
/// leaves the object's slot X moving under its own power alone: it stepped one
/// pixel left 143 times, and every one of the 142 gaps between steps was
/// exactly 3 frames. So the walk is a counter rather than an accumulator, at
/// one pixel per three frames.
///
/// 256 subpixels make a pixel here, and 256/3 is not a whole number, so 85 is
/// as close as this representation gets: 0.4% slow, about a pixel behind the
/// cartridge every 750 frames.
pub const ENEMY_WALK_SPEED: i32 = 85;
/// Downward speed of a falling enemy, in subpixels per frame.
///
/// Cutting the ground out from under the cartridge's walker (collision reads
/// the tilemap in video RAM, so a pit can be made where there was none) drops
/// it one pixel per frame, flat, for the whole fall. There is no acceleration
/// to measure: leave a floor 8 pixels down and it takes exactly 8 frames to
/// reach it (`tools/probe_enemy_ledge.py`).
pub const ENEMY_FALL_SPEED: i32 = crate::core::entity::SUBPIXEL;
/// How far past the screen edges an enemy may be before it despawns.
pub const DESPAWN_MARGIN: i32 = 32;
/// Upward speed a Fly gets on each hop. Provisional.
pub const HOP_VELOCITY: i32 = 520;
/// Frames a Fly waits on the ground between hops.
pub const HOP_INTERVAL: u32 = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnemyKind {
    /// Walks along the ground, turns at walls, and walks off ledges.
    Goomba,
    /// Walks but hops on a timer, so it does not respect ledges.
    Fly,
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
    /// Countdown to the next hop, for a Fly. Ignored by other kinds.
    pub hop_timer: u32,
}

impl Enemy {
    fn new(pixel_x: i32, pixel_y: i32, going_left: bool, kind: EnemyKind) -> Self {
        use crate::core::entity::pixels;
        let speed = if going_left { -ENEMY_WALK_SPEED } else { ENEMY_WALK_SPEED };
        Self {
            x: pixels(pixel_x),
            y: pixels(pixel_y),
            vx: speed,
            vy: 0,
            on_ground: false,
            alive: true,
            kind,
            hop_timer: HOP_INTERVAL,
        }
    }

    /// A Goomba at a whole-pixel position, walking left or right.
    pub fn goomba(pixel_x: i32, pixel_y: i32, going_left: bool) -> Self {
        Self::new(pixel_x, pixel_y, going_left, EnemyKind::Goomba)
    }

    /// A Fly at a whole-pixel position: it walks and hops.
    pub fn fly(pixel_x: i32, pixel_y: i32, going_left: bool) -> Self {
        Self::new(pixel_x, pixel_y, going_left, EnemyKind::Fly)
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

    // Walking stops while airborne. Measured: the walker's screen X does not
    // change by a single pixel through a fall, and picks up again the frame it
    // lands (`tools/probe_enemy_ledge.py`).
    if enemy.on_ground {
        enemy.x += enemy.vx;
    }
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

    // A Fly hops on a timer while it is on the ground.
    if enemy.kind == EnemyKind::Fly && enemy.on_ground {
        if enemy.hop_timer == 0 {
            enemy.vy = -HOP_VELOCITY;
            enemy.on_ground = false;
            enemy.hop_timer = HOP_INTERVAL;
        } else {
            enemy.hop_timer -= 1;
        }
    }

    // An enemy falls at a flat rate rather than accelerating. Cutting the
    // ground from under the cartridge's own walker drops it one pixel per
    // frame, every frame, with no build up at all.
    if !enemy.on_ground {
        enemy.vy = if enemy.vy < 0 {
            (enemy.vy + GRAVITY).min(ENEMY_FALL_SPEED)
        } else {
            ENEMY_FALL_SPEED
        };
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
    fn enemy_walks_forward_on_open_ground() {
        let solids = floor();
        let mut e = Enemy::goomba(80, 16, true); // walking left on a wide floor
        let start = e.x;
        for _ in 0..10 {
            update_enemy(&mut e, &solids);
        }
        assert!(e.x < start, "should have walked left");
        assert!(e.on_ground);
    }

    #[test]
    fn enemy_walks_a_pixel_every_three_frames() {
        // What the cartridge does: 143 steps of one pixel, 3 frames apart
        // every time. Over 300 frames that is 100 pixels, and ours lands on
        // the same pixel.
        let solids = floor();
        let mut e = Enemy::goomba(120, 16, true);
        let start = e.pixel_x();
        for _ in 0..300 {
            update_enemy(&mut e, &solids);
        }
        assert_eq!(start - e.pixel_x(), 100);
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
    fn a_walker_leaves_a_ledge_instead_of_turning() {
        // A short platform (tiles 5..9 on the floor row) with empty space
        // beyond. Rows are 20 wide, 4 tall; the platform is on the bottom row.
        let mut floor_row = ".".repeat(20);
        floor_row.replace_range(5..10, "#####");
        let solids = Solids::from_rows(&[
            &".".repeat(20),
            &".".repeat(20),
            &".".repeat(20),
            &floor_row,
        ]);

        // Walker standing on the platform (tile 6, pixel x 48, y 16), going right.
        let mut e = Enemy::goomba(48, 16, false);
        e.on_ground = true;
        let edge_x = 10 * 8; // right edge of the platform
        for _ in 0..300 {
            update_enemy(&mut e, &solids);
        }
        assert!(e.pixel_x() >= edge_x, "should have walked off the end");
        assert!(!e.on_ground, "and be falling");
    }

    #[test]
    fn a_falling_walker_drops_straight_down_at_one_pixel_a_frame() {
        // Ground under the left half only, so the walker runs out of floor.
        let mut floor_row = "#".repeat(10);
        floor_row.push_str(&".".repeat(10));
        let solids = Solids::from_rows(&[
            &".".repeat(20),
            &".".repeat(20),
            &".".repeat(20),
            &floor_row,
        ]);
        let mut e = Enemy::goomba(48, 16, false);
        e.on_ground = true;
        while e.on_ground {
            update_enemy(&mut e, &solids);
        }
        let (x, y) = (e.pixel_x(), e.pixel_y());
        for step in 1..=8 {
            update_enemy(&mut e, &solids);
            assert_eq!(e.pixel_x(), x, "no horizontal movement while falling");
            assert_eq!(e.pixel_y(), y + step, "one pixel down per frame");
        }
    }

    #[test]
    fn a_fly_hops_off_the_ground() {
        let solids = floor();
        let mut f = Enemy::fly(40, 16, false);
        f.vx = 0; // isolate the hop
        // Settle onto the floor.
        for _ in 0..10 {
            update_enemy(&mut f, &solids);
        }
        assert!(f.on_ground);
        let resting_y = f.pixel_y();
        // Within one hop interval it should leave the ground and rise.
        let mut rose = false;
        for _ in 0..(HOP_INTERVAL as usize + 5) {
            update_enemy(&mut f, &solids);
            if f.pixel_y() < resting_y {
                rose = true;
                break;
            }
        }
        assert!(rose, "a Fly should hop above its resting height");
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
