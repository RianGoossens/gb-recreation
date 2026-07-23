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
use crate::core::physics::step_motion;
use crate::core::powerup::{update_item, Item, ItemKind, ITEM_SIZE};
use crate::core::superball::{update_superball, Superball};
use crate::tuning::Tuning;
use crate::input::{Button, Buttons};
use crate::render::{render_background, Framebuffer, Palette, TileMap};
use crate::sound::SoundEvent;
use crate::tiles::Tile;

/// How long star invincibility lasts, in frames. Provisional.
const STAR_DURATION: u32 = 600;

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
    /// Set when the last life is spent. The scene freezes.
    pub game_over: bool,
    /// Interactive blocks Mario can bump from below.
    pub blocks: Vec<Block>,
    /// Active power-up items (mushrooms, stars, flowers).
    pub items: Vec<Item>,
    /// Superballs currently in flight.
    pub superballs: Vec<Superball>,
    /// Tracks the throw button so a held press throws only once.
    throw_latched: bool,
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
    /// Tunable movement values (walk, jump, gravity, stomp, timer).
    pub tuning: Tuning,
    /// Sound events emitted this frame, for a frontend to play. Cleared each step.
    sounds: Vec<SoundEvent>,
    bg_map: TileMap,
    bg_tiles: Vec<Tile>,
    mario_tile: Tile,
    enemy_tile: Tile,
    coin_tile: Tile,
    question_tile: Tile,
    used_tile: Tile,
    brick_tile: Tile,
    mushroom_tile: Tile,
    star_tile: Tile,
    flower_tile: Tile,
    superball_tile: Tile,
    end_tile: Tile,
    palette: Palette,
}

/// A small black dot: a superball.
fn superball_tile() -> Tile {
    let mut pixels = [[0u8; 8]; 8];
    for row in pixels.iter_mut().take(5).skip(1) {
        for cell in row.iter_mut().take(5).skip(1) {
            *cell = 3;
        }
    }
    Tile { pixels }
}

/// A light block with a dark center: a star.
fn star_tile() -> Tile {
    let mut pixels = [[1u8; 8]; 8];
    for row in pixels.iter_mut().take(6).skip(2) {
        for cell in row.iter_mut().take(6).skip(2) {
            *cell = 3;
        }
    }
    Tile { pixels }
}

/// A dark block with a light cross: a flower.
fn flower_tile() -> Tile {
    let mut pixels = [[2u8; 8]; 8];
    pixels[3] = [0; 8];
    pixels[4] = [0; 8];
    for row in pixels.iter_mut() {
        row[3] = 0;
        row[4] = 0;
    }
    Tile { pixels }
}

