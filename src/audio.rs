//! Square-wave tone playback for the windowed frontend.
//!
//! Only built with `--features gui`. The game itself never touches this: it
//! emits [`sml::sound::SoundEvent`]s (see `sml::sound`), and this module is
//! the thin layer that turns queued [`sml::frontend::Tone`]s into an actual
//! square wave on an output device, sequentially, one tone at a time.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat, SizedSample};
use sml::frontend::Tone;

struct Playback {
    queue: VecDeque<Tone>,
    playing_hz: f32,
    samples_left: u32,
    phase: f32,
}

/// Owns the output stream. Dropping it stops playback.
pub struct AudioPlayer {
    _stream: cpal::Stream,
    state: Arc<Mutex<Playback>>,
}

const AMPLITUDE: f32 = 0.15;

impl AudioPlayer {
    /// Open the default output device. Returns `None` (rather than failing
    /// the whole window) if there is no audio device to play to.
    pub fn try_new() -> Option<Self> {
        let device = cpal::default_host().default_output_device()?;
        let config = device.default_output_config().ok()?;
        let sample_format = config.sample_format();
        let stream_config: cpal::StreamConfig = config.into();
        let sample_rate = stream_config.sample_rate as f32;
        let channels = stream_config.channels as usize;

        let state = Arc::new(Mutex::new(Playback {
            queue: VecDeque::new(),
            playing_hz: 0.0,
            samples_left: 0,
            phase: 0.0,
        }));

        let stream = match sample_format {
            SampleFormat::F32 => build_stream::<f32>(&device, stream_config, sample_rate, channels, state.clone()),
            SampleFormat::I16 => build_stream::<i16>(&device, stream_config, sample_rate, channels, state.clone()),
            SampleFormat::U16 => build_stream::<u16>(&device, stream_config, sample_rate, channels, state.clone()),
            _ => None,
        }?;
        stream.play().ok()?;

        Some(Self { _stream: stream, state })
    }

    /// Queue a tone to play after whatever is already queued finishes.
    pub fn play(&self, tone: Tone) {
        if let Ok(mut state) = self.state.lock() {
            state.queue.push_back(tone);
        }
    }
}

fn build_stream<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    sample_rate: f32,
    channels: usize,
    state: Arc<Mutex<Playback>>,
) -> Option<cpal::Stream>
where
    T: SizedSample + FromSample<f32>,
{
    let err_fn = |err| eprintln!("audio stream error: {err}");
    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [T], _| write_samples(data, channels, sample_rate, &state),
            err_fn,
            None,
        )
        .ok()?;
    Some(stream)
}

fn write_samples<T>(output: &mut [T], channels: usize, sample_rate: f32, state: &Arc<Mutex<Playback>>)
where
    T: Sample + FromSample<f32>,
{
    let Ok(mut state) = state.lock() else { return };
    for frame in output.chunks_mut(channels) {
        if state.samples_left == 0 {
            match state.queue.pop_front() {
                Some(tone) => {
                    state.playing_hz = tone.frequency_hz;
                    state.samples_left = (tone.duration_ms as f32 * sample_rate / 1000.0) as u32;
                    state.phase = 0.0;
                }
                None => {
                    for sample in frame.iter_mut() {
                        *sample = T::from_sample(0.0);
                    }
                    continue;
                }
            }
        }

        let value = if state.phase < 0.5 { AMPLITUDE } else { -AMPLITUDE };
        state.phase = (state.phase + state.playing_hz / sample_rate).fract();
        state.samples_left -= 1;

        let sample = T::from_sample(value);
        for out in frame.iter_mut() {
            *out = sample;
        }
    }
}
