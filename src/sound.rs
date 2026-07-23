//! Sound as events, not audio.
//!
//! The game does not make noise directly. It emits [`SoundEvent`]s as things
//! happen, and a frontend decides what those sound like (a beep, a sample, or
//! nothing). Keeping sound as data means the game stays headless and every
//! "this should make a sound" moment is testable without an audio device.
//!
//! Playback is the frontend's job and is deliberately not here: a windowed
//! build can map each event to a tone. That is a thin follow-up over this model.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundEvent {
    Jump,
    Coin,
    BlockBump,
    Stomp,
    PowerUp,
    Shrink,
    OneUp,
    Death,
    LevelComplete,
    GameOver,
}
