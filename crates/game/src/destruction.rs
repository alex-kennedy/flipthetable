use avian3d::prelude::*;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;

use crate::audio::{RetroAudio, SoundEffect};
use crate::flip::CameraRig;
use crate::meshes::{outline_scale_for, SharedMeshes};
use crate::palette::VfxPalette;
use crate::prefabs::{
    BreakableDesk, DeskBreakoffPart, DeskTopVisual, DestructibleProp, EmitterStyle, GlassSurface,
    ImpactProfile, PlayerDesk, ScenarioEntity, ScreenSurface, VfxEmitter, LAYER_DEBRIS,
    LAYER_DEFAULT,
};
use crate::GamePhase;

#[derive(Clone)]
pub struct ScoreFeedEntry {
    pub text: String,
    pub age: f32,
}

#[derive(Resource, Default)]
pub struct ScoreState {
    pub total_score: u32,
    pub destruction_points: u32,
    pub topple_points: u32,
    pub combo_bonus: u32,
    pub feat_bonus: u32,
    pub feat_label: String,
    pub bonuses_finalized: bool,
    pub items_destroyed: u32,
    pub items_toppled: u32,
    pub desk_damage_stage: u8,
    pub hazards_detonated: u32,
    pub liquids_spilled: u32,
    pub combo: u32,
    pub max_combo: u32,
    pub combo_timer: f32,
    pub elapsed_since_flip: f32,
    pub settle_timer: f32,
    pub slowmo_timer: f32,
    pub slowmo_cooldown: f32,
    pub hitstop_timer: f32,
    pub hitstop_cooldown: f32,
    pub score_pulse: f32,
    pub last_impact_pos: Option<(Vec3, f32)>,
    pub recent_feed: Vec<ScoreFeedEntry>,
}

impl ScoreState {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    fn bump_combo(&mut self) -> u32 {
        self.combo = (self.combo + 1).min(25);
        self.max_combo = self.max_combo.max(self.combo);
        self.combo_timer = 1.85;
        self.combo
    }

    /// Smooth chain multiplier: 1.0x at x1, +0.10x per link up to 2.4x max so high ranks require real room carnage!
    fn chain_scaled_points(base: u32, combo: u32) -> u32 {
        let bonus_steps = combo.saturating_sub(1).min(14);
        (base * (10 + bonus_steps)) / 10
    }

    pub fn record_points(&mut self, pts: u32, label: String, world_pos: Vec3) {
        self.total_score += pts;
        self.score_pulse = 1.0;
        self.last_impact_pos = Some((world_pos, 0.9));
        self.recent_feed.insert(0, ScoreFeedEntry { text: label, age: 0.0 });
        self.recent_feed.truncate(5);
    }

    pub fn trigger_hitstop(&mut self, dur: f32) {
        if self.hitstop_cooldown <= 0.0 {
            self.hitstop_timer = dur.min(0.026);
            self.hitstop_cooldown = 0.48;
        }
    }

    pub fn trigger_slowmo(&mut self, dur: f32) {
        if self.slowmo_cooldown <= 0.0 {
            self.slowmo_timer = dur.min(0.13);
            self.slowmo_cooldown = 1.05;
        }
    }

