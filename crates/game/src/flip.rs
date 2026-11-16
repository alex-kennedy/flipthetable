use std::collections::VecDeque;

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::audio::{RetroAudio, SoundEffect};
use crate::destruction::ScoreState;
use crate::palette::CorporateJokeRng;
use crate::prefabs::{DeskRestPose, PlayerDesk};
use crate::render_pipeline::SceneCamera3d;
use crate::GamePhase;

#[derive(Resource, Default)]
pub struct FlipTracker {
    pub dragging: bool,
    pub grab_norm_x: f32,
    pub samples: VecDeque<(Vec2, f32)>,
    pub live_power: f32,
    pub live_aim: f32,
    pub last_launch_power: f32,
}

impl FlipTracker {
    pub fn reset(&mut self) {
        self.dragging = false;
        self.grab_norm_x = 0.0;
        self.samples.clear();
        self.live_power = 0.0;
        self.live_aim = 0.0;
        self.last_launch_power = 0.0;
    }

    fn push_sample(&mut self, pos: Vec2, t: f32) {
        self.samples.push_back((pos, t));
        while let Some(&(_, oldest_t)) = self.samples.front() {
            if t - oldest_t > 0.12 && self.samples.len() > 2 {
                self.samples.pop_front();
            } else {
                break;
            }
        }
    }

    /// Computes (power 0.015..=1.0, steer_aim -1.0..=1.0 where ±1.0 = ±58° diagonal launch!).
    fn compute_thrust(&self, win_w: f32, win_h: f32, now: f32) -> Option<(f32, f32)> {
        if self.samples.len() < 2 {
            return None;
        }
        let &(first_pos, first_t) = self.samples.front()?;
        let &(last_pos, last_t) = self.samples.back()?;

        let idle_gap = (now - last_t).max(0.0);
        let stall_decay = (1.0 - idle_gap * 8.5).clamp(0.12, 1.0);

        let total_dt = (last_t - first_t).max(0.012);
        let total_delta = last_pos - first_pos;
        let upward_px = -total_delta.y;
        let horiz_px = total_delta.x;

        // Allow steep diagonal swipes as long as there is meaningful upward + directional motion!
        if upward_px < 6.0 {
            return None;
        }

        let norm_h = win_h.max(320.0);
        let norm_w = win_w.max(320.0);
        let diag_norm = Vec2::new(horiz_px / norm_w, upward_px / norm_h).length();

        // 1. Sustained release speed over final ~48ms (includes diagonal whip speed!)
        let mut tail_idx = 0;
        for (i, &(_, st)) in self.samples.iter().enumerate() {
            if last_t - st <= 0.048 {
                tail_idx = i;
                break;
            }
        }
        let (tail_pos, tail_t) = self.samples[tail_idx];
        let tail_dt = (last_t - tail_t).max(0.012);
        let tail_d = last_pos - tail_pos;
        let tail_up = (-tail_d.y / norm_h).max(0.0);
        let tail_lat = (tail_d.x / norm_w).abs() * 0.65;
        let release_speed = Vec2::new(tail_lat, tail_up).length() / tail_dt;

        // 2. Smoothed 2-segment peak speed
        let mut smoothed_peak = diag_norm / total_dt;
        if self.samples.len() >= 3 {
            for i in 0..self.samples.len() - 2 {
                let (p0, t0) = self.samples[i];
                let (p2, t2) = self.samples[i + 2];
                let seg_dt = (t2 - t0).max(0.012);
                let up_s = (-(p2.y - p0.y) / norm_h).max(0.0);
                let lat_s = ((p2.x - p0.x) / norm_w).abs() * 0.65;
                let seg_v = Vec2::new(lat_s, up_s).length() / seg_dt;
                if seg_v > smoothed_peak {
                    smoothed_peak = seg_v;
                }
            }
        }

        let effective_speed = (release_speed * 0.68 + smoothed_peak * 0.32) * stall_decay;
        let linear = ((effective_speed - 0.22) / 5.1).clamp(0.015, 1.0);
        let power = linear.powf(1.58).clamp(0.015, 1.0);

        // True 2D swipe angle for responsive wide directional steering!
        let aspect_adj_dx = (horiz_px / norm_w) * 1.35;
        let aspect_adj_dy = (upward_px / norm_h).max(0.02);
        let raw_angle = aspect_adj_dx.atan2(aspect_adj_dy); // radians: ±0.95 rad for 45° swipe
        let aim = (raw_angle / 0.92).clamp(-1.0, 1.0);

        Some((power, aim))
    }
}

