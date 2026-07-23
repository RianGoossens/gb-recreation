//! The game loop as a headless, deterministic object.
//!
//! Everything the window would do lives here, minus the window. A [`Game`] holds
//! the level, Mario, and the camera, steps one frame from a button snapshot, and
//! renders to a [`Framebuffer`]. Because it never touches a window or the clock,
//! it can be driven by scripted input in tests and rendered to PNGs for visual
//! checks, so the game is fully testable without anyone opening a window.

use crate::camera::Camera;
use crate::core::animation::Animator;
use crate::core::block::{Block, BlockKind};
use crate::core::enemy::{despawn_offscreen, update_enemy, Enemy, ENEMY_SIZE};
use crate::core::entity::{pixels, Mario, Power};
use crate::core::level::{Level, TILE};
use crate::core::physics::{step_motion, STOMP_BOUNCE};
use crate::core::powerup::{update_mushroom, Mushroom, MUSHROOM_SIZE};
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
    /// Set once Mario reaches the level-end trigger. The scene freezes.
    pub completed: bool,
    /// Interactive blocks Mario can bump from below.
    pub blocks: Vec<Block>,
    /// Active mushroom power-ups.
    pub mushrooms: Vec<Mushroom>,
    /// Uncollected coins, top-left pixel.
    pub coins: Vec<(i32, i32)>,
    /// Coins collected (wraps every 100, which grants a life).
    pub coins_collected: u32,
    pub lives: u32,
    pub score: u32,
    /// Countdown shown on the HUD. Ticks down over time; running out is handled
    /// by the timer task, not here.
    pub timer: u32,
    timer_ticks: u32,
    bg_map: TileMap,
    bg_tiles: Vec<Tile>,
    mario_tile: Tile,
    enemy_tile: Tile,
    coin_tile: Tile,
    question_tile: Tile,
    used_tile: Tile,
    brick_tile: Tile,
    mushroom_tile: Tile,
    end_tile: Tile,
    palette: Palette,
}

/// A tall marker for the level end: a vertical pole.
fn end_tile() -> Tile {
    let mut pixels = [[0u8; 8]; 8];
    for row in pixels.iter_mut() {
        row[3] = 3;
        row[4] = 3;
    }
    Tile { pixels }
}

/// A dark cap over a light body: a mushroom.
fn mushroom_tile() -> Tile {
    let mut pixels = [[1u8; 8]; 8];
    for row in pixels.iter_mut().take(4) {
        *row = [2; 8];
    }
    Tile { pixels }
}

fn spawn_blocks(level: &Level) -> Vec<Block> {
    level
        .blocks
        .iter()
        .map(|&(x, y, kind)| Block::new(x, y, kind))
        .collect()
}

/// A dark block with a light center dot: an unused question block.
fn question_tile() -> Tile {
    let mut pixels = [[2u8; 8]; 8];
    for row in pixels.iter_mut().take(5).skip(2) {
        row[3] = 1;
        row[4] = 1;
    }
    Tile { pixels }
}

/// A dark block with light mortar lines: a brick.
fn brick_tile() -> Tile {
    let mut pixels = [[2u8; 8]; 8];
    pixels[0] = [0; 8];
    pixels[4] = [0; 8];
    for row in pixels.iter_mut() {
        row[0] = 0;
        row[4] = 0;
    }
    Tile { pixels }
}

