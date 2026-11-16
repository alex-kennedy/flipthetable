use bevy::prelude::*;
use web_sys::{AudioContext, BiquadFilterType, OscillatorType};

use crate::scene::DebrisStyle;

#[derive(Clone, Copy)]
pub enum SoundEffect {
    FlipLaunch(f32),
    Topple,
    Crack,
    Shatter(DebrisStyle),
}

#[derive(Resource, Default)]
pub struct RetroAudio {
    ctx: Option<AudioContext>,
    pub muted: bool,
    pub queue: Vec<SoundEffect>,
}

impl RetroAudio {
    pub fn push(&mut self, sfx: SoundEffect) {
        if !self.muted {
            self.queue.push(sfx);
        }
    }

    fn ensure_ctx(&mut self) -> Option<&AudioContext> {
        if self.ctx.is_none() {
            self.ctx = AudioContext::new().ok();
        }
        if let Some(ctx) = &self.ctx {
            let _ = ctx.resume();
        }
        self.ctx.as_ref()
    }
}

pub fn flush_audio_queue(
    keys: Res<ButtonInput<KeyCode>>,
    mut audio: ResMut<RetroAudio>,
) {
    if keys.just_pressed(KeyCode::KeyM) {
        audio.muted = !audio.muted;
        if audio.muted {
            audio.queue.clear();
        }
    }

    if audio.queue.is_empty() || audio.muted {
        audio.queue.clear();
        return;
    }

    let drained: Vec<SoundEffect> = audio.queue.drain(..).collect();
    let Some(ctx) = audio.ensure_ctx() else {
        return;
    };

    for sfx in drained {
        match sfx {
            SoundEffect::FlipLaunch(power) => play_flip_launch(ctx, power),
            SoundEffect::Topple => play_tone_sweep(ctx, OscillatorType::Triangle, 190.0, 72.0, 0.24, 0.22),
            SoundEffect::Crack => play_tone_sweep(ctx, OscillatorType::Square, 320.0, 140.0, 0.08, 0.14),
            SoundEffect::Shatter(style) => match style {
                DebrisStyle::Papers => play_noise_burst(ctx, BiquadFilterType::Bandpass, 1800.0, 0.18, 0.16),
                DebrisStyle::Water => {
                    play_tone_sweep(ctx, OscillatorType::Sine, 280.0, 620.0, 0.14, 0.18);
                    play_noise_burst(ctx, BiquadFilterType::Bandpass, 1100.0, 0.16, 0.14);
                }
                DebrisStyle::Electronics => {
                    play_tone_sweep(ctx, OscillatorType::Sawtooth, 740.0, 110.0, 0.16, 0.16);
                    play_noise_burst(ctx, BiquadFilterType::Highpass, 2400.0, 0.14, 0.18);
                }
                DebrisStyle::Ceramic => {
                    play_noise_burst(ctx, BiquadFilterType::Highpass, 2800.0, 0.12, 0.20);
                }
                DebrisStyle::Generic => {
                    play_tone_sweep(ctx, OscillatorType::Square, 160.0, 55.0, 0.16, 0.18);
                    play_noise_burst(ctx, BiquadFilterType::Lowpass, 1400.0, 0.15, 0.16);
                }
            },
        }
    }
}

fn play_flip_launch(ctx: &AudioContext, power: f32) {
    let base_freq = 120.0 + power * 90.0;
    play_tone_sweep(ctx, OscillatorType::Triangle, base_freq, 48.0, 0.22, 0.28);
    play_noise_burst(ctx, BiquadFilterType::Bandpass, 650.0 + power * 450.0, 0.18, 0.16);
}

fn play_tone_sweep(
    ctx: &AudioContext,
    wave: OscillatorType,
    start_hz: f32,
    end_hz: f32,
    dur_secs: f32,
    volume: f32,
) {
    let now = ctx.current_time();
    let Ok(osc) = ctx.create_oscillator() else { return };
    let Ok(gain) = ctx.create_gain() else { return };

    osc.set_type(wave);
    let _ = osc.frequency().set_value_at_time(start_hz, now);
    let _ = osc
        .frequency()
        .exponential_ramp_to_value_at_time(end_hz.max(20.0), now + dur_secs as f64);

    let _ = gain.gain().set_value_at_time(volume, now);
    let _ = gain
        .gain()
        .exponential_ramp_to_value_at_time(0.001, now + dur_secs as f64);

    if osc.connect_with_audio_node(&gain).is_ok()
        && gain.connect_with_audio_node(&ctx.destination()).is_ok()
    {
        let _ = osc.start_with_when(now);
        let _ = osc.stop_with_when(now + dur_secs as f64);
    }
}

fn play_noise_burst(
    ctx: &AudioContext,
    filter_type: BiquadFilterType,
    cutoff_hz: f32,
    dur_secs: f32,
    volume: f32,
) {
    let sample_rate = ctx.sample_rate();
    let len = ((sample_rate * dur_secs) as u32).max(64);
    let Ok(buffer) = ctx.create_buffer(1, len, sample_rate) else { return };

    // Deterministic retro LFSR-like white noise
    let mut seed: u32 = 0x1234_5678;
    let mut samples = vec![0.0_f32; len as usize];
    for s in &mut samples {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        *s = ((seed as f32 / u32::MAX as f32) * 2.0) - 1.0;
    }
    let _ = buffer.copy_to_channel(&samples, 0);

    let Ok(src) = ctx.create_buffer_source() else { return };
    src.set_buffer(Some(&buffer));

    let Ok(filter) = ctx.create_biquad_filter() else { return };
    filter.set_type(filter_type);
    filter.frequency().set_value(cutoff_hz);

    let Ok(gain) = ctx.create_gain() else { return };
    let now = ctx.current_time();
    let _ = gain.gain().set_value_at_time(volume, now);
    let _ = gain
        .gain()
        .exponential_ramp_to_value_at_time(0.001, now + dur_secs as f64);

    if src.connect_with_audio_node(&filter).is_ok()
        && filter.connect_with_audio_node(&gain).is_ok()
        && gain.connect_with_audio_node(&ctx.destination()).is_ok()
    {
        let _ = src.start_with_when(now);
    }
}