#[derive(Resource)]
pub struct CameraRig {
    pub seated_pos: Vec3,
    pub seated_look: Vec3,
    pub current_pos: Vec3,
    pub current_look: Vec3,
    pub base_fov_deg: f32,
    pub current_fov_deg: f32,
    pub fov_punch: f32,
    pub trauma: f32,
}

impl Default for CameraRig {
    fn default() -> Self {
        let seated_pos = Vec3::new(0.0, 1.28, 2.54);
        let seated_look = Vec3::new(0.0, 1.02, -1.80);
        Self {
            seated_pos,
            seated_look,
            current_pos: seated_pos,
            current_look: seated_look,
            base_fov_deg: 57.0,
            current_fov_deg: 57.0,
            fov_punch: 0.0,
            trauma: 0.0,
        }
    }
}

impl CameraRig {
    pub fn reset(&mut self) {
        self.current_pos = self.seated_pos;
        self.current_look = self.seated_look;
        self.current_fov_deg = self.base_fov_deg;
        self.fov_punch = 0.0;
        self.trauma = 0.0;
    }

    pub fn add_trauma(&mut self, amount: f32) {
        self.trauma = (self.trauma + amount).min(1.0);
    }
}

#[allow(clippy::too_many_arguments)]
pub fn handle_flip_gesture(
    time: Res<Time>,
    mouse: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut tracker: ResMut<FlipTracker>,
    mut joke_rng: ResMut<CorporateJokeRng>,
    mut desk_q: Query<
        (&DeskRestPose, &mut Transform, &mut LinearVelocity, &mut AngularVelocity),
        With<PlayerDesk>,
    >,
    mut next_state: ResMut<NextState<GamePhase>>,
    mut camera_rig: ResMut<CameraRig>,
    mut audio: ResMut<RetroAudio>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let win_w = window.width().max(320.0);
    let win_h = window.height().max(240.0);
    let pointer_pos = window
        .cursor_position()
        .or_else(|| touches.iter().next().map(|t| t.position()));

    let now = time.elapsed_secs();
    let pressed_just_now = mouse.just_pressed(MouseButton::Left) || touches.any_just_pressed();
    let held = mouse.pressed(MouseButton::Left) || touches.iter().next().is_some();
    let released = mouse.just_released(MouseButton::Left) || touches.any_just_released();

    if pressed_just_now {
        tracker.dragging = true;
        tracker.samples.clear();
        if let Some(pos) = pointer_pos {
            tracker.grab_norm_x = ((pos.x / win_w) * 2.0 - 1.0).clamp(-1.0, 1.0);
            tracker.push_sample(pos, now);
            joke_rng.advance_round((pos.x * 100.0 + now * 1000.0) as u64);
        }
    }

    if tracker.dragging && held {
        if let Some(pos) = pointer_pos {
            tracker.push_sample(pos, now);
        }
        if let Some((power, aim)) = tracker.compute_thrust(win_w, win_h, now) {
            tracker.live_power = power;
            tracker.live_aim = aim;
        } else {
            tracker.live_power *= 0.85;
        }

        // Tactile directional yaw, bank, and shudder preview while aiming!
        let p = tracker.live_power;
        let aim = tracker.live_aim;
        let shudder = Vec3::new(
            aim * 0.06 * p + (now * 62.0).sin() * 0.007 * p,
            p * 0.015,
            (now * 51.0).cos() * 0.005 * p,
        );
        for (rest, mut tf, mut lv, mut av) in &mut desk_q {
            lv.0 = Vec3::ZERO;
            av.0 = Vec3::ZERO;
            tf.translation = rest.pos + shudder;
            tf.rotation = rest.rot
                * Quat::from_euler(EulerRot::YXZ, -aim * 0.22, -0.055 * p, -aim * 0.07 * p);
        }
    }

    if tracker.dragging && released {
        tracker.dragging = false;
        if let Some((power, aim)) = tracker.compute_thrust(win_w, win_h, now) {
            tracker.last_launch_power = power;
            // Wide ±58° steering cone so diagonal flicks blast left/right wings cleanly!
            let steer_rad = aim * 1.02;
            let horiz_speed = 0.75 + power * 22.2;
            let vx = horiz_speed * steer_rad.sin();
            let vz = -horiz_speed * steer_rad.cos();
            let vy = 0.45 + power * 8.4;

            let grab_torque = tracker.grab_norm_x * (0.8 + power * 2.8);

            for (rest, mut tf, mut lin_vel, mut ang_vel) in &mut desk_q {
                tf.translation = rest.pos + Vec3::Y * (0.008 + power * 0.02);
                lin_vel.0 = Vec3::new(vx, vy, vz);

                // Combine end-over-end pitch along launch vector with strong yaw & barrel-roll spin!
                let pitch_mag = 1.15 + power * 15.2;
                ang_vel.0 = Vec3::new(
                    -pitch_mag * steer_rad.cos(),
                    -(aim * (1.2 + power * 5.6) + grab_torque),
                    -(aim * (0.8 + power * 5.8)),
                );
            }
            camera_rig.fov_punch = 9.0 * power;
            camera_rig.add_trauma(0.08 + power * 0.48);
            audio.push(SoundEffect::FlipLaunch(power));
            next_state.set(GamePhase::Flipping);
        } else {
            for (rest, mut tf, _, _) in &mut desk_q {
                tf.translation = rest.pos;
                tf.rotation = rest.rot;
            }
        }
        tracker.live_power = 0.0;
        tracker.live_aim = 0.0;
    }
}

