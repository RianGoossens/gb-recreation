//! The game loop as a headless, deterministic object.
//!
//! Everything the window would do lives here, minus the window. A [`Game`] holds
//! the level, Mario, and the camera, steps one frame from a button snapshot, and
//! renders to a [`Framebuffer`]. Because it never touches a window or the clock,
//! it can be driven by scripted input in tests and rendered to PNGs for visual
//! checks, so the game is fully testable without anyone opening a window.

use crate::camera::Camera;
use crate::core::animation::Animator;
use crate::core::enemy::{despawn_offscreen, update_enemy, Enemy, ENEMY_SIZE};
use crate::core::entity::Mario;
use crate::core::level::{Level, TILE};
use crate::core::physics::{step_motion, STOMP_BOUNCE};
use crate::input::Buttons;
use crate::render::{render_background, Framebuffer, Palette, TileMap};
use crate::tiles::Tile;

fn solid_tile(color_index: u8) -> Tile {
    Tile {
        pixels: [[color_index; 8]; 8],
    }
}

pub struct Game {
    pub level: Level,
    pub mario: Mario,
    pub enemies: Vec<Enemy>,
    pub camera: Camera,
    pub animator: Animator,
    /// How many times Mario has died and respawned this session.
    pub deaths: u32,
    bg_map: TileMap,
    bg_tiles: Vec<Tile>,
    mario_tile: Tile,
    enemy_tile: Tile,
    palette: Palette,
}

fn spawn_enemies(level: &Level) -> Vec<Enemy> {
    level
        .enemy_spawns
        .iter()
        .map(|&(px, py)| Enemy::goomba(px, py, true))
        .collect()
}

impl Game {
    pub fn new(level: Level) -> Self {
        let (w, h) = (level.solids.width, level.solids.height);
        let mut cells = Vec::with_capacity(w * h);
        for ty in 0..h {
            for tx in 0..w {
                let solid = level.solids.is_solid(tx as i32, ty as i32);
                cells.push(if solid { 1 } else { 0 });
            }
        }
        let mario = Mario::new(level.spawn.0, level.spawn.1);
        let enemies = spawn_enemies(&level);
        Self {
            level,
            mario,
            enemies,
            camera: Camera::new(),
            animator: Animator::new(),
            deaths: 0,
            bg_map: TileMap::new(w, h, cells),
            // Empty tiles render white, solid tiles dark, Mario a black block.
            bg_tiles: vec![solid_tile(0), solid_tile(2)],
            mario_tile: solid_tile(3),
            // Enemies render as a light-gray block so they stand out from both
            // the white background and the dark terrain.
            enemy_tile: solid_tile(1),
            palette: Palette::new(0xE4),
        }
    }

    /// A small hand-made test level: a floor, two platforms, Mario near the left.
    /// This is our own content, safe to render in CI and commit as a golden.
    pub fn demo_level() -> Level {
        let (w, h) = (40usize, 18usize);
        let mut rows: Vec<String> = Vec::new();
        for y in 0..h {
            let mut row = String::new();
            for x in 0..w {
                let floor = y >= h - 2;
                let platform =
                    (y == h - 5 && (10..14).contains(&x)) || (y == h - 8 && (20..26).contains(&x));
                let c = if floor || platform {
                    '#'
                } else if x == 2 && y == h - 3 {
                    'M'
                } else if x == 16 && y == h - 3 {
                    'G'
                } else {
                    '.'
                };
                row.push(c);
            }
            rows.push(row);
        }
        let refs: Vec<&str> = rows.iter().map(String::as_str).collect();
        Level::from_rows(&refs)
    }

    fn level_size(&self) -> (i32, i32) {
        (
            self.level.solids.width as i32 * TILE,
            self.level.solids.height as i32 * TILE,
        )
    }

    /// Advance one frame from the held buttons.
    pub fn step(&mut self, buttons: Buttons) {
        step_motion(&mut self.mario, buttons, &self.level.solids);
        self.animator.update(&self.mario);
        for enemy in &mut self.enemies {
            update_enemy(enemy, &self.level.solids);
        }
        self.resolve_interactions();
        let (lw, lh) = self.level_size();
        self.camera
            .follow(self.mario.pixel_x() + 4, self.mario.pixel_y() + 4, lw, lh);
        despawn_offscreen(&mut self.enemies, self.camera.x);

        if !self.mario.alive {
            self.deaths += 1;
            self.respawn();
        }
    }

    /// Resolve Mario touching enemies. Coming down onto an enemy's upper half is
    /// a stomp: the enemy dies and Mario bounces. Any other contact is a fatal
    /// hit (Mario is small; power-ups come later).
    fn resolve_interactions(&mut self) {
        let (mw, mh) = self.mario.size();
        let ml = self.mario.pixel_x();
        let mt = self.mario.pixel_y();
        let (mr, mb) = (ml + mw - 1, mt + mh - 1);
        let descending = self.mario.vy > 0;

        let mut stomped = false;
        let mut hit = false;
        for enemy in &mut self.enemies {
            if !enemy.alive {
                continue;
            }
            let (el, et, er, eb) = enemy.edges();
            let overlap = ml <= er && mr >= el && mt <= eb && mb >= et;
            if !overlap {
                continue;
            }
            if descending && mb <= et + ENEMY_SIZE / 2 {
                enemy.alive = false;
                stomped = true;
            } else {
                hit = true;
            }
        }
        if stomped {
            self.mario.vy = -STOMP_BOUNCE;
            self.mario.on_ground = false;
        }
        // A stomp in the same frame saves Mario from a simultaneous side hit.
        if hit && !stomped {
            self.mario.alive = false;
        }
    }

