//! Enemies: the framework for things that move and can be defeated.
//!
//! An enemy walks, falls under gravity, and collides with the world much like
//! Mario, but with simpler behavior. This module owns the shared parts: the
//! entity, one physics step (walk plus falling plus collision), and despawning
//! enemies that have scrolled off screen. Per-type quirks (like a hop)
//! build on top of this.

use crate::core::level::{Solids, TILE};
use crate::core::physics::GRAVITY;
use crate::SCREEN_WIDTH;

/// Enemies are one tile square. This is the body that walks into walls and
/// stands on floors.
///
/// Half measured. Walking a Chibibo into a wall written into the cartridge's
/// own tilemap stops it with its slot x four pixels inside the wall's column,
/// at two placements sixteen pixels apart (`tools/measure_enemy_body.py`), so
/// the edge the terrain stops on that side is three pixels inside its eight
/// pixel drawing rather than at its left column. The other edge needs an
/// object walking right into a wall and is not measured, so the 8 stands: a
/// single point tested there and a box whose left edge is there both produce
/// the same reading.
pub const ENEMY_SIZE: i32 = 8;
/// How much of an enemy hurts Mario, which is smaller than its body and
/// smaller than its drawing.
///
/// Measured by holding Mario at every offset from a Chibibo a pixel at a time
/// and watching the life counter (`tools/measure_enemy_box.py`). Both of them
/// have to be written every frame: a walker covers more than a screen in the
/// 212 frames a death takes to register, so left alone it reaches Mario from
/// any starting offset and every trial reports a hit.
///
/// Contact runs over a window of 15 across and 16 down. Mario's own box is 11
/// by 12, measured on the cartridge by an unrelated route, and 15 - 11 + 1 and
/// 16 - 12 + 1 both give 5. The two axes agreeing is the check: a wrong Mario
/// box would have to be wrong by the same amount on both to land there.
pub const ENEMY_CONTACT: i32 = 5;
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
/// Upward speed a Bouncer gets on each hop. Provisional, and ours.
pub const HOP_VELOCITY: i32 = 520;
/// Frames a Bouncer waits on the ground between hops. Ours.
pub const HOP_INTERVAL: u32 = 40;

/// The cartridge's jumper (object kind `0x0E`) runs on a fixed cycle of 102
/// frames, traced with the camera held still (`tools/trace_jumper.py`). It
/// moves on every third frame of the first 48 and then stands perfectly still
/// for the other 54.
pub const HOP_CYCLE: u32 = 102;
/// Frames between position updates while it is in the air.
pub const HOP_STEP_FRAMES: u32 = 3;
/// Pixels it covers sideways on each update, so 32 px over a whole hop.
pub const HOP_STEP: i32 = 2;
/// How far it rises on each of the sixteen updates of a hop. Read straight off
/// the trace rather than fitted: the shape is not constant deceleration, so it
/// is a table in the cartridge and a table here.
pub const HOP_RISE: [i32; 16] = [4, 4, 2, 2, 1, 1, 1, 0, 0, -1, -1, -1, -2, -2, -4, -4];
/// Frames of a cycle spent in the air.
pub const HOP_FRAMES: u32 = HOP_RISE.len() as u32 * HOP_STEP_FRAMES;
/// The peak of the arc, which is what the table sums to at its highest point.
pub const HOP_HEIGHT: i32 = 15;

/// Frames World 1-3's faller (object kind `0x0C`) waits before it drops.
///
/// Measured from the frame the game creates the object, not the frame it
/// scrolls into view, since it is created off the right of the screen
/// (`tools/probe_faller_trigger.py`). The same 175 comes back with Mario
/// directly under it, a screen away, and at the left edge, so it is a timer
/// rather than something he sets off.
pub const FALL_DELAY: u32 = 175;

