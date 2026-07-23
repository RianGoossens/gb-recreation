//! Data-driven tuning of Mario's feel.
//!
//! The numbers that decide how Mario moves live here rather than as hard
//! constants baked into the physics. A [`Tuning`] can be built from a small
//! `key = value` text block, so a level pack can retune the game without a
//! recompile. The defaults are the values pinned in the physics module.

use crate::core::physics::{
    FRICTION, GRAVITY, JUMP_CUT, JUMP_VELOCITY, MAX_FALL_SPEED, MAX_WALK_SPEED, STOMP_BOUNCE,
    WALK_ACCEL,
};

/// The starting value of the level timer.
pub const DEFAULT_TIMER_START: u32 = 400;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tuning {
    pub walk_accel: i32,
    pub friction: i32,
    pub max_walk_speed: i32,
    pub gravity: i32,
    pub max_fall_speed: i32,
    pub jump_velocity: i32,
    pub jump_cut: i32,
    pub stomp_bounce: i32,
    pub timer_start: u32,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            walk_accel: WALK_ACCEL,
            friction: FRICTION,
            max_walk_speed: MAX_WALK_SPEED,
            gravity: GRAVITY,
            max_fall_speed: MAX_FALL_SPEED,
            jump_velocity: JUMP_VELOCITY,
            jump_cut: JUMP_CUT,
            stomp_bounce: STOMP_BOUNCE,
            timer_start: DEFAULT_TIMER_START,
        }
    }
}

impl Tuning {
    /// Parse tuning overrides from `key = value` lines. Blank lines and lines
    /// starting with `#` are ignored. Unset keys keep their default. An unknown
    /// key or unparseable value is an error, so a typo is reported.
    pub fn from_text(text: &str) -> Result<Self, String> {
        let mut t = Self::default();
        for (n, raw) in text.lines().enumerate() {
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let (key, value) = line
                .split_once('=')
                .ok_or_else(|| format!("line {}: expected key = value", n + 1))?;
            let (key, value) = (key.trim(), value.trim());
            let parse_i = || value.parse::<i32>().map_err(|_| format!("line {}: bad number '{value}'", n + 1));
            let parse_u = || value.parse::<u32>().map_err(|_| format!("line {}: bad number '{value}'", n + 1));
            match key {
                "walk_accel" => t.walk_accel = parse_i()?,
                "friction" => t.friction = parse_i()?,
                "max_walk_speed" => t.max_walk_speed = parse_i()?,
                "gravity" => t.gravity = parse_i()?,
                "max_fall_speed" => t.max_fall_speed = parse_i()?,
                "jump_velocity" => t.jump_velocity = parse_i()?,
                "jump_cut" => t.jump_cut = parse_i()?,
                "stomp_bounce" => t.stomp_bounce = parse_i()?,
                "timer_start" => t.timer_start = parse_u()?,
                other => return Err(format!("line {}: unknown key '{other}'", n + 1)),
            }
        }
        Ok(t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_the_pinned_constants() {
        let t = Tuning::default();
        assert_eq!(t.jump_velocity, JUMP_VELOCITY);
        assert_eq!(t.gravity, GRAVITY);
        assert_eq!(t.timer_start, DEFAULT_TIMER_START);
    }

    #[test]
    fn from_text_overrides_only_the_given_keys() {
        let t = Tuning::from_text("# a higher jump\njump_velocity = 900\ntimer_start = 300\n").unwrap();
        assert_eq!(t.jump_velocity, 900);
        assert_eq!(t.timer_start, 300);
        // Untouched keys keep their defaults.
        assert_eq!(t.gravity, GRAVITY);
    }

    #[test]
    fn from_text_ignores_comments_and_blanks() {
        let t = Tuning::from_text("\n  # just a comment\n\ngravity=50  # inline comment\n").unwrap();
        assert_eq!(t.gravity, 50);
    }

    #[test]
    fn from_text_reports_unknown_keys() {
        let err = Tuning::from_text("wobble = 3").unwrap_err();
        assert!(err.contains("unknown key"), "{err}");
    }
}