    pub fn finalize_bonuses(&mut self) {
        if self.bonuses_finalized {
            return;
        }
        self.bonuses_finalized = true;

        self.combo_bonus = if self.max_combo >= 3 {
            self.max_combo * 85
        } else {
            0
        };

        let mut feat_pts = 0_u32;
        let mut feats: Vec<&'static str> = Vec::new();
        if self.desk_damage_stage >= 3 {
            feat_pts += 550;
            feats.push("TABLE RESTRUCTURED");
        } else if self.desk_damage_stage >= 1 {
            feat_pts += 220;
            feats.push("DESK DOWNSIZED");
        }
        if self.hazards_detonated > 0 {
            feat_pts += self.hazards_detonated * 450;
            feats.push("OSHA LAWSUIT");
        }
        if self.liquids_spilled >= 2 {
            feat_pts += 280;
            feats.push("COFFEE & TEARS");
        }
        if self.items_destroyed >= 18 {
            feat_pts += 650;
            feats.push("MASS Q4 RIF");
        } else if self.items_destroyed >= 10 {
            feat_pts += 320;
            feats.push("REORG SYNERGY");
        }

        self.feat_bonus = feat_pts;
        self.feat_label = if !feats.is_empty() {
            feats.truncate(2);
            feats.join(" + ")
        } else if self.total_score < 2600 {
            "PIP ISSUED // NO SEVERANCE".to_string()
        } else {
            "AT-WILL TERMINATION".to_string()
        };

        self.total_score += self.combo_bonus + self.feat_bonus;
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

/// Macro dynamic debris (detached desk drawers, snapped legs, ejected cabinet drawers).
#[derive(Component)]
pub struct DebrisChunk {
    pub age: f32,
    pub mat: Handle<StandardMaterial>,
    pub outline_thickness: f32,
}

/// Zero-physics-overhead kinematic shard that bounces off floor & walls and stays as room mess!
#[derive(Component)]
pub struct BallisticShard {
    pub vel: Vec3,
    pub spin: Vec3,
    pub age: f32,
    pub settled: bool,
}

#[derive(Component)]
pub struct VfxParticle {
    pub age: f32,
    pub duration: f32,
    pub velocity: Vec3,
    pub gravity: f32,
    pub drag: f32,
    pub start_scale: f32,
    pub peak_scale: f32,
    pub spin: Vec3,
}

#[derive(Component)]
pub struct FlutteringPaper {
    pub age: f32,
    pub vel: Vec3,
    pub phase: f32,
    pub sway_freq: f32,
    pub settled: bool,
    pub floor_y: f32,
}

#[derive(Component)]
pub struct FloorDecal {
    pub age: f32,
    pub target_scale: f32,
}

#[derive(Component)]
pub struct ExplosionFlashLight {
    pub age: f32,
    pub duration: f32,
    pub start_intensity: f32,
}

#[allow(clippy::too_many_arguments)]
pub fn process_destruction(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut virtual_time: ResMut<Time<Virtual>>,
    palette: Res<VfxPalette>,
    shared: Res<SharedMeshes>,
    mut score: ResMut<ScoreState>,
    mut popups: ResMut<PendingPopups>,
    mut camera_rig: ResMut<CameraRig>,
    mut audio: ResMut<RetroAudio>,
    particle_count_q: Query<(), With<VfxParticle>>,
    shard_count_q: Query<(), With<BallisticShard>>,
    paper_count_q: Query<(), With<FlutteringPaper>>,
    children_q: Query<&Children>,
    screen_q: Query<Entity, With<ScreenSurface>>,
    glass_q: Query<(Entity, &GlassSurface)>,
    mut prop_q: Query<(Entity, &Transform, &mut LinearVelocity, &mut DestructibleProp)>,
) {
    let dt = time.delta_secs();
    score.elapsed_since_flip += dt;
    score.score_pulse = (score.score_pulse - dt * 3.5).max(0.0);
    score.hitstop_cooldown = (score.hitstop_cooldown - dt).max(0.0);
    score.slowmo_cooldown = (score.slowmo_cooldown - dt).max(0.0);

    if let Some((_, ref mut ttl)) = score.last_impact_pos {
        *ttl -= dt;
        if *ttl <= 0.0 {
            score.last_impact_pos = None;
        }
    }
    for entry in &mut score.recent_feed {
        entry.age += dt;
    }
    score.recent_feed.retain(|e| e.age < 2.6);

    if score.combo_timer > 0.0 {
        score.combo_timer -= dt;
        if score.combo_timer <= 0.0 {
            score.combo = 0;
        }
    }

    if score.hitstop_timer > 0.0 {
        score.hitstop_timer -= dt;
        virtual_time.set_relative_speed(0.10);
    } else if score.slowmo_timer > 0.0 {
        score.slowmo_timer -= dt;
        virtual_time.set_relative_speed(0.65);
    } else {
        virtual_time.set_relative_speed(1.0);
    }

    let can_score_impacts = score.elapsed_since_flip > 0.06;
    let mut shockwaves: Vec<(Vec3, f32, f32)> = Vec::new();
    let mut decal_seq: u32 = (score.items_destroyed * 7 + score.items_toppled * 3) + 1;

    // Live VFX budget tracking so simultaneous 30-prop explosions stay locked at 60 FPS!
    let mut vfx_budget = 150_usize.saturating_sub(particle_count_q.iter().count());
    let mut shard_budget = 160_usize.saturating_sub(shard_count_q.iter().count());
    let mut paper_budget = 85_usize.saturating_sub(paper_count_q.iter().count());
    let mut popups_this_frame = 0_usize;

    for (entity, tf, lin_vel, mut prop) in &mut prop_q {
        // Stage 1a: Tall prop toppling
        if prop.can_topple && !prop.toppled && !prop.shattered && tf.up().dot(Vec3::Y) < 0.78 {
            prop.toppled = true;
            score.items_toppled += 1;
            let combo = score.bump_combo();
            let pts = ScoreState::chain_scaled_points(95, combo);
            score.topple_points += pts;
            let msg = format!("+{pts} {} TOPPLED!", prop.label);
            score.record_points(pts, msg.clone(), tf.translation);
            score.trigger_slowmo(0.12);
            camera_rig.add_trauma(0.14);
            audio.push(SoundEffect::Topple);
            if popups_this_frame < 4 {
                popups_this_frame += 1;
                popups.0.push(PopupSpawn {
                    world_pos: tf.translation + Vec3::Y * 0.75,
                    text: msg,
                    color: Color::srgb(0.52, 0.92, 0.98),
                    big: false,
                });
            }
        }

        let delta_v = (lin_vel.0 - prop.prev_velocity).length();
        prop.prev_velocity = lin_vel.0;

        if !can_score_impacts {
            continue;
        }

        // Stage 2: Shatter / Burst / Explode!
        if !prop.shattered && delta_v >= prop.break_impulse {
            prop.shattered = true;
            score.items_destroyed += 1;
            decal_seq += 1;
            let combo = score.bump_combo();
            let pts = ScoreState::chain_scaled_points(prop.base_points, combo);
            score.destruction_points += pts;

            let is_blast = prop.profile == ImpactProfile::PressurizedHazard;
            if is_blast {
                score.hazards_detonated += 1;
            }
            if matches!(prop.profile, ImpactProfile::Liquid { .. }) {
                score.liquids_spilled += 1;
            }

            let callout = if is_blast {
                format!("+{pts} {} KABOOM!", prop.label)
            } else if combo > 1 {
                format!("+{pts} {} x{combo}", prop.label)
            } else {
                format!("+{pts} {}", prop.label)
            };
            score.record_points(pts, callout.clone(), tf.translation);
            score.trigger_hitstop(if is_blast { 0.026 } else { 0.018 });
            camera_rig.add_trauma(if is_blast { 0.46 } else { 0.20 });
            audio.push(SoundEffect::Shatter(prop.sfx));

            if is_blast || popups_this_frame < 5 {
                popups_this_frame += 1;
                popups.0.push(PopupSpawn {
                    world_pos: tf.translation + Vec3::Y * 0.50,
                    text: callout,
                    color: if is_blast {
                        Color::srgb(1.0, 0.42, 0.24)
                    } else {
                        Color::srgb(0.99, 0.80, 0.24)
                    },
                    big: true,
                });
            }

            if is_blast {
                shockwaves.push((tf.translation, 3.25, 12.8));
            }

            spawn_shatter_spectacle(
                &mut commands,
                &palette,
                &shared,
                tf.translation,
                lin_vel.0,
                &prop,
                decal_seq,
                &mut vfx_budget,
                &mut shard_budget,
                &mut paper_budget,
            );

            commands.entity(entity).despawn();
        } else if !prop.damaged && !prop.shattered && delta_v >= prop.damage_impulse {
            // Stage 1b: Visible Cracked / Leaking / Sparking State!
            prop.damaged = true;
            let combo = score.combo.max(1);
            let pts = ScoreState::chain_scaled_points(65, combo);
            score.destruction_points += pts;
            let verb = match prop.profile {
                ImpactProfile::Liquid { .. } => "SPILLED!",
                ImpactProfile::PaperStack { .. } => "RIF'D!",
                ImpactProfile::PressurizedHazard => "RUPTURED!",
                _ => "CRACKED!",
            };
            let msg = format!("+{pts} {} {verb}", prop.label);
            score.record_points(pts, msg.clone(), tf.translation);
            camera_rig.add_trauma(0.09);
            audio.push(SoundEffect::Crack);
            if popups_this_frame < 4 {
                popups_this_frame += 1;
                popups.0.push(PopupSpawn {
                    world_pos: tf.translation + Vec3::Y * 0.38,
                    text: msg,
                    color: Color::srgb(0.98, 0.52, 0.36),
                    big: false,
                });
            }

            apply_cracked_visuals(
                entity,
                tf.translation,
                &prop,
                &mut commands,
                &palette,
                &shared,
                &children_q,
                &screen_q,
                &glass_q,
                &mut vfx_budget,
                &mut paper_budget,
            );
        }
    }

    if !shockwaves.is_empty() {
        for (_e, tf, mut vel, prop) in &mut prop_q {
            if prop.shattered {
                continue;
            }
            for &(origin, radius, strength) in &shockwaves {
                let offset = tf.translation - origin;
                let dist = offset.length();
                if dist > 0.05 && dist < radius {
                    let falloff = (1.0 - dist / radius).powf(0.75);
                    let dir = (offset.normalize_or_zero() + Vec3::Y * 0.55).normalize_or_zero();
                    vel.0 += dir * (strength * falloff);
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_cracked_visuals(
    entity: Entity,
    world_pos: Vec3,
    prop: &DestructibleProp,
    commands: &mut Commands,
    palette: &VfxPalette,
    shared: &SharedMeshes,
    children_q: &Query<&Children>,
    screen_q: &Query<Entity, With<ScreenSurface>>,
    glass_q: &Query<(Entity, &GlassSurface)>,
    vfx_budget: &mut usize,
    paper_budget: &mut usize,
) {
    if let Ok(children) = children_q.get(entity) {
        for child in children.iter() {
            if screen_q.get(child).is_ok() {
                commands
                    .entity(child)
                    .insert(MeshMaterial3d(palette.cracked_screen.clone()));
            }
            if let Ok((_, glass)) = glass_q.get(child) {
                commands.entity(child).with_children(|g| {
                    g.spawn((
                        NotShadowCaster,
                        Mesh3d(shared.unit_cube.clone()),
                        MeshMaterial3d(palette.spiderweb_crack.clone()),
                        Transform::from_xyz(0.0, 0.0, glass.z_offset).with_scale(Vec3::new(
                            glass.size.x * 0.85,
                            glass.size.y * 0.85,
                            0.005,
                        )),
                    ));
                });
            }
        }
    }

    match prop.profile {
        ImpactProfile::Electronics => {
            commands.entity(entity).insert(VfxEmitter {
                timer: 0.0,
                interval: 0.18,
                offset: Vec3::Y * 0.18,
                style: EmitterStyle::DamagedSparksAndSmoke,
            });
            spawn_spark_burst(commands, palette, shared, world_pos + Vec3::Y * 0.16, 4, vfx_budget);
        }
        ImpactProfile::PressurizedHazard => {
            commands.entity(entity).insert(VfxEmitter {
                timer: 0.0,
                interval: 0.11,
                offset: Vec3::Y * 0.25,
                style: EmitterStyle::PressurizedHiss,
            });
            spawn_spark_burst(commands, palette, shared, world_pos + Vec3::Y * 0.22, 4, vfx_budget);
        }
        ImpactProfile::Liquid { liquid_color, .. } => {
            let (drop_mat, _) = palette.liquid_handles(liquid_color);
            spawn_liquid_droplets(commands, drop_mat, shared, world_pos + Vec3::Y * 0.14, 5, vfx_budget);
        }
        ImpactProfile::PaperStack { is_blueprint } => {
            spawn_fluttering_sheets(
                commands,
                palette,
                shared,
                world_pos + Vec3::Y * 0.12,
                Vec3::Y * 1.2,
                3,
                is_blueprint,
                paper_budget,
            );
        }
        ImpactProfile::GlassCrystal | ImpactProfile::FilingCabinet | ImpactProfile::HeavyProp => {
            spawn_dust_puff_ring(commands, palette, shared, world_pos + Vec3::Y * 0.12, 3, vfx_budget);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_shatter_spectacle(
    commands: &mut Commands,
    palette: &VfxPalette,
    shared: &SharedMeshes,
    pos: Vec3,
    base_vel: Vec3,
    prop: &DestructibleProp,
    seq: u32,
    vfx_budget: &mut usize,
    shard_budget: &mut usize,
    paper_budget: &mut usize,
) {
    match prop.profile {
        ImpactProfile::Electronics => {
            if seq.is_multiple_of(2) {
                spawn_flash_light(commands, pos + Vec3::Y * 0.28, Color::srgb(0.32, 0.98, 0.92), 42_000.0, 0.18);
            }
            spawn_spark_burst(commands, palette, shared, pos + Vec3::Y * 0.18, 10, vfx_budget);
            spawn_smoke_column(commands, palette, shared, pos + Vec3::Y * 0.15, 4, vfx_budget);
            spawn_floor_decal(
                commands,
                shared,
                palette.scorch_decal.clone(),
                Vec3::new(pos.x, 0.006 + (seq % 7) as f32 * 0.0004, pos.z),
                0.90,
            );
            let m0 = palette.shard_handle_for(prop.debris_color);
            spawn_ballistic_shards(
                commands,
                shared,
                pos,
                base_vel,
                7,
                &[m0, palette.shard_phosphor.clone(), palette.shard_pcb.clone()],
                4.2,
                shard_budget,
            );
        }

        ImpactProfile::PressurizedHazard => {
            spawn_flash_light(commands, pos + Vec3::Y * 0.4, Color::srgb(1.0, 0.58, 0.18), 85_000.0, 0.28);
            spawn_fireball_explosion(commands, palette, shared, pos + Vec3::Y * 0.22, vfx_budget);
            spawn_spark_burst(commands, palette, shared, pos + Vec3::Y * 0.25, 12, vfx_budget);
            spawn_smoke_column(commands, palette, shared, pos + Vec3::Y * 0.25, 7, vfx_budget);
            spawn_floor_decal(
                commands,
                shared,
                palette.scorch_decal.clone(),
                Vec3::new(pos.x, 0.008 + (seq % 7) as f32 * 0.0004, pos.z),
                1.85,
            );
            let m1 = palette.shard_handle_for(prop.debris_color);
            let m2 = palette.shard_handle_for(prop.secondary_color);
            spawn_ballistic_shards(commands, shared, pos, base_vel, 10, &[m1, m2], 6.0, shard_budget);
        }

        ImpactProfile::Liquid {
            liquid_color,
            puddle_radius,
        } => {
            let (drop_mat, puddle_mat) = palette.liquid_handles(liquid_color);
            spawn_liquid_droplets(commands, drop_mat, shared, pos + Vec3::Y * 0.16, 12, vfx_budget);
            spawn_floor_decal(
                commands,
                shared,
                puddle_mat,
                Vec3::new(pos.x, 0.007 + (seq % 9) as f32 * 0.0005, pos.z),
                puddle_radius * 2.0,
            );
            let m1 = palette.shard_handle_for(prop.debris_color);
            spawn_ballistic_shards(commands, shared, pos, base_vel, 6, &[m1], 3.4, shard_budget);
        }

        ImpactProfile::PaperStack { is_blueprint } => {
            spawn_fluttering_sheets(
                commands,
                palette,
                shared,
                pos + Vec3::Y * 0.14,
                base_vel,
                12,
                is_blueprint,
                paper_budget,
            );
            spawn_dust_puff_ring(commands, palette, shared, pos + Vec3::Y * 0.10, 3, vfx_budget);
        }

        ImpactProfile::GlassCrystal => {
            spawn_dust_puff_ring(commands, palette, shared, pos + Vec3::Y * 0.2, 4, vfx_budget);
            let m2 = palette.shard_handle_for(prop.secondary_color);
            spawn_ballistic_shards(
                commands,
                shared,
                pos,
                base_vel,
                9,
                &[palette.shard_glass.clone(), m2],
                4.4,
                shard_budget,
            );
        }

        ImpactProfile::FilingCabinet => {
            spawn_ballistic_shards(
                commands,
                shared,
                pos,
                base_vel,
                7,
                &[palette.shard_olive.clone(), palette.shard_silver.clone()],
                3.6,
                shard_budget,
            );
            spawn_fluttering_sheets(
                commands,
                palette,
                shared,
                pos + Vec3::Y * 0.25,
                base_vel,
                10,
                false,
                paper_budget,
            );
            spawn_dust_puff_ring(commands, palette, shared, pos + Vec3::Y * 0.2, 4, vfx_budget);
        }

        ImpactProfile::HeavyProp => {
            let m1 = palette.shard_handle_for(prop.debris_color);
            let m2 = palette.shard_handle_for(prop.secondary_color);
            spawn_dust_puff_ring(commands, palette, shared, pos + Vec3::Y * 0.16, 4, vfx_budget);
            spawn_ballistic_shards(commands, shared, pos, base_vel, 7, &[m1, m2], 3.8, shard_budget);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_ballistic_shards(
    commands: &mut Commands,
    shared: &SharedMeshes,
    pos: Vec3,
    base_vel: Vec3,
    desired: usize,
    mats: &[Handle<StandardMaterial>],
    radial_speed: f32,
    budget: &mut usize,
) {
    let count = desired.min(*budget);
    *budget = budget.saturating_sub(count);
    for i in 0..count {
        let a = (i as f32 / count.max(1) as f32) * std::f32::consts::TAU + (i as f32 * 0.37);
        let up = 2.4 + (i % 4) as f32 * 0.95;
        let vel = base_vel * 0.38 + Vec3::new(a.cos() * radial_speed, up, a.sin() * radial_speed);
        let mesh = if i % 2 == 0 {
            shared.shard_wedge.clone()
        } else {
            shared.shard_plate.clone()
        };
        let scale = 0.78 + (i % 3) as f32 * 0.30;
        commands.spawn((
            ScenarioEntity,
            NotShadowCaster,
            BallisticShard {
                vel,
                spin: Vec3::new(a * 4.5, 6.2, -a * 3.9),
                age: 0.0,
                settled: false,
            },
            Mesh3d(mesh),
            MeshMaterial3d(mats[i % mats.len()].clone()),
            Transform::from_translation(pos + Vec3::Y * 0.16).with_scale(Vec3::splat(scale)),
        ));
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_fluttering_sheets(
    commands: &mut Commands,
    palette: &VfxPalette,
    shared: &SharedMeshes,
    pos: Vec3,
    base_vel: Vec3,
    desired: usize,
    is_blueprint: bool,
    budget: &mut usize,
) {
    let count = desired.min(*budget);
    *budget = budget.saturating_sub(count);
    let sheet_mat = if is_blueprint {
        palette.sheet_blueprint.clone()
    } else {
        palette.sheet_paper.clone()
    };

    for i in 0..count {
        let a = (i as f32 / count.max(1) as f32) * std::f32::consts::TAU + (i as f32 * 0.51);
        let speed = 1.6 + (i % 5) as f32 * 0.55;
        let up = 2.6 + (i % 4) as f32 * 0.85;
        commands.spawn((
            ScenarioEntity,
            NotShadowCaster,
            FlutteringPaper {
                age: 0.0,
                vel: base_vel * 0.30 + Vec3::new(a.cos() * speed, up, a.sin() * speed),
                phase: i as f32 * 1.13,
                sway_freq: 4.8 + (i % 4) as f32 * 0.9,
                settled: false,
                floor_y: 0.010 + (i % 11) as f32 * 0.0012,
            },
            Mesh3d(shared.paper_sheet.clone()),
            MeshMaterial3d(sheet_mat.clone()),
            Transform::from_translation(pos + Vec3::Y * (0.05 * (i % 4) as f32))
                .with_rotation(Quat::from_euler(EulerRot::XYZ, 0.3, a, -0.2)),
        ));
    }
}

fn spawn_spark_burst(
    commands: &mut Commands,
    palette: &VfxPalette,
    shared: &SharedMeshes,
    pos: Vec3,
    desired: usize,
    budget: &mut usize,
) {
    let count = desired.min(*budget);
    *budget = budget.saturating_sub(count);
    for i in 0..count {
        let a = (i as f32 / count.max(1) as f32) * std::f32::consts::TAU + i as f32 * 0.4;
        let spd = 3.2 + (i % 4) as f32 * 1.3;
        commands.spawn((
            ScenarioEntity,
            NotShadowCaster,
            VfxParticle {
                age: 0.0,
                duration: 0.32 + (i % 3) as f32 * 0.07,
                velocity: Vec3::new(a.cos() * spd, 2.6 + (i % 3) as f32 * 1.2, a.sin() * spd),
                gravity: -12.5,
                drag: 2.2,
                start_scale: 0.065,
                peak_scale: 0.085,
                spin: Vec3::new(9.0, 7.0, 5.0),
            },
            Mesh3d(shared.unit_cube.clone()),
            MeshMaterial3d(if i % 2 == 0 {
                palette.spark_cyan.clone()
            } else {
                palette.spark_gold.clone()
            }),
            Transform::from_translation(pos).with_scale(Vec3::splat(0.065)),
        ));
    }
}

fn spawn_smoke_column(
    commands: &mut Commands,
    palette: &VfxPalette,
    shared: &SharedMeshes,
    pos: Vec3,
    desired: usize,
    budget: &mut usize,
) {
    let count = desired.min(*budget);
    *budget = budget.saturating_sub(count);
    for i in 0..count {
        let a = (i as f32 / count.max(1) as f32) * std::f32::consts::TAU;
        commands.spawn((
            ScenarioEntity,
            NotShadowCaster,
            VfxParticle {
                age: 0.0,
                duration: 0.78 + (i % 3) as f32 * 0.18,
                velocity: Vec3::new(a.cos() * 0.9, 1.5 + (i % 3) as f32 * 0.65, a.sin() * 0.9),
                gravity: 0.6,
                drag: 1.8,
                start_scale: 0.12,
                peak_scale: 0.38,
                spin: Vec3::new(2.0, 3.0, 1.5),
            },
            Mesh3d(shared.unit_cube.clone()),
            MeshMaterial3d(palette.smoke.clone()),
            Transform::from_translation(pos).with_scale(Vec3::splat(0.12)),
        ));
    }
}

fn spawn_fireball_explosion(
    commands: &mut Commands,
    palette: &VfxPalette,
    shared: &SharedMeshes,
    pos: Vec3,
    budget: &mut usize,
) {
    if *budget == 0 {
        return;
    }
    *budget = budget.saturating_sub(1);
    commands.spawn((
        ScenarioEntity,
        NotShadowCaster,
        VfxParticle {
            age: 0.0,
            duration: 0.26,
            velocity: Vec3::Y * 0.5,
            gravity: 0.0,
            drag: 2.0,
            start_scale: 0.25,
            peak_scale: 1.30,
            spin: Vec3::new(5.0, 8.0, 3.0),
        },
        Mesh3d(shared.unit_cube.clone()),
        MeshMaterial3d(palette.fire_core.clone()),
        Transform::from_translation(pos).with_scale(Vec3::splat(0.25)),
    ));

    let count = 10_usize.min(*budget);
    *budget = budget.saturating_sub(count);
    for i in 0..count {
        let a = (i as f32 / count.max(1) as f32) * std::f32::consts::TAU;
        let spd = 4.4 + (i % 3) as f32 * 1.5;
        commands.spawn((
            ScenarioEntity,
            NotShadowCaster,
            VfxParticle {
                age: 0.0,
                duration: 0.44,
                velocity: Vec3::new(a.cos() * spd, 1.8 + (i % 4) as f32 * 0.9, a.sin() * spd),
                gravity: -4.0,
                drag: 3.2,
                start_scale: 0.18,
                peak_scale: 0.48,
                spin: Vec3::new(6.0, 5.0, 4.0),
            },
            Mesh3d(shared.unit_cube.clone()),
            MeshMaterial3d(if i % 3 == 0 {
                palette.foam_white.clone()
            } else {
                palette.fire_flame.clone()
            }),
            Transform::from_translation(pos).with_scale(Vec3::splat(0.18)),
        ));
    }
}

fn spawn_liquid_droplets(
    commands: &mut Commands,
    drop_mat: Handle<StandardMaterial>,
    shared: &SharedMeshes,
    pos: Vec3,
    desired: usize,
    budget: &mut usize,
) {
    let count = desired.min(*budget);
    *budget = budget.saturating_sub(count);
    for i in 0..count {
        let a = (i as f32 / count.max(1) as f32) * std::f32::consts::TAU + i as f32 * 0.29;
        let spd = 2.0 + (i % 5) as f32 * 0.85;
        commands.spawn((
            ScenarioEntity,
            NotShadowCaster,
            VfxParticle {
                age: 0.0,
                duration: 0.50 + (i % 3) as f32 * 0.10,
                velocity: Vec3::new(a.cos() * spd, 2.8 + (i % 4) as f32 * 0.95, a.sin() * spd),
                gravity: -13.5,
                drag: 1.1,
                start_scale: 0.075,
                peak_scale: 0.11,
                spin: Vec3::new(7.0, 6.0, 5.0),
            },
            Mesh3d(shared.unit_cube.clone()),
            MeshMaterial3d(drop_mat.clone()),
            Transform::from_translation(pos).with_scale(Vec3::splat(0.075)),
        ));
    }
}

fn spawn_dust_puff_ring(
    commands: &mut Commands,
    palette: &VfxPalette,
    shared: &SharedMeshes,
    pos: Vec3,
    desired: usize,
    budget: &mut usize,
) {
    let count = desired.min(*budget);
    *budget = budget.saturating_sub(count);
    for i in 0..count {
        let a = (i as f32 / count.max(1) as f32) * std::f32::consts::TAU;
        commands.spawn((
            ScenarioEntity,
            NotShadowCaster,
            VfxParticle {
                age: 0.0,
                duration: 0.38,
                velocity: Vec3::new(a.cos() * 2.1, 1.1 + (i % 2) as f32 * 0.5, a.sin() * 2.1),
                gravity: -1.5,
                drag: 3.4,
                start_scale: 0.10,
                peak_scale: 0.26,
                spin: Vec3::new(4.0, 5.0, 3.0),
            },
            Mesh3d(shared.unit_cube.clone()),
            MeshMaterial3d(if i % 2 == 0 {
                palette.dust_tan.clone()
            } else {
                palette.dust_grey.clone()
            }),
            Transform::from_translation(pos).with_scale(Vec3::splat(0.10)),
        ));
    }
}

fn spawn_floor_decal(
    commands: &mut Commands,
    shared: &SharedMeshes,
    mat: Handle<StandardMaterial>,
    floor_pos: Vec3,
    target_scale: f32,
) {
    commands.spawn((
        ScenarioEntity,
        NotShadowCaster,
        FloorDecal {
            age: 0.0,
            target_scale,
        },
        Mesh3d(shared.floor_quad.clone()),
        MeshMaterial3d(mat),
        Transform::from_translation(floor_pos)
            .with_rotation(Quat::from_rotation_y(floor_pos.x * 1.9 + floor_pos.z))
            .with_scale(Vec3::splat(0.12)),
    ));
}

fn spawn_flash_light(
    commands: &mut Commands,
    pos: Vec3,
    color: Color,
    intensity: f32,
    duration: f32,
) {
    commands.spawn((
        ScenarioEntity,
        ExplosionFlashLight {
            age: 0.0,
            duration,
            start_intensity: intensity,
        },
        PointLight {
            color,
            intensity,
            range: 6.2,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_translation(pos),
    ));
}

pub fn update_vfx_emitters(
    mut commands: Commands,
    time: Res<Time>,
    palette: Res<VfxPalette>,
    shared: Res<SharedMeshes>,
    particle_count_q: Query<(), With<VfxParticle>>,
    mut q: Query<(&GlobalTransform, &mut VfxEmitter)>,
) {
    let dt = time.delta_secs();
    let t_now = time.elapsed_secs();
    let mut budget = 140_usize.saturating_sub(particle_count_q.iter().count());

    for (gtf, mut emitter) in &mut q {
        emitter.timer -= dt;
        if emitter.timer > 0.0 || budget == 0 {
            continue;
        }
        emitter.timer = emitter.interval;
        let world_pos = gtf.translation() + emitter.offset;

        match emitter.style {
            EmitterStyle::CoffeeSteam => {
                budget = budget.saturating_sub(1);
                let sway = (t_now * 4.3 + world_pos.x * 5.0).sin() * 0.10;
                commands.spawn((
                    ScenarioEntity,
                    NotShadowCaster,
                    VfxParticle {
                        age: 0.0,
                        duration: 0.90,
                        velocity: Vec3::new(sway, 0.30, 0.0),
                        gravity: 0.05,
                        drag: 0.8,
                        start_scale: 0.014,
                        peak_scale: 0.025,
                        spin: Vec3::new(0.6, 1.2, 0.4),
                    },
                    Mesh3d(shared.unit_cube.clone()),
                    MeshMaterial3d(palette.steam_wisp.clone()),
                    Transform::from_translation(world_pos).with_scale(Vec3::splat(0.014)),
                ));
            }
            EmitterStyle::DamagedSparksAndSmoke => {
                spawn_spark_burst(&mut commands, &palette, &shared, world_pos, 2, &mut budget);
                spawn_smoke_column(&mut commands, &palette, &shared, world_pos, 1, &mut budget);
            }
            EmitterStyle::PressurizedHiss => {
                budget = budget.saturating_sub(1);
                commands.spawn((
                    ScenarioEntity,
                    NotShadowCaster,
                    VfxParticle {
                        age: 0.0,
                        duration: 0.32,
                        velocity: Vec3::new(
                            (t_now * 17.0).sin() * 1.4,
                            2.4,
                            (t_now * 13.0).cos() * 1.4,
                        ),
                        gravity: -1.0,
                        drag: 2.5,
                        start_scale: 0.05,
                        peak_scale: 0.16,
                        spin: Vec3::splat(5.0),
                    },
                    Mesh3d(shared.unit_cube.clone()),
                    MeshMaterial3d(palette.hiss_jet.clone()),
                    Transform::from_translation(world_pos).with_scale(Vec3::splat(0.05)),
                ));
            }
        }
    }
}

type DecalQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut FloorDecal, &'static mut Transform),
    (Without<VfxParticle>, Without<BallisticShard>),
>;

pub fn update_vfx_particles(
    mut commands: Commands,
    time: Res<Time>,
    mut p_q: Query<(Entity, &mut VfxParticle, &mut Transform)>,
    mut shard_q: Query<(&mut BallisticShard, &mut Transform), Without<VfxParticle>>,
    mut decal_q: DecalQuery,
    mut light_q: Query<(Entity, &mut ExplosionFlashLight, &mut PointLight)>,
) {
    let dt = time.delta_secs();

    for (entity, mut p, mut tf) in &mut p_q {
        p.age += dt;
        let t = (p.age / p.duration).clamp(0.0, 1.0);
        let damp = (-p.drag * dt).exp();
        p.velocity *= damp;
        p.velocity.y += p.gravity * dt;
        tf.translation += p.velocity * dt;
        if tf.translation.y < 0.02 && p.gravity < -5.0 {
            tf.translation.y = 0.02;
            p.velocity.y = p.velocity.y.abs() * 0.25;
        }
        tf.rotate_x(p.spin.x * dt);
        tf.rotate_y(p.spin.y * dt);
        tf.rotate_z(p.spin.z * dt);

        let scale = if t < 0.28 {
            p.start_scale + (t / 0.28) * (p.peak_scale - p.start_scale)
        } else {
            p.peak_scale * (1.0 - (t - 0.28) / 0.72)
        };
        tf.scale = Vec3::splat(scale.max(0.005));

        if p.age >= p.duration {
            commands.entity(entity).despawn();
        }
    }

    // Ultra-fast kinematic floor/wall-bouncing shards (zero narrowphase physics overhead!)
    for (mut shard, mut tf) in &mut shard_q {
        if shard.settled {
            continue;
        }
        shard.age += dt;
        shard.vel.y -= 13.8 * dt;
        shard.vel.x *= (-0.85 * dt).exp();
        shard.vel.z *= (-0.85 * dt).exp();
        tf.translation += shard.vel * dt;

        // Room wall bounce
        if tf.translation.x.abs() > 5.02 {
            tf.translation.x = tf.translation.x.signum() * 5.02;
            shard.vel.x = -shard.vel.x * 0.45;
        }
        if tf.translation.z < -6.02 {
            tf.translation.z = -6.02;
            shard.vel.z = -shard.vel.z * 0.45;
        }

        // Floor bounce & settle
        if tf.translation.y <= 0.028 {
            tf.translation.y = 0.028;
            if shard.vel.y.abs() > 1.25 && shard.age < 2.4 {
                shard.vel.y = -shard.vel.y * 0.36;
                shard.vel.x *= 0.65;
                shard.vel.z *= 0.65;
                shard.spin *= 0.55;
            } else {
                shard.settled = true;
                let (yaw, _, _) = tf.rotation.to_euler(EulerRot::YXZ);
                tf.rotation = Quat::from_rotation_y(yaw);
                continue;
            }
        }

        tf.rotate_x(shard.spin.x * dt);
        tf.rotate_y(shard.spin.y * dt);
        tf.rotate_z(shard.spin.z * dt);
    }

    for (mut decal, mut tf) in &mut decal_q {
        if decal.age < 0.30 {
            decal.age += dt;
            let k = (decal.age / 0.30).clamp(0.0, 1.0);
            let eased = 1.0 - (1.0 - k) * (1.0 - k);
            let s = 0.12 + eased * (decal.target_scale - 0.12);
            tf.scale = Vec3::new(s, 1.0, s);
        }
    }

    for (entity, mut fl, mut light) in &mut light_q {
        fl.age += dt;
        let t = (fl.age / fl.duration).clamp(0.0, 1.0);
        light.intensity = fl.start_intensity * (1.0 - t) * (1.0 - t);
        if fl.age >= fl.duration {
            commands.entity(entity).despawn();
        }
    }
}

pub fn update_fluttering_papers(
    time: Res<Time>,
    mut q: Query<(&mut FlutteringPaper, &mut Transform)>,
) {
    let dt = time.delta_secs();
    for (mut paper, mut tf) in &mut q {
        if paper.settled {
            continue;
        }
        paper.age += dt;

        paper.vel.x *= (-2.6 * dt).exp();
        paper.vel.z *= (-2.6 * dt).exp();
        paper.vel.y = (paper.vel.y - 6.2 * dt).max(-1.35);

        let sway_phase = paper.age * paper.sway_freq + paper.phase;
        let lateral = Vec3::new(
            sway_phase.sin() * 1.15,
            (sway_phase * 2.0).cos() * 0.22,
            (sway_phase * 0.78).cos() * 1.15,
        );
        tf.translation += (paper.vel + lateral) * dt;
        tf.translation.x = tf.translation.x.clamp(-5.0, 5.0);
        tf.translation.z = tf.translation.z.clamp(-6.0, 3.0);

        let pitch = sway_phase.cos() * 0.48;
        let roll = sway_phase.sin() * 0.55;
        let yaw = paper.phase + paper.age * 1.4;
        tf.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);

        if tf.translation.y <= paper.floor_y {
            tf.translation.y = paper.floor_y;
            tf.rotation = Quat::from_rotation_y(yaw);
            paper.settled = true;
        }
    }
}

pub fn update_debris_lifetime(
    mut commands: Commands,
    time: Res<Time>,
    shared: Res<SharedMeshes>,
    mut q: Query<(Entity, &Transform, &mut DebrisChunk, Option<&LinearVelocity>)>,
) {
    let dt = time.delta_secs();
    for (entity, tf, mut chunk, lin_vel) in &mut q {
        chunk.age += dt;
        let slow = lin_vel.map(|v| v.0.length() < 0.20).unwrap_or(true);
        if (chunk.age > 2.0 && slow) || chunk.age > 4.0 {
            let final_tf = *tf;
            let mat = chunk.mat.clone();
            let outline_thickness = chunk.outline_thickness;
            commands.entity(entity).despawn();
            commands
                .spawn((
                    ScenarioEntity,
                    Mesh3d(shared.unit_cube.clone()),
                    MeshMaterial3d(mat),
                    final_tf,
                ))
                .with_children(|c| {
                    c.spawn((
                        NotShadowCaster,
                        Mesh3d(shared.unit_cube.clone()),
                        MeshMaterial3d(shared.ink_outline_mat.clone()),
                        Transform::from_scale(outline_scale_for(final_tf.scale, outline_thickness)),
                    ));
                });
        }
    }
}

/// Breaks the player's desk apart in 3 progressive impact stages with ZERO runtime mesh/material allocations!
#[allow(clippy::too_many_arguments)]
pub fn process_desk_breakup(
    mut commands: Commands,
    palette: Res<VfxPalette>,
    shared: Res<SharedMeshes>,
    mut score: ResMut<ScoreState>,
    mut popups: ResMut<PendingPopups>,
    mut camera_rig: ResMut<CameraRig>,
    mut audio: ResMut<RetroAudio>,
    mut desk_q: Query<
        (Entity, &Transform, &LinearVelocity, &mut BreakableDesk, &Children),
        With<PlayerDesk>,
    >,
    part_q: Query<&DeskBreakoffPart>,
    mut top_vis_q: Query<&mut Transform, (With<DeskTopVisual>, Without<PlayerDesk>)>,
) {
    if score.elapsed_since_flip < 0.10 {
        for (_, _, lv, mut bd, _) in &mut desk_q {
            bd.prev_vel = lv.0;
        }
        return;
    }

    let mut shard_budget = 24_usize;
    let mut paper_budget = 10_usize;

    for (desk_entity, desk_tf, lin_vel, mut bd, children) in &mut desk_q {
        let delta_v = (lin_vel.0 - bd.prev_vel).length();
        bd.prev_vel = lin_vel.0;

        let target_stage = if delta_v >= 7.0 {
            3
        } else if delta_v >= 4.4 {
            bd.stage.max(2)
        } else if delta_v >= 2.3 {
            bd.stage.max(1)
        } else {
            bd.stage
        };

        if target_stage <= bd.stage {
            continue;
        }

        let prev_stage = bd.stage;
        bd.stage = target_stage;
        score.desk_damage_stage = score.desk_damage_stage.max(target_stage);

        for child in children.iter() {
            if let Ok(part) = part_q.get(child)
                && part.trigger_stage > prev_stage
                && part.trigger_stage <= target_stage
            {
                let world_p = desk_tf.transform_point(part.local_offset);
                let world_eject = desk_tf.rotation * part.eject_dir;

                commands
                    .spawn((
                        ScenarioEntity,
                        DebrisChunk {
                            age: 0.0,
                            mat: part.mat.clone(),
                            outline_thickness: 0.011,
                        },
                        RigidBody::Dynamic,
                        CollisionLayers::from_bits(LAYER_DEBRIS, LAYER_DEFAULT),
                        Mass(1.6),
                        Restitution::new(0.44),
                        Friction::new(0.32),
                        LinearDamping(0.14),
                        AngularDamping(0.16),
                        Collider::cuboid(part.size.x, part.size.y, part.size.z),
                        Mesh3d(shared.unit_cube.clone()),
                        MeshMaterial3d(part.mat.clone()),
                        Transform::from_translation(world_p)
                            .with_rotation(desk_tf.rotation)
                            .with_scale(part.size),
                        LinearVelocity(lin_vel.0 * 0.55 + world_eject),
                        AngularVelocity(Vec3::new(3.8, -4.4, 3.2)),
                    ))
                    .with_children(|c| {
                        c.spawn((
                            NotShadowCaster,
                            Mesh3d(shared.unit_cube.clone()),
                            MeshMaterial3d(shared.ink_outline_mat.clone()),
                            Transform::from_scale(outline_scale_for(part.size, 0.011)),
                        ));
                    });

                commands.entity(child).despawn();
            }
        }

        let m1 = palette.shard_handle_for(bd.wood_color);
        let m2 = palette.shard_handle_for(bd.leg_color);
        spawn_ballistic_shards(
            &mut commands,
            &shared,
            desk_tf.translation + Vec3::Y * 0.25,
            lin_vel.0,
            5 + (target_stage as usize) * 2,
            &[m1, m2],
            3.6,
            &mut shard_budget,
        );

        if prev_stage == 0 && bd.spill_papers {
            spawn_fluttering_sheets(
                &mut commands,
                &palette,
                &shared,
                desk_tf.translation + Vec3::Y * 0.2,
                lin_vel.0,
                6,
                false,
                &mut paper_budget,
            );
        }

        if target_stage >= 3 && prev_stage < 3 {
            let half_size = Vec3::new(bd.top_size.x * 0.48, bd.top_size.y, bd.top_size.z);
            let quarter_x = bd.top_size.x * 0.25;

            for child in children.iter() {
                if let Ok(mut vis_tf) = top_vis_q.get_mut(child) {
                    vis_tf.translation.x = -quarter_x;
                    vis_tf.scale.x *= 0.48;
                }
            }

            commands.entity(desk_entity).insert(Collider::compound(vec![(
                Vec3::new(-quarter_x, 0.34, 0.0),
                Quat::IDENTITY,
                Collider::cuboid(half_size.x, half_size.y, half_size.z),
            )]));

            let world_right = desk_tf.transform_point(Vec3::new(quarter_x, 0.34, 0.0));
            let right_kick = desk_tf.rotation * Vec3::new(3.4, 2.2, -1.2);
            commands
                .spawn((
                    ScenarioEntity,
                    DebrisChunk {
                        age: 0.0,
                        mat: bd.top_mat.clone(),
                        outline_thickness: 0.014,
                    },
                    RigidBody::Dynamic,
                    CollisionLayers::from_bits(LAYER_DEBRIS, LAYER_DEFAULT),
                    Mass(6.5),
                    Restitution::new(0.42),
                    Friction::new(0.34),
                    LinearDamping(0.14),
                    AngularDamping(0.16),
                    Collider::cuboid(half_size.x, half_size.y, half_size.z),
                    Mesh3d(shared.unit_cube.clone()),
                    MeshMaterial3d(bd.top_mat.clone()),
                    Transform::from_translation(world_right)
                        .with_rotation(desk_tf.rotation)
                        .with_scale(half_size),
                    LinearVelocity(lin_vel.0 * 0.65 + right_kick),
                    AngularVelocity(Vec3::new(-4.5, 3.2, -5.2)),
                ))
                .with_children(|h| {
                    h.spawn((
                        NotShadowCaster,
                        Mesh3d(shared.unit_cube.clone()),
                        MeshMaterial3d(shared.ink_outline_mat.clone()),
                        Transform::from_scale(outline_scale_for(half_size, 0.014)),
                    ));
                });
        } else if target_stage >= 2 && prev_stage < 2 {
            let ts = bd.top_size;
            commands.entity(desk_entity).insert(Collider::compound(vec![(
                Vec3::new(0.0, 0.34, 0.0),
                Quat::IDENTITY,
                Collider::cuboid(ts.x, ts.y, ts.z),
            )]));
        }

        let (pts, callout) = match target_stage {
            1 => (250, "+250 DESK DOWNSIZED!"),
            2 => (480, "+480 BUDGET LEGS CUT!"),
            _ => (750, "+750 RESTRUCTURED IN HALF!!"),
        };
        score.destruction_points += pts;
        score.record_points(pts, callout.to_string(), desk_tf.translation);
        score.trigger_hitstop(if target_stage >= 3 { 0.026 } else { 0.018 });
        camera_rig.add_trauma(0.14 + target_stage as f32 * 0.09);
        audio.push(SoundEffect::DeskBreak(target_stage));
        popups.0.push(PopupSpawn {
            world_pos: desk_tf.translation + Vec3::Y * 0.55,
            text: callout.to_string(),
            color: Color::srgb(1.0, 0.64, 0.24),
            big: target_stage >= 2,
        });
    }
}

type SettlingVelocityQuery<'w, 's> = Query<
    'w,
    's,
    &'static mut LinearVelocity,
    Or<(With<DestructibleProp>, With<PlayerDesk>)>,
>;

pub fn check_run_settled(
    time: Res<Time<Real>>,
    mut virtual_time: ResMut<Time<Virtual>>,
    mut score: ResMut<ScoreState>,
    mut vel_q: SettlingVelocityQuery,
    mut next_state: ResMut<NextState<GamePhase>>,
) {
    let dt = time.delta_secs();
    if score.elapsed_since_flip < 0.70 {
        return;
    }

    if score.elapsed_since_flip > 2.2 {
        let extra_damp = (-2.5 * dt).exp();
        for mut v in &mut vel_q {
            v.0 *= extra_damp;
        }
    }

    let max_speed = vel_q.iter().map(|v| v.0.length()).fold(0.0_f32, f32::max);
    let threshold = if score.elapsed_since_flip > 2.5 {
        0.98
    } else {
        0.58
    };

    if max_speed < threshold {
        score.settle_timer += dt;
    } else {
        score.settle_timer = 0.0;
    }

    if score.settle_timer >= 0.34 || score.elapsed_since_flip >= 4.7 {
        virtual_time.set_relative_speed(1.0);
        score.finalize_bonuses();
        next_state.set(GamePhase::Settled);
    }
}