/// Frames Bunbun (object kind `0x42`) spends moving in each cycle, one pixel
/// a frame. Traced with the camera held still (`tools/measure_flyer.py`).
pub const FLIGHT_FRAMES: u32 = 41;
/// Frames it holds still between bursts.
pub const FLIGHT_HOLD: u32 = 33;
/// The whole cycle, so 41 pixels every 74 frames.
pub const FLIGHT_CYCLE: u32 = FLIGHT_FRAMES + FLIGHT_HOLD;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnemyKind {
    /// Walks along the ground, turns at walls, and walks off ledges.
    Goomba,
    /// Walks the same way but turns at a ledge instead of walking off it.
    ///
    /// A separate kind because the cartridge has both. Writing a wall and then
    /// a pit into the tilemap in front of each (`tools/probe_walker_turn.py`)
    /// gets object kind `0x00` to turn at the wall and fall into the pit, and
    /// kind `0x04` to turn at both. They share a walk speed exactly.
    LedgeTurner,
    /// Walks but hops on a timer, so it does not respect ledges.
    ///
    /// Ours, not the cartridge's. It predates any of the object list work and
    /// is dropped by [`Level::without_non_canonical`], so the default game
    /// never sees one. It keeps the marker `F` so a custom level written
    /// against it still loads.
    ///
    /// [`Level::without_non_canonical`]: crate::core::level::Level::without_non_canonical
    Bouncer,
    /// World 1-3's faller: holds its position for 175 frames and then drops
    /// straight down at a pixel a frame. It never moves sideways. Its contact
    /// sweep matches a walker's exactly, so it hurts Mario from every side but
    /// a stomp (`tools/probe_object_contact.py`).
    Faller,
    /// The cartridge's jumper: still for 54 frames, then a 32 px hop 15 px
    /// high over 48. It turns at a wall and hops straight off a ledge, both
    /// settled against a control run with no obstacle
    /// (`tools/probe_walker_turn.py`), which a hopper needs because it
    /// reverses and falls on its own anyway.
    ///
    /// Named for what the cartridge calls object kind `0x0E`. It is drawn from
    /// atlas tiles `0xA0`, `0xA1`, `0xB0`, `0xB1`, measured on the running
    /// game (`docs/reference/sprites.md`).
    Fly,
    /// Bunbun: flies level, in bursts, and ignores everything.
    ///
    /// One pixel a frame for 41 frames, then 33 frames still, repeating. It
    /// never reverses and never changes height, and the trace is the same
    /// with Mario pinned high, pinned low, or moved to its far side, so
    /// nothing about it follows him (`tools/measure_flyer.py`). On contact it
    /// is an ordinary enemy: it hurts from every side but a stomp, the same
    /// six lines as the walker that is that probe's positive control.
    ///
    /// World 1-2 carries 19, more of one kind than any other World 1 level.
    Bunbun,
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
    /// Countdown to the next hop, for a Bouncer. Ignored by other kinds.
    pub hop_timer: u32,
    /// Frames into the current cycle, for a Fly. Ignored by other kinds.
    pub phase: u32,
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
            phase: 0,
        }
    }

    /// A Goomba at a whole-pixel position, walking left or right.
    pub fn goomba(pixel_x: i32, pixel_y: i32, going_left: bool) -> Self {
        Self::new(pixel_x, pixel_y, going_left, EnemyKind::Goomba)
    }

    /// The walker that turns at a ledge rather than walking off it.
    pub fn ledge_turner(pixel_x: i32, pixel_y: i32, going_left: bool) -> Self {
        Self::new(pixel_x, pixel_y, going_left, EnemyKind::LedgeTurner)
    }

    /// A Bouncer at a whole-pixel position: it walks and hops.
    pub fn bouncer(pixel_x: i32, pixel_y: i32, going_left: bool) -> Self {
        Self::new(pixel_x, pixel_y, going_left, EnemyKind::Bouncer)
    }

    /// World 1-3's faller, at the start of its wait.
    pub fn faller(pixel_x: i32, pixel_y: i32) -> Self {
        let mut faller = Self::new(pixel_x, pixel_y, true, EnemyKind::Faller);
        faller.vx = 0;
        faller
    }

    /// The cartridge's jumper, resting at the start of its cycle.
    pub fn fly(pixel_x: i32, pixel_y: i32, going_left: bool) -> Self {
        let mut hopper = Self::new(pixel_x, pixel_y, going_left, EnemyKind::Fly);
        // The still half of the cycle comes second in the trace, but starting
        // there gives it a moment on the ground before its first hop.
        hopper.phase = HOP_FRAMES;
        hopper
    }

    /// Bunbun, at the start of a burst. Every one traced set off left.
    pub fn bunbun(pixel_x: i32, pixel_y: i32) -> Self {
        Self::new(pixel_x, pixel_y, true, EnemyKind::Bunbun)
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

    /// The smaller box that decides whether Mario is hurt or stomps, centred
    /// in the body. See [`ENEMY_CONTACT`].
    pub fn contact_edges(&self) -> (i32, i32, i32, i32) {
        let inset = (ENEMY_SIZE - ENEMY_CONTACT) / 2;
        let l = self.pixel_x() + inset;
        let t = self.pixel_y() + inset;
        (l, t, l + ENEMY_CONTACT - 1, t + ENEMY_CONTACT - 1)
    }
}

