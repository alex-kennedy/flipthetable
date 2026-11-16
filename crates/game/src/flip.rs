use std::collections::VecDeque;

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::scene::PlayerDesk;
use crate::GamePhase;

#[derive(Resource, Default)]
pub struct FlipTracker {
    pub dragging: bool,
    pub samples: VecDeque<(Vec2, f32)>,
    pub live_power: f32,
    pub live_aim: f32,
    pub last_launch_power: f32,
}

impl FlipTracker {
    pub fn reset(&mut self) {
        self.dragging = false;
        self.samples.clear();
        self.live_power = 0.0;
        self.live_aim = 0.0;
        self.last_launch_power = 0.0;
    }

    fn push_sample(&mut self, pos: Vec2, t: f32) {
        self.samples.push_back((pos, t));
        while let Some(&(_, oldest_t)) = self.samples.front() {
            if t - oldest_t > 0.14 && self.samples.len() > 2 {
                self.samples.pop_front();
            } else {
                break;
            }
        }
    }

    /// Computes (normalized_power 0..=1, lateral_aim -1..=1) from velocity + upward acceleration.
    fn compute_thrust(&self) -> Option<(f32, f32)> {
        if self.samples.len() < 2 {
            return None;
        }
        let &(first_pos, first_t) = self.samples.front()?;
        let &(last_pos, last_t) = self.samples.back()?;
        let dt = (last_t - first_t).max(0.008);

        // Screen Y grows downward, so upward swipe has negative dy.
        let delta = last_pos - first_pos;
        let upward_px = -delta.y;
        if upward_px < 18.0 {
            return None;
        }

        let upward_vel = upward_px / dt;

        // Measure peak upward acceleration between first half and second half of the window.
        let mid_idx = self.samples.len() / 2;
        let (mid_pos, mid_t) = self.samples[mid_idx];
        let dt1 = (mid_t - first_t).max(0.004);
        let dt2 = (last_t - mid_t).max(0.004);
        let v1 = -(mid_pos.y - first_pos.y) / dt1;
        let v2 = -(last_pos.y - mid_pos.y) / dt2;
        let accel = ((v2 - v1) / ((dt1 + dt2) * 0.5)).max(0.0);

        let combined = upward_vel * 0.72 + accel * 0.085;
        let power = ((combined - 180.0) / 2200.0).clamp(0.18, 1.0);

        let lateral_vel = delta.x / dt;
        let aim = (lateral_vel / 1400.0).clamp(-1.0, 1.0);
        Some((power, aim))
    }
}

#[derive(Resource)]
pub struct CameraRig {
    pub seated_pos: Vec3,
    pub seated_look: Vec3,
    pub current_pos: Vec3,
    pub current_look: Vec3,
    pub trauma: f32,
}

impl Default for CameraRig {
    fn default() -> Self {
        let seated_pos = Vec3::new(0.0, 1.48, 2.25);
        let seated_look = Vec3::new(0.0, 0.85, -1.6);
        Self {
            seated_pos,
            seated_look,
            current_pos: seated_pos,
            current_look: seated_look,
            trauma: 0.0,
        }
    }
}

impl CameraRig {
    pub fn reset(&mut self) {
        self.current_pos = self.seated_pos;
        self.current_look = self.seated_look;
        self.trauma = 0.0;
    }

    pub fn add_trauma(&mut self, amount: f32) {
        self.trauma = (self.trauma + amount).min(1.0);
    }
}

