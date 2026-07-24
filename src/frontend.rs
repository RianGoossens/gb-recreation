//! The bridge from our framebuffer (and sound events) to a window.
//!
//! The pixel conversion and the sound event to tone mapping live here,
//! testable on their own, so the windowing and audio code (behind the
//! optional `gui` feature) stay thin loops. `to_argb` turns a 160x144 shade
//! buffer into a scaled 0xAARRGGBB buffer a window can blit. `tone_for` turns
//! a [`SoundEvent`] into a square-wave tone an audio backend can play.

use crate::render::Framebuffer;
use crate::sound::SoundEvent;
use crate::{SCREEN_HEIGHT, SCREEN_WIDTH};

/// Convert the framebuffer to 0xAARRGGBB pixels, each source pixel expanded into
/// a `scale` by `scale` block. Fully opaque. Scale is at least 1.
pub fn to_argb(fb: &Framebuffer, scale: usize) -> Vec<u32> {
    let scale = scale.max(1);
    let sw = SCREEN_WIDTH as usize;
    let sh = SCREEN_HEIGHT as usize;
    let w = sw * scale;
    let mut buf = vec![0u32; w * sh * scale];
    for y in 0..sh {
        for x in 0..sw {
            let g = fb.get(x as u32, y as u32).to_gray() as u32;
            let argb = 0xFF00_0000 | (g << 16) | (g << 8) | g;
            for dy in 0..scale {
                let row = (y * scale + dy) * w;
                for dx in 0..scale {
                    buf[row + x * scale + dx] = argb;
                }
            }
        }
    }
    buf
}

/// The pixel size of the scaled buffer: (width, height).
pub fn scaled_size(scale: usize) -> (usize, usize) {
    let scale = scale.max(1);
    (SCREEN_WIDTH as usize * scale, SCREEN_HEIGHT as usize * scale)
}

/// A single square-wave beep: how an audio backend plays one [`SoundEvent`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tone {
    pub frequency_hz: f32,
    pub duration_ms: u32,
}

/// Map a [`SoundEvent`] to the tone that stands in for it.
///
/// These frequencies are not read from the Game Boy's APU; the real sound
/// effects have not been extracted yet. This is a placeholder mapping so the
/// event model has an audible frontend, labeled provisional in
/// `docs/reference/faithfulness.md` until the cartridge's own tones are pinned.
pub fn tone_for(event: SoundEvent) -> Tone {
    use SoundEvent::*;
    match event {
        Jump => Tone { frequency_hz: 880.0, duration_ms: 100 },
        Coin => Tone { frequency_hz: 1318.5, duration_ms: 150 },
        BlockBump => Tone { frequency_hz: 220.0, duration_ms: 80 },
        Stomp => Tone { frequency_hz: 440.0, duration_ms: 80 },
        PowerUp => Tone { frequency_hz: 660.0, duration_ms: 300 },
        Shrink => Tone { frequency_hz: 330.0, duration_ms: 300 },
        OneUp => Tone { frequency_hz: 987.8, duration_ms: 400 },
        Death => Tone { frequency_hz: 196.0, duration_ms: 500 },
        LevelComplete => Tone { frequency_hz: 523.3, duration_ms: 600 },
        GameOver => Tone { frequency_hz: 165.0, duration_ms: 800 },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::Shade;

    #[test]
    fn buffer_has_scaled_dimensions() {
        let fb = Framebuffer::new();
        let buf = to_argb(&fb, 3);
        let (w, h) = scaled_size(3);
        assert_eq!(w, SCREEN_WIDTH as usize * 3);
        assert_eq!(h, SCREEN_HEIGHT as usize * 3);
        assert_eq!(buf.len(), w * h);
    }

    #[test]
    fn white_is_opaque_white_black_is_opaque_black() {
        let mut fb = Framebuffer::new(); // all white
        fb.set(0, 0, Shade::Black);
        let buf = to_argb(&fb, 1);
        assert_eq!(buf[0], 0xFF00_0000); // black, opaque
        assert_eq!(buf[1], 0xFFFF_FFFF); // white, opaque
    }

    #[test]
    fn every_sound_event_maps_to_an_audible_tone() {
        let events = [
            SoundEvent::Jump,
            SoundEvent::Coin,
            SoundEvent::BlockBump,
            SoundEvent::Stomp,
            SoundEvent::PowerUp,
            SoundEvent::Shrink,
            SoundEvent::OneUp,
            SoundEvent::Death,
            SoundEvent::LevelComplete,
            SoundEvent::GameOver,
        ];
        for event in events {
            let tone = tone_for(event);
            assert!(tone.frequency_hz > 0.0, "{event:?} has no pitch");
            assert!(tone.duration_ms > 0, "{event:?} is silent");
        }
    }

    #[test]
    fn distinct_events_get_distinct_tones() {
        // Not a hard rule forever, but today every event should be tellable
        // apart by ear; catches an accidental copy-paste of one mapping.
        let events = [
            SoundEvent::Jump,
            SoundEvent::Coin,
            SoundEvent::BlockBump,
            SoundEvent::Stomp,
            SoundEvent::PowerUp,
            SoundEvent::Shrink,
            SoundEvent::OneUp,
            SoundEvent::Death,
            SoundEvent::LevelComplete,
            SoundEvent::GameOver,
        ];
        let tones: Vec<Tone> = events.iter().map(|&e| tone_for(e)).collect();
        for i in 0..tones.len() {
            for j in (i + 1)..tones.len() {
                assert_ne!(tones[i], tones[j], "{:?} and {:?} sound identical", events[i], events[j]);
            }
        }
    }

    #[test]
    fn scaling_replicates_each_pixel_into_a_block() {
        let mut fb = Framebuffer::new();
        fb.set(0, 0, Shade::Black);
        let scale = 2;
        let buf = to_argb(&fb, scale);
        let (w, _h) = scaled_size(scale);
        // The top-left 2x2 block should all be black.
        assert_eq!(buf[0], 0xFF00_0000);
        assert_eq!(buf[1], 0xFF00_0000);
        assert_eq!(buf[w], 0xFF00_0000);
        assert_eq!(buf[w + 1], 0xFF00_0000);
    }
}