/// One position update of a hop: sideways, then the table's rise or fall.
fn step_hop(enemy: &mut Enemy, solids: &Solids, rise: i32) {
    use crate::core::entity::pixels;

    let dx = if enemy.vx < 0 { -HOP_STEP } else { HOP_STEP };
    enemy.x += pixels(dx);
    let (l, t, r, b) = enemy.edges();
    if dx > 0 && solids.rect_hits_solid(r, t, r, b) {
        enemy.x = pixels(r.div_euclid(TILE) * TILE - ENEMY_SIZE);
        enemy.vx = -enemy.vx;
    } else if dx < 0 && solids.rect_hits_solid(l, t, l, b) {
        enemy.x = pixels(l.div_euclid(TILE) * TILE + TILE);
        enemy.vx = -enemy.vx;
    }

    enemy.y -= pixels(rise);
    if rise < 0 {
        let (l, _t, r, b) = enemy.edges();
        if solids.rect_hits_solid(l, b, r, b) {
            enemy.y = pixels(b.div_euclid(TILE) * TILE - ENEMY_SIZE);
        }
    }
}

/// Advance the cartridge's jumper a frame.
///
/// It ignores the velocity model the walkers use. The trace shows a fixed
/// table of rises applied every third frame; running it off a counter is what
/// reproduces the arc.
fn update_hopper(enemy: &mut Enemy, solids: &Solids) {
    if enemy.phase < HOP_FRAMES {
        if enemy.phase.is_multiple_of(HOP_STEP_FRAMES) {
            let step = (enemy.phase / HOP_STEP_FRAMES) as usize;
            step_hop(enemy, solids, HOP_RISE[step]);
        }
    } else {
        // Standing still, unless it came down over a pit, in which case it
        // keeps going the way any other enemy does.
        let (l, _t, r, b) = enemy.edges();
        if !solids.rect_hits_solid(l, b + 1, r, b + 1) {
            enemy.y += ENEMY_FALL_SPEED;
        }
    }
    enemy.phase = (enemy.phase + 1) % HOP_CYCLE;
    let (l, _t, r, b) = enemy.edges();
    enemy.on_ground = solids.rect_hits_solid(l, b + 1, r, b + 1);
}