    /// Put Mario back at the spawn and restore the enemies. The camera snaps back
    /// on the next step via follow.
    fn respawn(&mut self) {
        self.mario = Mario::new(self.level.spawn.0, self.level.spawn.1);
        self.enemies = spawn_enemies(&self.level);
        self.animator = Animator::new();
    }

    /// Render the current frame.
    pub fn render(&self) -> Framebuffer {
        let mut fb = Framebuffer::new();
        render_background(
            &mut fb,
            &self.bg_map,
            &self.bg_tiles,
            self.camera.x,
            self.camera.y,
            &self.palette,
        );
        for enemy in &self.enemies {
            if enemy.alive {
                fb.draw_tile(
                    &self.enemy_tile,
                    enemy.pixel_x() - self.camera.x,
                    enemy.pixel_y() - self.camera.y,
                    &self.palette,
                );
            }
        }
        fb.draw_tile(
            &self.mario_tile,
            self.mario.pixel_x() - self.camera.x,
            self.mario.pixel_y() - self.camera.y,
            &self.palette,
        );
        fb
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Button;

    fn held(button: Button) -> Buttons {
        let mut b = Buttons::default();
        b.set(button, true);
        b
    }

    #[test]
    fn mario_starts_at_the_level_spawn() {
        let game = Game::new(Game::demo_level());
        assert_eq!(
            (game.mario.pixel_x(), game.mario.pixel_y()),
            game.level.spawn
        );
    }

    #[test]
    fn standing_still_mario_settles_on_the_ground() {
        let mut game = Game::new(Game::demo_level());
        for _ in 0..60 {
            game.step(Buttons::default());
        }
        assert!(game.mario.on_ground);
        assert_eq!(game.mario.vx, 0);
        // The camera cannot scroll past the left edge.
        assert_eq!(game.camera.x, 0);
    }

    #[test]
    fn walking_right_moves_mario_and_eventually_scrolls() {
        use crate::core::level::Level;
        // A wide, enemy-free lane so this isolates scrolling from any collision.
        let top = format!("M{}", ".".repeat(39));
        let empty = ".".repeat(40);
        let floor = "#".repeat(40);
        let level = Level::from_rows(&[&top, &empty, &empty, &floor]);
        let mut game = Game::new(level);

        let start_x = game.mario.pixel_x();
        for _ in 0..120 {
            game.step(held(Button::Right));
        }
        assert!(game.mario.pixel_x() > start_x, "Mario should have moved right");
        assert!(game.mario.on_ground, "should stay on the floor");
        // Far enough right that the camera has left the left edge.
        assert!(game.camera.x > 0, "camera should have scrolled");
    }

    #[test]
    fn demo_level_has_a_goomba_that_moves() {
        let mut game = Game::new(Game::demo_level());
        assert_eq!(game.enemies.len(), 1);
        let start = game.enemies[0].x;
        for _ in 0..30 {
            game.step(Buttons::default());
        }
        assert!(game.enemies[0].x != start, "the goomba should have walked");
    }

    #[test]
    fn falling_onto_a_goomba_stomps_it_and_bounces_mario() {
        use crate::core::level::Level;
        // Mario above a Goomba, both over a floor.
        let level = Level::from_rows(&[".M..", "....", "....", ".G..", "####"]);
        let mut game = Game::new(level);
        game.enemies[0].vx = 0; // keep the goomba under Mario for a clean drop

        let mut bounced = false;
        for _ in 0..120 {
            game.step(Buttons::default());
            if game.enemies.is_empty() {
                // The goomba was stomped (dead enemies are despawned).
                bounced = game.mario.vy < 0;
                break;
            }
        }
        assert!(game.enemies.is_empty(), "the goomba should have been stomped");
        assert!(bounced, "Mario should bounce up off the stomp");
    }

    #[test]
    fn walking_into_a_goomba_from_the_side_kills_and_respawns_mario() {
        use crate::core::level::Level;
        // Mario and a Goomba on the same flat floor, a gap apart.
        let level = Level::from_rows(&["M...G", "#####"]);
        let mut game = Game::new(level);

        for _ in 0..200 {
            game.step(held(Button::Right));
            if game.deaths > 0 {
                break;
            }
        }
        assert!(game.deaths > 0, "side contact with the goomba should kill Mario");
        // After respawn Mario is alive again, back near the spawn.
        assert!(game.mario.alive);
        assert!(game.mario.pixel_x() <= 8, "should be back at the spawn");
    }

    #[test]
    fn render_produces_all_shades_present_in_the_scene() {
        let game = Game::new(Game::demo_level());
        let fb = game.render();
        let grays: std::collections::HashSet<u8> = fb.to_gray().into_iter().collect();
        // The demo scene has empty (white), solid (dark), and Mario (black).
        assert!(grays.contains(&255));
        assert!(grays.len() >= 2);
    }
}