pub fn update_follow_camera(
    time: Res<Time>,
    phase: Res<State<GamePhase>>,
    tracker: Res<FlipTracker>,
    score: Res<ScoreState>,
    desk_q: Query<&Transform, (With<PlayerDesk>, Without<SceneCamera3d>)>,
    mut cam_q: Query<(&mut Transform, &mut Projection), With<SceneCamera3d>>,
    mut rig: ResMut<CameraRig>,
) {
    let dt = time.delta_secs();
    let Ok((mut cam_tf, mut proj)) = cam_q.single_mut() else {
        return;
    };

    let (target_pos, target_look, target_fov) = match phase.get() {
        GamePhase::Ready => {
            let brace = if tracker.dragging {
                tracker.live_power.max(0.06)
            } else {
                0.0
            };
            let pos = rig.seated_pos + Vec3::new(tracker.live_aim * 0.14, -0.04 * brace, -0.08 * brace);
            let look = rig.seated_look + Vec3::new(tracker.live_aim * 0.72, -0.05 * brace, 0.0);
            (pos, look, rig.base_fov_deg - brace * 3.8)
        }
        GamePhase::Flipping | GamePhase::Settled => {
            if let Ok(desk_tf) = desk_q.single() {
                let d = desk_tf.translation;
                let follow_pos = Vec3::new(
                    (d.x * 0.62).clamp(-4.0, 4.0),
                    (1.38 + d.y * 0.34).clamp(1.28, 2.75),
                    (d.z + 3.45).clamp(-1.6, 2.54),
                );
                let mut follow_look = Vec3::new(
                    (d.x * 0.92).clamp(-4.8, 4.8),
                    (d.y * 0.65 + 0.38).clamp(0.35, 2.4),
                    (d.z - 0.85).clamp(-5.8, 0.4),
                );
                if let Some((impact_pos, ttl)) = score.last_impact_pos {
                    let w = (ttl * 0.28).clamp(0.0, 0.28);
                    follow_look = follow_look.lerp(impact_pos, w);
                }
                (follow_pos, follow_look, rig.base_fov_deg + 2.0)
            } else {
                (rig.seated_pos, rig.seated_look, rig.base_fov_deg)
            }
        }
    };

    let smooth = 1.0 - (-6.4 * dt).exp();
    rig.current_pos = rig.current_pos.lerp(target_pos, smooth);
    rig.current_look = rig.current_look.lerp(target_look, smooth);

    rig.fov_punch *= (-5.5 * dt).exp();
    let desired_fov = target_fov + rig.fov_punch;
    rig.current_fov_deg += (desired_fov - rig.current_fov_deg) * (1.0 - (-10.0 * dt).exp());
    if let Projection::Perspective(ref mut persp) = *proj {
        persp.fov = rig.current_fov_deg.to_radians();
    }

    rig.trauma = (rig.trauma - dt * 1.9).max(0.0);
    let shake = rig.trauma * rig.trauma;
    let t = time.elapsed_secs() * 41.0;
    let shake_offset = Vec3::new(
        (t * 1.13).sin() * 0.14 * shake,
        (t * 1.71).cos() * 0.13 * shake,
        0.0,
    );
    let roll = (t * 1.37).sin() * 0.055 * shake;

    cam_tf.translation = rig.current_pos + shake_offset;
    cam_tf.look_at(rig.current_look, Vec3::Y);
    cam_tf.rotate_local_z(roll);
}