fn spawn_items(level: &Level) -> Vec<Item> {
    level
        .items
        .iter()
        .map(|&(x, y, kind)| match kind {
            ItemKind::Mushroom => Item::mushroom(x, y, false),
            ItemKind::Star => Item::star(x, y, false),
            ItemKind::Flower => Item::flower(x, y, false),
        })
        .collect()
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
    use crate::core::enemy::EnemyKind;
    level
        .enemy_spawns
        .iter()
        .map(|&(px, py, kind)| match kind {
            EnemyKind::Goomba => Enemy::goomba(px, py, true),
            EnemyKind::Fly => Enemy::fly(px, py, true),
        })
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
        let items = spawn_items(&level);
        let tuning = Tuning::default();
        Self {
            level,
            mario,
            enemies,
            blocks,
            items,
            superballs: Vec::new(),
            throw_latched: false,
            coins,
            coins_collected: 0,
            lives: 3,
            score: 0,
            timer: tuning.timer_start,
            timer_ticks: 0,
            tuning,
            sounds: Vec::new(),
            camera: Camera::new(),
            animator: Animator::new(),
            deaths: 0,
            completed: false,
            game_over: false,
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
            star_tile: star_tile(),
            flower_tile: flower_tile(),
            superball_tile: superball_tile(),
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

    /// Sound events emitted on the most recent step, drained by a frontend.
    pub fn drain_sounds(&mut self) -> Vec<SoundEvent> {
        std::mem::take(&mut self.sounds)
    }

    /// Advance one frame from the held buttons.
    pub fn step(&mut self, buttons: Buttons) {
        self.sounds.clear();
        // Once the level is complete or Mario is out of lives, the scene freezes.
        if self.completed || self.game_over {
            return;
        }
        let rising = self.mario.vy < 0;
        let was_grounded = self.mario.on_ground;
        step_motion(&mut self.mario, buttons, &self.level.solids, &self.tuning);
        if was_grounded && !self.mario.on_ground && self.mario.vy < 0 {
            self.sounds.push(SoundEvent::Jump);
        }
        if rising {
            self.bump_blocks();
        }
        self.handle_throw(buttons);
        self.animator.update(&self.mario);
        for enemy in &mut self.enemies {
            update_enemy(enemy, &self.level.solids);
        }
        for item in &mut self.items {
            update_item(item, &self.level.solids);
        }
        self.update_superballs();
        if self.mario.invuln > 0 {
            self.mario.invuln -= 1;
        }
        if self.mario.invincible > 0 {
            self.mario.invincible -= 1;
        }
        self.collect_items();
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
                self.sounds.push(SoundEvent::LevelComplete);
                return;
            }
        }

        // Running out of time is fatal.
        if self.timer == 0 {
            self.mario.alive = false;
        }

        if !self.mario.alive {
            self.deaths += 1;
            self.sounds.push(SoundEvent::Death);
            self.lives = self.lives.saturating_sub(1);
            if self.lives == 0 {
                self.game_over = true;
                self.sounds.push(SoundEvent::GameOver);
                return;
            }
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

        let invincible = self.mario.invincible > 0;
        let mut stomped = false;
        let mut hit = false;
        let mut plowed = false;
        for enemy in &mut self.enemies {
            if !enemy.alive {
                continue;
            }
            let (el, et, er, eb) = enemy.edges();
            let overlap = ml <= er && mr >= el && mt <= eb && mb >= et;
            if !overlap {
                continue;
            }
            if invincible {
                // A star runs straight through enemies, defeating them.
                enemy.alive = false;
                plowed = true;
            } else if descending && mb <= et + ENEMY_SIZE / 2 {
                enemy.alive = false;
                stomped = true;
            } else {
                hit = true;
            }
        }
        if plowed {
            self.score += 100;
            self.sounds.push(SoundEvent::Stomp);
        }
        if stomped {
            self.mario.vy = -self.tuning.stomp_bounce;
            self.mario.on_ground = false;
            self.score += 100;
            self.sounds.push(SoundEvent::Stomp);
        }
        // A stomp in the same frame saves Mario from a simultaneous side hit.
        // A hit shrinks big Mario (with brief invulnerability) but kills small
        // Mario. Invulnerability frames ignore hits entirely.
        if hit && !stomped && self.mario.invuln == 0 {
            if self.mario.power != Power::Small {
                // Big or Fire Mario shrinks back to small instead of dying.
                self.mario.power = Power::Small;
                self.mario.y += pixels(ITEM_SIZE); // shrink, feet stay put
                self.mario.invuln = 90;
                self.sounds.push(SoundEvent::Shrink);
            } else {
                self.mario.alive = false;
            }
        }
    }

    /// Fire Mario throws a superball on a fresh press of B, up to two at a time.
    fn handle_throw(&mut self, buttons: Buttons) {
        use crate::core::entity::Facing;
        let b = buttons.is_held(Button::B);
        let pressed = b && !self.throw_latched;
        self.throw_latched = b;
        if pressed && self.mario.power == Power::Fire && self.superballs.len() < 2 {
            let going_left = self.mario.facing == Facing::Left;
            let ball = Superball::new(self.mario.pixel_x(), self.mario.pixel_y(), going_left);
            self.superballs.push(ball);
        }
    }

    /// Advance superballs, remove fizzled ones, and defeat any enemy a ball hits.
    /// A ball that connects is spent.
    fn update_superballs(&mut self) {
        let solids = &self.level.solids;
        self.superballs.retain_mut(|s| update_superball(s, solids));

        let mut spent = vec![false; self.superballs.len()];
        let mut hits = 0;
        for (i, ball) in self.superballs.iter().enumerate() {
            let (bl, bt, br, bb) = ball.edges();
            for enemy in &mut self.enemies {
                if !enemy.alive {
                    continue;
                }
                let (el, et, er, eb) = enemy.edges();
                if bl <= er && br >= el && bt <= eb && bb >= et {
                    enemy.alive = false;
                    spent[i] = true;
                    hits += 1;
                }
            }
        }
        if hits > 0 {
            self.score += 100 * hits;
            self.sounds.push(SoundEvent::Stomp);
        }
        let mut i = 0;
        self.superballs.retain(|_| {
            let keep = !spent[i];
            i += 1;
            keep
        });

        // Superballs also collect coins they pass through, without being spent.
        let ball_boxes: Vec<(i32, i32, i32, i32)> =
            self.superballs.iter().map(|b| b.edges()).collect();
        let mut coin_hits = 0;
        self.coins.retain(|&(cx, cy)| {
            let (cl, ct, cr, cb) = (cx, cy, cx + TILE - 1, cy + TILE - 1);
            let hit = ball_boxes
                .iter()
                .any(|&(bl, bt, br, bb)| bl <= cr && br >= cl && bt <= cb && bb >= ct);
            if hit {
                coin_hits += 1;
                false
            } else {
                true
            }
        });
        for _ in 0..coin_hits {
            self.gain_coin();
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

        // A power block gives a mushroom to small Mario, a flower otherwise, the
        // way the original decides an item by Mario's current size.
        let power_item = if self.mario.power == Power::Small {
            ItemKind::Mushroom
        } else {
            ItemKind::Flower
        };

        // Big or fire Mario breaks bricks; small Mario only bumps them.
        let can_break = self.mario.power != Power::Small;

        let mut got_coin = false;
        let mut item_at: Option<(i32, i32)> = None;
        let mut break_tiles: Vec<(i32, i32)> = Vec::new();
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
                    // The item emerges from the top of the block.
                    item_at = Some((block.x, block.y - TILE));
                }
                BlockKind::Brick => {
                    if can_break {
                        block.used = true;
                        break_tiles.push((block.x / TILE, block.y / TILE));
                    }
                }
            }
            self.sounds.push(SoundEvent::BlockBump);
        }
        if got_coin {
            self.gain_coin();
        }
        if let Some((x, y)) = item_at {
            let item = match power_item {
                ItemKind::Flower => Item::flower(x, y, false),
                _ => Item::mushroom(x, y, false),
            };
            self.items.push(item);
        }
        // Remove broken bricks and clear their solidity so Mario can pass.
        for &(tx, ty) in &break_tiles {
            self.level.solids.clear(tx, ty);
        }
        self.blocks.retain(|b| {
            !(b.kind == BlockKind::Brick && break_tiles.contains(&(b.x / TILE, b.y / TILE)))
        });
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

    /// Pick up any item Mario overlaps: a mushroom grows small Mario, a star
    /// grants invincibility. Either way it is worth points.
    fn collect_items(&mut self) {
        let (mw, mh) = self.mario.size();
        let ml = self.mario.pixel_x();
        let mt = self.mario.pixel_y();
        let (mr, mb) = (ml + mw - 1, mt + mh - 1);

        let mut picked = Vec::new();
        self.items.retain(|item| {
            let (el, et, er, eb) = item.edges();
            let overlap = ml <= er && mr >= el && mt <= eb && mb >= et;
            if overlap {
                picked.push(item.kind);
                false
            } else {
                true
            }
        });
        for kind in picked {
            match kind {
                ItemKind::Mushroom => self.grow_mario(),
                ItemKind::Star => self.mario.invincible = STAR_DURATION,
                ItemKind::Flower => {
                    // A flower makes Mario fire-powered, growing him if small.
                    self.grow_mario();
                    self.mario.power = Power::Fire;
                }
            }
            self.score += 1000;
            self.sounds.push(SoundEvent::PowerUp);
        }
    }

    /// Make small Mario big, lifting him so his feet stay on the ground. No
    /// effect if he is already big. Public so tools can capture the big state.
    pub fn grow_mario(&mut self) {
        if self.mario.power == Power::Small {
            self.mario.power = Power::Big;
            self.mario.y -= pixels(ITEM_SIZE);
        }
    }

    fn gain_coin(&mut self) {
        self.coins_collected += 1;
        self.score += 100;
        self.sounds.push(SoundEvent::Coin);
        if self.coins_collected >= 100 {
            self.coins_collected -= 100;
            self.lives += 1;
            self.sounds.push(SoundEvent::OneUp);
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
        self.camera = Camera::new(); // the one-way view resets to the spawn
        self.timer = self.tuning.timer_start;
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
        for item in &self.items {
            let tile = match item.kind {
                ItemKind::Mushroom => &self.mushroom_tile,
                ItemKind::Star => &self.star_tile,
                ItemKind::Flower => &self.flower_tile,
            };
            fb.draw_tile(
                tile,
                item.pixel_x() - self.camera.x,
                item.pixel_y() - self.camera.y,
                &self.palette,
            );
        }
        for ball in &self.superballs {
            fb.draw_tile(
                &self.superball_tile,
                ball.pixel_x() - self.camera.x,
                ball.pixel_y() - self.camera.y,
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
        let flicker = self.mario.invuln.max(self.mario.invincible);
        let visible = flicker == 0 || (flicker / 4).is_multiple_of(2);
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
        use crate::core::powerup::Item;

        let level = Level::from_rows(&["M...", "####"]);
        let mut game = Game::new(level);
        assert_eq!(game.mario.power, Power::Small);
        game.items.push(Item::mushroom(0, 0, false)); // right on top of Mario

        for _ in 0..30 {
            game.step(Buttons::default());
            if game.mario.power == Power::Big {
                break;
            }
        }
        assert_eq!(game.mario.power, Power::Big);
        assert!(game.items.is_empty(), "the mushroom is consumed");
    }

    #[test]
    fn power_state_machine_grows_then_shrinks() {
        use crate::core::entity::Power;
        use crate::core::level::Level;
        use crate::core::powerup::Item;

        let level = Level::from_rows(&["M..G", "####"]);
        let mut game = Game::new(level);

        // Small -> Big by picking up a mushroom dropped on Mario.
        game.items
            .push(Item::mushroom(game.level.spawn.0, game.level.spawn.1, false));
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
    fn a_star_makes_mario_invincible_and_plows_through_enemies() {
        use crate::core::level::Level;
        use crate::core::powerup::Item;

        // Mario, a goomba to his right, on a flat floor.
        let level = Level::from_rows(&["M...G", "#####"]);
        let mut game = Game::new(level);
        // Drop a star on Mario so he grabs it right away.
        game.items
            .push(Item::star(game.level.spawn.0, game.level.spawn.1, false));
        for _ in 0..5 {
            game.step(Buttons::default());
            if game.mario.invincible > 0 {
                break;
            }
        }
        assert!(game.mario.invincible > 0, "the star grants invincibility");

        // Walk into the goomba: it dies, Mario lives and does not shrink.
        for _ in 0..200 {
            game.step(held(Button::Right));
            if game.enemies.is_empty() {
                break;
            }
        }
        assert!(game.enemies.is_empty(), "the star defeats the enemy on contact");
        assert!(game.mario.alive);
        assert_eq!(game.deaths, 0);
    }

    #[test]
    fn fire_mario_throws_a_superball_that_defeats_an_enemy() {
        use crate::core::entity::{pixels, Power};
        use crate::core::level::Level;

        // Fire Mario facing a goomba a few tiles to his right.
        let level = Level::from_rows(&["M....G", "######"]);
        let mut game = Game::new(level);
        game.mario.power = Power::Fire;
        game.mario.y -= pixels(8);
        game.mario.facing = crate::core::entity::Facing::Right;

        // Throw (press B once).
        game.step(held(Button::B));
        assert!(!game.superballs.is_empty(), "pressing B throws a superball");

        // Let the ball travel into the goomba.
        for _ in 0..120 {
            game.step(Buttons::default());
            if game.enemies.is_empty() {
                break;
            }
        }
        assert!(game.enemies.is_empty(), "the superball should defeat the enemy");
    }

    #[test]
    fn a_superball_collects_a_coin_in_its_path() {
        use crate::core::entity::{pixels, Power};
        use crate::core::level::Level;

        let mut game = Game::new(Level::from_rows(&["M.C.", "####"]));
        game.mario.power = Power::Fire;
        game.mario.y -= pixels(8);

        game.step(held(Button::B)); // throw
        for _ in 0..40 {
            game.step(Buttons::default());
            if game.coins_collected > 0 {
                break;
            }
        }
        assert_eq!(game.coins_collected, 1, "the superball should collect the coin");
        assert!(game.coins.is_empty());
    }

    #[test]
    fn a_flower_makes_mario_fire_powered() {
        use crate::core::entity::Power;
        use crate::core::level::Level;
        use crate::core::powerup::Item;

        let level = Level::from_rows(&["M...", "####"]);
        let mut game = Game::new(level);
        game.items
            .push(Item::flower(game.level.spawn.0, game.level.spawn.1, false));
        for _ in 0..10 {
            game.step(Buttons::default());
            if game.mario.power == Power::Fire {
                break;
            }
        }
        assert_eq!(game.mario.power, Power::Fire);
    }

    #[test]
    fn a_hit_shrinks_fire_mario_back_to_small() {
        use crate::core::entity::{pixels, Power};
        use crate::core::level::Level;

        let level = Level::from_rows(&["M..G", "####"]);
        let mut game = Game::new(level);
        game.mario.power = Power::Fire;
        game.mario.y -= pixels(8); // stand properly as the taller sprite

        for _ in 0..200 {
            game.step(held(Button::Right));
            if game.mario.power == Power::Small {
                break;
            }
        }
        assert_eq!(game.mario.power, Power::Small);
        assert!(game.mario.alive);
        assert_eq!(game.deaths, 0);
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
    fn losing_all_lives_ends_the_game() {
        use crate::core::level::Level;
        // Mario walks straight into a goomba, dies, respawns, repeats.
        let level = Level::from_rows(&["M.G", "###"]);
        let mut game = Game::new(level);
        let starting_lives = game.lives;

        for _ in 0..2000 {
            game.step(held(Button::Right));
            if game.game_over {
                break;
            }
        }
        assert!(game.game_over, "running out of lives ends the game");
        assert_eq!(game.lives, 0);
        assert_eq!(game.deaths, starting_lives, "one death per life");

        // The scene is frozen once it is over.
        let before = game.mario.pixel_x();
        game.step(held(Button::Right));
        assert_eq!(game.mario.pixel_x(), before);
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
    fn tuning_changes_how_high_mario_jumps() {
        use crate::core::level::Level;
        let level = || Level::from_rows(&["....", "....", "....", "M...", "####"]);

        // Default jump.
        let mut normal = Game::new(level());
        let mut normal_apex = normal.mario.pixel_y();
        for _ in 0..40 {
            normal.step(held(Button::A));
            normal_apex = normal_apex.min(normal.mario.pixel_y());
        }

        // A stronger jump via tuning should reach higher (a smaller y).
        let mut floaty = Game::new(level());
        floaty.tuning.jump_velocity = 1100;
        let mut floaty_apex = floaty.mario.pixel_y();
        for _ in 0..40 {
            floaty.step(held(Button::A));
            floaty_apex = floaty_apex.min(floaty.mario.pixel_y());
        }

        assert!(floaty_apex < normal_apex, "a bigger jump_velocity jumps higher");
    }

    #[test]
    fn jumping_emits_a_jump_sound() {
        use crate::sound::SoundEvent;
        let (mut game, _) = resting_game();
        // Step until grounded, draining sounds, then jump.
        for _ in 0..60 {
            game.step(Buttons::default());
            let _ = game.drain_sounds();
        }
        game.step(held(Button::A));
        assert!(game.drain_sounds().contains(&SoundEvent::Jump));
    }

    #[test]
    fn collecting_a_coin_emits_a_coin_sound() {
        use crate::core::level::Level;
        use crate::sound::SoundEvent;
        let level = Level::from_rows(&["MC..", "####"]);
        let mut game = Game::new(level);
        let mut heard = false;
        for _ in 0..30 {
            game.step(held(Button::Right));
            if game.drain_sounds().contains(&SoundEvent::Coin) {
                heard = true;
                break;
            }
        }
        assert!(heard, "collecting a coin should emit a Coin sound");
    }

    #[test]
    fn dying_emits_a_death_sound() {
        use crate::core::level::Level;
        use crate::sound::SoundEvent;
        let level = Level::from_rows(&["M.G", "###"]);
        let mut game = Game::new(level);
        let mut heard = false;
        for _ in 0..300 {
            game.step(held(Button::Right));
            if game.drain_sounds().contains(&SoundEvent::Death) {
                heard = true;
                break;
            }
        }
        assert!(heard, "a fatal hit should emit a Death sound");
    }

    /// A flat level with Mario, settled onto the floor.
    fn resting_game() -> (Game, ()) {
        use crate::core::level::Level;
        let mut game = Game::new(Level::from_rows(&["M...", "####"]));
        for _ in 0..30 {
            game.step(Buttons::default());
        }
        (game, ())
    }

    #[test]
    fn big_mario_breaks_a_brick_small_mario_does_not() {
        use crate::core::entity::{pixels, Power};
        use crate::core::level::Level;

        let rows = &[".B..", "....", "....", ".M..", "####"];

        // Small Mario: the brick survives.
        let mut small = Game::new(Level::from_rows(rows));
        for _ in 0..40 {
            small.step(held(Button::A));
        }
        assert_eq!(small.blocks.len(), 1, "small Mario does not break the brick");

        // Big Mario: the brick breaks and its tile stops being solid.
        let mut big = Game::new(Level::from_rows(rows));
        big.mario.power = Power::Big;
        big.mario.y -= pixels(8);
        for _ in 0..40 {
            big.step(held(Button::A));
            if big.blocks.is_empty() {
                break;
            }
        }
        assert!(big.blocks.is_empty(), "big Mario breaks the brick");
        assert!(!big.level.solids.is_solid(1, 0), "the broken tile is no longer solid");
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
