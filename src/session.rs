//! Session flow: playing a sequence of levels end to end.
//!
//! A single [`crate::game::Game`] is one level. A [`Session`] strings levels
//! together: advance to the next when one is completed, carry the running totals
//! forward, win after the last, and end the run when the lives are gone. The
//! title and restart wiring builds on top of this.

use crate::core::level::Level;
use crate::game::Game;
use crate::input::{Button, Buttons};
use crate::render::Framebuffer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Title,
    Playing,
    GameOver,
    Win,
}

pub struct Session {
    levels: Vec<Level>,
    current: usize,
    pub game: Game,
    pub phase: Phase,
    /// Tracks the Start button so a held press does not skip through screens.
    start_latched: bool,
}

impl Session {
    /// Start a session at the title screen. There must be at least one level.
    pub fn new(levels: Vec<Level>) -> Self {
        assert!(!levels.is_empty(), "a session needs at least one level");
        let game = Game::new(levels[0].clone());
        Self {
            levels,
            current: 0,
            game,
            phase: Phase::Title,
            start_latched: false,
        }
    }

    /// A session over the built-in demo level, for the window frontend.
    pub fn demo() -> Self {
        Self::new(vec![Game::demo_level()])
    }

    /// Which level index is being played (0-based).
    pub fn current_level(&self) -> usize {
        self.current
    }

    /// Advance one frame. Start moves off the title and the end screens; while
    /// playing, the level steps and reacts to completing or running out of lives.
    pub fn step(&mut self, buttons: Buttons) {
        let start = buttons.is_held(Button::Start);
        let start_pressed = start && !self.start_latched;
        self.start_latched = start;

        match self.phase {
            Phase::Title => {
                if start_pressed {
                    self.begin();
                }
            }
            Phase::Playing => {
                self.game.step(buttons);
                if self.game.completed {
                    self.advance();
                } else if self.game.game_over {
                    self.phase = Phase::GameOver;
                }
            }
            Phase::GameOver | Phase::Win => {
                if start_pressed {
                    self.return_to_title();
                }
            }
        }
    }

    /// Begin a fresh run from the first level.
    fn begin(&mut self) {
        self.current = 0;
        self.game = Game::new(self.levels[0].clone());
        self.phase = Phase::Playing;
    }

    /// Return to the title screen with a fresh first level behind it.
    fn return_to_title(&mut self) {
        self.current = 0;
        self.game = Game::new(self.levels[0].clone());
        self.phase = Phase::Title;
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

    /// Sound events from the most recent step, for a frontend to play.
    pub fn drain_sounds(&mut self) -> Vec<crate::sound::SoundEvent> {
        self.game.drain_sounds()
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

    /// Press Start once (edge) to leave the title screen.
    fn press_start(session: &mut Session) {
        session.step(held(Button::Start));
        session.step(Buttons::default());
    }

    #[test]
    fn a_session_starts_on_the_title_and_start_begins_play() {
        let mut session = Session::new(vec![short_level()]);
        assert_eq!(session.phase, Phase::Title);
        press_start(&mut session);
        assert_eq!(session.phase, Phase::Playing);
    }

    #[test]
    fn completing_the_last_level_wins() {
        let mut session = Session::new(vec![short_level(), short_level()]);
        press_start(&mut session);
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
        press_start(&mut session);
        // Bank some progress on level one before it is completed.
        session.game.score = 500;
        session.game.lives = 5;

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
    fn running_out_of_lives_reaches_game_over_then_start_returns_to_title() {
        // A level with a goomba and no reachable end: Mario keeps dying.
        let level = Level::from_rows(&["M.G", "###"]);
        let mut session = Session::new(vec![level]);
        press_start(&mut session);

        for _ in 0..3000 {
            session.step(held(Button::Right));
            if session.phase == Phase::GameOver {
                break;
            }
        }
        assert_eq!(session.phase, Phase::GameOver);

        // Start from game over goes back to the title, ready for another run.
        press_start(&mut session);
        assert_eq!(session.phase, Phase::Title);
        assert_eq!(session.game.lives, 3, "a fresh run has full lives");
    }

    #[test]
    fn completing_a_middle_level_advances_and_keeps_playing() {
        let mut session = Session::new(vec![short_level(), short_level()]);
        press_start(&mut session);
        for _ in 0..400 {
            session.step(held(Button::Right));
            if session.current_level() == 1 {
                break;
            }
        }
        // The completion trigger advanced the level but the run continues.
        assert_eq!(session.current_level(), 1);
        assert_eq!(session.phase, Phase::Playing);
    }

    #[test]
    fn winning_then_start_returns_to_title() {
        let mut session = Session::new(vec![short_level()]);
        press_start(&mut session);
        for _ in 0..400 {
            session.step(held(Button::Right));
            if session.phase == Phase::Win {
                break;
            }
        }
        assert_eq!(session.phase, Phase::Win);
        press_start(&mut session);
        assert_eq!(session.phase, Phase::Title);
    }
}
