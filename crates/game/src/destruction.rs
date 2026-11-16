use avian3d::prelude::*;
use bevy::prelude::*;

use crate::flip::CameraRig;
use crate::scene::{DestructibleProp, ScenarioEntity};
use crate::GamePhase;

#[derive(Resource, Default)]
pub struct ScoreState {
    pub total_score: u32,
    pub items_destroyed: u32,
    pub items_toppled: u32,
    pub combo: u32,
    pub max_combo: u32,
    pub combo_timer: f32,
    pub elapsed_since_flip: f32,
    pub settle_timer: f32,
    pub slowmo_timer: f32,
}

impl ScoreState {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    fn bump_combo(&mut self) -> u32 {
        self.combo = (self.combo + 1).min(12);
        self.max_combo = self.max_combo.max(self.combo);
        self.combo_timer = 2.0;
        self.combo
    }
}

pub struct PopupSpawn {
    pub world_pos: Vec3,
    pub text: String,
    pub color: Color,
    pub big: bool,
}

#[derive(Resource, Default)]
pub struct PendingPopups(pub Vec<PopupSpawn>);

#[derive(Component)]
pub struct DebrisChunk {
    pub ttl: f32,
}

pub fn process_destruction(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut virtual_time: ResMut<Time<Virtual>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut score: ResMut<ScoreState>,
    mut popups: ResMut<PendingPopups>,
    mut camera_rig: ResMut<CameraRig>,
    mut prop_q: Query<(Entity, &Transform, &LinearVelocity, &mut DestructibleProp)>,
) {
    let dt = time.delta_secs();
    score.elapsed_since_flip += dt;

    if score.combo_timer > 0.0 {
        score.combo_timer -= dt;
        if score.combo_timer <= 0.0 {
            score.combo = 0;
        }
    }

    if score.slowmo_timer > 0.0 {
        score.slowmo_timer -= dt;
        virtual_time.set_relative_speed(0.45);
    } else {
        virtual_time.set_relative_speed(1.0);
    }

    let can_score_impacts = score.elapsed_since_flip > 0.08;

    for (entity, tf, lin_vel, mut prop) in &mut prop_q {
        // Stage 1a: Toppling detection for tall props (gives popup as they slowly fall)
        if prop.can_topple && !prop.toppled && !prop.shattered {
            if tf.up().dot(Vec3::Y) < 0.78 {
                prop.toppled = true;
                score.items_toppled += 1;
                let mult = score.bump_combo();
                let pts = 75 * mult;
                score.total_score += pts;
                score.slowmo_timer = score.slowmo_timer.max(0.22);
                camera_rig.add_trauma(0.16);
                popups.0.push(PopupSpawn {
                    world_pos: tf.translation + Vec3::Y * 0.7,
                    text: format!("+{pts} TOPPLED!"),
                    color: Color::srgb(0.55, 0.92, 0.98),
                    big: false,
                });
            }
        }

        // Stage 1b & 2: Impact-based damage & shattering
        let delta_v = (lin_vel.0 - prop.prev_velocity).length();
        prop.prev_velocity = lin_vel.0;

        if !can_score_impacts {
            continue;
        }

        if !prop.shattered && delta_v >= prop.break_impulse {
            prop.shattered = true;
            score.items_destroyed += 1;
            let mult = score.bump_combo();
            let pts = prop.base_points * mult;
            score.total_score += pts;
            camera_rig.add_trauma(0.28);

            let label = if mult > 1 {
                format!("+{pts} {} x{mult}!", prop.label)
            } else {
                format!("+{pts} {}!", prop.label)
            };

            popups.0.push(PopupSpawn {
                world_pos: tf.translation + Vec3::Y * 0.45,
                text: label,
                color: Color::srgb(0.98, 0.78, 0.25),
                big: true,
            });

            // Spawn low-poly debris chunks
            let debris_mesh = meshes.add(Cuboid::new(0.14, 0.10, 0.14));
            let debris_mat = materials.add(StandardMaterial {
                base_color: prop.debris_color,
                perceptual_roughness: 0.75,
                ..default()
            });

            let count = prop.debris_count.max(3);
            for i in 0..count {
                let angle = (i as f32 / count as f32) * std::f32::consts::TAU;
                let spread = Vec3::new(angle.cos() * 2.8, 2.4 + (i % 3) as f32 * 0.9, angle.sin() * 2.8);
                commands.spawn((
                    ScenarioEntity,
                    DebrisChunk { ttl: 4.5 },
                    RigidBody::Dynamic,
                    Mass(0.35),
                    Collider::cuboid(0.14, 0.10, 0.14),
                    Mesh3d(debris_mesh.clone()),
                    MeshMaterial3d(debris_mat.clone()),
                    Transform::from_translation(tf.translation + Vec3::Y * 0.15),
                    LinearVelocity(lin_vel.0 * 0.35 + spread),
                    AngularVelocity(Vec3::new(angle * 3.0, 4.5, -angle * 2.5)),
                ));
            }

            commands.entity(entity).despawn();
        } else if !prop.damaged && !prop.shattered && delta_v >= prop.damage_impulse {
            prop.damaged = true;
            let pts = 50;
            score.total_score += pts;
            camera_rig.add_trauma(0.12);
            popups.0.push(PopupSpawn {
                world_pos: tf.translation + Vec3::Y * 0.35,
                text: format!("+{pts} CRACKED!"),
                color: Color::srgb(0.98, 0.48, 0.38),
                big: false,
            });
        }
    }
}

pub fn update_debris_lifetime(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(Entity, &mut DebrisChunk, &mut Transform)>,
) {
    let dt = time.delta_secs();
    for (entity, mut chunk, mut tf) in &mut q {
        chunk.ttl -= dt;
        if chunk.ttl < 0.6 {
            let s = (chunk.ttl / 0.6).max(0.01);
            tf.scale = Vec3::splat(s);
        }
        if chunk.ttl <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

pub fn check_run_settled(
    time: Res<Time<Real>>,
    mut virtual_time: ResMut<Time<Virtual>>,
    mut score: ResMut<ScoreState>,
    vel_q: Query<&LinearVelocity, With<DestructibleProp>>,
    mut next_state: ResMut<NextState<GamePhase>>,
) {
    let dt = time.delta_secs();
    if score.elapsed_since_flip < 1.3 {
        return;
    }

    let max_speed = vel_q
        .iter()
        .map(|v| v.0.length())
        .fold(0.0_f32, f32::max);

    if max_speed < 0.28 {
        score.settle_timer += dt;
    } else {
        score.settle_timer = 0.0;
    }

    if score.settle_timer >= 0.9 || score.elapsed_since_flip >= 7.5 {
        virtual_time.set_relative_speed(1.0);
        next_state.set(GamePhase::Settled);
    }
}