pub fn handle_flip_gesture(
    time: Res<Time>,
    mouse: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut tracker: ResMut<FlipTracker>,
    mut desk_q: Query<(&mut LinearVelocity, &mut AngularVelocity), With<PlayerDesk>>,
    mut next_state: ResMut<NextState<GamePhase>>,
    mut camera_rig: ResMut<CameraRig>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let pointer_pos = window
        .cursor_position()
        .or_else(|| touches.iter().next().map(|t| t.position()));

    let now = time.elapsed_secs();

    let pressed_just_now =
        mouse.just_pressed(MouseButton::Left) || touches.any_just_pressed();
    let held = mouse.pressed(MouseButton::Left) || touches.iter().next().is_some();
    let released =
        mouse.just_released(MouseButton::Left) || touches.any_just_released();

    if pressed_just_now {
        tracker.dragging = true;
        tracker.samples.clear();
        if let Some(pos) = pointer_pos {
            tracker.push_sample(pos, now);
        }
    }

    if tracker.dragging && held {
        if let Some(pos) = pointer_pos {
            tracker.push_sample(pos, now);
        }
        if let Some((power, aim)) = tracker.compute_thrust() {
            tracker.live_power = power;
            tracker.live_aim = aim;
        } else {
            tracker.live_power *= 0.85;
        }
    }

    if tracker.dragging && released {
        tracker.dragging = false;
        if let Some((power, aim)) = tracker.compute_thrust() {
            tracker.last_launch_power = power;
            for (mut lin_vel, mut ang_vel) in &mut desk_q {
                // Launch forward (-Z), upward (+Y), and laterally steered (X).
                let forward_speed = 6.5 + power * 10.5;
                let upward_speed = 3.2 + power * 4.8;
                let lateral_speed = aim * (3.5 + power * 5.0);
                lin_vel.0 = Vec3::new(lateral_speed, upward_speed, -forward_speed);

                // Pitch forward/away (-X) so it flips end-over-end, with slight roll/yaw.
                ang_vel.0 = Vec3::new(
                    -(5.5 + power * 7.5),
                    -aim * 2.2,
                    -aim * 3.0,
                );
            }
            camera_rig.add_trauma(0.28 + power * 0.25);
            next_state.set(GamePhase::Flipping);
        }
        tracker.live_power = 0.0;
        tracker.live_aim = 0.0;
    }
}

pub fn update_follow_camera(
    time: Res<Time>,
    phase: Res<State<GamePhase>>,
    desk_q: Query<&Transform, (With<PlayerDesk>, Without<Camera3d>)>,
    mut cam_q: Query<&mut Transform, With<Camera3d>>,
    mut rig: ResMut<CameraRig>,
) {
    let dt = time.delta_secs();
    let Ok(mut cam_tf) = cam_q.single_mut() else {
        return;
    };

    let (target_pos, target_look) = match phase.get() {
        GamePhase::Ready => (rig.seated_pos, rig.seated_look),
        GamePhase::Flipping | GamePhase::Settled => {
            if let Ok(desk_tf) = desk_q.single() {
                let d = desk_tf.translation;
                // Subtly follow the flying table while staying inside the room.
                let follow_pos = Vec3::new(
                    (d.x * 0.45).clamp(-3.2, 3.2),
                    (1.55 + d.y * 0.35).clamp(1.45, 2.8),
                    (d.z + 3.4).clamp(-1.8, 2.35),
                );
                let follow_look = Vec3::new(
                    (d.x * 0.75).clamp(-4.0, 4.0),
                    (d.y * 0.7 + 0.35).clamp(0.4, 2.4),
                    (d.z - 0.8).clamp(-5.8, 0.5),
                );
                (follow_pos, follow_look)
            } else {
                (rig.seated_pos, rig.seated_look)
            }
        }
    };

    let smooth = 1.0 - (-5.5 * dt).exp();
    rig.current_pos = rig.current_pos.lerp(target_pos, smooth);
    rig.current_look = rig.current_look.lerp(target_look, smooth);

    // Trauma-based retro screen shake
    rig.trauma = (rig.trauma - dt * 1.9).max(0.0);
    let shake = rig.trauma * rig.trauma;
    let t = time.elapsed_secs() * 38.0;
    let shake_offset = Vec3::new(
        (t * 1.1).sin() * 0.14 * shake,
        (t * 1.7).cos() * 0.14 * shake,
        0.0,
    );

    cam_tf.translation = rig.current_pos + shake_offset;
    cam_tf.look_at(rig.current_look, Vec3::Y);
}