/// A small dark mark on an otherwise empty tile, so coins read differently from
/// Mario (solid black) and enemies (solid light).
fn coin_tile() -> Tile {
    let mut pixels = [[0u8; 8]; 8];
    for row in pixels.iter_mut().take(6).skip(2) {
        row[3] = 2;
        row[4] = 2;
    }
    Tile { pixels }
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
        let coins = level.coins.clone();
        let blocks = spawn_blocks(&level);
        Self {
            level,
            mario,
            enemies,
            blocks,
            mushrooms: Vec::new(),
            coins,
            coins_collected: 0,
            lives: 3,
            score: 0,
            timer: 400,
            timer_ticks: 0,
            camera: Camera::new(),
            animator: Animator::new(),
            deaths: 0,
            completed: false,
            bg_map: TileMap::new(w, h, cells),
            // Empty tiles render white, solid tiles dark, Mario a black block.
            bg_tiles: vec![solid_tile(0), solid_tile(2)],
            mario_tile: solid_tile(3),
            // Enemies render as a light-gray block so they stand out from both
            // the white background and the dark terrain.
            enemy_tile: solid_tile(1),
            coin_tile: coin_tile(),
            question_tile: question_tile(),
            used_tile: solid_tile(2),
            brick_tile: brick_tile(),
            mushroom_tile: mushroom_tile(),
            end_tile: end_tile(),
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
                } else if x == 4 && y == h - 3 {
                    'C'
                } else if x == 38 && y == h - 3 {
                    'E'
                } else if x == 6 && y == h - 6 {
                    '?'
                } else if x == 9 && y == h - 6 {
                    'P'
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
        // Once the level is complete, the scene freezes.
        if self.completed {
            return;
        }
        let rising = self.mario.vy < 0;
        step_motion(&mut self.mario, buttons, &self.level.solids);
        if rising {
            self.bump_blocks();
        }
        self.animator.update(&self.mario);
        for enemy in &mut self.enemies {
            update_enemy(enemy, &self.level.solids);
        }
        for mushroom in &mut self.mushrooms {
            update_mushroom(mushroom, &self.level.solids);
        }
        if self.mario.invuln > 0 {
            self.mario.invuln -= 1;
        }
        self.collect_mushrooms();
        self.resolve_interactions();
        self.collect_coins();
        self.tick_timer();
        let (lw, lh) = self.level_size();
        self.camera
            .follow(self.mario.pixel_x() + 4, self.mario.pixel_y() + 4, lw, lh);
        despawn_offscreen(&mut self.enemies, self.camera.x);

        // Reaching the end trigger completes the level.
        if let Some((ex, ey)) = self.level.end {
            let (mw, mh) = self.mario.size();
            let (ml, mt) = (self.mario.pixel_x(), self.mario.pixel_y());
            let (mr, mb) = (ml + mw - 1, mt + mh - 1);
            let (el, et, er, eb) = (ex, ey, ex + TILE - 1, ey + TILE - 1);
            if ml <= er && mr >= el && mt <= eb && mb >= et {
                self.completed = true;
                return;
            }
        }

        // Running out of time is fatal.
        if self.timer == 0 {
            self.mario.alive = false;
        }

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
            self.score += 100;
        }
        // A stomp in the same frame saves Mario from a simultaneous side hit.
        // A hit shrinks big Mario (with brief invulnerability) but kills small
        // Mario. Invulnerability frames ignore hits entirely.
        if hit && !stomped && self.mario.invuln == 0 {
            if self.mario.power == Power::Big {
                self.mario.power = Power::Small;
                self.mario.y += pixels(MUSHROOM_SIZE); // shrink, feet stay put
                self.mario.invuln = 90;
            } else {
                self.mario.alive = false;
            }
        }
    }

    /// Mario just moved up this frame. If his head is flush against an unused
    /// block, bump it. A question block gives a coin and becomes used; a brick
    /// takes the hit but does not break (small Mario).
    fn bump_blocks(&mut self) {
        let (mw, _mh) = self.mario.size();
        let ml = self.mario.pixel_x();
        let mt = self.mario.pixel_y();
        let mr = ml + mw - 1;

        let mut got_coin = false;
        let mut mushroom_at: Option<(i32, i32)> = None;
        for block in &mut self.blocks {
            if block.used {
                continue;
            }
            let (bl, _bt, br, bb) = block.edges();
            let flush_below = mt == bb + 1;
            let overlap_x = ml <= br && mr >= bl;
            if !(flush_below && overlap_x) {
                continue;
            }
            match block.kind {
                BlockKind::Question => {
                    block.used = true;
                    got_coin = true;
                }
                BlockKind::PowerUp => {
                    block.used = true;
                    // The mushroom emerges from the top of the block.
                    mushroom_at = Some((block.x, block.y - TILE));
                }
                BlockKind::Brick => {}
            }
        }
        if got_coin {
            self.gain_coin();
        }
        if let Some((x, y)) = mushroom_at {
            self.mushrooms.push(Mushroom::new(x, y, false));
        }
    }

    /// Collect any coins Mario overlaps. Every 100 coins grants a life.
    fn collect_coins(&mut self) {
        let (mw, mh) = self.mario.size();
        let ml = self.mario.pixel_x();
        let mt = self.mario.pixel_y();
        let (mr, mb) = (ml + mw - 1, mt + mh - 1);

        let mut got = 0u32;
        self.coins.retain(|&(cx, cy)| {
            let (cl, ct, cr, cb) = (cx, cy, cx + TILE - 1, cy + TILE - 1);
            let overlap = ml <= cr && mr >= cl && mt <= cb && mb >= ct;
            if overlap {
                got += 1;
                false
            } else {
                true
            }
        });
        for _ in 0..got {
            self.gain_coin();
        }
    }

    /// Pick up any mushroom Mario overlaps: small Mario grows, and either way it
    /// is worth points.
    fn collect_mushrooms(&mut self) {
        let (mw, mh) = self.mario.size();
        let ml = self.mario.pixel_x();
        let mt = self.mario.pixel_y();
        let (mr, mb) = (ml + mw - 1, mt + mh - 1);

        let mut grew = false;
        self.mushrooms.retain(|m| {
            let (el, et, er, eb) = m.edges();
            let overlap = ml <= er && mr >= el && mt <= eb && mb >= et;
            if overlap {
                grew = true;
                false
            } else {
                true
            }
        });
        if grew {
            self.grow_mario();
            self.score += 1000;
        }
    }

    /// Make small Mario big, lifting him so his feet stay on the ground. No
    /// effect if he is already big. Public so tools can capture the big state.
    pub fn grow_mario(&mut self) {
        if self.mario.power == Power::Small {
            self.mario.power = Power::Big;
            self.mario.y -= pixels(MUSHROOM_SIZE);
        }
    }

    fn gain_coin(&mut self) {
        self.coins_collected += 1;
        self.score += 100;
        if self.coins_collected >= 100 {
            self.coins_collected -= 100;
            self.lives += 1;
        }
    }

    /// Count down the level timer, one unit roughly every 24 frames.
    fn tick_timer(&mut self) {
        self.timer_ticks += 1;
        if self.timer_ticks >= 24 {
            self.timer_ticks = 0;
            self.timer = self.timer.saturating_sub(1);
        }
    }

    /// Put Mario back at the spawn and restore the enemies. Collected coins,
    /// lives, and the death count carry over. The camera snaps back on the next
    /// step via follow.
    fn respawn(&mut self) {
        self.mario = Mario::new(self.level.spawn.0, self.level.spawn.1);
        self.enemies = spawn_enemies(&self.level);
        self.animator = Animator::new();
        self.timer = 400;
        self.timer_ticks = 0;
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
        if let Some((ex, ey)) = self.level.end {
            fb.draw_tile(&self.end_tile, ex - self.camera.x, ey - self.camera.y, &self.palette);
        }
        for block in &self.blocks {
            let tile = match (block.kind, block.used) {
                (_, true) => &self.used_tile,
                (BlockKind::Question | BlockKind::PowerUp, false) => &self.question_tile,
                (BlockKind::Brick, false) => &self.brick_tile,
            };
            fb.draw_tile(tile, block.x - self.camera.x, block.y - self.camera.y, &self.palette);
        }
        for &(cx, cy) in &self.coins {
            fb.draw_tile(&self.coin_tile, cx - self.camera.x, cy - self.camera.y, &self.palette);
        }
        for mushroom in &self.mushrooms {
            fb.draw_tile(
                &self.mushroom_tile,
                mushroom.pixel_x() - self.camera.x,
                mushroom.pixel_y() - self.camera.y,
                &self.palette,
            );
        }
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
        // Flicker Mario while invulnerable. Big Mario is two tiles tall.
        let visible = self.mario.invuln == 0 || (self.mario.invuln / 4).is_multiple_of(2);
        if visible {
            let (_mw, mh) = self.mario.size();
            let mx = self.mario.pixel_x() - self.camera.x;
            let my = self.mario.pixel_y() - self.camera.y;
            let mut ty = 0;
            while ty < mh {
                fb.draw_tile(&self.mario_tile, mx, my + ty, &self.palette);
                ty += TILE;
            }
        }
        self.draw_hud(&mut fb);
        fb
    }

    /// Draw the HUD along the top: a coin icon and count, a Mario icon and life
    /// count, the score, and the timer. Numbers use the small digit font.
    fn draw_hud(&self, fb: &mut Framebuffer) {
        use crate::font::draw_number;
        use crate::render::Shade;

        // Coin icon + count.
        fb.draw_tile(&self.coin_tile, 2, 1, &self.palette);
        draw_number(fb, 11, 2, self.coins_collected, Shade::Black);
        // Mario icon + lives.
        fb.draw_tile(&self.mario_tile, 40, 1, &self.palette);
        draw_number(fb, 49, 2, self.lives, Shade::Black);
        // Score and timer as plain numbers.
        draw_number(fb, 78, 2, self.score, Shade::Black);
        draw_number(fb, 135, 2, self.timer, Shade::Black);
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
    fn mario_collects_a_coin_he_touches() {
        use crate::core::level::Level;
        // A coin sitting right where Mario spawns.
        let level = Level::from_rows(&["MC..", "####"]);
        let mut game = Game::new(level);
        assert_eq!(game.coins.len(), 1);
        // Walk right into the coin.
        for _ in 0..30 {
            game.step(held(Button::Right));
            if game.coins.is_empty() {
                break;
            }
        }
        assert!(game.coins.is_empty(), "the coin should be collected");
        assert_eq!(game.coins_collected, 1);
    }

    #[test]
    fn bumping_a_question_block_gives_a_coin_and_spends_it() {
        use crate::core::level::Level;
        // Mario a couple tiles under a question block, over a floor.
        let level = Level::from_rows(&[".?..", "....", ".M..", "####"]);
        let mut game = Game::new(level);
        assert_eq!(game.blocks.len(), 1);
        assert!(!game.blocks[0].used);

        for _ in 0..30 {
            game.step(held(Button::A)); // jump up into the block
            if game.blocks[0].used {
                break;
            }
        }
        assert!(game.blocks[0].used, "the question block should be spent");
        assert_eq!(game.coins_collected, 1, "it should give one coin");
    }

    #[test]
    fn picking_up_a_mushroom_makes_mario_big() {
        use crate::core::entity::Power;
        use crate::core::level::Level;
        use crate::core::powerup::Mushroom;

        let level = Level::from_rows(&["M...", "####"]);
        let mut game = Game::new(level);
        assert_eq!(game.mario.power, Power::Small);
        game.mushrooms.push(Mushroom::new(0, 0, false)); // right on top of Mario

        for _ in 0..30 {
            game.step(Buttons::default());
            if game.mario.power == Power::Big {
                break;
            }
        }
        assert_eq!(game.mario.power, Power::Big);
        assert!(game.mushrooms.is_empty(), "the mushroom is consumed");
    }

    #[test]
    fn power_state_machine_grows_then_shrinks() {
        use crate::core::entity::Power;
        use crate::core::level::Level;
        use crate::core::powerup::Mushroom;

        let level = Level::from_rows(&["M..G", "####"]);
        let mut game = Game::new(level);

        // Small -> Big by picking up a mushroom dropped on Mario.
        game.mushrooms
            .push(Mushroom::new(game.level.spawn.0, game.level.spawn.1, false));
        for _ in 0..10 {
            game.step(Buttons::default());
            if game.mario.power == Power::Big {
                break;
            }
        }
        assert_eq!(game.mario.power, Power::Big);

        // Big -> Small by walking into the goomba, without dying.
        for _ in 0..200 {
            game.step(held(Button::Right));
            if game.mario.power == Power::Small {
                break;
            }
        }
        assert_eq!(game.mario.power, Power::Small);
        assert!(game.mario.alive);
        assert_eq!(game.deaths, 0, "shrinking is not dying");
    }

    #[test]
    fn big_mario_shrinks_instead_of_dying_on_a_hit() {
        use crate::core::entity::{pixels, Power};
        use crate::core::level::Level;

        let level = Level::from_rows(&["M..G", "####"]);
        let mut game = Game::new(level);
        // Grow Mario the way a pickup does: bigger, and lifted so his feet stay
        // on the floor rather than embedded in it.
        game.mario.power = Power::Big;
        game.mario.y -= pixels(8);

        let mut shrank = false;
        for _ in 0..200 {
            game.step(held(Button::Right));
            if game.mario.power == Power::Small && game.mario.invuln > 0 {
                shrank = true;
                break;
            }
        }
        assert!(shrank, "big Mario should shrink, not die");
        assert!(game.mario.alive);
        assert_eq!(game.deaths, 0);
    }

    #[test]
    fn reaching_the_end_completes_the_level_and_freezes() {
        use crate::core::level::Level;
        let level = Level::from_rows(&["M..E", "####"]);
        let mut game = Game::new(level);
        assert!(!game.completed);

        for _ in 0..200 {
            game.step(held(Button::Right));
            if game.completed {
                break;
            }
        }
        assert!(game.completed, "walking into the end should complete the level");

        // The scene is frozen: further steps do nothing.
        let before = (game.mario.pixel_x(), game.timer);
        game.step(held(Button::Right));
        assert_eq!((game.mario.pixel_x(), game.timer), before);
    }

    #[test]
    fn running_out_of_time_kills_and_respawns() {
        use crate::core::level::Level;
        // A calm level with nothing to bump into.
        let level = Level::from_rows(&["M...", "####"]);
        let mut game = Game::new(level);
        game.timer = 1; // about to run out

        let mut died_to_time = false;
        for _ in 0..60 {
            game.step(Buttons::default());
            if game.deaths > 0 {
                died_to_time = true;
                break;
            }
        }
        assert!(died_to_time, "the clock hitting zero should kill Mario");
        assert!(game.mario.alive, "and then respawn him");
        assert_eq!(game.timer, 400, "the timer resets on respawn");
    }

    #[test]
    fn timer_counts_down_over_time() {
        let mut game = Game::new(Game::demo_level());
        let start = game.timer;
        for _ in 0..24 {
            game.step(Buttons::default());
        }
        assert_eq!(game.timer, start - 1);
    }

    #[test]
    fn collecting_a_coin_adds_score() {
        use crate::core::level::Level;
        let level = Level::from_rows(&["MC..", "####"]);
        let mut game = Game::new(level);
        for _ in 0..30 {
            game.step(held(Button::Right));
            if game.coins.is_empty() {
                break;
            }
        }
        assert_eq!(game.score, 100);
    }

    #[test]
    fn one_hundred_coins_grants_a_life() {
        let mut game = Game::new(Game::demo_level());
        let lives_before = game.lives;
        for _ in 0..100 {
            game.gain_coin();
        }
        assert_eq!(game.lives, lives_before + 1);
        assert_eq!(game.coins_collected, 0, "counter wraps after 100");
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
