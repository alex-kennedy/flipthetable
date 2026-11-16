use bevy::prelude::*;
use web_sys::{AudioBuffer, AudioContext, BiquadFilterType, OscillatorType};

use crate::prefabs::SfxMaterial;

#[derive(Clone, Copy)]
pub enum SoundEffect {
    FlipLaunch(f32),
    Topple,
    Crack,
    DeskBreak(u8),
    Shatter(SfxMaterial),
    TallySlam(u8),
    TallyTick(f32),
    RankVerdict(u8),
}

impl SoundEffect {
    fn priority(self) -> u8 {
        match self {
            SoundEffect::RankVerdict(_) | SoundEffect::FlipLaunch(_) => 10,
            SoundEffect::TallySlam(_) => 9,
            SoundEffect::Shatter(SfxMaterial::PressurizedBlast) | SoundEffect::DeskBreak(_) => 8,
            SoundEffect::Shatter(_) => 5,
            SoundEffect::Topple => 4,
            SoundEffect::TallyTick(_) => 3,
            SoundEffect::Crack => 2,
        }
    }
}

#[derive(Resource, Default)]
pub struct RetroAudio {
    ctx: Option<AudioContext>,
    noise_buf: Option<AudioBuffer>,
    pub muted: bool,
    pub queue: Vec<SoundEffect>,
    seq: u32,
}

impl RetroAudio {
    pub fn push(&mut self, sfx: SoundEffect) {
        if !self.muted && self.queue.len() < 16 {
            self.queue.push(sfx);
        }
    }

    fn ensure_ctx_and_noise(&mut self) -> Option<(&AudioContext, &AudioBuffer)> {
        if self.ctx.is_none() {
            self.ctx = AudioContext::new().ok();
        }
        let ctx = self.ctx.as_ref()?;
        let _ = ctx.resume();

        if self.noise_buf.is_none() {
            let sr = ctx.sample_rate();
            let len = ((sr * 0.50) as u32).max(1024);
            if let Ok(buf) = ctx.create_buffer(1, len, sr) {
                let mut seed: u32 = 0x1234_5678;
                let mut samples = vec![0.0_f32; len as usize];
                for s in &mut samples {
                    seed ^= seed << 13;
                    seed ^= seed >> 17;
                    seed ^= seed << 5;
                    *s = ((seed as f32 / u32::MAX as f32) * 2.0) - 1.0;
                }
                let _ = buf.copy_to_channel(&samples, 0);
                self.noise_buf = Some(buf);
            }
        }

        Some((self.ctx.as_ref()?, self.noise_buf.as_ref()?))
    }
}