/// Advance one enemy a frame: walk, reverse at walls, fall, land on floors.
pub fn update_enemy(enemy: &mut Enemy, solids: &Solids) {
    use crate::core::entity::pixels;
    if !enemy.alive {
        return;
    }
    if enemy.kind == EnemyKind::Fly {
        update_hopper(enemy, solids);
        return;
    }
    // Bunbun flies through everything it crosses. Whether terrain stops it is
    // not measured: the rows it flew along in World 1-2 had nothing solid on
    // them (docs/reference/faithfulness.md).
    if enemy.kind == EnemyKind::Bunbun {
        if enemy.phase < FLIGHT_FRAMES {
            enemy.x -= crate::core::entity::SUBPIXEL;
        }
        enemy.phase = (enemy.phase + 1) % FLIGHT_CYCLE;
        return;
    }
    // The faller holds still, then drops. Once the wait is over the shared
    // path below does the rest, since its horizontal speed is zero.
    if enemy.kind == EnemyKind::Faller && enemy.phase < FALL_DELAY {
        enemy.phase += 1;
        return;
    }

    // One of the two ground walkers stops at a ledge. Probe the ground just
    // past its leading foot and turn before stepping off.
    if enemy.on_ground && enemy.kind == EnemyKind::LedgeTurner && enemy.vx != 0 {
        let (l, _t, r, b) = enemy.edges();
        let ahead = if enemy.vx > 0 { r + 1 } else { l - 1 };
        if !solids.rect_hits_solid(ahead, b + 1, ahead, b + 1) {
            enemy.vx = -enemy.vx;
        }
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

    // A Bouncer hops on a timer while it is on the ground.
    if enemy.kind == EnemyKind::Bouncer && enemy.on_ground {
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

    /// The two windows measured on the cartridge with
    /// `tools/measure_enemy_box.py`: Mario is hurt across 15 pixels of
    /// horizontal offset and 16 of vertical. Those are what fix
    /// `ENEMY_CONTACT` at 5, given his own box of 11 by 12, and the engine has
    /// to reproduce them or the constant is decoration.
    #[test]
    fn contact_covers_the_window_the_cartridge_covers() {
        let enemy = Enemy::goomba(100, 100, true);
        let (el, et, er, eb) = enemy.contact_edges();
        let (mw, mh) = (11, 12);

        let across = (60..160)
            .filter(|&mx| mx <= er && mx + mw > el)
            .count();
        let down = (60..160)
            .filter(|&my| my <= eb && my + mh > et)
            .count();
        assert_eq!(across, 15, "horizontal contact window");
        assert_eq!(down, 16, "vertical contact window");
    }
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
    fn a_ledge_turner_stays_on_its_platform() {
        // A short platform (tiles 5..9 on the floor row) with empty space
        // beyond, so a walker that ignores the edge leaves it.
        let mut floor_row = ".".repeat(20);
        floor_row.replace_range(5..10, "#####");
        let solids = Solids::from_rows(&[
            &".".repeat(20),
            &".".repeat(20),
            &".".repeat(20),
            &floor_row,
        ]);
        let mut e = Enemy::ledge_turner(48, 16, false);
        e.on_ground = true;
        for _ in 0..300 {
            update_enemy(&mut e, &solids);
        }
        let (l, _t, r, b) = e.edges();
        assert!(e.on_ground, "it should never have left the platform");
        assert!(solids.rect_hits_solid(l, b + 1, r, b + 1));
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
        let mut f = Enemy::bouncer(40, 16, false);
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
        assert!(rose, "a Bouncer should hop above its resting height");
    }

    /// The whole traced cycle, checked against the numbers the cartridge gave.
    #[test]
    fn a_hopper_reproduces_the_traced_arc() {
        let solids = floor();
        let mut h = Enemy::fly(80, 16, true);
        update_enemy(&mut h, &solids);
        assert!(h.on_ground);
        let (rest_x, rest_y) = (h.pixel_x(), h.pixel_y());

        let mut peak = rest_y;
        let mut airborne = 0;
        for _ in 0..HOP_CYCLE {
            update_enemy(&mut h, &solids);
            peak = peak.min(h.pixel_y());
            if h.pixel_y() < rest_y {
                airborne += 1;
            }
        }
        assert_eq!(rest_y - peak, HOP_HEIGHT, "15 px high");
        assert_eq!(rest_x - h.pixel_x(), 32, "32 px sideways per hop");
        assert_eq!(h.pixel_y(), rest_y, "and back on the ground");
        // 48 frames of the cycle move it; the first update leaves the ground
        // and the last one puts it back, so 45 are spent above the row.
        assert_eq!(airborne, 45);
    }

    #[test]
    fn a_hopper_stands_perfectly_still_between_hops() {
        let solids = floor();
        let mut h = Enemy::fly(80, 16, true);
        for _ in 0..2 {
            update_enemy(&mut h, &solids);
        }
        let at = (h.pixel_x(), h.pixel_y());
        // It starts in the still half of the cycle, so nothing should move
        // until the hop comes round.
        for _ in 0..(HOP_CYCLE - HOP_FRAMES - 4) {
            update_enemy(&mut h, &solids);
            assert_eq!((h.pixel_x(), h.pixel_y()), at);
        }
    }

    #[test]
    fn a_hopper_turns_at_a_wall() {
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

        let mut h = Enemy::fly(40, 16, false); // hopping toward the wall
        let mut reversed = false;
        for _ in 0..(HOP_CYCLE * 3) {
            update_enemy(&mut h, &solids);
            if h.vx < 0 {
                reversed = true;
            }
            assert!(h.pixel_x() <= 48, "it must never pass the wall at x=56");
        }
        assert!(reversed);
    }

    #[test]
    fn a_hopper_goes_off_a_ledge_rather_than_turning() {
        let mut floor_row = ".".repeat(20);
        floor_row.replace_range(5..10, "#####");
        let solids = Solids::from_rows(&[
            &".".repeat(20),
            &".".repeat(20),
            &".".repeat(20),
            &floor_row,
        ]);
        let mut h = Enemy::fly(48, 16, false);
        let start_y = h.pixel_y();
        for _ in 0..(HOP_CYCLE * 3) {
            update_enemy(&mut h, &solids);
        }
        assert!(h.pixel_x() >= 80, "it should have hopped past the ledge");
        assert!(h.pixel_y() > start_y, "and be on its way down");
    }

    #[test]
    fn a_faller_holds_still_and_then_drops() {
        let solids = floor();
        let mut f = Enemy::faller(40, 0);
        let start = (f.pixel_x(), f.pixel_y());
        for _ in 0..FALL_DELAY {
            update_enemy(&mut f, &solids);
            assert_eq!((f.pixel_x(), f.pixel_y()), start, "it waits where it is");
        }
        for step in 1..=8 {
            update_enemy(&mut f, &solids);
            assert_eq!(f.pixel_x(), start.0, "and never moves sideways");
            assert_eq!(f.pixel_y(), start.1 + step, "one pixel down per frame");
        }
    }

    #[test]
    fn a_faller_comes_to_rest_on_a_floor() {
        let solids = floor();
        let mut f = Enemy::faller(40, 0);
        for _ in 0..(FALL_DELAY as usize + 200) {
            update_enemy(&mut f, &solids);
        }
        assert_eq!(f.pixel_y(), 16, "floor top is y=24 and it is 8 tall");
        assert!(f.on_ground);
    }

    #[test]
    fn bunbun_flies_in_bursts_the_length_the_trace_gives() {
        let solids = floor();
        let mut b = Enemy::bunbun(120, 40);
        let start = (b.pixel_x(), b.pixel_y());
        for step in 1..=FLIGHT_FRAMES {
            update_enemy(&mut b, &solids);
            assert_eq!(b.pixel_x(), start.0 - step as i32, "a pixel a frame, left");
            assert_eq!(b.pixel_y(), start.1, "and never a pixel of height");
        }
        let held = b.pixel_x();
        for _ in 0..FLIGHT_HOLD {
            update_enemy(&mut b, &solids);
            assert_eq!(b.pixel_x(), held, "then it holds still");
        }
        update_enemy(&mut b, &solids);
        assert_eq!(b.pixel_x(), held - 1, "and the next burst starts");
    }

    #[test]
    fn bunbun_covers_forty_one_pixels_every_seventy_four_frames() {
        let solids = floor();
        let mut b = Enemy::bunbun(400, 40);
        let start = b.pixel_x();
        for _ in 0..(FLIGHT_CYCLE * 4) {
            update_enemy(&mut b, &solids);
        }
        assert_eq!(start - b.pixel_x(), 4 * FLIGHT_FRAMES as i32);
    }

    #[test]
    fn bunbun_flies_through_the_floor_it_meets() {
        // The cartridge was never seen crossing terrain, so ours passes
        // through rather than inventing a collision (docs/reference/faithfulness.md).
        let solids = floor();
        let mut b = Enemy::bunbun(40, 24);
        for _ in 0..FLIGHT_FRAMES {
            update_enemy(&mut b, &solids);
        }
        assert_eq!(b.pixel_y(), 24, "the floor at y=24 does not hold it up");
        assert_eq!(b.pixel_x(), 40 - FLIGHT_FRAMES as i32, "or stop it");
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
