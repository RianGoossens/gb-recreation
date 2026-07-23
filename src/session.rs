//! Session flow: playing a sequence of levels end to end.
//!
//! A single [`crate::game::Game`] is one level. A [`Session`] strings levels
//! together: advance to the next when one is completed, carry the running totals
//! forward, win after the last, and end the run when the lives are gone. The
//! title and restart wiring builds on top of this.

use crate::core::level::Level;
use crate::game::Game;
use crate::input::Buttons;
use crate::render::Framebuffer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Playing,
    GameOver,
    Win,
}

pub struct Session {
    levels: Vec<Level>,
    current: usize,
    pub game: Game,
    pub phase: Phase,
}

impl Session {
    /// Start a session on the first of `levels`. There must be at least one.
    pub fn new(levels: Vec<Level>) -> Self {
        assert!(!levels.is_empty(), "a session needs at least one level");
        let game = Game::new(levels[0].clone());
        Self {
            levels,
            current: 0,
            game,
            phase: Phase::Playing,
        }
    }

    /// Which level index is being played (0-based).
    pub fn current_level(&self) -> usize {
        self.current
    }

    /// Advance one frame. While playing, step the level and react to it ending:
    /// completing advances (or wins on the last level), running out of lives
    /// ends the run. Other phases hold until the flow wiring moves them.
    pub fn step(&mut self, buttons: Buttons) {
        if self.phase != Phase::Playing {
            return;
        }
        self.game.step(buttons);
        if self.game.completed {
            self.advance();
        } else if self.game.game_over {
            self.phase = Phase::GameOver;
        }
    }

    fn advance(&mut self) {
        self.current += 1;
        if self.current >= self.levels.len() {
            self.phase = Phase::Win;
            return;
        }
        // Carry the running totals into the next level.
        let (lives, score, coins) =
            (self.game.lives, self.game.score, self.game.coins_collected);
        let mut next = Game::new(self.levels[self.current].clone());
        next.lives = lives;
        next.score = score;
        next.coins_collected = coins;
        self.game = next;
        self.phase = Phase::Playing;
    }

    pub fn render(&self) -> Framebuffer {
        self.game.render()
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

    fn short_level() -> Level {
        // Mario reaches the end just by walking right.
        Level::from_rows(&["M.E", "###"])
    }

    #[test]
    fn completing_the_last_level_wins() {
        let mut session = Session::new(vec![short_level(), short_level()]);
        assert_eq!(session.phase, Phase::Playing);
        assert_eq!(session.current_level(), 0);

        for _ in 0..400 {
            session.step(held(Button::Right));
            if session.phase == Phase::Win {
                break;
            }
        }
        assert_eq!(session.phase, Phase::Win);
        assert_eq!(session.current_level(), 2);
    }

    #[test]
    fn advancing_carries_score_and_lives_forward() {
        let mut session = Session::new(vec![short_level(), short_level()]);
        // Bank some progress on level one before it is completed.
        session.game.score = 500;
        session.game.lives = 5;

        // Finish level one to trigger the transition.
        for _ in 0..400 {
            session.step(held(Button::Right));
            if session.current_level() == 1 {
                break;
            }
        }
        assert_eq!(session.current_level(), 1);
        assert_eq!(session.game.score, 500);
        assert_eq!(session.game.lives, 5);
    }

    #[test]
    fn running_out_of_lives_reaches_game_over() {
        // A level with a goomba and no reachable end: Mario keeps dying.
        let level = Level::from_rows(&["M.G", "###"]);
        let mut session = Session::new(vec![level]);
        for _ in 0..3000 {
            session.step(held(Button::Right));
            if session.phase == Phase::GameOver {
                break;
            }
        }
        assert_eq!(session.phase, Phase::GameOver);
    }
}