pub fn flush_audio_queue(keys: Res<ButtonInput<KeyCode>>, mut audio: ResMut<RetroAudio>) {
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

    let mut drained: Vec<SoundEffect> = audio.queue.drain(..).collect();
    // Cap per-frame WebAudio node spawns to 3 highest-priority sounds so huge chain reactions never stall!
    if drained.len() > 3 {
        drained.sort_by_key(|s| std::cmp::Reverse(s.priority()));
        drained.truncate(3);
    }

    audio.seq = audio.seq.wrapping_add(1);
    let pitch_jitter = 0.92 + ((audio.seq % 7) as f32) * 0.028;

    let Some((ctx, noise_buf)) = audio.ensure_ctx_and_noise() else {
        return;
    };

    for sfx in drained {
        match sfx {
            SoundEffect::FlipLaunch(power) => play_flip_launch(ctx, noise_buf, power),
            SoundEffect::Topple => {
                play_tone_sweep(
                    ctx,
                    OscillatorType::Triangle,
                    185.0 * pitch_jitter,
                    64.0,
                    0.22,
                    0.20,
                );
            }
            SoundEffect::Crack => {
                play_tone_sweep(
                    ctx,
                    OscillatorType::Square,
                    360.0 * pitch_jitter,
                    135.0,
                    0.07,
                    0.12,
                );
            }
            SoundEffect::DeskBreak(stage) => {
                let low = if stage >= 3 { 38.0 } else { 52.0 };
                play_tone_sweep(ctx, OscillatorType::Sawtooth, 210.0, low, 0.22, 0.25);
                play_cached_noise(ctx, noise_buf, BiquadFilterType::Bandpass, 950.0, 0.20, 0.20);
            }
            SoundEffect::TallySlam(idx) => {
                let base = 135.0 + (idx as f32) * 32.0;
                play_tone_sweep(
                    ctx,
                    OscillatorType::Square,
                    base * 1.8,
                    base * 0.65,
                    0.08,
                    0.18,
                );
                play_cached_noise(ctx, noise_buf, BiquadFilterType::Lowpass, 900.0, 0.055, 0.14);
            }
            SoundEffect::TallyTick(norm) => {
                let hz = 420.0 + norm.clamp(0.0, 1.2) * 620.0;
                play_tone_sweep(ctx, OscillatorType::Triangle, hz, hz * 1.06, 0.03, 0.07);
            }
            SoundEffect::RankVerdict(tier) => play_rank_verdict(ctx, noise_buf, tier),
            SoundEffect::Shatter(mat) => match mat {
                SfxMaterial::PressurizedBlast => {
                    play_tone_sweep(ctx, OscillatorType::Sawtooth, 145.0, 28.0, 0.38, 0.30);
                    play_cached_noise(ctx, noise_buf, BiquadFilterType::Lowpass, 720.0, 0.38, 0.26);
                }
                SfxMaterial::Electronics => {
                    play_tone_sweep(
                        ctx,
                        OscillatorType::Sawtooth,
                        880.0 * pitch_jitter,
                        95.0,
                        0.17,
                        0.16,
                    );
                    play_cached_noise(ctx, noise_buf, BiquadFilterType::Highpass, 2500.0, 0.14, 0.17);
                }
                SfxMaterial::Liquid => {
                    play_tone_sweep(
                        ctx,
                        OscillatorType::Sine,
                        250.0 * pitch_jitter,
                        680.0,
                        0.15,
                        0.17,
                    );
                    play_cached_noise(ctx, noise_buf, BiquadFilterType::Bandpass, 1150.0, 0.18, 0.14);
                }
                SfxMaterial::Paper => {
                    play_cached_noise(
                        ctx,
                        noise_buf,
                        BiquadFilterType::Bandpass,
                        1850.0 * pitch_jitter,
                        0.18,
                        0.15,
                    );
                }
                SfxMaterial::Glass => {
                    play_tone_sweep(
                        ctx,
                        OscillatorType::Triangle,
                        1420.0 * pitch_jitter,
                        620.0,
                        0.10,
                        0.13,
                    );
                    play_cached_noise(ctx, noise_buf, BiquadFilterType::Highpass, 3200.0, 0.14, 0.19);
                }
                SfxMaterial::Metal => {
                    play_tone_sweep(
                        ctx,
                        OscillatorType::Square,
                        290.0 * pitch_jitter,
                        82.0,
                        0.19,
                        0.17,
                    );
                    play_cached_noise(ctx, noise_buf, BiquadFilterType::Bandpass, 1600.0, 0.13, 0.14);
                }
                SfxMaterial::WoodCeramic => {
                    play_tone_sweep(
                        ctx,
                        OscillatorType::Square,
                        175.0 * pitch_jitter,
                        58.0,
                        0.15,
                        0.16,
                    );
                    play_cached_noise(ctx, noise_buf, BiquadFilterType::Lowpass, 1450.0, 0.13, 0.15);
                }
            },
        }
    }
}

fn play_flip_launch(ctx: &AudioContext, noise_buf: &AudioBuffer, power: f32) {
    let vol = 0.11 + power * 0.20;
    let base_freq = 85.0 + power * 125.0;
    play_tone_sweep(
        ctx,
        OscillatorType::Triangle,
        base_freq,
        42.0,
        0.14 + power * 0.12,
        vol,
    );
    play_cached_noise(
        ctx,
        noise_buf,
        BiquadFilterType::Bandpass,
        420.0 + power * 680.0,
        0.10 + power * 0.12,
        0.07 + power * 0.12,
    );
}

fn play_rank_verdict(ctx: &AudioContext, noise_buf: &AudioBuffer, tier: u8) {
    match tier {
        0 => {
            play_tone_sweep_delayed(ctx, OscillatorType::Sawtooth, 195.0, 165.0, 0.0, 0.18, 0.18);
            play_tone_sweep_delayed(ctx, OscillatorType::Sawtooth, 155.0, 88.0, 0.19, 0.38, 0.22);
        }
        1 => {
            play_tone_sweep_delayed(ctx, OscillatorType::Triangle, 330.0, 330.0, 0.0, 0.12, 0.18);
            play_tone_sweep_delayed(ctx, OscillatorType::Triangle, 392.0, 392.0, 0.11, 0.20, 0.18);
        }
        2 => {
            for (i, hz) in [392.0, 493.88, 587.33].into_iter().enumerate() {
                play_tone_sweep_delayed(
                    ctx,
                    OscillatorType::Square,
                    hz,
                    hz * 1.01,
                    i as f64 * 0.065,
                    0.18,
                    0.16,
                );
            }
        }
        3 => {
            play_cached_noise(ctx, noise_buf, BiquadFilterType::Lowpass, 950.0, 0.14, 0.20);
            for (i, hz) in [440.0, 554.37, 659.25, 880.0].into_iter().enumerate() {
                play_tone_sweep_delayed(
                    ctx,
                    OscillatorType::Square,
                    hz,
                    hz * 1.02,
                    i as f64 * 0.055,
                    0.22,
                    0.18,
                );
            }
        }
        _ => {
            play_cached_noise(ctx, noise_buf, BiquadFilterType::Lowpass, 1100.0, 0.20, 0.24);
            for (i, hz) in [523.25, 659.25, 783.99, 1046.5, 1318.5]
                .into_iter()
                .enumerate()
            {
                play_tone_sweep_delayed(
                    ctx,
                    OscillatorType::Square,
                    hz,
                    hz * 1.03,
                    i as f64 * 0.05,
                    0.26,
                    0.20,
                );
            }
        }
    }
}

fn play_tone_sweep(
    ctx: &AudioContext,
    wave: OscillatorType,
    start_hz: f32,
    end_hz: f32,
    dur_secs: f32,
    volume: f32,
) {
    play_tone_sweep_delayed(ctx, wave, start_hz, end_hz, 0.0, dur_secs, volume);
}

fn play_tone_sweep_delayed(
    ctx: &AudioContext,
    wave: OscillatorType,
    start_hz: f32,
    end_hz: f32,
    delay_secs: f64,
    dur_secs: f32,
    volume: f32,
) {
    let start_t = ctx.current_time() + delay_secs;
    let end_t = start_t + dur_secs as f64;
    let Ok(osc) = ctx.create_oscillator() else {
        return;
    };
    let Ok(gain) = ctx.create_gain() else {
        return;
    };

    osc.set_type(wave);
    let _ = osc.frequency().set_value_at_time(start_hz, start_t);
    let _ = osc
        .frequency()
        .exponential_ramp_to_value_at_time(end_hz.max(20.0), end_t);

    let _ = gain.gain().set_value_at_time(0.0001, ctx.current_time());
    let _ = gain.gain().set_value_at_time(volume, start_t);
    let _ = gain.gain().exponential_ramp_to_value_at_time(0.001, end_t);

    if osc.connect_with_audio_node(&gain).is_ok()
        && gain.connect_with_audio_node(&ctx.destination()).is_ok()
    {
        let _ = osc.start_with_when(start_t);
        let _ = osc.stop_with_when(end_t);
    }
}

fn play_cached_noise(
    ctx: &AudioContext,
    noise_buf: &AudioBuffer,
    filter_type: BiquadFilterType,
    cutoff_hz: f32,
    dur_secs: f32,
    volume: f32,
) {
    let Ok(src) = ctx.create_buffer_source() else {
        return;
    };
    src.set_buffer(Some(noise_buf));

    let Ok(filter) = ctx.create_biquad_filter() else {
        return;
    };
    filter.set_type(filter_type);
    filter.frequency().set_value(cutoff_hz);

    let Ok(gain) = ctx.create_gain() else {
        return;
    };
    let now = ctx.current_time();
    let end_t = now + dur_secs.min(0.48) as f64;
    let _ = gain.gain().set_value_at_time(volume, now);
    let _ = gain.gain().exponential_ramp_to_value_at_time(0.001, end_t);

    if src.connect_with_audio_node(&filter).is_ok()
        && filter.connect_with_audio_node(&gain).is_ok()
        && gain.connect_with_audio_node(&ctx.destination()).is_ok()
    {
        let _ = src.start_with_when(now);
    }
}
