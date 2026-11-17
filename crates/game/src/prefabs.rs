use std::collections::HashMap;

use avian3d::prelude::*;
use bevy::light::NotShadowCaster;
use bevy::math::Affine2;
use bevy::prelude::*;
use bevy::render::render_resource::Face;

use crate::meshes::{
    make_floor_quad_mesh, make_frustum_y_mesh, make_frustum_z_mesh, make_upright_quad_mesh,
    outline_scale_for, SharedMeshes,
};
use crate::palette::CorporateJokeRng;
use crate::pixel_art::ProceduralTextures;
use crate::scenario_data::{DeskDef, PropPlacement, PropPrefab, RoomThemeDef, ScenarioDef};

#[derive(Component)]
pub struct ScenarioEntity;

#[derive(Component)]
pub struct PlayerDesk;

pub const LAYER_DEFAULT: u32 = 0b0001;
pub const LAYER_DESK: u32 = 0b0010;
pub const LAYER_DEBRIS: u32 = 0b0100;

#[inline]
fn f32_key(v: f32) -> u32 {
    if v == 0.0 {
        0.0_f32.to_bits()
    } else {
        v.to_bits()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct MatCacheKey {
    base_color: [u32; 4],
    base_tex: Option<AssetId<Image>>,
    emissive: [u32; 4],
    emissive_tex: Option<AssetId<Image>>,
    roughness: u32,
    metallic: u32,
    alpha_mode_tag: u8,
    alpha_mask_bits: u32,
    unlit: bool,
    cull_mode: Option<u8>,
    uv_transform: [u32; 6],
}

impl MatCacheKey {
    fn from_mat(mat: &StandardMaterial) -> Self {
        let bc = mat.base_color.to_srgba();
        let em = mat.emissive;
        let (alpha_mode_tag, alpha_mask_bits) = match mat.alpha_mode {
            AlphaMode::Opaque => (0, 0),
            AlphaMode::Mask(v) => (1, f32_key(v)),
            AlphaMode::Blend => (2, 0),
            AlphaMode::Premultiplied => (3, 0),
            AlphaMode::AlphaToCoverage => (4, 0),
            AlphaMode::Add => (5, 0),
            AlphaMode::Multiply => (6, 0),
        };
        let cull_mode = mat.cull_mode.map(|f| match f {
            Face::Front => 0,
            Face::Back => 1,
        });
        Self {
            base_color: [
                f32_key(bc.red),
                f32_key(bc.green),
                f32_key(bc.blue),
                f32_key(bc.alpha),
            ],
            base_tex: mat.base_color_texture.as_ref().map(Handle::id),
            emissive: [
                f32_key(em.red),
                f32_key(em.green),
                f32_key(em.blue),
                f32_key(em.alpha),
            ],
            emissive_tex: mat.emissive_texture.as_ref().map(Handle::id),
            roughness: f32_key(mat.perceptual_roughness),
            metallic: f32_key(mat.metallic),
            alpha_mode_tag,
            alpha_mask_bits,
            unlit: mat.unlit,
            cull_mode,
            uv_transform: mat.uv_transform.to_cols_array().map(f32_key),
        }
    }
}

#[derive(Resource, Default)]
pub struct PrefabAssetCache {
    cuboids: HashMap<[u32; 3], Handle<Mesh>>,
    cylinders: HashMap<[u32; 2], Handle<Mesh>>,
    upright_quads: HashMap<[u32; 2], Handle<Mesh>>,
    floor_quads: HashMap<[u32; 2], Handle<Mesh>>,
    frustums_y: HashMap<[u32; 5], Handle<Mesh>>,
    frustums_z: HashMap<[u32; 5], Handle<Mesh>>,
    materials: HashMap<MatCacheKey, Handle<StandardMaterial>>,
}

impl PrefabAssetCache {
    pub fn cuboid(&mut self, meshes: &mut Assets<Mesh>, x: f32, y: f32, z: f32) -> Handle<Mesh> {
        let key = [f32_key(x), f32_key(y), f32_key(z)];
        self.cuboids
            .entry(key)
            .or_insert_with(|| meshes.add(Cuboid::new(x, y, z)))
            .clone()
    }

    pub fn cylinder(&mut self, meshes: &mut Assets<Mesh>, r: f32, h: f32) -> Handle<Mesh> {
        let key = [f32_key(r), f32_key(h)];
        self.cylinders
            .entry(key)
            .or_insert_with(|| meshes.add(Cylinder::new(r, h)))
            .clone()
    }

    pub fn upright_quad(&mut self, meshes: &mut Assets<Mesh>, w: f32, h: f32) -> Handle<Mesh> {
        let key = [f32_key(w), f32_key(h)];
        self.upright_quads
            .entry(key)
            .or_insert_with(|| meshes.add(make_upright_quad_mesh(w, h)))
            .clone()
    }

    pub fn floor_quad(&mut self, meshes: &mut Assets<Mesh>, w: f32, d: f32) -> Handle<Mesh> {
        let key = [f32_key(w), f32_key(d)];
        self.floor_quads
            .entry(key)
            .or_insert_with(|| meshes.add(make_floor_quad_mesh(w, d)))
            .clone()
    }

    pub fn frustum_y(
        &mut self,
        meshes: &mut Assets<Mesh>,
        bottom_xz: Vec2,
        top_xz: Vec2,
        height: f32,
    ) -> Handle<Mesh> {
        let key = [
            f32_key(bottom_xz.x),
            f32_key(bottom_xz.y),
            f32_key(top_xz.x),
            f32_key(top_xz.y),
            f32_key(height),
        ];
        self.frustums_y
            .entry(key)
            .or_insert_with(|| meshes.add(make_frustum_y_mesh(bottom_xz, top_xz, height)))
            .clone()
    }

    pub fn frustum_z(
        &mut self,
        meshes: &mut Assets<Mesh>,
        front_wh: Vec2,
        back_wh: Vec2,
        depth: f32,
    ) -> Handle<Mesh> {
        let key = [
            f32_key(front_wh.x),
            f32_key(front_wh.y),
            f32_key(back_wh.x),
            f32_key(back_wh.y),
            f32_key(depth),
        ];
        self.frustums_z
            .entry(key)
            .or_insert_with(|| meshes.add(make_frustum_z_mesh(front_wh, back_wh, depth)))
            .clone()
    }

    pub fn material(
        &mut self,
        materials: &mut Assets<StandardMaterial>,
        mat: StandardMaterial,
    ) -> Handle<StandardMaterial> {
        let key = MatCacheKey::from_mat(&mat);
        self.materials
            .entry(key)
            .or_insert_with(|| materials.add(mat))
            .clone()
    }
}

#[derive(Component, Clone, Copy)]
pub struct DeskRestPose {
    pub pos: Vec3,
    pub rot: Quat,
}

#[derive(Component)]
pub struct BreakableDesk {
    pub stage: u8,
    pub prev_vel: Vec3,
    pub top_size: Vec3,
    pub top_mat: Handle<StandardMaterial>,
    pub wood_color: Color,
    pub leg_color: Color,
    pub spill_papers: bool,
}

#[derive(Component)]
pub struct DeskTopVisual;

#[derive(Component, Clone)]
pub struct DeskBreakoffPart {
    pub trigger_stage: u8,
    pub local_offset: Vec3,
    pub size: Vec3,
    pub mat: Handle<StandardMaterial>,
    pub eject_dir: Vec3,
}

/// Marks a child mesh whose material swaps to cracked CRT static when its parent prop cracks.
#[derive(Component)]
pub struct ScreenSurface;

/// Marks a child mesh that receives a spiderweb crack overlay when its parent prop cracks.
#[derive(Component)]
pub struct GlassSurface {
    pub size: Vec2,
    pub z_offset: f32,
}

/// Emits gentle rising pixel steam wisps (coffee mug, espresso boiler) or damaged electronics smoke!
#[derive(Component)]
pub struct VfxEmitter {
    pub timer: f32,
    pub interval: f32,
    pub offset: Vec3,
    pub style: EmitterStyle,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EmitterStyle {
    CoffeeSteam,
    DamagedSparksAndSmoke,
    PressurizedHiss,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ImpactProfile {
    Electronics,
    PressurizedHazard,
    Liquid {
        liquid_color: Color,
        puddle_radius: f32,
    },
    PaperStack {
        is_blueprint: bool,
    },
    GlassCrystal,
    FilingCabinet,
    HeavyProp,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SfxMaterial {
    Electronics,
    PressurizedBlast,
    Liquid,
    Paper,
    Glass,
    Metal,
    WoodCeramic,
}

#[derive(Component, Clone)]
pub struct DestructibleProp {
    pub label: String,
    pub base_points: u32,
    pub can_topple: bool,
    pub toppled: bool,
    pub damaged: bool,
    pub shattered: bool,
    pub damage_impulse: f32,
    pub break_impulse: f32,
    pub debris_color: Color,
    pub secondary_color: Color,
    pub profile: ImpactProfile,
    pub sfx: SfxMaterial,
    pub debris_count: u8,
    pub prev_velocity: Vec3,
}

impl DestructibleProp {
    pub fn small(
        label: impl Into<String>,
        base_points: u32,
        debris_color: Color,
        secondary_color: Color,
        profile: ImpactProfile,
        sfx: SfxMaterial,
    ) -> Self {
        Self {
            label: label.into(),
            base_points,
            can_topple: false,
            toppled: false,
            damaged: false,
            shattered: false,
            damage_impulse: 1.25,
            break_impulse: 2.45,
            debris_color,
            secondary_color,
            profile,
            sfx,
            debris_count: 8,
            prev_velocity: Vec3::ZERO,
        }
    }

    pub fn tall(
        label: impl Into<String>,
        base_points: u32,
        debris_color: Color,
        secondary_color: Color,
        profile: ImpactProfile,
        sfx: SfxMaterial,
    ) -> Self {
        Self {
            label: label.into(),
            base_points,
            can_topple: true,
            toppled: false,
            damaged: false,
            shattered: false,
            damage_impulse: 2.0,
            break_impulse: 4.2,
            debris_color,
            secondary_color,
            profile,
            sfx,
            debris_count: 10,
            prev_velocity: Vec3::ZERO,
        }
    }
}

/// Spawns a child box mesh AND a 1px dark inverted-hull pixel outline around it!
#[allow(clippy::too_many_arguments)]
fn spawn_outlined_box(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    cache: &mut PrefabAssetCache,
    shared: &SharedMeshes,
    size: Vec3,
    mat: Handle<StandardMaterial>,
    tf: Transform,
    outline_thickness: f32,
) {
    let box_mesh = cache.cuboid(meshes, size.x, size.y, size.z);
    parent.spawn((Mesh3d(box_mesh.clone()), MeshMaterial3d(mat), tf));
    if outline_thickness > 0.0 {
        let mut out_tf = tf;
        out_tf.scale *= outline_scale_for(size, outline_thickness);
        parent.spawn((
            NotShadowCaster,
            Mesh3d(box_mesh),
            MeshMaterial3d(shared.ink_outline_mat.clone()),
            out_tf,
        ));
    }
}

#[allow(clippy::too_many_arguments)]
pub fn spawn_scenario_world(
    def: &ScenarioDef,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    cache: &mut PrefabAssetCache,
    textures: &ProceduralTextures,
    shared: &SharedMeshes,
    joke_rng: &mut CorporateJokeRng,
) {
    spawn_room_architecture(
        &def.theme,
        commands,
        meshes,
        materials,
        cache,
        textures,
        shared,
    );
    spawn_player_desk(&def.desk, commands, meshes, materials, cache, textures, shared);
    for placement in &def.props {
        spawn_prop_prefab(
            placement,
            commands,
            meshes,
            materials,
            cache,
            textures,
            shared,
            joke_rng,
        );
    }
}

fn resolve_tiled_texture(name: &str, textures: &ProceduralTextures) -> Handle<Image> {
    match name {
        "plywood_floor" => textures.plywood_floor.clone(),
        "parquet_floor" => textures.parquet_floor.clone(),
        "corrugated_wall" => textures.corrugated_wall.clone(),
        "mahogany_panel" => textures.mahogany_panel.clone(),
        "hazard_stripe" => textures.hazard_stripe.clone(),
        "ceiling_tbar" => textures.ceiling_tbar.clone(),
        "slate_wall" => textures.slate_wall.clone(),
        "stage_deck" => textures.stage_deck.clone(),
        "velvet_curtain" => textures.velvet_curtain.clone(),
        "linoleum_tile" => textures.linoleum_tile.clone(),
        "breakroom_mint" => textures.breakroom_mint.clone(),
        "warehouse_concrete" => textures.warehouse_concrete.clone(),
        _ => textures.carpet_navy.clone(),
    }
}

fn spawn_room_architecture(
    theme: &RoomThemeDef,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    cache: &mut PrefabAssetCache,
    textures: &ProceduralTextures,
    shared: &SharedMeshes,
) {
    let floor_mat = cache.material(
        materials,
        StandardMaterial {
            base_color_texture: Some(resolve_tiled_texture(&theme.floor_texture, textures)),
            uv_transform: Affine2::from_scale(Vec2::splat(9.0)),
            perceptual_roughness: 0.92,
            ..default()
        },
    );
    let wall_mat = cache.material(
        materials,
        StandardMaterial {
            base_color_texture: Some(resolve_tiled_texture(&theme.wall_texture, textures)),
            uv_transform: Affine2::from_scale(Vec2::new(7.0, 1.0)),
            perceptual_roughness: 0.88,
            ..default()
        },
    );
    let ceiling_mat = cache.material(
        materials,
        StandardMaterial {
            base_color_texture: Some(resolve_tiled_texture(&theme.ceiling_texture, textures)),
            uv_transform: Affine2::from_scale(Vec2::splat(7.0)),
            perceptual_roughness: 0.92,
            ..default()
        },
    );
    let trim_mat = match theme.trim_texture.as_str() {
        "hazard_stripe" => cache.material(
            materials,
            StandardMaterial {
                base_color_texture: Some(textures.hazard_stripe.clone()),
                uv_transform: Affine2::from_scale(Vec2::new(18.0, 1.0)),
                perceptual_roughness: 0.65,
                ..default()
            },
        ),
        "brass_trim" => cache.material(
            materials,
            StandardMaterial {
                base_color: Color::srgb(0.68, 0.52, 0.24),
                metallic: 0.55,
                perceptual_roughness: 0.42,
                ..default()
            },
        ),
        "magenta_led_strip" => cache.material(
            materials,
            StandardMaterial {
                base_color: Color::srgb(0.95, 0.24, 0.72),
                emissive: LinearRgba::rgb(1.45, 0.18, 0.92),
                perceptual_roughness: 0.35,
                ..default()
            },
        ),
        "linoleum_cove" => cache.material(
            materials,
            StandardMaterial {
                base_color: Color::srgb(0.24, 0.30, 0.28),
                perceptual_roughness: 0.78,
                ..default()
            },
        ),
        _ => cache.material(
            materials,
            StandardMaterial {
                base_color: Color::srgb(0.12, 0.15, 0.21),
                perceptual_roughness: 0.82,
                ..default()
            },
        ),
    };

    // Floor
    commands.spawn((
        ScenarioEntity,
        RigidBody::Static,
        Restitution::new(0.42),
        Friction::new(0.36),
        Collider::cuboid(12.0, 0.4, 12.0),
        Mesh3d(cache.cuboid(meshes, 12.0, 0.4, 12.0)),
        MeshMaterial3d(floor_mat),
        Transform::from_xyz(0.0, -0.2, -1.0),
    ));

    // Recessed dark T-bar Ceiling
    commands.spawn((
        ScenarioEntity,
        Mesh3d(cache.cuboid(meshes, 12.0, 0.24, 12.0)),
        MeshMaterial3d(ceiling_mat),
        Transform::from_xyz(0.0, 4.25, -1.0),
    ));

    // Back Wall + dark baseboard + cornice molding
    commands.spawn((
        ScenarioEntity,
        RigidBody::Static,
        Restitution::new(0.50),
        Friction::new(0.28),
        Collider::cuboid(12.0, 4.5, 0.4),
        Mesh3d(cache.cuboid(meshes, 12.0, 4.5, 0.4)),
        MeshMaterial3d(wall_mat.clone()),
        Transform::from_xyz(0.0, 2.20, -6.4),
    ));
    commands.spawn((
        ScenarioEntity,
        Mesh3d(cache.cuboid(meshes, 12.0, 0.20, 0.07)),
        MeshMaterial3d(trim_mat.clone()),
        Transform::from_xyz(0.0, 0.10, -6.17),
    ));

    // Left & Right Walls + baseboards
    for x in [-5.4_f32, 5.4_f32] {
        commands.spawn((
            ScenarioEntity,
            RigidBody::Static,
            Restitution::new(0.50),
            Friction::new(0.28),
            Collider::cuboid(0.4, 4.5, 12.0),
            Mesh3d(cache.cuboid(meshes, 0.4, 4.5, 12.0)),
            MeshMaterial3d(wall_mat.clone()),
            Transform::from_xyz(x, 2.20, -1.0),
        ));
        let inner_x = if x < 0.0 { x + 0.22 } else { x - 0.22 };
        commands.spawn((
            ScenarioEntity,
            Mesh3d(cache.cuboid(meshes, 0.07, 0.20, 12.0)),
            MeshMaterial3d(trim_mat.clone()),
            Transform::from_xyz(inner_x, 0.10, -1.0),
        ));
    }

    // Back-right Structural Pillar + Round Analog Wall Clock (`loading-desk.jpg`!)
    if theme.has_pillar_clock {
        let pillar_mat = cache.material(
            materials,
            StandardMaterial {
                base_color: Color::srgb(0.14, 0.18, 0.25),
                perceptual_roughness: 0.88,
                ..default()
            },
        );
        commands
            .spawn((
                ScenarioEntity,
                RigidBody::Static,
                Collider::cuboid(0.85, 4.3, 0.55),
                Visibility::default(),
                Transform::from_xyz(2.95, 2.15, -5.95),
            ))
            .with_children(|pil| {
                spawn_outlined_box(
                    pil,
                    meshes,
                    cache,
                    shared,
                    Vec3::new(0.85, 4.3, 0.55),
                    pillar_mat,
                    Transform::IDENTITY,
                    0.016,
                );
                let clock_mat = cache.material(
                    materials,
                    StandardMaterial {
                        base_color_texture: Some(textures.clock_face.clone()),
                        alpha_mode: AlphaMode::Mask(0.5),
                        perceptual_roughness: 0.6,
                        ..default()
                    },
                );
                pil.spawn((
                    Mesh3d(cache.upright_quad(meshes, 0.48, 0.48)),
                    MeshMaterial3d(clock_mat),
                    Transform::from_xyz(0.0, 0.92, 0.29),
                ));
            });
    }

    // Back Window treatment + Volumetric Sunbeam Shafts + Slatted Floor Reflections
    let beam_rgba = theme.sunbeam_color;
    let sunbeam_mat = cache.material(
        materials,
        StandardMaterial {
            base_color: Color::srgba(beam_rgba[0], beam_rgba[1], beam_rgba[2], beam_rgba[3]),
            emissive: LinearRgba::rgb(
                beam_rgba[0] * 0.45,
                beam_rgba[1] * 0.35,
                beam_rgba[2] * 0.18,
            ),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            cull_mode: None,
            ..default()
        },
    );
    let floor_slat_mat = cache.material(
        materials,
        StandardMaterial {
            base_color: Color::srgba(beam_rgba[0], beam_rgba[1], beam_rgba[2], 0.34),
            emissive: LinearRgba::rgb(
                beam_rgba[0] * 0.42,
                beam_rgba[1] * 0.30,
                beam_rgba[2] * 0.12,
            ),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        },
    );

    match theme.window_style.as_str() {
        "city_sunset_glass" => {
            let sky_mat = cache.material(
                materials,
                StandardMaterial {
                    base_color_texture: Some(textures.city_skyline.clone()),
                    emissive: LinearRgba::rgb(0.45, 0.32, 0.25),
                    emissive_texture: Some(textures.city_skyline.clone()),
                    unlit: true,
                    ..default()
                },
            );
            commands.spawn((
                ScenarioEntity,
                Mesh3d(cache.upright_quad(meshes, 4.6, 2.35)),
                MeshMaterial3d(sky_mat),
                Transform::from_xyz(-0.35, 2.35, -6.16),
            ));
            let mullion = cache.cuboid(meshes, 0.07, 2.4, 0.08);
            for mx in [-2.65, -1.12, 0.42, 1.95] {
                commands.spawn((
                    ScenarioEntity,
                    Mesh3d(mullion.clone()),
                    MeshMaterial3d(trim_mat.clone()),
                    Transform::from_xyz(mx, 2.35, -6.14),
                ));
            }
        }
        "site_daylight" => {
            let sky_mat = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.66, 0.86, 0.98),
                    emissive: LinearRgba::rgb(0.55, 0.75, 0.92),
                    unlit: true,
                    ..default()
                },
            );
            commands.spawn((
                ScenarioEntity,
                Mesh3d(cache.upright_quad(meshes, 3.4, 1.65)),
                MeshMaterial3d(sky_mat),
                Transform::from_xyz(0.0, 2.35, -6.16),
            ));
            // Exterior scaffolding bars visible through trailer window
            let bar_mat = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.24, 0.28, 0.34),
                    ..default()
                },
            );
            commands.spawn((
                ScenarioEntity,
                Mesh3d(cache.cuboid(meshes, 3.5, 0.06, 0.06)),
                MeshMaterial3d(bar_mat.clone()),
                Transform::from_xyz(0.0, 2.15, -6.13),
            ));
            commands.spawn((
                ScenarioEntity,
                Mesh3d(cache.cuboid(meshes, 3.6, 0.05, 0.05)),
                MeshMaterial3d(bar_mat),
                Transform::from_xyz(0.0, 2.35, -6.13).with_rotation(Quat::from_rotation_z(0.38)),
            ));
        }
        "townhall_led_wall" => {
            // Giant All-Hands Stage Backdrop Banner + Studio Rigging Truss + Spotlight Cones!
            let slide_mat = cache.material(
                materials,
                StandardMaterial {
                    base_color_texture: Some(textures.townhall_slide.clone()),
                    emissive: LinearRgba::rgb(0.65, 0.42, 0.85),
                    emissive_texture: Some(textures.townhall_slide.clone()),
                    unlit: true,
                    ..default()
                },
            );
            let truss_mat = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.22, 0.24, 0.30),
                    metallic: 0.7,
                    perceptual_roughness: 0.38,
                    ..default()
                },
            );
            commands.spawn((
                ScenarioEntity,
                Mesh3d(cache.upright_quad(meshes, 4.8, 2.55)),
                MeshMaterial3d(slide_mat),
                Transform::from_xyz(0.0, 2.38, -6.15),
            ));
            // Overhead lighting truss beam + vertical stage towers
            commands.spawn((
                ScenarioEntity,
                Mesh3d(cache.cuboid(meshes, 7.4, 0.18, 0.18)),
                MeshMaterial3d(truss_mat.clone()),
                Transform::from_xyz(0.0, 3.82, -5.85),
            ));
            for tx in [-3.55_f32, 3.55_f32] {
                commands.spawn((
                    ScenarioEntity,
                    Mesh3d(cache.cuboid(meshes, 0.16, 3.85, 0.16)),
                    MeshMaterial3d(truss_mat.clone()),
                    Transform::from_xyz(tx, 1.92, -5.85),
                ));
                commands.spawn((
                    ScenarioEntity,
                    Mesh3d(cache.cuboid(meshes, 1.5, 2.8, 0.02)),
                    MeshMaterial3d(sunbeam_mat.clone()),
                    Transform::from_xyz(tx * 0.55, 2.25, -5.15)
                        .with_rotation(Quat::from_euler(EulerRot::XYZ, 0.36, tx.signum() * -0.22, 0.0)),
                ));
            }
        }
        "breakroom_transom" => {
            // High Frosted Clerestory Transom Windows + Passive-Aggressive Laminated Fridge Rules!
            let transom_glow = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.86, 0.94, 0.90),
                    emissive: LinearRgba::rgb(0.62, 0.72, 0.65),
                    unlit: true,
                    ..default()
                },
            );
            let frame_mat = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.32, 0.38, 0.36),
                    ..default()
                },
            );
            for wx in [-2.0_f32, 0.65_f32] {
                commands.spawn((
                    ScenarioEntity,
                    Mesh3d(cache.upright_quad(meshes, 2.1, 0.72)),
                    MeshMaterial3d(transom_glow.clone()),
                    Transform::from_xyz(wx, 3.35, -6.16),
                ));
                commands.spawn((
                    ScenarioEntity,
                    Mesh3d(cache.cuboid(meshes, 2.18, 0.06, 0.08)),
                    MeshMaterial3d(frame_mat.clone()),
                    Transform::from_xyz(wx, 2.98, -6.13),
                ));
            }
            // Big "HANG IN THERE" & laminated "CLEAN YOUR OWN MUG!" breakroom signs
            let poster_mat = cache.material(
                materials,
                StandardMaterial {
                    base_color_texture: Some(textures.cat_poster.clone()),
                    perceptual_roughness: 0.55,
                    ..default()
                },
            );
            commands.spawn((
                ScenarioEntity,
                Mesh3d(cache.upright_quad(meshes, 0.95, 1.25)),
                MeshMaterial3d(poster_mat),
                Transform::from_xyz(1.35, 2.05, -6.15),
            ));
        }
        "warehouse_safety_board" => {
            // Steel Roll-Up Loading Dock Door + Giant "0 DAYS SINCE INJURY // QUOTA" LED Board!
            let shutter_mat = cache.material(
                materials,
                StandardMaterial {
                    base_color_texture: Some(textures.corrugated_wall.clone()),
                    base_color: Color::srgb(0.62, 0.66, 0.72),
                    uv_transform: Affine2::from_scale(Vec2::new(3.0, 5.0)),
                    metallic: 0.45,
                    perceptual_roughness: 0.55,
                    ..default()
                },
            );
            let board_mat = cache.material(
                materials,
                StandardMaterial {
                    base_color_texture: Some(textures.injury_board.clone()),
                    emissive: LinearRgba::rgb(0.85, 0.45, 0.22),
                    emissive_texture: Some(textures.injury_board.clone()),
                    unlit: true,
                    ..default()
                },
            );
            // Roll-up bay shutter on back-left
            commands.spawn((
                ScenarioEntity,
                Mesh3d(cache.cuboid(meshes, 3.1, 3.1, 0.08)),
                MeshMaterial3d(shutter_mat),
                Transform::from_xyz(-2.35, 1.55, -6.15),
            ));
            // Glowing Safety & Pick-Rate Scoreboard center-right
            commands.spawn((
                ScenarioEntity,
                Mesh3d(cache.upright_quad(meshes, 2.85, 1.55)),
                MeshMaterial3d(board_mat),
                Transform::from_xyz(1.15, 2.55, -6.15),
            ));
        }
        _ => {
            // Iconic Warm Golden Venetian-Blind Window (`loading-desk.jpg`!)
            let blind_glow = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(1.0, 0.84, 0.48),
                    emissive: LinearRgba::rgb(1.35, 0.92, 0.36),
                    unlit: true,
                    ..default()
                },
            );
            let slat_dark = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.56, 0.36, 0.18),
                    perceptual_roughness: 0.7,
                    ..default()
                },
            );

            // Back Venetian window (center-right of back wall, matching loading-desk.jpg!)
            let win_x = 0.85;
            commands.spawn((
                ScenarioEntity,
                Mesh3d(cache.upright_quad(meshes, 2.65, 1.85)),
                MeshMaterial3d(blind_glow.clone()),
                Transform::from_xyz(win_x, 2.32, -6.16),
            ));
            // Valance header
            commands.spawn((
                ScenarioEntity,
                Mesh3d(cache.cuboid(meshes, 2.80, 0.12, 0.12)),
                MeshMaterial3d(slat_dark.clone()),
                Transform::from_xyz(win_x, 3.28, -6.12),
            ));
            let back_slat = cache.cuboid(meshes, 2.66, 0.036, 0.06);
            for i in 0..12 {
                let sy = 1.46 + i as f32 * 0.15;
                commands.spawn((
                    ScenarioEntity,
                    Mesh3d(back_slat.clone()),
                    MeshMaterial3d(slat_dark.clone()),
                    Transform::from_xyz(win_x, sy, -6.13),
                ));
            }
            // Diagonal golden sunbeam wedge slanting down from back window
            commands.spawn((
                ScenarioEntity,
                Mesh3d(cache.cuboid(meshes, 2.55, 2.6, 0.02)),
                MeshMaterial3d(sunbeam_mat.clone()),
                Transform::from_xyz(win_x + 0.25, 1.55, -5.25)
                    .with_rotation(Quat::from_euler(EulerRot::XYZ, 0.48, -0.14, 0.0)),
            ));

            // Secondary far-right Venetian window (beyond pillar, matching loading-desk.jpg!)
            let r_win_x = 4.35;
            commands.spawn((
                ScenarioEntity,
                Mesh3d(cache.upright_quad(meshes, 1.55, 1.65)),
                MeshMaterial3d(blind_glow.clone()),
                Transform::from_xyz(r_win_x, 2.35, -6.16),
            ));
            let r_slat = cache.cuboid(meshes, 1.56, 0.036, 0.06);
            for i in 0..10 {
                let sy = 1.58 + i as f32 * 0.15;
                commands.spawn((
                    ScenarioEntity,
                    Mesh3d(r_slat.clone()),
                    MeshMaterial3d(slat_dark.clone()),
                    Transform::from_xyz(r_win_x, sy, -6.13),
                ));
            }

            // Crisp slatted golden Venetian-blind stripes across the dark carpet!
            let stripe_mesh = cache.floor_quad(meshes, 2.1, 0.11);
            for i in 0..7 {
                commands.spawn((
                    ScenarioEntity,
                    Mesh3d(stripe_mesh.clone()),
                    MeshMaterial3d(floor_slat_mat.clone()),
                    Transform::from_xyz(0.35 + i as f32 * 0.05, 0.005, -4.95 + i as f32 * 0.22),
                ));
            }

            // Left-wall Venetian blind window + diagonal golden sunbeam (`loading-desk.jpg` top-left!)
            if theme.has_left_blind_window {
                commands.spawn((
                    ScenarioEntity,
                    Mesh3d(cache.cuboid(meshes, 0.04, 1.75, 2.75)),
                    MeshMaterial3d(blind_glow),
                    Transform::from_xyz(-5.16, 2.48, -1.65),
                ));
                let left_slat = cache.cuboid(meshes, 0.07, 0.036, 2.76);
                for i in 0..11 {
                    let sy = 1.68 + i as f32 * 0.15;
                    commands.spawn((
                        ScenarioEntity,
                        Mesh3d(left_slat.clone()),
                        MeshMaterial3d(slat_dark.clone()),
                        Transform::from_xyz(-5.13, sy, -1.65),
                    ));
                }
                // Angled golden sunbeam streaming in from left blinds!
                commands.spawn((
                    ScenarioEntity,
                    Mesh3d(cache.cuboid(meshes, 2.8, 0.02, 2.45)),
                    MeshMaterial3d(sunbeam_mat),
                    Transform::from_xyz(-3.85, 1.85, -1.55)
                        .with_rotation(Quat::from_rotation_z(-0.52)),
                ));
            }
        }
    }

    // ── Side-Wall Architectural Details (Door, EXIT Sign, Corkboards, AC Vents) ──
    let door_mat = cache.material(
        materials,
        StandardMaterial {
            base_color: Color::srgb(0.38, 0.23, 0.14),
            perceptual_roughness: 0.68,
            ..default()
        },
    );
    let door_glass = cache.material(
        materials,
        StandardMaterial {
            base_color: Color::srgba(0.62, 0.82, 0.92, 0.55),
            emissive: LinearRgba::rgb(0.12, 0.20, 0.26),
            alpha_mode: AlphaMode::Blend,
            ..default()
        },
    );
    let exit_glow = cache.material(
        materials,
        StandardMaterial {
            base_color: Color::srgb(0.98, 0.22, 0.20),
            emissive: LinearRgba::rgb(2.4, 0.25, 0.20),
            unlit: true,
            ..default()
        },
    );
    let brass_knob = cache.material(
        materials,
        StandardMaterial {
            base_color: Color::srgb(0.84, 0.68, 0.28),
            metallic: 0.75,
            perceptual_roughness: 0.3,
            ..default()
        },
    );

    // Right-wall 3D Office / Trailer Door + Glowing EXIT Sign
    commands
        .spawn((
            ScenarioEntity,
            Transform::from_xyz(5.16, 1.12, -2.95),
            Visibility::default(),
        ))
        .with_children(|dr| {
            spawn_outlined_box(
                dr,
                meshes,
                cache,
                shared,
                Vec3::new(0.08, 2.24, 1.12),
                trim_mat.clone(),
                Transform::IDENTITY,
                0.010,
            );
            spawn_outlined_box(
                dr,
                meshes,
                cache,
                shared,
                Vec3::new(0.06, 2.14, 0.98),
                door_mat,
                Transform::from_xyz(-0.02, -0.02, 0.0),
                0.010,
            );
            dr.spawn((
                Mesh3d(cache.cuboid(meshes, 0.03, 0.72, 0.56)),
                MeshMaterial3d(door_glass),
                Transform::from_xyz(-0.05, 0.42, 0.0),
            ));
            dr.spawn((
                Mesh3d(cache.cuboid(meshes, 0.06, 0.04, 0.14)),
                MeshMaterial3d(brass_knob),
                Transform::from_xyz(-0.06, -0.08, 0.36),
            ));
            // Glowing red EXIT sign box above door
            spawn_outlined_box(
                dr,
                meshes,
                cache,
                shared,
                Vec3::new(0.08, 0.20, 0.46),
                exit_glow,
                Transform::from_xyz(-0.04, 1.32, 0.0),
                0.008,
            );
        });

    // Wall-mounted Corkboards & HVAC Vents so left/right/back walls look lived-in
    let cork_mat = cache.material(
        materials,
        StandardMaterial {
            base_color: Color::srgb(0.66, 0.46, 0.26),
            perceptual_roughness: 0.88,
            ..default()
        },
    );
    let memos_wall = cache.material(
        materials,
        StandardMaterial {
            base_color_texture: Some(textures.pinned_memos.clone()),
            alpha_mode: AlphaMode::Mask(0.5),
            ..default()
        },
    );
    let vent_mat = cache.material(
        materials,
        StandardMaterial {
            base_color: Color::srgb(0.32, 0.36, 0.44),
            metallic: 0.35,
            ..default()
        },
    );

    // Right-wall corkboard with pinned notices
    commands
        .spawn((
            ScenarioEntity,
            Transform::from_xyz(5.15, 1.75, -0.55)
                .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)),
            Visibility::default(),
        ))
        .with_children(|cb| {
            spawn_outlined_box(
                cb,
                meshes,
                cache,
                shared,
                Vec3::new(1.35, 0.88, 0.05),
                cork_mat.clone(),
                Transform::IDENTITY,
                0.011,
            );
            for mx in [-0.36_f32, 0.28_f32] {
                cb.spawn((
                    Mesh3d(cache.upright_quad(meshes, 0.46, 0.46)),
                    MeshMaterial3d(memos_wall.clone()),
                    Transform::from_xyz(mx, 0.05, 0.032),
                ));
            }
        });

    // Left-wall notice board (forward of left blinds) + upper HVAC wall vents
    commands
        .spawn((
            ScenarioEntity,
            Transform::from_xyz(-5.15, 1.68, -4.45)
                .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
            Visibility::default(),
        ))
        .with_children(|cb| {
            spawn_outlined_box(
                cb,
                meshes,
                cache,
                shared,
                Vec3::new(1.15, 0.78, 0.05),
                cork_mat,
                Transform::IDENTITY,
                0.010,
            );
            cb.spawn((
                Mesh3d(cache.upright_quad(meshes, 0.48, 0.48)),
                MeshMaterial3d(memos_wall),
                Transform::from_xyz(0.0, 0.02, 0.032),
            ));
        });

    for (vx, vz) in [(-3.2_f32, -6.14_f32), (1.8_f32, -6.14_f32)] {
        commands.spawn((
            ScenarioEntity,
            Mesh3d(cache.cuboid(meshes, 0.75, 0.26, 0.06)),
            MeshMaterial3d(vent_mat.clone()),
            Transform::from_xyz(vx, 3.68, vz),
        ));
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_player_desk(
    desk: &DeskDef,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    cache: &mut PrefabAssetCache,
    textures: &ProceduralTextures,
    shared: &SharedMeshes,
) {
    let base_pos = Vec3::from_array(desk.pos);
    let base_rot = Quat::IDENTITY;

    match desk.archetype.as_str() {
        "folding_site" => {
            let top_mat = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.52, 0.56, 0.60),
                    perceptual_roughness: 0.58,
                    ..default()
                },
            );
            let steel_mat = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.20, 0.23, 0.28),
                    metallic: 0.5,
                    perceptual_roughness: 0.45,
                    ..default()
                },
            );

            commands
                .spawn((
                    ScenarioEntity,
                    PlayerDesk,
                    DeskRestPose {
                        pos: base_pos,
                        rot: base_rot,
                    },
                    BreakableDesk {
                        stage: 0,
                        prev_vel: Vec3::ZERO,
                        top_size: Vec3::new(2.1, 0.07, 0.98),
                        top_mat: top_mat.clone(),
                        wood_color: Color::srgb(0.52, 0.56, 0.60),
                        leg_color: Color::srgb(0.20, 0.23, 0.28),
                        spill_papers: false,
                    },
                    RigidBody::Dynamic,
                    CollisionLayers::from_bits(LAYER_DESK, LAYER_DEFAULT),
                    Mass(desk.mass),
                    Friction::new(0.30),
                    Restitution::new(0.44),
                    LinearDamping(0.10),
                    AngularDamping(0.10),
                    Collider::compound(vec![
                        (
                            Vec3::new(0.0, 0.34, 0.0),
                            Quat::IDENTITY,
                            Collider::cuboid(2.1, 0.07, 0.98),
                        ),
                        (
                            Vec3::new(-0.90, -0.02, -0.38),
                            Quat::IDENTITY,
                            Collider::cuboid(0.07, 0.68, 0.07),
                        ),
                        (
                            Vec3::new(0.90, -0.02, -0.38),
                            Quat::IDENTITY,
                            Collider::cuboid(0.07, 0.68, 0.07),
                        ),
                        (
                            Vec3::new(-0.90, -0.02, 0.38),
                            Quat::IDENTITY,
                            Collider::cuboid(0.07, 0.68, 0.07),
                        ),
                        (
                            Vec3::new(0.90, -0.02, 0.38),
                            Quat::IDENTITY,
                            Collider::cuboid(0.07, 0.68, 0.07),
                        ),
                    ]),
                    Transform::from_translation(base_pos),
                    Visibility::default(),
                ))
                .with_children(|d| {
                    let top_mesh = cache.cuboid(meshes, 2.1, 0.07, 0.98);
                    d.spawn((
                        DeskTopVisual,
                        Mesh3d(top_mesh.clone()),
                        MeshMaterial3d(top_mat.clone()),
                        Transform::from_xyz(0.0, 0.34, 0.0),
                    ));
                    d.spawn((
                        DeskTopVisual,
                        Mesh3d(top_mesh),
                        MeshMaterial3d(shared.ink_outline_mat.clone()),
                        Transform::from_xyz(0.0, 0.34, 0.0)
                            .with_scale(outline_scale_for(Vec3::new(2.1, 0.07, 0.98), 0.014)),
                    ));
                    // Under-table steel cross-apron (breaks off at Stage 1!)
                    d.spawn((
                        DeskBreakoffPart {
                            trigger_stage: 1,
                            local_offset: Vec3::new(0.0, 0.26, 0.38),
                            size: Vec3::new(1.76, 0.05, 0.05),
                            mat: steel_mat.clone(),
                            eject_dir: Vec3::new(0.0, 1.4, 1.6),
                        },
                        Mesh3d(cache.cuboid(meshes, 1.76, 0.05, 0.05)),
                        MeshMaterial3d(steel_mat.clone()),
                        Transform::from_xyz(0.0, 0.26, 0.38),
                    ));
                    for (idx, (lx, lz)) in
                        [(-0.90_f32, -0.38_f32), (0.90, -0.38), (-0.90, 0.38), (0.90, 0.38)]
                            .into_iter()
                            .enumerate()
                    {
                        let leg_size = Vec3::new(0.07, 0.68, 0.07);
                        let stage = if idx < 2 { 1 } else { 2 };
                        d.spawn((
                            DeskBreakoffPart {
                                trigger_stage: stage,
                                local_offset: Vec3::new(lx, -0.02, lz),
                                size: leg_size,
                                mat: steel_mat.clone(),
                                eject_dir: Vec3::new(lx.signum() * 2.2, 1.8, lz.signum() * 1.5),
                            },
                            Mesh3d(cache.cuboid(meshes, leg_size.x, leg_size.y, leg_size.z)),
                            MeshMaterial3d(steel_mat.clone()),
                            Transform::from_xyz(lx, -0.02, lz),
                        ));
                    }
                });
        }
        "townhall_panel" | "laminate_break" | "supervisor_steel" => {
            let is_townhall = desk.archetype == "townhall_panel";
            let is_breakroom = desk.archetype == "laminate_break";
            let (top_col, leg_col, accent_col, spill) = if is_townhall {
                (
                    Color::srgb(0.14, 0.14, 0.20),
                    Color::srgb(0.28, 0.22, 0.42),
                    Color::srgb(0.95, 0.24, 0.72),
                    true,
                )
            } else if is_breakroom {
                (
                    Color::srgb(0.90, 0.91, 0.86),
                    Color::srgb(0.68, 0.74, 0.78),
                    Color::srgb(0.32, 0.44, 0.40),
                    false,
                )
            } else {
                // supervisor_steel
                (
                    Color::srgb(0.26, 0.30, 0.35),
                    Color::srgb(0.92, 0.70, 0.14),
                    Color::srgb(0.92, 0.70, 0.14),
                    true,
                )
            };

            let top_mat = cache.material(
                materials,
                StandardMaterial {
                    base_color: top_col,
                    metallic: if is_breakroom { 0.05 } else { 0.42 },
                    perceptual_roughness: if is_breakroom { 0.45 } else { 0.52 },
                    ..default()
                },
            );
            let leg_mat = cache.material(
                materials,
                StandardMaterial {
                    base_color: leg_col,
                    metallic: 0.55,
                    perceptual_roughness: 0.40,
                    ..default()
                },
            );
            let accent_mat = cache.material(
                materials,
                StandardMaterial {
                    base_color: accent_col,
                    emissive: if is_townhall {
                        LinearRgba::rgb(1.3, 0.15, 0.85)
                    } else {
                        LinearRgba::NONE
                    },
                    perceptual_roughness: 0.45,
                    ..default()
                },
            );

            commands
                .spawn((
                    ScenarioEntity,
                    PlayerDesk,
                    DeskRestPose {
                        pos: base_pos,
                        rot: base_rot,
                    },
                    BreakableDesk {
                        stage: 0,
                        prev_vel: Vec3::ZERO,
                        top_size: Vec3::new(2.16, 0.075, 1.00),
                        top_mat: top_mat.clone(),
                        wood_color: top_col,
                        leg_color: leg_col,
                        spill_papers: spill,
                    },
                    RigidBody::Dynamic,
                    CollisionLayers::from_bits(LAYER_DESK, LAYER_DEFAULT),
                    Mass(desk.mass),
                    Friction::new(0.32),
                    Restitution::new(0.42),
                    LinearDamping(0.10),
                    AngularDamping(0.10),
                    Collider::compound(vec![
                        (
                            Vec3::new(0.0, 0.34, 0.0),
                            Quat::IDENTITY,
                            Collider::cuboid(2.16, 0.075, 1.00),
                        ),
                        (
                            Vec3::new(-0.92, -0.02, -0.38),
                            Quat::IDENTITY,
                            Collider::cuboid(0.08, 0.68, 0.08),
                        ),
                        (
                            Vec3::new(0.92, -0.02, -0.38),
                            Quat::IDENTITY,
                            Collider::cuboid(0.08, 0.68, 0.08),
                        ),
                        (
                            Vec3::new(-0.92, -0.02, 0.38),
                            Quat::IDENTITY,
                            Collider::cuboid(0.08, 0.68, 0.08),
                        ),
                        (
                            Vec3::new(0.92, -0.02, 0.38),
                            Quat::IDENTITY,
                            Collider::cuboid(0.08, 0.68, 0.08),
                        ),
                    ]),
                    Transform::from_translation(base_pos),
                    Visibility::default(),
                ))
                .with_children(|d| {
                    let top_mesh = cache.cuboid(meshes, 2.16, 0.075, 1.00);
                    d.spawn((
                        DeskTopVisual,
                        Mesh3d(top_mesh.clone()),
                        MeshMaterial3d(top_mat.clone()),
                        Transform::from_xyz(0.0, 0.34, 0.0),
                    ));
                    d.spawn((
                        DeskTopVisual,
                        Mesh3d(top_mesh),
                        MeshMaterial3d(shared.ink_outline_mat.clone()),
                        Transform::from_xyz(0.0, 0.34, 0.0)
                            .with_scale(outline_scale_for(Vec3::new(2.16, 0.075, 1.00), 0.014)),
                    ));
                    // Front modesty panel / trim strip (snaps off at Stage 1!)
                    let panel_h = if is_breakroom { 0.08 } else { 0.34 };
                    let panel_y = if is_breakroom { 0.28 } else { 0.14 };
                    d.spawn((
                        DeskBreakoffPart {
                            trigger_stage: 1,
                            local_offset: Vec3::new(0.0, panel_y, -0.44),
                            size: Vec3::new(1.88, panel_h, 0.05),
                            mat: accent_mat.clone(),
                            eject_dir: Vec3::new(0.0, 1.8, -2.2),
                        },
                        Mesh3d(cache.cuboid(meshes, 1.88, panel_h, 0.05)),
                        MeshMaterial3d(accent_mat),
                        Transform::from_xyz(0.0, panel_y, -0.44),
                    ));
                    for (idx, (lx, lz)) in
                        [(-0.92_f32, -0.38_f32), (0.92, -0.38), (-0.92, 0.38), (0.92, 0.38)]
                            .into_iter()
                            .enumerate()
                    {
                        let leg_size = Vec3::new(0.08, 0.68, 0.08);
                        let stage = if idx < 2 { 1 } else { 2 };
                        d.spawn((
                            DeskBreakoffPart {
                                trigger_stage: stage,
                                local_offset: Vec3::new(lx, -0.02, lz),
                                size: leg_size,
                                mat: leg_mat.clone(),
                                eject_dir: Vec3::new(lx.signum() * 2.3, 1.9, lz.signum() * 1.6),
                            },
                            Mesh3d(cache.cuboid(meshes, leg_size.x, leg_size.y, leg_size.z)),
                            MeshMaterial3d(leg_mat.clone()),
                            Transform::from_xyz(lx, -0.02, lz),
                        ));
                    }
                });
        }
        _ => {
            // Rich Walnut Executive / Cubicle Pedestal Desk (`loading-desk.jpg`!)
            let is_boardroom = desk.archetype == "mahogany_boardroom";
            let wood_mat = cache.material(
                materials,
                StandardMaterial {
                    base_color_texture: Some(textures.walnut_desk.clone()),
                    base_color: if is_boardroom {
                        Color::srgb(0.86, 0.64, 0.56)
                    } else {
                        Color::WHITE
                    },
                    uv_transform: Affine2::from_scale(Vec2::new(3.0, 1.5)),
                    perceptual_roughness: if is_boardroom { 0.42 } else { 0.64 },
                    ..default()
                },
            );
            let dark_wood = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.24, 0.13, 0.07),
                    perceptual_roughness: 0.72,
                    ..default()
                },
            );

            commands
                .spawn((
                    ScenarioEntity,
                    PlayerDesk,
                    DeskRestPose {
                        pos: base_pos,
                        rot: base_rot,
                    },
                    BreakableDesk {
                        stage: 0,
                        prev_vel: Vec3::ZERO,
                        top_size: Vec3::new(2.24, 0.08, 1.04),
                        top_mat: wood_mat.clone(),
                        wood_color: Color::srgb(0.56, 0.32, 0.16),
                        leg_color: Color::srgb(0.28, 0.15, 0.08),
                        spill_papers: true,
                    },
                    RigidBody::Dynamic,
                    CollisionLayers::from_bits(LAYER_DESK, LAYER_DEFAULT),
                    Mass(desk.mass),
                    Friction::new(0.34),
                    Restitution::new(0.40),
                    LinearDamping(0.10),
                    AngularDamping(0.10),
                    Collider::compound(vec![
                        (
                            Vec3::new(0.0, 0.34, 0.0),
                            Quat::IDENTITY,
                            Collider::cuboid(2.24, 0.08, 1.04),
                        ),
                        (
                            Vec3::new(-0.78, -0.03, 0.0),
                            Quat::IDENTITY,
                            Collider::cuboid(0.50, 0.68, 0.90),
                        ),
                        (
                            Vec3::new(0.78, -0.03, 0.0),
                            Quat::IDENTITY,
                            Collider::cuboid(0.50, 0.68, 0.90),
                        ),
                    ]),
                    Transform::from_translation(base_pos),
                    Visibility::default(),
                ))
                .with_children(|d| {
                    // Thick beveled tabletop (splits in two at Stage 3!)
                    let top_mesh = cache.cuboid(meshes, 2.24, 0.08, 1.04);
                    d.spawn((
                        DeskTopVisual,
                        Mesh3d(top_mesh.clone()),
                        MeshMaterial3d(wood_mat.clone()),
                        Transform::from_xyz(0.0, 0.34, 0.0),
                    ));
                    d.spawn((
                        DeskTopVisual,
                        Mesh3d(top_mesh),
                        MeshMaterial3d(shared.ink_outline_mat.clone()),
                        Transform::from_xyz(0.0, 0.34, 0.0)
                            .with_scale(outline_scale_for(Vec3::new(2.24, 0.08, 1.04), 0.015)),
                    ));
                    // Front beveled overhang lip (snaps off at Stage 1!)
                    d.spawn((
                        DeskBreakoffPart {
                            trigger_stage: 1,
                            local_offset: Vec3::new(0.0, 0.315, 0.51),
                            size: Vec3::new(2.26, 0.04, 0.05),
                            mat: dark_wood.clone(),
                            eject_dir: Vec3::new(0.0, 1.8, 2.2),
                        },
                        Mesh3d(cache.cuboid(meshes, 2.26, 0.035, 0.04)),
                        MeshMaterial3d(dark_wood.clone()),
                        Transform::from_xyz(0.0, 0.315, 0.51),
                    ));
                    // Left & Right Drawer Pedestals (drawers rip out at Stage 1, pedestals snap off at Stage 2!)
                    for px in [-0.78_f32, 0.78_f32] {
                        let ped_size = Vec3::new(0.50, 0.68, 0.90);
                        let ped_mesh = cache.cuboid(meshes, ped_size.x, ped_size.y, ped_size.z);
                        d.spawn((
                            DeskBreakoffPart {
                                trigger_stage: 2,
                                local_offset: Vec3::new(px, -0.03, 0.0),
                                size: ped_size,
                                mat: wood_mat.clone(),
                                eject_dir: Vec3::new(px.signum() * 2.6, 1.9, -0.8),
                            },
                            Mesh3d(ped_mesh.clone()),
                            MeshMaterial3d(wood_mat.clone()),
                            Transform::from_xyz(px, -0.03, 0.0),
                        ))
                        .with_children(|ped| {
                            ped.spawn((
                                Mesh3d(ped_mesh),
                                MeshMaterial3d(shared.ink_outline_mat.clone()),
                                Transform::from_scale(outline_scale_for(ped_size, 0.013)),
                            ));
                        });
                        // Ejectable desk drawer box (rips free at Stage 1!)
                        d.spawn((
                            DeskBreakoffPart {
                                trigger_stage: 1,
                                local_offset: Vec3::new(px, 0.14, 0.45),
                                size: Vec3::new(0.42, 0.22, 0.44),
                                mat: dark_wood.clone(),
                                eject_dir: Vec3::new(px.signum() * 1.6, 2.2, 2.6),
                            },
                            Mesh3d(cache.cuboid(meshes, 0.40, 0.22, 0.03)),
                            MeshMaterial3d(dark_wood.clone()),
                            Transform::from_xyz(px, 0.14, 0.45),
                        ));
                    }
                });
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_prop_prefab(
    p: &PropPlacement,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    cache: &mut PrefabAssetCache,
    textures: &ProceduralTextures,
    shared: &SharedMeshes,
    joke_rng: &mut CorporateJokeRng,
) {
    let tf = Transform::from_translation(Vec3::from_array(p.pos))
        .with_rotation(Quat::from_rotation_y(p.rot_y.to_radians()));

    let chosen_label = joke_rng.label_for(p.prefab, p.label.as_deref());
    let lbl = |_default: &str| chosen_label.clone();
    let pts = |default: u32| p.points.unwrap_or(default);

    match p.prefab {
        PropPrefab::CrtMonitor => {
            let beige = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.84, 0.79, 0.67),
                    perceptual_roughness: 0.66,
                    ..default()
                },
            );
            let bezel_dark = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.16, 0.18, 0.22),
                    perceptual_roughness: 0.75,
                    ..default()
                },
            );
            let screen_mat = cache.material(
                materials,
                StandardMaterial {
                    base_color_texture: Some(textures.crt_phosphor.clone()),
                    emissive: LinearRgba::rgb(0.28, 1.05, 0.96),
                    emissive_texture: Some(textures.crt_phosphor.clone()),
                    perceptual_roughness: 0.18,
                    ..default()
                },
            );
            let led_green = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.25, 0.98, 0.45),
                    emissive: LinearRgba::rgb(0.4, 2.2, 0.6),
                    ..default()
                },
            );

            let prop = DestructibleProp::small(
                lbl("CRT MONITOR"),
                pts(350),
                Color::srgb(0.84, 0.79, 0.67),
                Color::srgb(0.18, 0.92, 0.84),
                ImpactProfile::Electronics,
                SfxMaterial::Electronics,
            );

            commands
                .spawn((
                    ScenarioEntity,
                    prop,
                    RigidBody::Dynamic,
                    Mass(4.2),
                    Restitution::new(0.48),
                    Friction::new(0.32),
                    LinearDamping(0.10),
                    AngularDamping(0.10),
                    Collider::cuboid(0.48, 0.44, 0.44),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|crt| {
                    // Wide beveled base + swivel neck (`loading-desk.jpg`!)
                    spawn_outlined_box(
                        crt,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.34, 0.045, 0.28),
                        beige.clone(),
                        Transform::from_xyz(0.0, -0.20, 0.02),
                        0.011,
                    );
                    spawn_outlined_box(
                        crt,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.18, 0.05, 0.18),
                        bezel_dark.clone(),
                        Transform::from_xyz(0.0, -0.16, 0.02),
                        0.008,
                    );
                    // Main bulky front bezel housing
                    spawn_outlined_box(
                        crt,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.48, 0.39, 0.22),
                        beige.clone(),
                        Transform::from_xyz(0.0, 0.03, 0.08),
                        0.013,
                    );
                    // Tapered rear CRT electron-gun shell (`make_frustum_z_mesh`!)
                    let rear_mesh = cache.frustum_z(
                        meshes,
                        Vec2::new(0.42, 0.33),
                        Vec2::new(0.26, 0.22),
                        0.22,
                    );
                    crt.spawn((
                        Mesh3d(rear_mesh.clone()),
                        MeshMaterial3d(beige),
                        Transform::from_xyz(0.0, 0.03, -0.13),
                    ));
                    crt.spawn((
                        Mesh3d(rear_mesh),
                        MeshMaterial3d(shared.ink_outline_mat.clone()),
                        Transform::from_xyz(0.0, 0.03, -0.13).with_scale(Vec3::splat(1.06)),
                    ));
                    // Dark recessed inner screen frame
                    crt.spawn((
                        Mesh3d(cache.cuboid(meshes, 0.40, 0.30, 0.02)),
                        MeshMaterial3d(bezel_dark),
                        Transform::from_xyz(0.0, 0.04, 0.186),
                    ));
                    // Upright glowing phosphor screen (swaps to cracked static on damage!)
                    crt.spawn((
                        ScreenSurface,
                        Mesh3d(cache.upright_quad(meshes, 0.37, 0.27)),
                        MeshMaterial3d(screen_mat),
                        Transform::from_xyz(0.0, 0.04, 0.202),
                    ));
                    // Green power LED bottom-right
                    crt.spawn((
                        Mesh3d(cache.cuboid(meshes, 0.025, 0.014, 0.015)),
                        MeshMaterial3d(led_green),
                        Transform::from_xyz(0.16, -0.13, 0.195),
                    ));
                });
        }

        PropPrefab::MechanicalKeyboard => {
            let chassis_mat = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.82, 0.77, 0.65),
                    perceptual_roughness: 0.68,
                    ..default()
                },
            );
            let top_mat = cache.material(
                materials,
                StandardMaterial {
                    base_color_texture: Some(textures.keyboard_top.clone()),
                    perceptual_roughness: 0.62,
                    ..default()
                },
            );
            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::small(
                        lbl("KEYBOARD"),
                        pts(140),
                        Color::srgb(0.84, 0.79, 0.67),
                        Color::srgb(0.28, 0.32, 0.36),
                        ImpactProfile::Electronics,
                        SfxMaterial::Electronics,
                    ),
                    RigidBody::Dynamic,
                    Mass(1.05),
                    Restitution::new(0.52),
                    Friction::new(0.30),
                    LinearDamping(0.10),
                    AngularDamping(0.10),
                    Collider::cuboid(0.56, 0.045, 0.21),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|kb| {
                    // Slight ergonomic wedge tilt toward player
                    spawn_outlined_box(
                        kb,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.56, 0.036, 0.21),
                        chassis_mat,
                        Transform::IDENTITY.with_rotation(Quat::from_rotation_x(0.08)),
                        0.011,
                    );
                    kb.spawn((
                        Mesh3d(cache.floor_quad(meshes, 0.53, 0.185)),
                        MeshMaterial3d(top_mat),
                        Transform::from_xyz(0.0, 0.021, 0.0)
                            .with_rotation(Quat::from_rotation_x(0.08)),
                    ));
                });
        }

        PropPrefab::RedStapler => {
            let red = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.84, 0.14, 0.15),
                    perceptual_roughness: 0.38,
                    ..default()
                },
            );
            let chrome = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.76, 0.80, 0.84),
                    metallic: 0.7,
                    perceptual_roughness: 0.3,
                    ..default()
                },
            );
            let dark_base = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.12, 0.13, 0.16),
                    ..default()
                },
            );
            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::small(
                        lbl("RED STAPLER!"),
                        pts(220),
                        Color::srgb(0.86, 0.16, 0.16),
                        Color::srgb(0.76, 0.80, 0.84),
                        ImpactProfile::HeavyProp,
                        SfxMaterial::Metal,
                    ),
                    RigidBody::Dynamic,
                    Mass(0.65),
                    Restitution::new(0.58),
                    Friction::new(0.28),
                    LinearDamping(0.10),
                    AngularDamping(0.10),
                    Collider::cuboid(0.17, 0.065, 0.065),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|st| {
                    spawn_outlined_box(
                        st,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.17, 0.018, 0.062),
                        dark_base,
                        Transform::from_xyz(0.0, -0.02, 0.0),
                        0.008,
                    );
                    st.spawn((
                        Mesh3d(cache.cuboid(meshes, 0.14, 0.010, 0.045)),
                        MeshMaterial3d(chrome),
                        Transform::from_xyz(0.01, -0.006, 0.0),
                    ));
                    spawn_outlined_box(
                        st,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.165, 0.032, 0.058),
                        red,
                        Transform::from_xyz(-0.004, 0.016, 0.0)
                            .with_rotation(Quat::from_rotation_z(0.11)),
                        0.009,
                    );
                });
        }

        PropPrefab::SteamingCoffeeMug => {
            let ceramic = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.92, 0.91, 0.88),
                    perceptual_roughness: 0.32,
                    ..default()
                },
            );
            let coffee = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.20, 0.10, 0.05),
                    perceptual_roughness: 0.15,
                    ..default()
                },
            );
            let mut prop = DestructibleProp::small(
                lbl("HOT COFFEE!"),
                pts(180),
                Color::srgb(0.94, 0.92, 0.88),
                Color::srgb(0.24, 0.12, 0.06),
                ImpactProfile::Liquid {
                    liquid_color: Color::srgb(0.26, 0.13, 0.06),
                    puddle_radius: 0.62,
                },
                SfxMaterial::Liquid,
            );
            prop.damage_impulse = 0.95;
            prop.break_impulse = 1.85;

            let body_mesh = cache.cylinder(meshes, 0.058, 0.125);
            commands
                .spawn((
                    ScenarioEntity,
                    prop,
                    VfxEmitter {
                        timer: 0.1,
                        interval: 0.34,
                        offset: Vec3::new(0.0, 0.085, 0.0),
                        style: EmitterStyle::CoffeeSteam,
                    },
                    RigidBody::Dynamic,
                    Mass(0.55),
                    Restitution::new(0.48),
                    Friction::new(0.30),
                    LinearDamping(0.10),
                    AngularDamping(0.10),
                    Collider::cylinder(0.06, 0.125),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|mug| {
                    mug.spawn((Mesh3d(body_mesh.clone()), MeshMaterial3d(ceramic.clone())));
                    mug.spawn((
                        Mesh3d(body_mesh),
                        MeshMaterial3d(shared.ink_outline_mat.clone()),
                        Transform::from_scale(Vec3::new(1.14, 1.09, 1.14)),
                    ));
                    mug.spawn((
                        Mesh3d(cache.cylinder(meshes, 0.048, 0.014)),
                        MeshMaterial3d(coffee),
                        Transform::from_xyz(0.0, 0.058, 0.0),
                    ));
                    // C-shaped handle on the right (`loading-desk.jpg`!)
                    for (hy, hz, sx, sy) in [
                        (0.028, 0.0, 0.036, 0.016),
                        (-0.028, 0.0, 0.036, 0.016),
                        (0.0, 0.0, 0.016, 0.068),
                    ] {
                        let tx = if sy > 0.04 { 0.076 } else { 0.064 };
                        spawn_outlined_box(
                            mug,
                            meshes,
                            cache,
                            shared,
                            Vec3::new(sx, sy, 0.020 + hz),
                            ceramic.clone(),
                            Transform::from_xyz(tx, hy, 0.0),
                            0.006,
                        );
                    }
                });
        }

        PropPrefab::PaperStackTall | PropPrefab::PaperStackMessy | PropPrefab::LoosePaperFan => {
            let page_top = cache.material(
                materials,
                StandardMaterial {
                    base_color_texture: Some(textures.printed_page.clone()),
                    perceptual_roughness: 0.88,
                    ..default()
                },
            );
            let edge_white = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.90, 0.92, 0.95),
                    perceptual_roughness: 0.9,
                    ..default()
                },
            );

            let (layers, total_h, default_pts, default_lbl) = match p.prefab {
                PropPrefab::PaperStackTall => (5, 0.16, 160, "AUDIT STACK!"),
                PropPrefab::PaperStackMessy => (3, 0.10, 130, "TPS REPORTS"),
                _ => (2, 0.045, 110, "MEMO SHEETS"),
            };

            let mut prop = DestructibleProp::small(
                lbl(default_lbl),
                pts(default_pts),
                Color::srgb(0.93, 0.95, 0.97),
                Color::srgb(0.84, 0.88, 0.93),
                ImpactProfile::PaperStack { is_blueprint: false },
                SfxMaterial::Paper,
            );
            prop.damage_impulse = 0.85;
            prop.break_impulse = 1.55;
            prop.debris_count = 18;

            let slice_h = total_h / layers as f32;
            commands
                .spawn((
                    ScenarioEntity,
                    prop,
                    RigidBody::Dynamic,
                    Mass(0.65),
                    Restitution::new(0.40),
                    Friction::new(0.32),
                    LinearDamping(0.10),
                    AngularDamping(0.10),
                    Collider::cuboid(0.25, total_h.max(0.04), 0.31),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|st| {
                    for i in 0..layers {
                        let y = -total_h * 0.5 + (i as f32 + 0.5) * slice_h;
                        let yaw = ((i as f32 * 1.7).sin()) * 0.12;
                        let ox = ((i as f32 * 2.3).cos()) * 0.015;
                        let sub_tf = Transform::from_xyz(ox, y, 0.0)
                            .with_rotation(Quat::from_rotation_y(yaw));
                        spawn_outlined_box(
                            st,
                            meshes,
                            cache,
                            shared,
                            Vec3::new(0.24, (slice_h * 0.88).max(0.012), 0.30),
                            edge_white.clone(),
                            sub_tf,
                            0.008,
                        );
                        if i == layers - 1 {
                            let mut top_tf = sub_tf;
                            top_tf.translation.y += slice_h * 0.46 + 0.002;
                            st.spawn((
                                Mesh3d(cache.floor_quad(meshes, 0.23, 0.29)),
                                MeshMaterial3d(page_top.clone()),
                                top_tf,
                            ));
                        }
                    }
                });
        }

        PropPrefab::CubicleWallPinned | PropPrefab::CubicleWallPoster => {
            let fabric = cache.material(
                materials,
                StandardMaterial {
                    base_color_texture: Some(textures.cubicle_fabric.clone()),
                    uv_transform: Affine2::from_scale(Vec2::new(4.0, 3.0)),
                    perceptual_roughness: 0.92,
                    ..default()
                },
            );
            let alum_trim = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.46, 0.52, 0.60),
                    metallic: 0.35,
                    perceptual_roughness: 0.55,
                    ..default()
                },
            );
            let memos_mat = cache.material(
                materials,
                StandardMaterial {
                    base_color_texture: Some(textures.pinned_memos.clone()),
                    alpha_mode: AlphaMode::Mask(0.5),
                    perceptual_roughness: 0.85,
                    ..default()
                },
            );
            let cat_mat = cache.material(
                materials,
                StandardMaterial {
                    base_color_texture: Some(textures.cat_poster.clone()),
                    perceptual_roughness: 0.65,
                    ..default()
                },
            );

            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::tall(
                        lbl("CUBICLE WALL"),
                        pts(360),
                        Color::srgb(0.32, 0.40, 0.50),
                        Color::srgb(0.55, 0.60, 0.68),
                        ImpactProfile::HeavyProp,
                        SfxMaterial::WoodCeramic,
                    ),
                    RigidBody::Dynamic,
                    Mass(12.5),
                    Restitution::new(0.44),
                    Friction::new(0.34),
                    LinearDamping(0.10),
                    AngularDamping(0.10),
                    Collider::cuboid(1.68, 1.60, 0.13),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|part| {
                    spawn_outlined_box(
                        part,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(1.64, 1.54, 0.10),
                        fabric,
                        Transform::IDENTITY,
                        0.015,
                    );
                    // Top & side aluminum frame rails (`loading-desk.jpg`!)
                    spawn_outlined_box(
                        part,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(1.70, 0.06, 0.13),
                        alum_trim.clone(),
                        Transform::from_xyz(0.0, 0.78, 0.0),
                        0.010,
                    );
                    for sx in [-0.83_f32, 0.83_f32] {
                        part.spawn((
                            Mesh3d(cache.cuboid(meshes, 0.05, 1.58, 0.12)),
                            MeshMaterial3d(alum_trim.clone()),
                            Transform::from_xyz(sx, 0.0, 0.0),
                        ));
                    }
                    // Pinned memos on front & back
                    part.spawn((
                        Mesh3d(cache.upright_quad(meshes, 0.42, 0.42)),
                        MeshMaterial3d(memos_mat),
                        Transform::from_xyz(-0.34, 0.22, 0.062),
                    ));
                    if p.prefab == PropPrefab::CubicleWallPoster {
                        part.spawn((
                            GlassSurface {
                                size: Vec2::new(0.38, 0.48),
                                z_offset: 0.075,
                            },
                            Mesh3d(cache.upright_quad(meshes, 0.38, 0.48)),
                            MeshMaterial3d(cat_mat),
                            Transform::from_xyz(0.28, 0.18, 0.064)
                                .with_rotation(Quat::from_rotation_z(-0.07)),
                        ));
                    }
                });
        }

        PropPrefab::FilingCabinetOlive3 | PropPrefab::FilingCabinetOlive4 => {
            let olive = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.36, 0.41, 0.28),
                    perceptual_roughness: 0.62,
                    ..default()
                },
            );
            let olive_drawer = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.30, 0.35, 0.23),
                    perceptual_roughness: 0.58,
                    ..default()
                },
            );
            let chrome = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.76, 0.80, 0.82),
                    metallic: 0.6,
                    perceptual_roughness: 0.35,
                    ..default()
                },
            );

            let (drawers, height, pts_def) = if p.prefab == PropPrefab::FilingCabinetOlive4 {
                (4, 1.62_f32, 480)
            } else {
                (3, 1.24_f32, 420)
            };

            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::tall(
                        lbl("FILING CABINET"),
                        pts(pts_def),
                        Color::srgb(0.36, 0.41, 0.28),
                        Color::srgb(0.92, 0.94, 0.96),
                        ImpactProfile::FilingCabinet,
                        SfxMaterial::Metal,
                    ),
                    RigidBody::Dynamic,
                    Mass(14.5),
                    Restitution::new(0.46),
                    Friction::new(0.32),
                    LinearDamping(0.10),
                    AngularDamping(0.10),
                    Collider::cuboid(0.62, height, 0.64),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|cab| {
                    spawn_outlined_box(
                        cab,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.62, height, 0.64),
                        olive,
                        Transform::IDENTITY,
                        0.014,
                    );
                    let step = (height - 0.14) / drawers as f32;
                    for i in 0..drawers {
                        let dy = -height * 0.5 + 0.07 + (i as f32 + 0.5) * step;
                        spawn_outlined_box(
                            cab,
                            meshes,
                            cache,
                            shared,
                            Vec3::new(0.52, step * 0.82, 0.03),
                            olive_drawer.clone(),
                            Transform::from_xyz(0.0, dy, 0.325),
                            0.008,
                        );
                        // Silver label holder + pull handle (`loading-desk.jpg`!)
                        cab.spawn((
                            Mesh3d(cache.cuboid(meshes, 0.11, 0.045, 0.02)),
                            MeshMaterial3d(chrome.clone()),
                            Transform::from_xyz(0.0, dy + 0.04, 0.345),
                        ));
                        cab.spawn((
                            Mesh3d(cache.cuboid(meshes, 0.13, 0.02, 0.025)),
                            MeshMaterial3d(chrome.clone()),
                            Transform::from_xyz(0.0, dy - 0.04, 0.348),
                        ));
                    }
                });
        }

        PropPrefab::BinderPair => {
            let dark_binder = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.18, 0.21, 0.27),
                    perceptual_roughness: 0.75,
                    ..default()
                },
            );
            let spine_tag = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.90, 0.88, 0.80),
                    ..default()
                },
            );
            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::small(
                        lbl("OFFICE BINDERS!"),
                        pts(160),
                        Color::srgb(0.20, 0.24, 0.32),
                        Color::srgb(0.94, 0.95, 0.96),
                        ImpactProfile::PaperStack { is_blueprint: false },
                        SfxMaterial::Paper,
                    ),
                    RigidBody::Dynamic,
                    Mass(1.2),
                    Restitution::new(0.46),
                    Friction::new(0.30),
                    Collider::cuboid(0.22, 0.30, 0.26),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|b| {
                    for bx in [-0.052_f32, 0.052_f32] {
                        spawn_outlined_box(
                            b,
                            meshes,
                            cache,
                            shared,
                            Vec3::new(0.085, 0.30, 0.25),
                            dark_binder.clone(),
                            Transform::from_xyz(bx, 0.0, 0.0),
                            0.008,
                        );
                        b.spawn((
                            Mesh3d(cache.cuboid(meshes, 0.045, 0.09, 0.015)),
                            MeshMaterial3d(spine_tag.clone()),
                            Transform::from_xyz(bx, 0.05, 0.128),
                        ));
                    }
                });
        }

        PropPrefab::PaperInTray => {
            let tray_mat = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.16, 0.19, 0.24),
                    perceptual_roughness: 0.65,
                    ..default()
                },
            );
            let page_mat = cache.material(
                materials,
                StandardMaterial {
                    base_color_texture: Some(textures.printed_page.clone()),
                    ..default()
                },
            );
            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::small(
                        lbl("INBOX TRAY!"),
                        pts(150),
                        Color::srgb(0.18, 0.22, 0.28),
                        Color::srgb(0.94, 0.95, 0.97),
                        ImpactProfile::PaperStack { is_blueprint: false },
                        SfxMaterial::Paper,
                    ),
                    RigidBody::Dynamic,
                    Mass(1.1),
                    Restitution::new(0.48),
                    Friction::new(0.30),
                    Collider::cuboid(0.34, 0.18, 0.28),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|tr| {
                    for ty in [-0.05_f32, 0.05_f32] {
                        spawn_outlined_box(
                            tr,
                            meshes,
                            cache,
                            shared,
                            Vec3::new(0.34, 0.05, 0.28),
                            tray_mat.clone(),
                            Transform::from_xyz(0.0, ty, 0.0),
                            0.008,
                        );
                        tr.spawn((
                            Mesh3d(cache.floor_quad(meshes, 0.28, 0.22)),
                            MeshMaterial3d(page_mat.clone()),
                            Transform::from_xyz(0.0, ty + 0.028, 0.0),
                        ));
                    }
                });
        }

        PropPrefab::WaterCooler => {
            let body_mat = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.84, 0.86, 0.89),
                    perceptual_roughness: 0.48,
                    ..default()
                },
            );
            let recess_mat = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.14, 0.17, 0.22),
                    ..default()
                },
            );
            let jug_mat = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgba(0.18, 0.62, 0.92, 0.82),
                    emissive: LinearRgba::rgb(0.06, 0.26, 0.44),
                    alpha_mode: AlphaMode::Blend,
                    perceptual_roughness: 0.16,
                    ..default()
                },
            );
            let tap_red = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.90, 0.22, 0.20),
                    ..default()
                },
            );
            let tap_blue = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.20, 0.56, 0.95),
                    ..default()
                },
            );

            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::tall(
                        lbl("WATER COOLER!"),
                        pts(520),
                        Color::srgb(0.24, 0.72, 0.98),
                        Color::srgb(0.88, 0.90, 0.94),
                        ImpactProfile::Liquid {
                            liquid_color: Color::srgb(0.22, 0.68, 0.96),
                            puddle_radius: 1.35,
                        },
                        SfxMaterial::Liquid,
                    ),
                    RigidBody::Dynamic,
                    Mass(10.5),
                    Restitution::new(0.50),
                    Friction::new(0.30),
                    LinearDamping(0.10),
                    AngularDamping(0.10),
                    Collider::cuboid(0.46, 1.12, 0.46),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|wc| {
                    spawn_outlined_box(
                        wc,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.44, 0.98, 0.44),
                        body_mat.clone(),
                        Transform::from_xyz(0.0, -0.06, 0.0),
                        0.013,
                    );
                    // Tap recess + red/blue hot/cold taps (`loading-desk.jpg`!)
                    wc.spawn((
                        Mesh3d(cache.cuboid(meshes, 0.26, 0.22, 0.03)),
                        MeshMaterial3d(recess_mat),
                        Transform::from_xyz(0.0, 0.18, 0.215),
                    ));
                    wc.spawn((
                        Mesh3d(cache.cuboid(meshes, 0.04, 0.05, 0.04)),
                        MeshMaterial3d(tap_red),
                        Transform::from_xyz(-0.06, 0.22, 0.23),
                    ));
                    wc.spawn((
                        Mesh3d(cache.cuboid(meshes, 0.04, 0.05, 0.04)),
                        MeshMaterial3d(tap_blue),
                        Transform::from_xyz(0.06, 0.22, 0.23),
                    ));
                    // Side cup dispenser tube
                    wc.spawn((
                        Mesh3d(cache.cylinder(meshes, 0.04, 0.36)),
                        MeshMaterial3d(body_mat),
                        Transform::from_xyz(0.25, 0.16, 0.08),
                    ));
                    // Ribbed 5-gallon translucent blue water jug
                    let jug_mesh = cache.cylinder(meshes, 0.185, 0.44);
                    wc.spawn((
                        Mesh3d(jug_mesh.clone()),
                        MeshMaterial3d(jug_mat),
                        Transform::from_xyz(0.0, 0.66, 0.0),
                    ));
                    wc.spawn((
                        Mesh3d(jug_mesh),
                        MeshMaterial3d(shared.ink_outline_mat.clone()),
                        Transform::from_xyz(0.0, 0.66, 0.0).with_scale(Vec3::new(1.08, 1.05, 1.08)),
                    ));
                });
        }

        PropPrefab::TrashBin => {
            let bin_mat = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.24, 0.28, 0.34),
                    perceptual_roughness: 0.78,
                    ..default()
                },
            );
            let paper_ball = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.90, 0.92, 0.94),
                    ..default()
                },
            );
            let bin_mesh = cache.frustum_y(
                meshes,
                Vec2::new(0.24, 0.24),
                Vec2::new(0.34, 0.34),
                0.42,
            );
            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::small(
                        lbl("TRASH CAN!"),
                        pts(140),
                        Color::srgb(0.26, 0.30, 0.36),
                        Color::srgb(0.92, 0.94, 0.96),
                        ImpactProfile::PaperStack { is_blueprint: false },
                        SfxMaterial::Paper,
                    ),
                    RigidBody::Dynamic,
                    Mass(1.4),
                    Restitution::new(0.54),
                    Friction::new(0.28),
                    Collider::cuboid(0.32, 0.42, 0.32),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|tb| {
                    tb.spawn((Mesh3d(bin_mesh.clone()), MeshMaterial3d(bin_mat)));
                    tb.spawn((
                        Mesh3d(bin_mesh),
                        MeshMaterial3d(shared.ink_outline_mat.clone()),
                        Transform::from_scale(Vec3::splat(1.07)),
                    ));
                    for (px, pz) in [(-0.05, 0.03), (0.06, -0.04), (0.01, 0.05)] {
                        tb.spawn((
                            Mesh3d(cache.cuboid(meshes, 0.09, 0.09, 0.09)),
                            MeshMaterial3d(paper_ball.clone()),
                            Transform::from_xyz(px, 0.16, pz)
                                .with_rotation(Quat::from_rotation_y(0.6)),
                        ));
                    }
                });
        }

        PropPrefab::SwivelChair | PropPrefab::LeatherExecChair => {
            let is_exec = p.prefab == PropPrefab::LeatherExecChair;
            let cushion = cache.material(
                materials,
                StandardMaterial {
                    base_color: if is_exec {
                        Color::srgb(0.28, 0.14, 0.10)
                    } else {
                        Color::srgb(0.16, 0.19, 0.25)
                    },
                    perceptual_roughness: if is_exec { 0.45 } else { 0.78 },
                    ..default()
                },
            );
            let frame_mat = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.12, 0.14, 0.18),
                    metallic: 0.4,
                    perceptual_roughness: 0.5,
                    ..default()
                },
            );
            let back_h = if is_exec { 0.64 } else { 0.44 };

            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::tall(
                        lbl(if is_exec { "EXEC LEATHER CHAIR!" } else { "SWIVEL CHAIR" }),
                        pts(if is_exec { 320 } else { 220 }),
                        if is_exec {
                            Color::srgb(0.32, 0.16, 0.11)
                        } else {
                            Color::srgb(0.18, 0.22, 0.28)
                        },
                        Color::srgb(0.12, 0.14, 0.18),
                        ImpactProfile::HeavyProp,
                        SfxMaterial::Metal,
                    ),
                    RigidBody::Dynamic,
                    Mass(if is_exec { 8.0 } else { 6.0 }),
                    Restitution::new(0.56),
                    Friction::new(0.24),
                    LinearDamping(0.08),
                    AngularDamping(0.08),
                    Collider::cuboid(0.54, 0.94, 0.54),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|ch| {
                    // 5-star caster legs
                    for i in 0..5 {
                        let a = (i as f32 / 5.0) * std::f32::consts::TAU;
                        ch.spawn((
                            Mesh3d(cache.cuboid(meshes, 0.05, 0.04, 0.28)),
                            MeshMaterial3d(frame_mat.clone()),
                            Transform::from_xyz(a.sin() * 0.13, -0.42, a.cos() * 0.13)
                                .with_rotation(Quat::from_rotation_y(a)),
                        ));
                    }
                    ch.spawn((
                        Mesh3d(cache.cylinder(meshes, 0.038, 0.38)),
                        MeshMaterial3d(frame_mat.clone()),
                        Transform::from_xyz(0.0, -0.22, 0.0),
                    ));
                    spawn_outlined_box(
                        ch,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.50, 0.09, 0.48),
                        cushion.clone(),
                        Transform::from_xyz(0.0, 0.0, 0.0),
                        0.011,
                    );
                    spawn_outlined_box(
                        ch,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.46, back_h, 0.09),
                        cushion,
                        Transform::from_xyz(0.0, back_h * 0.5 + 0.04, -0.21),
                        0.011,
                    );
                    // Armrests (`loading-desk.jpg`!)
                    for ax in [-0.25_f32, 0.25_f32] {
                        ch.spawn((
                            Mesh3d(cache.cuboid(meshes, 0.04, 0.18, 0.04)),
                            MeshMaterial3d(frame_mat.clone()),
                            Transform::from_xyz(ax, 0.10, -0.04),
                        ));
                        ch.spawn((
                            Mesh3d(cache.cuboid(meshes, 0.06, 0.035, 0.28)),
                            MeshMaterial3d(frame_mat.clone()),
                            Transform::from_xyz(ax, 0.19, -0.02),
                        ));
                    }
                });
        }

        PropPrefab::CoworkerDesk => {
            let wood = cache.material(
                materials,
                StandardMaterial {
                    base_color_texture: Some(textures.walnut_desk.clone()),
                    uv_transform: Affine2::from_scale(Vec2::new(2.0, 1.0)),
                    perceptual_roughness: 0.65,
                    ..default()
                },
            );
            let leg = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.20, 0.12, 0.07),
                    ..default()
                },
            );
            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::tall(
                        lbl("COWORKER DESK"),
                        pts(400),
                        Color::srgb(0.56, 0.32, 0.16),
                        Color::srgb(0.24, 0.14, 0.08),
                        ImpactProfile::HeavyProp,
                        SfxMaterial::WoodCeramic,
                    ),
                    RigidBody::Dynamic,
                    Mass(15.5),
                    Restitution::new(0.44),
                    Friction::new(0.32),
                    LinearDamping(0.10),
                    AngularDamping(0.10),
                    Collider::cuboid(1.75, 0.76, 0.84),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|dk| {
                    spawn_outlined_box(
                        dk,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(1.75, 0.07, 0.84),
                        wood.clone(),
                        Transform::from_xyz(0.0, 0.34, 0.0),
                        0.014,
                    );
                    for lx in [-0.78_f32, 0.78_f32] {
                        spawn_outlined_box(
                            dk,
                            meshes,
                            cache,
                            shared,
                            Vec3::new(0.12, 0.68, 0.76),
                            leg.clone(),
                            Transform::from_xyz(lx, -0.03, 0.0),
                            0.011,
                        );
                    }
                });
        }

        PropPrefab::PcTower => {
            let chassis = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.20, 0.23, 0.29),
                    perceptual_roughness: 0.62,
                    ..default()
                },
            );
            let drive = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.12, 0.14, 0.18),
                    ..default()
                },
            );
            let led = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.25, 0.98, 0.45),
                    emissive: LinearRgba::rgb(0.4, 2.2, 0.6),
                    ..default()
                },
            );
            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::small(
                        lbl("SERVER TOWER!"),
                        pts(420),
                        Color::srgb(0.20, 0.24, 0.30),
                        Color::srgb(0.16, 0.62, 0.30),
                        ImpactProfile::Electronics,
                        SfxMaterial::Electronics,
                    ),
                    RigidBody::Dynamic,
                    Mass(4.6),
                    Restitution::new(0.48),
                    Friction::new(0.30),
                    Collider::cuboid(0.24, 0.52, 0.48),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|pc| {
                    spawn_outlined_box(
                        pc,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.24, 0.52, 0.48),
                        chassis,
                        Transform::IDENTITY,
                        0.011,
                    );
                    for dy in [0.16_f32, 0.08] {
                        pc.spawn((
                            Mesh3d(cache.cuboid(meshes, 0.18, 0.045, 0.02)),
                            MeshMaterial3d(drive.clone()),
                            Transform::from_xyz(0.0, dy, 0.242),
                        ));
                    }
                    pc.spawn((
                        Mesh3d(cache.cuboid(meshes, 0.025, 0.025, 0.02)),
                        MeshMaterial3d(led),
                        Transform::from_xyz(-0.06, -0.04, 0.244),
                    ));
                });
        }

        PropPrefab::OfficeCopier => {
            let body = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.82, 0.81, 0.76),
                    perceptual_roughness: 0.55,
                    ..default()
                },
            );
            let dark = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.18, 0.21, 0.26),
                    ..default()
                },
            );
            let led = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.98, 0.32, 0.24),
                    emissive: LinearRgba::rgb(2.4, 0.4, 0.25),
                    ..default()
                },
            );
            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::tall(
                        lbl("PC LOAD LETTER!"),
                        pts(520),
                        Color::srgb(0.84, 0.83, 0.78),
                        Color::srgb(0.18, 0.22, 0.28),
                        ImpactProfile::PressurizedHazard,
                        SfxMaterial::PressurizedBlast,
                    ),
                    RigidBody::Dynamic,
                    Mass(12.0),
                    Restitution::new(0.48),
                    Friction::new(0.30),
                    Collider::cuboid(0.76, 0.98, 0.64),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|pr| {
                    spawn_outlined_box(
                        pr,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.74, 0.58, 0.62),
                        body.clone(),
                        Transform::from_xyz(0.0, 0.14, 0.0),
                        0.014,
                    );
                    spawn_outlined_box(
                        pr,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.64, 0.38, 0.54),
                        dark.clone(),
                        Transform::from_xyz(0.0, -0.32, 0.0),
                        0.011,
                    );
                    // Scanner top lid
                    spawn_outlined_box(
                        pr,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.70, 0.06, 0.58),
                        body,
                        Transform::from_xyz(0.0, 0.46, -0.02)
                            .with_rotation(Quat::from_rotation_x(-0.08)),
                        0.010,
                    );
                    // Blinking red jam indicator!
                    pr.spawn((
                        Mesh3d(cache.cuboid(meshes, 0.09, 0.04, 0.03)),
                        MeshMaterial3d(led),
                        Transform::from_xyz(0.24, 0.30, 0.315),
                    ));
                });
        }

        PropPrefab::BookshelfManuals => {
            let wood = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.46, 0.28, 0.15),
                    perceptual_roughness: 0.74,
                    ..default()
                },
            );
            let book_colors = [
                Color::srgb(0.82, 0.22, 0.22),
                Color::srgb(0.20, 0.50, 0.82),
                Color::srgb(0.88, 0.68, 0.20),
                Color::srgb(0.24, 0.64, 0.44),
            ];
            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::tall(
                        lbl("SYNERGY MANUALS!"),
                        pts(480),
                        Color::srgb(0.50, 0.32, 0.18),
                        Color::srgb(0.94, 0.95, 0.97),
                        ImpactProfile::PaperStack { is_blueprint: false },
                        SfxMaterial::Paper,
                    ),
                    RigidBody::Dynamic,
                    Mass(13.5),
                    Restitution::new(0.45),
                    Friction::new(0.30),
                    Collider::cuboid(1.12, 1.66, 0.42),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|bs| {
                    spawn_outlined_box(
                        bs,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(1.12, 1.66, 0.42),
                        wood,
                        Transform::IDENTITY,
                        0.015,
                    );
                    for (r, by) in [-0.42_f32, 0.08, 0.54].iter().enumerate() {
                        for b_idx in 0..5 {
                            let bm = cache.material(
                                materials,
                                StandardMaterial {
                                    base_color: book_colors[(r + b_idx) % book_colors.len()],
                                    perceptual_roughness: 0.68,
                                    ..default()
                                },
                            );
                            let bx = -0.36 + b_idx as f32 * 0.18;
                            spawn_outlined_box(
                                bs,
                                meshes,
                                cache,
                                shared,
                                Vec3::new(0.14, 0.26, 0.28),
                                bm,
                                Transform::from_xyz(bx, *by, 0.08),
                                0.007,
                            );
                        }
                    }
                });
        }

        PropPrefab::OfficeFern => {
            let pot_mat = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.76, 0.38, 0.21),
                    perceptual_roughness: 0.80,
                    ..default()
                },
            );
            let leaf_mat = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.18, 0.62, 0.30),
                    perceptual_roughness: 0.72,
                    ..default()
                },
            );
            let pot_mesh = cache.frustum_y(
                meshes,
                Vec2::new(0.24, 0.24),
                Vec2::new(0.38, 0.38),
                0.38,
            );
            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::tall(
                        lbl("OFFICE PLANT!"),
                        pts(260),
                        Color::srgb(0.22, 0.66, 0.34),
                        Color::srgb(0.76, 0.38, 0.21),
                        ImpactProfile::HeavyProp,
                        SfxMaterial::WoodCeramic,
                    ),
                    RigidBody::Dynamic,
                    Mass(4.2),
                    Restitution::new(0.48),
                    Friction::new(0.30),
                    Collider::cylinder(0.24, 0.88),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|pl| {
                    pl.spawn((
                        Mesh3d(pot_mesh.clone()),
                        MeshMaterial3d(pot_mat),
                        Transform::from_xyz(0.0, -0.24, 0.0),
                    ));
                    pl.spawn((
                        Mesh3d(pot_mesh),
                        MeshMaterial3d(shared.ink_outline_mat.clone()),
                        Transform::from_xyz(0.0, -0.24, 0.0).with_scale(Vec3::splat(1.07)),
                    ));
                    // Splayed leafy fronds!
                    for i in 0..6 {
                        let a = (i as f32 / 6.0) * std::f32::consts::TAU;
                        spawn_outlined_box(
                            pl,
                            meshes,
                            cache,
                            shared,
                            Vec3::new(0.14, 0.38, 0.06),
                            leaf_mat.clone(),
                            Transform::from_xyz(a.cos() * 0.12, 0.14, a.sin() * 0.12)
                                .with_rotation(
                                    Quat::from_rotation_y(-a) * Quat::from_rotation_z(-0.55),
                                ),
                            0.008,
                        );
                    }
                });
        }

        PropPrefab::FluorescentFixture => {
            let housing = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.22, 0.26, 0.34),
                    ..default()
                },
            );
            let tube = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.94, 0.98, 1.0),
                    emissive: LinearRgba::rgb(1.45, 1.55, 1.65),
                    ..default()
                },
            );
            commands
                .spawn((
                    ScenarioEntity,
                    Mesh3d(cache.cuboid(meshes, 2.3, 0.07, 0.48)),
                    MeshMaterial3d(housing.clone()),
                    tf,
                ))
                .with_children(|fl| {
                    fl.spawn((
                        Mesh3d(cache.cuboid(meshes, 2.18, 0.03, 0.36)),
                        MeshMaterial3d(tube),
                        Transform::from_xyz(0.0, -0.035, 0.0),
                    ));
                    for cx in [-0.85_f32, 0.85_f32] {
                        fl.spawn((
                            Mesh3d(cache.cylinder(meshes, 0.012, 0.45)),
                            MeshMaterial3d(housing.clone()),
                            Transform::from_xyz(cx, 0.24, 0.0),
                        ));
                    }
                });
        }

        PropPrefab::HardHat => {
            let yellow = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.98, 0.76, 0.10),
                    perceptual_roughness: 0.34,
                    ..default()
                },
            );
            let dome = cache.frustum_y(
                meshes,
                Vec2::new(0.26, 0.30),
                Vec2::new(0.15, 0.19),
                0.13,
            );
            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::small(
                        lbl("OSHA VIOLATION!"),
                        pts(250),
                        Color::srgb(0.98, 0.76, 0.12),
                        Color::srgb(0.20, 0.22, 0.26),
                        ImpactProfile::HeavyProp,
                        SfxMaterial::WoodCeramic,
                    ),
                    RigidBody::Dynamic,
                    Mass(0.75),
                    Restitution::new(0.62),
                    Friction::new(0.26),
                    Collider::cylinder(0.15, 0.14),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|hat| {
                    hat.spawn((Mesh3d(dome.clone()), MeshMaterial3d(yellow.clone())));
                    hat.spawn((
                        Mesh3d(dome),
                        MeshMaterial3d(shared.ink_outline_mat.clone()),
                        Transform::from_scale(Vec3::splat(1.08)),
                    ));
                    spawn_outlined_box(
                        hat,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.28, 0.022, 0.36),
                        yellow,
                        Transform::from_xyz(0.0, -0.055, 0.03),
                        0.008,
                    );
                });
        }

        PropPrefab::RolledBlueprints => {
            let bp_mat = cache.material(
                materials,
                StandardMaterial {
                    base_color_texture: Some(textures.blueprint_page.clone()),
                    perceptual_roughness: 0.80,
                    ..default()
                },
            );
            let band_mat = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.82, 0.28, 0.24),
                    ..default()
                },
            );
            let mut prop = DestructibleProp::small(
                lbl("BLUEPRINTS!"),
                pts(180),
                Color::srgb(0.22, 0.56, 0.94),
                Color::srgb(0.88, 0.94, 1.0),
                ImpactProfile::PaperStack { is_blueprint: true },
                SfxMaterial::Paper,
            );
            prop.break_impulse = 1.6;
            let cyl = cache.cylinder(meshes, 0.052, 0.54);
            commands
                .spawn((
                    ScenarioEntity,
                    prop,
                    RigidBody::Dynamic,
                    Mass(0.65),
                    Restitution::new(0.50),
                    Friction::new(0.28),
                    Collider::cuboid(0.11, 0.11, 0.54),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|bp| {
                    let rot = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
                    bp.spawn((
                        Mesh3d(cyl.clone()),
                        MeshMaterial3d(bp_mat),
                        Transform::from_rotation(rot),
                    ));
                    bp.spawn((
                        Mesh3d(cyl),
                        MeshMaterial3d(shared.ink_outline_mat.clone()),
                        Transform::from_rotation(rot).with_scale(Vec3::new(1.14, 1.03, 1.14)),
                    ));
                    bp.spawn((
                        Mesh3d(cache.cylinder(meshes, 0.056, 0.03)),
                        MeshMaterial3d(band_mat),
                        Transform::from_rotation(rot),
                    ));
                });
        }

        PropPrefab::RedThermos => {
            let red = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.78, 0.18, 0.16),
                    perceptual_roughness: 0.38,
                    ..default()
                },
            );
            let steel = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.72, 0.76, 0.80),
                    metallic: 0.65,
                    perceptual_roughness: 0.32,
                    ..default()
                },
            );
            let body = cache.cylinder(meshes, 0.062, 0.25);
            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::small(
                        lbl("HOT THERMOS!"),
                        pts(200),
                        Color::srgb(0.80, 0.20, 0.18),
                        Color::srgb(0.72, 0.76, 0.80),
                        ImpactProfile::Liquid {
                            liquid_color: Color::srgb(0.26, 0.13, 0.06),
                            puddle_radius: 0.72,
                        },
                        SfxMaterial::Liquid,
                    ),
                    RigidBody::Dynamic,
                    Mass(0.95),
                    Restitution::new(0.55),
                    Friction::new(0.28),
                    Collider::cylinder(0.064, 0.28),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|th| {
                    th.spawn((Mesh3d(body.clone()), MeshMaterial3d(red)));
                    th.spawn((
                        Mesh3d(body),
                        MeshMaterial3d(shared.ink_outline_mat.clone()),
                        Transform::from_scale(Vec3::new(1.12, 1.05, 1.12)),
                    ));
                    th.spawn((
                        Mesh3d(cache.cylinder(meshes, 0.056, 0.06)),
                        MeshMaterial3d(steel),
                        Transform::from_xyz(0.0, 0.14, 0.0),
                    ));
                });
        }

        PropPrefab::WalkieTalkie => {
            let dark = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.16, 0.18, 0.22),
                    ..default()
                },
            );
            let amber = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(1.0, 0.72, 0.18),
                    emissive: LinearRgba::rgb(2.0, 1.2, 0.2),
                    ..default()
                },
            );
            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::small(
                        lbl("SITE RADIO!"),
                        pts(210),
                        Color::srgb(0.18, 0.20, 0.25),
                        Color::srgb(1.0, 0.76, 0.18),
                        ImpactProfile::Electronics,
                        SfxMaterial::Electronics,
                    ),
                    RigidBody::Dynamic,
                    Mass(0.65),
                    Restitution::new(0.56),
                    Friction::new(0.28),
                    Collider::cuboid(0.09, 0.20, 0.06),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|r| {
                    spawn_outlined_box(
                        r,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.085, 0.18, 0.055),
                        dark.clone(),
                        Transform::IDENTITY,
                        0.008,
                    );
                    r.spawn((
                        Mesh3d(cache.cylinder(meshes, 0.010, 0.14)),
                        MeshMaterial3d(dark),
                        Transform::from_xyz(0.025, 0.15, 0.0),
                    ));
                    r.spawn((
                        Mesh3d(cache.cuboid(meshes, 0.05, 0.035, 0.01)),
                        MeshMaterial3d(amber),
                        Transform::from_xyz(0.0, 0.04, 0.030),
                    ));
                });
        }

        PropPrefab::SiteWhiteboard => {
            let board_mat = cache.material(
                materials,
                StandardMaterial {
                    base_color_texture: Some(textures.whiteboard_doodle.clone()),
                    perceptual_roughness: 0.35,
                    ..default()
                },
            );
            let frame_mat = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.52, 0.56, 0.62),
                    metallic: 0.45,
                    perceptual_roughness: 0.45,
                    ..default()
                },
            );
            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::tall(
                        lbl("SITE WHITEBOARD!"),
                        pts(440),
                        Color::srgb(0.94, 0.96, 0.98),
                        Color::srgb(0.45, 0.50, 0.58),
                        ImpactProfile::GlassCrystal,
                        SfxMaterial::Glass,
                    ),
                    RigidBody::Dynamic,
                    Mass(11.0),
                    Restitution::new(0.48),
                    Friction::new(0.28),
                    Collider::cuboid(1.56, 1.50, 0.14),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|wb| {
                    spawn_outlined_box(
                        wb,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(1.54, 1.12, 0.06),
                        frame_mat.clone(),
                        Transform::from_xyz(0.0, 0.14, 0.0),
                        0.013,
                    );
                    wb.spawn((
                        GlassSurface {
                            size: Vec2::new(1.46, 1.04),
                            z_offset: 0.04,
                        },
                        Mesh3d(cache.upright_quad(meshes, 1.46, 1.04)),
                        MeshMaterial3d(board_mat),
                        Transform::from_xyz(0.0, 0.14, 0.034),
                    ));
                    for lx in [-0.72_f32, 0.72_f32] {
                        wb.spawn((
                            Mesh3d(cache.cuboid(meshes, 0.05, 0.72, 0.05)),
                            MeshMaterial3d(frame_mat.clone()),
                            Transform::from_xyz(lx, -0.38, 0.0),
                        ));
                    }
                });
        }

        PropPrefab::MiniFridge => {
            let body = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.86, 0.88, 0.90),
                    perceptual_roughness: 0.40,
                    ..default()
                },
            );
            let handle = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.22, 0.25, 0.30),
                    ..default()
                },
            );
            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::tall(
                        lbl("LUNCH FRIDGE!"),
                        pts(480),
                        Color::srgb(0.88, 0.90, 0.92),
                        Color::srgb(0.20, 0.76, 0.96),
                        ImpactProfile::Liquid {
                            liquid_color: Color::srgb(0.92, 0.56, 0.14),
                            puddle_radius: 0.95,
                        },
                        SfxMaterial::Metal,
                    ),
                    RigidBody::Dynamic,
                    Mass(14.5),
                    Restitution::new(0.45),
                    Friction::new(0.32),
                    Collider::cuboid(0.66, 0.96, 0.64),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|fr| {
                    spawn_outlined_box(
                        fr,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.66, 0.96, 0.64),
                        body,
                        Transform::IDENTITY,
                        0.014,
                    );
                    fr.spawn((
                        Mesh3d(cache.cuboid(meshes, 0.04, 0.28, 0.04)),
                        MeshMaterial3d(handle),
                        Transform::from_xyz(-0.24, 0.18, 0.335),
                    ));
                });
        }

        PropPrefab::TrafficCone => {
            let orange = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.98, 0.40, 0.08),
                    perceptual_roughness: 0.48,
                    ..default()
                },
            );
            let white = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.95, 0.96, 0.98),
                    emissive: LinearRgba::rgb(0.25, 0.25, 0.25),
                    ..default()
                },
            );
            let cone_mesh = cache.frustum_y(
                meshes,
                Vec2::new(0.32, 0.32),
                Vec2::new(0.08, 0.08),
                0.52,
            );
            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::small(
                        lbl("SAFETY CONE!"),
                        pts(170),
                        Color::srgb(0.98, 0.42, 0.10),
                        Color::srgb(0.95, 0.96, 0.98),
                        ImpactProfile::HeavyProp,
                        SfxMaterial::WoodCeramic,
                    ),
                    RigidBody::Dynamic,
                    Mass(1.5),
                    Restitution::new(0.66),
                    Friction::new(0.24),
                    Collider::cone(0.18, 0.56),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|cn| {
                    spawn_outlined_box(
                        cn,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.38, 0.04, 0.38),
                        orange.clone(),
                        Transform::from_xyz(0.0, -0.25, 0.0),
                        0.009,
                    );
                    cn.spawn((Mesh3d(cone_mesh.clone()), MeshMaterial3d(orange)));
                    cn.spawn((
                        Mesh3d(cone_mesh),
                        MeshMaterial3d(shared.ink_outline_mat.clone()),
                        Transform::from_scale(Vec3::splat(1.07)),
                    ));
                    cn.spawn((
                        Mesh3d(cache.frustum_y(
                            meshes,
                            Vec2::new(0.21, 0.21),
                            Vec2::new(0.16, 0.16),
                            0.10,
                        )),
                        MeshMaterial3d(white),
                        Transform::from_xyz(0.0, 0.06, 0.0),
                    ));
                });
        }

        PropPrefab::RedToolbox => {
            let red = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.82, 0.15, 0.15),
                    perceptual_roughness: 0.42,
                    ..default()
                },
            );
            let latch = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.80, 0.82, 0.86),
                    metallic: 0.65,
                    ..default()
                },
            );
            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::tall(
                        lbl("TOOLBOX SMASH!"),
                        pts(440),
                        Color::srgb(0.84, 0.16, 0.16),
                        Color::srgb(0.75, 0.78, 0.82),
                        ImpactProfile::HeavyProp,
                        SfxMaterial::Metal,
                    ),
                    RigidBody::Dynamic,
                    Mass(9.5),
                    Restitution::new(0.48),
                    Friction::new(0.28),
                    Collider::cuboid(0.68, 0.42, 0.36),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|tb| {
                    spawn_outlined_box(
                        tb,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.68, 0.42, 0.36),
                        red,
                        Transform::IDENTITY,
                        0.012,
                    );
                    for lx in [-0.18_f32, 0.18_f32] {
                        tb.spawn((
                            Mesh3d(cache.cuboid(meshes, 0.06, 0.07, 0.03)),
                            MeshMaterial3d(latch.clone()),
                            Transform::from_xyz(lx, 0.08, 0.185),
                        ));
                    }
                });
        }

        PropPrefab::PaintCanBlue | PropPrefab::PaintCanRed | PropPrefab::PaintCanYellow => {
            let paint_col = match p.prefab {
                PropPrefab::PaintCanBlue => Color::srgb(0.18, 0.58, 0.98),
                PropPrefab::PaintCanRed => Color::srgb(0.96, 0.24, 0.20),
                _ => Color::srgb(0.98, 0.80, 0.14),
            };
            let tin = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.74, 0.77, 0.81),
                    metallic: 0.55,
                    perceptual_roughness: 0.38,
                    ..default()
                },
            );
            let stripe = cache.material(
                materials,
                StandardMaterial {
                    base_color: paint_col,
                    perceptual_roughness: 0.32,
                    ..default()
                },
            );
            let mut prop = DestructibleProp::small(
                lbl("PAINT SPLATTER!"),
                pts(210),
                paint_col,
                Color::srgb(0.75, 0.78, 0.82),
                ImpactProfile::Liquid {
                    liquid_color: paint_col,
                    puddle_radius: 1.15,
                },
                SfxMaterial::Liquid,
            );
            prop.break_impulse = 1.8;
            let cyl = cache.cylinder(meshes, 0.135, 0.30);
            commands
                .spawn((
                    ScenarioEntity,
                    prop,
                    RigidBody::Dynamic,
                    Mass(2.2),
                    Restitution::new(0.55),
                    Friction::new(0.26),
                    Collider::cylinder(0.14, 0.30),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|pc| {
                    pc.spawn((Mesh3d(cyl.clone()), MeshMaterial3d(tin)));
                    pc.spawn((
                        Mesh3d(cyl),
                        MeshMaterial3d(shared.ink_outline_mat.clone()),
                        Transform::from_scale(Vec3::new(1.10, 1.05, 1.10)),
                    ));
                    pc.spawn((
                        Mesh3d(cache.cylinder(meshes, 0.139, 0.15)),
                        MeshMaterial3d(stripe),
                    ));
                });
        }

        PropPrefab::FireExtinguisher => {
            let red = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.90, 0.12, 0.12),
                    perceptual_roughness: 0.32,
                    ..default()
                },
            );
            let dark = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.14, 0.16, 0.20),
                    ..default()
                },
            );
            let mut prop = DestructibleProp::tall(
                lbl("EXTINGUISHER BLAST!"),
                pts(600),
                Color::srgb(0.95, 0.97, 1.0),
                Color::srgb(0.90, 0.14, 0.14),
                ImpactProfile::PressurizedHazard,
                SfxMaterial::PressurizedBlast,
            );
            prop.damage_impulse = 1.2;
            prop.break_impulse = 2.4;
            let tank = cache.cylinder(meshes, 0.11, 0.56);
            commands
                .spawn((
                    ScenarioEntity,
                    prop,
                    RigidBody::Dynamic,
                    Mass(4.5),
                    Restitution::new(0.55),
                    Friction::new(0.26),
                    Collider::cylinder(0.12, 0.68),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|fe| {
                    fe.spawn((Mesh3d(tank.clone()), MeshMaterial3d(red)));
                    fe.spawn((
                        Mesh3d(tank),
                        MeshMaterial3d(shared.ink_outline_mat.clone()),
                        Transform::from_scale(Vec3::new(1.12, 1.04, 1.12)),
                    ));
                    spawn_outlined_box(
                        fe,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.10, 0.10, 0.12),
                        dark,
                        Transform::from_xyz(0.0, 0.32, 0.02),
                        0.008,
                    );
                });
        }

        PropPrefab::StepLadder => {
            let alum = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.68, 0.72, 0.78),
                    metallic: 0.55,
                    perceptual_roughness: 0.42,
                    ..default()
                },
            );
            let cap = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.96, 0.74, 0.14),
                    ..default()
                },
            );
            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::tall(
                        lbl("STEP LADDER!"),
                        pts(350),
                        Color::srgb(0.72, 0.76, 0.82),
                        Color::srgb(0.96, 0.74, 0.14),
                        ImpactProfile::HeavyProp,
                        SfxMaterial::Metal,
                    ),
                    RigidBody::Dynamic,
                    Mass(6.8),
                    Restitution::new(0.52),
                    Friction::new(0.26),
                    Collider::cuboid(0.56, 1.36, 0.62),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|ld| {
                    for z_sign in [-1.0_f32, 1.0_f32] {
                        for lx in [-0.24_f32, 0.24_f32] {
                            spawn_outlined_box(
                                ld,
                                meshes,
                                cache,
                                shared,
                                Vec3::new(0.045, 1.38, 0.045),
                                alum.clone(),
                                Transform::from_xyz(lx, 0.0, z_sign * 0.18)
                                    .with_rotation(Quat::from_rotation_x(-z_sign * 0.22)),
                                0.008,
                            );
                        }
                    }
                    for ry in [-0.42_f32, -0.10, 0.22] {
                        ld.spawn((
                            Mesh3d(cache.cuboid(meshes, 0.48, 0.03, 0.08)),
                            MeshMaterial3d(alum.clone()),
                            Transform::from_xyz(0.0, ry, 0.16 - ry * 0.18),
                        ));
                    }
                    spawn_outlined_box(
                        ld,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.52, 0.06, 0.22),
                        cap,
                        Transform::from_xyz(0.0, 0.66, 0.0),
                        0.009,
                    );
                });
        }

        PropPrefab::LaptopSilver => {
            let silver = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.76, 0.79, 0.84),
                    metallic: 0.65,
                    perceptual_roughness: 0.32,
                    ..default()
                },
            );
            let screen = cache.material(
                materials,
                StandardMaterial {
                    base_color_texture: Some(textures.stock_crash_tv.clone()),
                    emissive: LinearRgba::rgb(0.45, 0.55, 0.75),
                    emissive_texture: Some(textures.stock_crash_tv.clone()),
                    ..default()
                },
            );
            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::small(
                        lbl("CEO LAPTOP!"),
                        pts(420),
                        Color::srgb(0.76, 0.79, 0.84),
                        Color::srgb(0.22, 0.88, 0.95),
                        ImpactProfile::Electronics,
                        SfxMaterial::Electronics,
                    ),
                    RigidBody::Dynamic,
                    Mass(1.6),
                    Restitution::new(0.50),
                    Friction::new(0.28),
                    Collider::cuboid(0.38, 0.24, 0.28),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|lp| {
                    spawn_outlined_box(
                        lp,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.36, 0.02, 0.25),
                        silver.clone(),
                        Transform::from_xyz(0.0, -0.09, 0.02),
                        0.008,
                    );
                    let lid_rot = Quat::from_rotation_x(-0.22);
                    spawn_outlined_box(
                        lp,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.36, 0.24, 0.018),
                        silver,
                        Transform::from_xyz(0.0, 0.03, -0.10).with_rotation(lid_rot),
                        0.008,
                    );
                    lp.spawn((
                        ScreenSurface,
                        Mesh3d(cache.upright_quad(meshes, 0.33, 0.20)),
                        MeshMaterial3d(screen),
                        Transform::from_xyz(0.0, 0.03, -0.088).with_rotation(lid_rot),
                    ));
                });
        }

        PropPrefab::CrystalDecanter | PropPrefab::CrystalGlass => {
            let is_dec = p.prefab == PropPrefab::CrystalDecanter;
            let crystal = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgba(0.84, 0.94, 1.0, 0.72),
                    emissive: LinearRgba::rgb(0.15, 0.22, 0.30),
                    alpha_mode: AlphaMode::Blend,
                    perceptual_roughness: 0.12,
                    ..default()
                },
            );
            let amber = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.76, 0.38, 0.08),
                    emissive: LinearRgba::rgb(0.35, 0.14, 0.02),
                    perceptual_roughness: 0.18,
                    ..default()
                },
            );
            let h = if is_dec { 0.24 } else { 0.11 };
            let w = if is_dec { 0.12 } else { 0.075 };
            let mut prop = DestructibleProp::small(
                lbl(if is_dec { "AGED SCOTCH!" } else { "CRYSTAL GLASS!" }),
                pts(if is_dec { 380 } else { 210 }),
                Color::srgb(0.86, 0.95, 1.0),
                Color::srgb(0.82, 0.44, 0.10),
                ImpactProfile::Liquid {
                    liquid_color: Color::srgb(0.78, 0.40, 0.09),
                    puddle_radius: if is_dec { 0.78 } else { 0.45 },
                },
                SfxMaterial::Glass,
            );
            prop.damage_impulse = 0.8;
            prop.break_impulse = 1.5;

            commands
                .spawn((
                    ScenarioEntity,
                    prop,
                    RigidBody::Dynamic,
                    Mass(if is_dec { 0.9 } else { 0.35 }),
                    Restitution::new(0.45),
                    Friction::new(0.28),
                    Collider::cuboid(w, h, w),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|cg| {
                    spawn_outlined_box(
                        cg,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(w, h, w),
                        crystal.clone(),
                        Transform::IDENTITY,
                        0.007,
                    );
                    cg.spawn((
                        Mesh3d(cache.cuboid(meshes, w * 0.76, h * 0.45, w * 0.76)),
                        MeshMaterial3d(amber),
                        Transform::from_xyz(0.0, -h * 0.2, 0.0),
                    ));
                    if is_dec {
                        spawn_outlined_box(
                            cg,
                            meshes,
                            cache,
                            shared,
                            Vec3::new(0.055, 0.06, 0.055),
                            crystal,
                            Transform::from_xyz(0.0, h * 0.62, 0.0),
                            0.006,
                        );
                    }
                });
        }

        PropPrefab::StockCrashTv => {
            let bezel = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.10, 0.12, 0.16),
                    perceptual_roughness: 0.42,
                    ..default()
                },
            );
            let screen = cache.material(
                materials,
                StandardMaterial {
                    base_color_texture: Some(textures.stock_crash_tv.clone()),
                    emissive: LinearRgba::rgb(0.95, 0.42, 0.42),
                    emissive_texture: Some(textures.stock_crash_tv.clone()),
                    ..default()
                },
            );
            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::tall(
                        lbl("Q4 EARNINGS TV!"),
                        pts(700),
                        Color::srgb(0.14, 0.16, 0.22),
                        Color::srgb(1.0, 0.28, 0.28),
                        ImpactProfile::Electronics,
                        SfxMaterial::Electronics,
                    ),
                    RigidBody::Dynamic,
                    Mass(8.5),
                    Restitution::new(0.48),
                    Friction::new(0.28),
                    Collider::cuboid(1.72, 1.02, 0.14),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|tv| {
                    spawn_outlined_box(
                        tv,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(1.72, 0.98, 0.08),
                        bezel.clone(),
                        Transform::IDENTITY,
                        0.014,
                    );
                    tv.spawn((
                        ScreenSurface,
                        Mesh3d(cache.upright_quad(meshes, 1.60, 0.88)),
                        MeshMaterial3d(screen),
                        Transform::from_xyz(0.0, 0.0, 0.046),
                    ));
                    spawn_outlined_box(
                        tv,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.54, 0.06, 0.30),
                        bezel,
                        Transform::from_xyz(0.0, -0.52, 0.02),
                        0.009,
                    );
                });
        }

        PropPrefab::GlassPartition => {
            let glass = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgba(0.62, 0.84, 0.96, 0.42),
                    emissive: LinearRgba::rgb(0.08, 0.16, 0.22),
                    alpha_mode: AlphaMode::Blend,
                    perceptual_roughness: 0.12,
                    ..default()
                },
            );
            let frame = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.14, 0.17, 0.22),
                    metallic: 0.5,
                    ..default()
                },
            );
            let mut prop = DestructibleProp::tall(
                lbl("GLASS WALL!"),
                pts(520),
                Color::srgb(0.72, 0.92, 1.0),
                Color::srgb(0.16, 0.20, 0.26),
                ImpactProfile::GlassCrystal,
                SfxMaterial::Glass,
            );
            prop.damage_impulse = 1.4;
            prop.break_impulse = 2.9;

            commands
                .spawn((
                    ScenarioEntity,
                    prop,
                    RigidBody::Dynamic,
                    Mass(10.0),
                    Restitution::new(0.46),
                    Friction::new(0.28),
                    Collider::cuboid(1.48, 1.82, 0.10),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|gp| {
                    gp.spawn((
                        GlassSurface {
                            size: Vec2::new(1.36, 1.68),
                            z_offset: 0.03,
                        },
                        Mesh3d(cache.cuboid(meshes, 1.38, 1.72, 0.03)),
                        MeshMaterial3d(glass),
                    ));
                    for y in [-0.88_f32, 0.88_f32] {
                        spawn_outlined_box(
                            gp,
                            meshes,
                            cache,
                            shared,
                            Vec3::new(1.48, 0.06, 0.09),
                            frame.clone(),
                            Transform::from_xyz(0.0, y, 0.0),
                            0.009,
                        );
                    }
                    for x in [-0.71_f32, 0.71_f32] {
                        gp.spawn((
                            Mesh3d(cache.cuboid(meshes, 0.06, 1.82, 0.09)),
                            MeshMaterial3d(frame.clone()),
                            Transform::from_xyz(x, 0.0, 0.0),
                        ));
                    }
                });
        }

        PropPrefab::EspressoMachine => {
            let chrome = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.82, 0.84, 0.88),
                    metallic: 0.75,
                    perceptual_roughness: 0.24,
                    ..default()
                },
            );
            let red_accent = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.76, 0.14, 0.14),
                    ..default()
                },
            );
            let mut prop = DestructibleProp::small(
                lbl("ESPRESSO BOILER!"),
                pts(650),
                Color::srgb(0.84, 0.86, 0.90),
                Color::srgb(0.26, 0.13, 0.06),
                ImpactProfile::PressurizedHazard,
                SfxMaterial::PressurizedBlast,
            );
            prop.damage_impulse = 1.2;
            prop.break_impulse = 2.4;

            commands
                .spawn((
                    ScenarioEntity,
                    prop,
                    VfxEmitter {
                        timer: 0.0,
                        interval: 0.28,
                        offset: Vec3::new(0.0, 0.22, 0.0),
                        style: EmitterStyle::CoffeeSteam,
                    },
                    RigidBody::Dynamic,
                    Mass(5.2),
                    Restitution::new(0.48),
                    Friction::new(0.30),
                    Collider::cuboid(0.46, 0.42, 0.38),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|em| {
                    spawn_outlined_box(
                        em,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.44, 0.40, 0.36),
                        chrome,
                        Transform::IDENTITY,
                        0.012,
                    );
                    spawn_outlined_box(
                        em,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.46, 0.06, 0.38),
                        red_accent,
                        Transform::from_xyz(0.0, 0.18, 0.0),
                        0.008,
                    );
                });
        }

        PropPrefab::ComputerMouseAndPad => {
            let pad_mat = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.12, 0.44, 0.52),
                    perceptual_roughness: 0.88,
                    ..default()
                },
            );
            let mouse_mat = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.84, 0.79, 0.67),
                    perceptual_roughness: 0.58,
                    ..default()
                },
            );
            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::small(
                        lbl("BALL MOUSE!"),
                        pts(120),
                        Color::srgb(0.84, 0.79, 0.67),
                        Color::srgb(0.12, 0.44, 0.52),
                        ImpactProfile::Electronics,
                        SfxMaterial::Electronics,
                    ),
                    RigidBody::Dynamic,
                    Mass(0.35),
                    Restitution::new(0.54),
                    Friction::new(0.28),
                    Collider::cuboid(0.20, 0.04, 0.22),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|m| {
                    m.spawn((
                        Mesh3d(cache.cuboid(meshes, 0.21, 0.008, 0.23)),
                        MeshMaterial3d(pad_mat),
                        Transform::from_xyz(0.0, -0.016, 0.0),
                    ));
                    spawn_outlined_box(
                        m,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.075, 0.034, 0.115),
                        mouse_mat,
                        Transform::from_xyz(0.0, 0.005, 0.01),
                        0.006,
                    );
                });
        }

        PropPrefab::DeskPhone => {
            let body = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.16, 0.18, 0.23),
                    perceptual_roughness: 0.62,
                    ..default()
                },
            );
            let keys = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.76, 0.78, 0.82),
                    ..default()
                },
            );
            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::small(
                        lbl("HOLD LINE 2!"),
                        pts(190),
                        Color::srgb(0.18, 0.20, 0.26),
                        Color::srgb(0.78, 0.80, 0.84),
                        ImpactProfile::Electronics,
                        SfxMaterial::Electronics,
                    ),
                    RigidBody::Dynamic,
                    Mass(0.85),
                    Restitution::new(0.52),
                    Friction::new(0.30),
                    Collider::cuboid(0.21, 0.09, 0.20),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|ph| {
                    spawn_outlined_box(
                        ph,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.20, 0.055, 0.19),
                        body.clone(),
                        Transform::IDENTITY.with_rotation(Quat::from_rotation_x(0.15)),
                        0.008,
                    );
                    spawn_outlined_box(
                        ph,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.055, 0.04, 0.21),
                        body,
                        Transform::from_xyz(-0.06, 0.04, 0.0),
                        0.007,
                    );
                    ph.spawn((
                        Mesh3d(cache.cuboid(meshes, 0.08, 0.015, 0.09)),
                        MeshMaterial3d(keys),
                        Transform::from_xyz(0.035, 0.032, 0.02)
                            .with_rotation(Quat::from_rotation_x(0.15)),
                    ));
                });
        }

        PropPrefab::DeskLamp => {
            let brass = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.76, 0.58, 0.22),
                    metallic: 0.7,
                    perceptual_roughness: 0.32,
                    ..default()
                },
            );
            let shade = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.16, 0.52, 0.30),
                    emissive: LinearRgba::rgb(0.18, 0.55, 0.24),
                    perceptual_roughness: 0.35,
                    ..default()
                },
            );
            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::small(
                        lbl("BANKER LAMP!"),
                        pts(230),
                        Color::srgb(0.18, 0.58, 0.34),
                        Color::srgb(0.78, 0.60, 0.24),
                        ImpactProfile::GlassCrystal,
                        SfxMaterial::Glass,
                    ),
                    RigidBody::Dynamic,
                    Mass(1.1),
                    Restitution::new(0.48),
                    Friction::new(0.30),
                    Collider::cuboid(0.22, 0.34, 0.18),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|lm| {
                    spawn_outlined_box(
                        lm,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.15, 0.03, 0.15),
                        brass.clone(),
                        Transform::from_xyz(0.0, -0.15, 0.0),
                        0.007,
                    );
                    lm.spawn((
                        Mesh3d(cache.cylinder(meshes, 0.016, 0.26)),
                        MeshMaterial3d(brass),
                        Transform::from_xyz(0.0, -0.01, -0.02),
                    ));
                    let shade_mesh = cache.frustum_y(
                        meshes,
                        Vec2::new(0.23, 0.15),
                        Vec2::new(0.16, 0.09),
                        0.09,
                    );
                    lm.spawn((
                        Mesh3d(shade_mesh.clone()),
                        MeshMaterial3d(shade),
                        Transform::from_xyz(0.0, 0.13, 0.02),
                    ));
                    lm.spawn((
                        Mesh3d(shade_mesh),
                        MeshMaterial3d(shared.ink_outline_mat.clone()),
                        Transform::from_xyz(0.0, 0.13, 0.02).with_scale(Vec3::splat(1.09)),
                    ));
                });
        }

        PropPrefab::PencilCup => {
            let cup_mat = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.22, 0.26, 0.34),
                    ..default()
                },
            );
            let yellow = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.96, 0.78, 0.18),
                    ..default()
                },
            );
            let blue = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.22, 0.56, 0.94),
                    ..default()
                },
            );
            let cup = cache.cylinder(meshes, 0.048, 0.11);
            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::small(
                        lbl("PENCIL CUP!"),
                        pts(140),
                        Color::srgb(0.96, 0.78, 0.18),
                        Color::srgb(0.22, 0.56, 0.94),
                        ImpactProfile::HeavyProp,
                        SfxMaterial::WoodCeramic,
                    ),
                    RigidBody::Dynamic,
                    Mass(0.45),
                    Restitution::new(0.55),
                    Friction::new(0.28),
                    Collider::cylinder(0.052, 0.16),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|pc| {
                    pc.spawn((
                        Mesh3d(cup.clone()),
                        MeshMaterial3d(cup_mat),
                        Transform::from_xyz(0.0, -0.025, 0.0),
                    ));
                    pc.spawn((
                        Mesh3d(cup),
                        MeshMaterial3d(shared.ink_outline_mat.clone()),
                        Transform::from_xyz(0.0, -0.025, 0.0).with_scale(Vec3::splat(1.12)),
                    ));
                    for (i, (px, pz, tilt)) in [
                        (-0.018, 0.01, 0.18_f32),
                        (0.018, -0.01, -0.20),
                        (0.0, 0.018, 0.12),
                        (-0.01, -0.018, -0.14),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        pc.spawn((
                            Mesh3d(cache.cylinder(meshes, 0.007, 0.14)),
                            MeshMaterial3d(if i % 2 == 0 {
                                yellow.clone()
                            } else {
                                blue.clone()
                            }),
                            Transform::from_xyz(px, 0.035, pz)
                                .with_rotation(Quat::from_rotation_z(tilt)),
                        ));
                    }
                });
        }

        PropPrefab::PictureFrameDesk => {
            let frame = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.48, 0.30, 0.16),
                    ..default()
                },
            );
            let pic = cache.material(
                materials,
                StandardMaterial {
                    base_color_texture: Some(textures.cat_poster.clone()),
                    ..default()
                },
            );
            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::small(
                        lbl("FAMILY PHOTO!"),
                        pts(170),
                        Color::srgb(0.78, 0.92, 1.0),
                        Color::srgb(0.48, 0.30, 0.16),
                        ImpactProfile::GlassCrystal,
                        SfxMaterial::Glass,
                    ),
                    RigidBody::Dynamic,
                    Mass(0.4),
                    Restitution::new(0.52),
                    Friction::new(0.28),
                    Collider::cuboid(0.15, 0.18, 0.06),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|pf| {
                    let r = Quat::from_rotation_x(-0.22);
                    spawn_outlined_box(
                        pf,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.15, 0.18, 0.022),
                        frame,
                        Transform::from_rotation(r),
                        0.007,
                    );
                    pf.spawn((
                        Mesh3d(cache.upright_quad(meshes, 0.12, 0.15)),
                        MeshMaterial3d(pic),
                        Transform::from_xyz(0.0, 0.0, 0.015).with_rotation(r),
                    ));
                });
        }

        PropPrefab::CardboardBox => {
            let kraft = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.68, 0.50, 0.31),
                    perceptual_roughness: 0.88,
                    ..default()
                },
            );
            let lid = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.88, 0.86, 0.80),
                    perceptual_roughness: 0.85,
                    ..default()
                },
            );
            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::small(
                        lbl("ARCHIVE BOX!"),
                        pts(210),
                        Color::srgb(0.68, 0.50, 0.31),
                        Color::srgb(0.92, 0.94, 0.96),
                        ImpactProfile::PaperStack { is_blueprint: false },
                        SfxMaterial::Paper,
                    ),
                    RigidBody::Dynamic,
                    Mass(2.6),
                    Restitution::new(0.44),
                    Friction::new(0.32),
                    Collider::cuboid(0.44, 0.34, 0.36),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|bx| {
                    spawn_outlined_box(
                        bx,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.42, 0.30, 0.34),
                        kraft,
                        Transform::from_xyz(0.0, -0.02, 0.0),
                        0.011,
                    );
                    spawn_outlined_box(
                        bx,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.45, 0.06, 0.37),
                        lid,
                        Transform::from_xyz(0.0, 0.14, 0.0),
                        0.009,
                    );
                });
        }

        PropPrefab::CoatRack => {
            let wood = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.42, 0.24, 0.13),
                    ..default()
                },
            );
            let coat = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.22, 0.30, 0.44),
                    perceptual_roughness: 0.88,
                    ..default()
                },
            );
            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::tall(
                        lbl("COAT RACK!"),
                        pts(280),
                        Color::srgb(0.44, 0.26, 0.14),
                        Color::srgb(0.22, 0.30, 0.44),
                        ImpactProfile::HeavyProp,
                        SfxMaterial::WoodCeramic,
                    ),
                    RigidBody::Dynamic,
                    Mass(4.5),
                    Restitution::new(0.50),
                    Friction::new(0.28),
                    Collider::cuboid(0.44, 1.72, 0.44),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|cr| {
                    for a_idx in 0..4 {
                        let a = (a_idx as f32 / 4.0) * std::f32::consts::TAU;
                        cr.spawn((
                            Mesh3d(cache.cuboid(meshes, 0.06, 0.05, 0.34)),
                            MeshMaterial3d(wood.clone()),
                            Transform::from_xyz(a.sin() * 0.14, -0.82, a.cos() * 0.14)
                                .with_rotation(Quat::from_rotation_y(a)),
                        ));
                    }
                    spawn_outlined_box(
                        cr,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.07, 1.68, 0.07),
                        wood,
                        Transform::IDENTITY,
                        0.010,
                    );
                    // Hanging trenchcoat / jacket on one side!
                    spawn_outlined_box(
                        cr,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.22, 0.62, 0.16),
                        coat,
                        Transform::from_xyz(0.12, 0.28, 0.04)
                            .with_rotation(Quat::from_rotation_z(-0.10)),
                        0.009,
                    );
                });
        }

        PropPrefab::CoffeeMakerPot => {
            let dark = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.14, 0.16, 0.20),
                    perceptual_roughness: 0.48,
                    ..default()
                },
            );
            let carafe = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.22, 0.10, 0.05),
                    perceptual_roughness: 0.16,
                    ..default()
                },
            );
            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::small(
                        lbl("COFFEE POT!"),
                        pts(320),
                        Color::srgb(0.16, 0.18, 0.22),
                        Color::srgb(0.24, 0.12, 0.05),
                        ImpactProfile::Liquid {
                            liquid_color: Color::srgb(0.24, 0.11, 0.05),
                            puddle_radius: 1.05,
                        },
                        SfxMaterial::Glass,
                    ),
                    VfxEmitter {
                        timer: 0.0,
                        interval: 0.36,
                        offset: Vec3::new(0.0, 0.20, 0.04),
                        style: EmitterStyle::CoffeeSteam,
                    },
                    RigidBody::Dynamic,
                    Mass(2.4),
                    Restitution::new(0.48),
                    Friction::new(0.30),
                    Collider::cuboid(0.26, 0.34, 0.28),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|cm| {
                    spawn_outlined_box(
                        cm,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.24, 0.32, 0.14),
                        dark.clone(),
                        Transform::from_xyz(0.0, 0.0, -0.06),
                        0.009,
                    );
                    spawn_outlined_box(
                        cm,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.24, 0.06, 0.26),
                        dark,
                        Transform::from_xyz(0.0, 0.13, 0.0),
                        0.008,
                    );
                    cm.spawn((
                        Mesh3d(cache.cylinder(meshes, 0.075, 0.16)),
                        MeshMaterial3d(carafe),
                        Transform::from_xyz(0.0, -0.04, 0.04),
                    ));
                });
        }

        PropPrefab::Microwave => {
            let white = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.86, 0.87, 0.89),
                    perceptual_roughness: 0.42,
                    ..default()
                },
            );
            let window = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.10, 0.12, 0.16),
                    perceptual_roughness: 0.2,
                    ..default()
                },
            );
            let timer = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.25, 0.98, 0.48),
                    emissive: LinearRgba::rgb(0.3, 2.0, 0.5),
                    ..default()
                },
            );
            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::small(
                        lbl("BREAKROOM MICROWAVE!"),
                        pts(360),
                        Color::srgb(0.86, 0.87, 0.89),
                        Color::srgb(0.25, 0.98, 0.48),
                        ImpactProfile::Electronics,
                        SfxMaterial::Electronics,
                    ),
                    RigidBody::Dynamic,
                    Mass(4.4),
                    Restitution::new(0.48),
                    Friction::new(0.30),
                    Collider::cuboid(0.48, 0.28, 0.34),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|mw| {
                    spawn_outlined_box(
                        mw,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.48, 0.28, 0.34),
                        white,
                        Transform::IDENTITY,
                        0.011,
                    );
                    mw.spawn((
                        Mesh3d(cache.cuboid(meshes, 0.30, 0.20, 0.02)),
                        MeshMaterial3d(window),
                        Transform::from_xyz(-0.06, 0.0, 0.172),
                    ));
                    mw.spawn((
                        Mesh3d(cache.cuboid(meshes, 0.08, 0.035, 0.02)),
                        MeshMaterial3d(timer),
                        Transform::from_xyz(0.16, 0.07, 0.172),
                    ));
                });
        }

        PropPrefab::PottedCactus => {
            let pot_mat = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.78, 0.40, 0.22),
                    ..default()
                },
            );
            let green = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.20, 0.66, 0.34),
                    ..default()
                },
            );
            let pot = cache.frustum_y(
                meshes,
                Vec2::new(0.10, 0.10),
                Vec2::new(0.15, 0.15),
                0.12,
            );
            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::small(
                        lbl("DESK CACTUS!"),
                        pts(190),
                        Color::srgb(0.22, 0.68, 0.36),
                        Color::srgb(0.78, 0.40, 0.22),
                        ImpactProfile::HeavyProp,
                        SfxMaterial::WoodCeramic,
                    ),
                    RigidBody::Dynamic,
                    Mass(0.75),
                    Restitution::new(0.50),
                    Friction::new(0.30),
                    Collider::cylinder(0.08, 0.26),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|cc| {
                    cc.spawn((
                        Mesh3d(pot.clone()),
                        MeshMaterial3d(pot_mat),
                        Transform::from_xyz(0.0, -0.06, 0.0),
                    ));
                    cc.spawn((
                        Mesh3d(pot),
                        MeshMaterial3d(shared.ink_outline_mat.clone()),
                        Transform::from_xyz(0.0, -0.06, 0.0).with_scale(Vec3::splat(1.10)),
                    ));
                    spawn_outlined_box(
                        cc,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.065, 0.18, 0.065),
                        green.clone(),
                        Transform::from_xyz(0.0, 0.06, 0.0),
                        0.006,
                    );
                    for (ax, ay) in [(-0.055, 0.05), (0.055, 0.08)] {
                        spawn_outlined_box(
                            cc,
                            meshes,
                            cache,
                            shared,
                            Vec3::new(0.05, 0.07, 0.045),
                            green.clone(),
                            Transform::from_xyz(ax, ay, 0.0),
                            0.005,
                        );
                    }
                });
        }

        PropPrefab::PottedSnakePlant => {
            let planter = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.86, 0.85, 0.80),
                    ..default()
                },
            );
            let blade = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.16, 0.54, 0.28),
                    ..default()
                },
            );
            let pot = cache.frustum_y(
                meshes,
                Vec2::new(0.28, 0.28),
                Vec2::new(0.38, 0.38),
                0.44,
            );
            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::tall(
                        lbl("SNAKE PLANT!"),
                        pts(270),
                        Color::srgb(0.18, 0.58, 0.30),
                        Color::srgb(0.86, 0.85, 0.80),
                        ImpactProfile::HeavyProp,
                        SfxMaterial::WoodCeramic,
                    ),
                    RigidBody::Dynamic,
                    Mass(4.6),
                    Restitution::new(0.48),
                    Friction::new(0.30),
                    Collider::cylinder(0.22, 1.05),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|sp| {
                    sp.spawn((
                        Mesh3d(pot.clone()),
                        MeshMaterial3d(planter),
                        Transform::from_xyz(0.0, -0.28, 0.0),
                    ));
                    sp.spawn((
                        Mesh3d(pot),
                        MeshMaterial3d(shared.ink_outline_mat.clone()),
                        Transform::from_xyz(0.0, -0.28, 0.0).with_scale(Vec3::splat(1.07)),
                    ));
                    for i in 0..5 {
                        let a = (i as f32 / 5.0) * std::f32::consts::TAU;
                        let h = 0.58 + (i % 2) as f32 * 0.14;
                        spawn_outlined_box(
                            sp,
                            meshes,
                            cache,
                            shared,
                            Vec3::new(0.09, h, 0.025),
                            blade.clone(),
                            Transform::from_xyz(a.cos() * 0.07, 0.16, a.sin() * 0.07)
                                .with_rotation(Quat::from_rotation_y(a) * Quat::from_rotation_z(0.08)),
                            0.006,
                        );
                    }
                });
        }

        PropPrefab::ClipboardDesk => {
            let board = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.58, 0.38, 0.20),
                    ..default()
                },
            );
            let page = cache.material(
                materials,
                StandardMaterial {
                    base_color_texture: Some(textures.printed_page.clone()),
                    ..default()
                },
            );
            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::small(
                        lbl("SAFETY CLIPBOARD!"),
                        pts(150),
                        Color::srgb(0.58, 0.38, 0.20),
                        Color::srgb(0.94, 0.96, 0.98),
                        ImpactProfile::PaperStack { is_blueprint: false },
                        SfxMaterial::Paper,
                    ),
                    RigidBody::Dynamic,
                    Mass(0.5),
                    Restitution::new(0.48),
                    Friction::new(0.30),
                    Collider::cuboid(0.24, 0.03, 0.32),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|cb| {
                    spawn_outlined_box(
                        cb,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.24, 0.018, 0.32),
                        board,
                        Transform::IDENTITY,
                        0.007,
                    );
                    cb.spawn((
                        Mesh3d(cache.floor_quad(meshes, 0.20, 0.26)),
                        MeshMaterial3d(page),
                        Transform::from_xyz(0.0, 0.012, 0.01),
                    ));
                });
        }

        PropPrefab::TapeMeasure => {
            let yellow = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.98, 0.78, 0.12),
                    perceptual_roughness: 0.38,
                    ..default()
                },
            );
            let grip = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.14, 0.16, 0.20),
                    ..default()
                },
            );
            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::small(
                        lbl("TAPE MEASURE!"),
                        pts(150),
                        Color::srgb(0.98, 0.78, 0.12),
                        Color::srgb(0.14, 0.16, 0.20),
                        ImpactProfile::HeavyProp,
                        SfxMaterial::Metal,
                    ),
                    RigidBody::Dynamic,
                    Mass(0.55),
                    Restitution::new(0.60),
                    Friction::new(0.28),
                    Collider::cuboid(0.09, 0.09, 0.05),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|tm| {
                    spawn_outlined_box(
                        tm,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.09, 0.085, 0.048),
                        yellow,
                        Transform::IDENTITY,
                        0.007,
                    );
                    tm.spawn((
                        Mesh3d(cache.cuboid(meshes, 0.05, 0.05, 0.052)),
                        MeshMaterial3d(grip),
                    ));
                });
        }

        PropPrefab::Sawhorse => {
            let hazard = cache.material(
                materials,
                StandardMaterial {
                    base_color_texture: Some(textures.hazard_stripe.clone()),
                    uv_transform: Affine2::from_scale(Vec2::new(6.0, 1.0)),
                    ..default()
                },
            );
            let lumber = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.72, 0.54, 0.32),
                    ..default()
                },
            );
            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::tall(
                        lbl("SAWHORSE BARRIER!"),
                        pts(340),
                        Color::srgb(0.72, 0.54, 0.32),
                        Color::srgb(0.96, 0.76, 0.16),
                        ImpactProfile::HeavyProp,
                        SfxMaterial::WoodCeramic,
                    ),
                    RigidBody::Dynamic,
                    Mass(6.2),
                    Restitution::new(0.48),
                    Friction::new(0.28),
                    Collider::cuboid(1.18, 0.82, 0.46),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|sh| {
                    spawn_outlined_box(
                        sh,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(1.18, 0.12, 0.12),
                        hazard,
                        Transform::from_xyz(0.0, 0.32, 0.0),
                        0.011,
                    );
                    for lx in [-0.46_f32, 0.46_f32] {
                        for z_sign in [-1.0_f32, 1.0_f32] {
                            spawn_outlined_box(
                                sh,
                                meshes,
                                cache,
                                shared,
                                Vec3::new(0.06, 0.78, 0.06),
                                lumber.clone(),
                                Transform::from_xyz(lx, -0.04, z_sign * 0.14)
                                    .with_rotation(Quat::from_rotation_x(-z_sign * 0.24)),
                                0.008,
                            );
                        }
                    }
                });
        }

        PropPrefab::CinderBlockStack => {
            let concrete = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.54, 0.56, 0.58),
                    perceptual_roughness: 0.94,
                    ..default()
                },
            );
            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::small(
                        lbl("CINDER BLOCKS!"),
                        pts(260),
                        Color::srgb(0.56, 0.58, 0.60),
                        Color::srgb(0.36, 0.38, 0.40),
                        ImpactProfile::HeavyProp,
                        SfxMaterial::WoodCeramic,
                    ),
                    RigidBody::Dynamic,
                    Mass(8.5),
                    Restitution::new(0.35),
                    Friction::new(0.42),
                    Collider::cuboid(0.44, 0.38, 0.26),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|cb| {
                    for (by, yaw) in [(-0.09_f32, 0.0_f32), (0.09, 0.14)] {
                        spawn_outlined_box(
                            cb,
                            meshes,
                            cache,
                            shared,
                            Vec3::new(0.40, 0.18, 0.20),
                            concrete.clone(),
                            Transform::from_xyz(0.0, by, 0.0)
                                .with_rotation(Quat::from_rotation_y(yaw)),
                            0.009,
                        );
                    }
                });
        }

        PropPrefab::SiteGenerator => {
            let yellow = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.96, 0.74, 0.12),
                    perceptual_roughness: 0.42,
                    ..default()
                },
            );
            let engine = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.16, 0.18, 0.22),
                    metallic: 0.55,
                    ..default()
                },
            );
            let mut prop = DestructibleProp::tall(
                lbl("DIESEL GENERATOR!"),
                pts(680),
                Color::srgb(0.96, 0.74, 0.12),
                Color::srgb(0.18, 0.20, 0.24),
                ImpactProfile::PressurizedHazard,
                SfxMaterial::PressurizedBlast,
            );
            prop.damage_impulse = 1.4;
            prop.break_impulse = 2.9;

            commands
                .spawn((
                    ScenarioEntity,
                    prop,
                    RigidBody::Dynamic,
                    Mass(13.0),
                    Restitution::new(0.46),
                    Friction::new(0.32),
                    Collider::cuboid(0.76, 0.62, 0.56),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|gn| {
                    spawn_outlined_box(
                        gn,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.74, 0.24, 0.52),
                        yellow,
                        Transform::from_xyz(0.0, 0.15, 0.0),
                        0.012,
                    );
                    spawn_outlined_box(
                        gn,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.68, 0.34, 0.46),
                        engine,
                        Transform::from_xyz(0.0, -0.12, 0.0),
                        0.011,
                    );
                });
        }

        PropPrefab::ShelvingRack => {
            let steel = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.28, 0.32, 0.38),
                    metallic: 0.5,
                    ..default()
                },
            );
            let crate_col = [
                Color::srgb(0.72, 0.52, 0.32),
                Color::srgb(0.86, 0.24, 0.20),
                Color::srgb(0.20, 0.58, 0.90),
            ];
            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::tall(
                        lbl("SUPPLY RACK!"),
                        pts(490),
                        Color::srgb(0.32, 0.36, 0.42),
                        Color::srgb(0.72, 0.52, 0.32),
                        ImpactProfile::HeavyProp,
                        SfxMaterial::Metal,
                    ),
                    RigidBody::Dynamic,
                    Mass(12.5),
                    Restitution::new(0.46),
                    Friction::new(0.30),
                    Collider::cuboid(1.18, 1.72, 0.48),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|sr| {
                    for sx in [-0.55_f32, 0.55_f32] {
                        for sz in [-0.20_f32, 0.20_f32] {
                            sr.spawn((
                                Mesh3d(cache.cuboid(meshes, 0.05, 1.72, 0.05)),
                                MeshMaterial3d(steel.clone()),
                                Transform::from_xyz(sx, 0.0, sz),
                            ));
                        }
                    }
                    for (idx, sy) in [-0.68_f32, -0.14, 0.40, 0.82].into_iter().enumerate() {
                        spawn_outlined_box(
                            sr,
                            meshes,
                            cache,
                            shared,
                            Vec3::new(1.16, 0.04, 0.46),
                            steel.clone(),
                            Transform::from_xyz(0.0, sy, 0.0),
                            0.009,
                        );
                        if idx < 3 {
                            let cm = cache.material(
                                materials,
                                StandardMaterial {
                                    base_color: crate_col[idx % crate_col.len()],
                                    ..default()
                                },
                            );
                            spawn_outlined_box(
                                sr,
                                meshes,
                                cache,
                                shared,
                                Vec3::new(0.36, 0.26, 0.32),
                                cm,
                                Transform::from_xyz(-0.24 + idx as f32 * 0.22, sy + 0.16, 0.0),
                                0.008,
                            );
                        }
                    }
                });
        }

        PropPrefab::BarCart => {
            let brass = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.82, 0.64, 0.26),
                    metallic: 0.75,
                    perceptual_roughness: 0.28,
                    ..default()
                },
            );
            let bottle = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgba(0.18, 0.56, 0.32, 0.82),
                    alpha_mode: AlphaMode::Blend,
                    ..default()
                },
            );
            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::tall(
                        lbl("VIP BAR CART!"),
                        pts(540),
                        Color::srgb(0.82, 0.64, 0.26),
                        Color::srgb(0.78, 0.40, 0.09),
                        ImpactProfile::Liquid {
                            liquid_color: Color::srgb(0.78, 0.40, 0.09),
                            puddle_radius: 1.15,
                        },
                        SfxMaterial::Glass,
                    ),
                    RigidBody::Dynamic,
                    Mass(7.5),
                    Restitution::new(0.48),
                    Friction::new(0.28),
                    Collider::cuboid(0.78, 0.86, 0.46),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|bc| {
                    for ty in [-0.28_f32, 0.20] {
                        spawn_outlined_box(
                            bc,
                            meshes,
                            cache,
                            shared,
                            Vec3::new(0.74, 0.04, 0.44),
                            brass.clone(),
                            Transform::from_xyz(0.0, ty, 0.0),
                            0.009,
                        );
                    }
                    let leg_mesh = cache.cuboid(meshes, 0.035, 0.82, 0.035);
                    for lx in [-0.34_f32, 0.34_f32] {
                        for lz in [-0.19_f32, 0.19_f32] {
                            bc.spawn((
                                Mesh3d(leg_mesh.clone()),
                                MeshMaterial3d(brass.clone()),
                                Transform::from_xyz(lx, -0.02, lz),
                            ));
                        }
                    }
                    let bottle_mesh = cache.cylinder(meshes, 0.045, 0.22);
                    for bx in [-0.18_f32, 0.02, 0.20] {
                        bc.spawn((
                            Mesh3d(bottle_mesh.clone()),
                            MeshMaterial3d(bottle.clone()),
                            Transform::from_xyz(bx, 0.33, 0.0),
                        ));
                    }
                });
        }

        PropPrefab::AwardTrophy => {
            let gold = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.98, 0.80, 0.22),
                    emissive: LinearRgba::rgb(0.35, 0.24, 0.04),
                    metallic: 0.85,
                    perceptual_roughness: 0.20,
                    ..default()
                },
            );
            let plinth = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.12, 0.13, 0.16),
                    ..default()
                },
            );
            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::small(
                        lbl("SYNERGY TROPHY!"),
                        pts(360),
                        Color::srgb(0.98, 0.80, 0.22),
                        Color::srgb(0.14, 0.15, 0.18),
                        ImpactProfile::HeavyProp,
                        SfxMaterial::Metal,
                    ),
                    RigidBody::Dynamic,
                    Mass(1.4),
                    Restitution::new(0.55),
                    Friction::new(0.28),
                    Collider::cuboid(0.16, 0.32, 0.16),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|tr| {
                    spawn_outlined_box(
                        tr,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.14, 0.08, 0.14),
                        plinth,
                        Transform::from_xyz(0.0, -0.11, 0.0),
                        0.007,
                    );
                    let cup = cache.frustum_y(
                        meshes,
                        Vec2::new(0.06, 0.06),
                        Vec2::new(0.15, 0.15),
                        0.18,
                    );
                    tr.spawn((
                        Mesh3d(cup.clone()),
                        MeshMaterial3d(gold),
                        Transform::from_xyz(0.0, 0.04, 0.0),
                    ));
                    tr.spawn((
                        Mesh3d(cup),
                        MeshMaterial3d(shared.ink_outline_mat.clone()),
                        Transform::from_xyz(0.0, 0.04, 0.0).with_scale(Vec3::splat(1.10)),
                    ));
                });
        }

        PropPrefab::CredenzaCabinet => {
            let wood = cache.material(
                materials,
                StandardMaterial {
                    base_color_texture: Some(textures.walnut_desk.clone()),
                    base_color: Color::srgb(0.86, 0.64, 0.56),
                    uv_transform: Affine2::from_scale(Vec2::new(2.5, 1.0)),
                    perceptual_roughness: 0.45,
                    ..default()
                },
            );
            let brass = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.80, 0.64, 0.26),
                    metallic: 0.7,
                    ..default()
                },
            );
            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::tall(
                        lbl("EXEC CREDENZA!"),
                        pts(460),
                        Color::srgb(0.48, 0.26, 0.16),
                        Color::srgb(0.80, 0.64, 0.26),
                        ImpactProfile::HeavyProp,
                        SfxMaterial::WoodCeramic,
                    ),
                    RigidBody::Dynamic,
                    Mass(16.0),
                    Restitution::new(0.42),
                    Friction::new(0.34),
                    Collider::cuboid(1.65, 0.76, 0.52),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|cd| {
                    spawn_outlined_box(
                        cd,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(1.65, 0.68, 0.52),
                        wood,
                        Transform::from_xyz(0.0, 0.03, 0.0),
                        0.014,
                    );
                    let handle_mesh = cache.cuboid(meshes, 0.10, 0.025, 0.025);
                    for hx in [-0.42_f32, 0.0, 0.42] {
                        cd.spawn((
                            Mesh3d(handle_mesh.clone()),
                            MeshMaterial3d(brass.clone()),
                            Transform::from_xyz(hx, 0.12, 0.27),
                        ));
                    }
                });
        }

        PropPrefab::EaselChart => {
            let wood = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.56, 0.38, 0.20),
                    ..default()
                },
            );
            let chart = cache.material(
                materials,
                StandardMaterial {
                    base_color_texture: Some(textures.whiteboard_doodle.clone()),
                    ..default()
                },
            );
            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::tall(
                        lbl("KPI FLIPCHART!"),
                        pts(340),
                        Color::srgb(0.94, 0.96, 0.98),
                        Color::srgb(0.56, 0.38, 0.20),
                        ImpactProfile::PaperStack { is_blueprint: false },
                        SfxMaterial::Paper,
                    ),
                    RigidBody::Dynamic,
                    Mass(5.0),
                    Restitution::new(0.48),
                    Friction::new(0.28),
                    Collider::cuboid(0.76, 1.56, 0.48),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|ez| {
                    spawn_outlined_box(
                        ez,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.74, 0.92, 0.05),
                        wood.clone(),
                        Transform::from_xyz(0.0, 0.26, 0.0)
                            .with_rotation(Quat::from_rotation_x(-0.10)),
                        0.011,
                    );
                    ez.spawn((
                        Mesh3d(cache.upright_quad(meshes, 0.68, 0.84)),
                        MeshMaterial3d(chart),
                        Transform::from_xyz(0.0, 0.26, 0.032)
                            .with_rotation(Quat::from_rotation_x(-0.10)),
                    ));
                    let leg_mesh = cache.cuboid(meshes, 0.045, 1.48, 0.045);
                    for lx in [-0.28_f32, 0.28_f32] {
                        ez.spawn((
                            Mesh3d(leg_mesh.clone()),
                            MeshMaterial3d(wood.clone()),
                            Transform::from_xyz(lx, -0.04, 0.06)
                                .with_rotation(Quat::from_rotation_x(-0.12)),
                        ));
                    }
                    ez.spawn((
                        Mesh3d(leg_mesh),
                        MeshMaterial3d(wood),
                        Transform::from_xyz(0.0, -0.04, -0.22)
                            .with_rotation(Quat::from_rotation_x(0.24)),
                    ));
                });
        }

        PropPrefab::PizzaBoxStack => {
            let box_mat = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.90, 0.86, 0.76),
                    perceptual_roughness: 0.85,
                    ..default()
                },
            );
            let red_logo = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.86, 0.20, 0.18),
                    ..default()
                },
            );
            commands
                .spawn((
                    ScenarioEntity,
                    DestructibleProp::small(
                        lbl("LAYOFF PIZZA PARTY!"),
                        pts(320),
                        Color::srgb(0.90, 0.86, 0.76),
                        Color::srgb(0.86, 0.22, 0.18),
                        ImpactProfile::Liquid {
                            liquid_color: Color::srgb(0.86, 0.22, 0.14),
                            puddle_radius: 0.72,
                        },
                        SfxMaterial::Paper,
                    ),
                    RigidBody::Dynamic,
                    Mass(1.4),
                    Restitution::new(0.45),
                    Friction::new(0.32),
                    Collider::cuboid(0.42, 0.18, 0.42),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|pz| {
                    for i in 0..3 {
                        let y = -0.055 + i as f32 * 0.055;
                        let yaw = (i as f32 - 1.0) * 0.14;
                        spawn_outlined_box(
                            pz,
                            meshes,
                            cache,
                            shared,
                            Vec3::new(0.40, 0.045, 0.40),
                            box_mat.clone(),
                            Transform::from_xyz(0.0, y, 0.0)
                                .with_rotation(Quat::from_rotation_y(yaw)),
                            0.008,
                        );
                    }
                    pz.spawn((
                        Mesh3d(cache.cuboid(meshes, 0.18, 0.01, 0.18)),
                        MeshMaterial3d(red_logo),
                        Transform::from_xyz(0.0, 0.082, 0.0),
                    ));
                });
        }

        PropPrefab::OfficeShredder => {
            let body = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.18, 0.20, 0.25),
                    perceptual_roughness: 0.58,
                    ..default()
                },
            );
            let slot = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.72, 0.76, 0.82),
                    metallic: 0.6,
                    ..default()
                },
            );
            let mut prop = DestructibleProp::tall(
                lbl("SEVERANCE SHREDDER!"),
                pts(580),
                Color::srgb(0.18, 0.20, 0.25),
                Color::srgb(0.94, 0.96, 0.98),
                ImpactProfile::PaperStack { is_blueprint: false },
                SfxMaterial::Electronics,
            );
            prop.damage_impulse = 1.3;
            prop.break_impulse = 2.6;
            commands
                .spawn((
                    ScenarioEntity,
                    prop,
                    RigidBody::Dynamic,
                    Mass(6.0),
                    Restitution::new(0.48),
                    Friction::new(0.30),
                    Collider::cuboid(0.42, 0.64, 0.32),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|sh| {
                    spawn_outlined_box(
                        sh,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.42, 0.64, 0.32),
                        body,
                        Transform::IDENTITY,
                        0.011,
                    );
                    sh.spawn((
                        Mesh3d(cache.cuboid(meshes, 0.32, 0.02, 0.08)),
                        MeshMaterial3d(slot),
                        Transform::from_xyz(0.0, 0.325, 0.0),
                    ));
                });
        }

        // ── Scene 4: All-Hands Town Hall Stage Prefabs ───────────────────────
        PropPrefab::LedVideoWall => {
            let frame = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.12, 0.12, 0.18),
                    metallic: 0.55,
                    perceptual_roughness: 0.42,
                    ..default()
                },
            );
            let screen = cache.material(
                materials,
                StandardMaterial {
                    base_color_texture: Some(textures.townhall_slide.clone()),
                    emissive: LinearRgba::rgb(0.85, 0.50, 1.15),
                    emissive_texture: Some(textures.townhall_slide.clone()),
                    perceptual_roughness: 0.18,
                    ..default()
                },
            );
            let mut prop = DestructibleProp::tall(
                lbl("15% RIF VIDEO WALL!"),
                pts(780),
                Color::srgb(0.16, 0.16, 0.24),
                Color::srgb(0.95, 0.28, 0.78),
                ImpactProfile::Electronics,
                SfxMaterial::Electronics,
            );
            prop.damage_impulse = 1.6;
            prop.break_impulse = 3.2;
            prop.debris_count = 14;
            commands
                .spawn((
                    ScenarioEntity,
                    prop,
                    RigidBody::Dynamic,
                    Mass(12.0),
                    Restitution::new(0.44),
                    Friction::new(0.32),
                    Collider::cuboid(1.90, 1.36, 0.32),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|vw| {
                    spawn_outlined_box(
                        vw,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(1.88, 1.06, 0.10),
                        frame.clone(),
                        Transform::from_xyz(0.0, 0.14, 0.0),
                        0.014,
                    );
                    vw.spawn((
                        ScreenSurface,
                        Mesh3d(cache.upright_quad(meshes, 1.76, 0.94)),
                        MeshMaterial3d(screen),
                        Transform::from_xyz(0.0, 0.14, 0.056),
                    ));
                    let leg_mesh = cache.cuboid(meshes, 0.08, 0.38, 0.32);
                    for lx in [-0.68_f32, 0.68_f32] {
                        vw.spawn((
                            Mesh3d(leg_mesh.clone()),
                            MeshMaterial3d(frame.clone()),
                            Transform::from_xyz(lx, -0.49, 0.0),
                        ));
                    }
                });
        }

        PropPrefab::TeleprompterRig => {
            let stand = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.16, 0.18, 0.22),
                    metallic: 0.6,
                    ..default()
                },
            );
            let glass = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgba(0.62, 0.92, 0.98, 0.58),
                    emissive: LinearRgba::rgb(0.18, 0.52, 0.64),
                    alpha_mode: AlphaMode::Blend,
                    perceptual_roughness: 0.10,
                    ..default()
                },
            );
            let mut prop = DestructibleProp::tall(
                lbl("FAKE EMPATHY RIG!"),
                pts(440),
                Color::srgb(0.76, 0.93, 1.0),
                Color::srgb(0.18, 0.20, 0.26),
                ImpactProfile::GlassCrystal,
                SfxMaterial::Glass,
            );
            prop.damage_impulse = 1.0;
            prop.break_impulse = 2.1;
            commands
                .spawn((
                    ScenarioEntity,
                    prop,
                    RigidBody::Dynamic,
                    Mass(3.2),
                    Restitution::new(0.46),
                    Friction::new(0.30),
                    Collider::cuboid(0.46, 1.32, 0.42),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|tp| {
                    spawn_outlined_box(
                        tp,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.42, 0.05, 0.38),
                        stand.clone(),
                        Transform::from_xyz(0.0, -0.63, 0.0),
                        0.009,
                    );
                    tp.spawn((
                        Mesh3d(cache.cuboid(meshes, 0.05, 1.08, 0.05)),
                        MeshMaterial3d(stand),
                        Transform::from_xyz(0.0, -0.08, 0.0),
                    ));
                    tp.spawn((
                        GlassSurface {
                            size: Vec2::new(0.40, 0.34),
                            z_offset: 0.02,
                        },
                        Mesh3d(cache.cuboid(meshes, 0.42, 0.36, 0.025)),
                        MeshMaterial3d(glass),
                        Transform::from_xyz(0.0, 0.48, 0.04)
                            .with_rotation(Quat::from_rotation_x(-0.62)),
                    ));
                });
        }

        PropPrefab::ConfettiCannon => {
            let gold = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.96, 0.76, 0.18),
                    metallic: 0.65,
                    perceptual_roughness: 0.32,
                    ..default()
                },
            );
            let mag = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.92, 0.22, 0.68),
                    emissive: LinearRgba::rgb(0.65, 0.08, 0.45),
                    ..default()
                },
            );
            let mut prop = DestructibleProp::tall(
                lbl("BUYBACK CONFETTI!"),
                pts(640),
                Color::srgb(0.96, 0.76, 0.18),
                Color::srgb(0.92, 0.22, 0.68),
                ImpactProfile::PressurizedHazard,
                SfxMaterial::PressurizedBlast,
            );
            prop.damage_impulse = 1.1;
            prop.break_impulse = 2.2;
            commands
                .spawn((
                    ScenarioEntity,
                    prop,
                    VfxEmitter {
                        timer: 0.0,
                        interval: 0.34,
                        offset: Vec3::new(0.0, 0.42, 0.0),
                        style: EmitterStyle::PressurizedHiss,
                    },
                    RigidBody::Dynamic,
                    Mass(4.5),
                    Restitution::new(0.50),
                    Friction::new(0.30),
                    Collider::cuboid(0.38, 0.82, 0.38),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|cc| {
                    spawn_outlined_box(
                        cc,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.36, 0.10, 0.36),
                        mag.clone(),
                        Transform::from_xyz(0.0, -0.36, 0.0),
                        0.009,
                    );
                    let barrel =
                        cache.frustum_y(meshes, Vec2::splat(0.22), Vec2::splat(0.32), 0.68);
                    cc.spawn((
                        Mesh3d(barrel.clone()),
                        MeshMaterial3d(gold),
                        Transform::from_xyz(0.0, 0.04, 0.0)
                            .with_rotation(Quat::from_rotation_x(0.18)),
                    ));
                    cc.spawn((
                        NotShadowCaster,
                        Mesh3d(barrel),
                        MeshMaterial3d(shared.ink_outline_mat.clone()),
                        Transform::from_xyz(0.0, 0.04, 0.0)
                            .with_rotation(Quat::from_rotation_x(0.18))
                            .with_scale(Vec3::splat(1.07)),
                    ));
                });
        }

        PropPrefab::PaSpeakerStack => {
            let cab = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.13, 0.14, 0.18),
                    perceptual_roughness: 0.78,
                    ..default()
                },
            );
            let cone = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.28, 0.30, 0.38),
                    ..default()
                },
            );
            let mut prop = DestructibleProp::tall(
                lbl("TOWN HALL PA STACK!"),
                pts(520),
                Color::srgb(0.15, 0.16, 0.22),
                Color::srgb(0.45, 0.92, 0.98),
                ImpactProfile::Electronics,
                SfxMaterial::Electronics,
            );
            prop.damage_impulse = 1.6;
            prop.break_impulse = 3.2;
            commands
                .spawn((
                    ScenarioEntity,
                    prop,
                    RigidBody::Dynamic,
                    Mass(9.5),
                    Restitution::new(0.44),
                    Friction::new(0.34),
                    Collider::cuboid(0.58, 1.38, 0.50),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|pa| {
                    spawn_outlined_box(
                        pa,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.56, 0.74, 0.48),
                        cab.clone(),
                        Transform::from_xyz(0.0, -0.32, 0.0),
                        0.012,
                    );
                    spawn_outlined_box(
                        pa,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.48, 0.58, 0.42),
                        cab,
                        Transform::from_xyz(0.0, 0.36, 0.0),
                        0.011,
                    );
                    let cone_mesh = cache.cuboid(meshes, 0.28, 0.22, 0.03);
                    for wy in [-0.32_f32, 0.26_f32, 0.50_f32] {
                        pa.spawn((
                            Mesh3d(cone_mesh.clone()),
                            MeshMaterial3d(cone.clone()),
                            Transform::from_xyz(0.0, wy, 0.24),
                        ));
                    }
                });
        }

        PropPrefab::StageMicStand => {
            let chrome = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.76, 0.80, 0.86),
                    metallic: 0.75,
                    perceptual_roughness: 0.28,
                    ..default()
                },
            );
            let black = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.14, 0.15, 0.19),
                    ..default()
                },
            );
            let mut prop = DestructibleProp::tall(
                lbl("MUTED Q&A MIC!"),
                pts(240),
                Color::srgb(0.76, 0.80, 0.86),
                Color::srgb(0.16, 0.18, 0.22),
                ImpactProfile::Electronics,
                SfxMaterial::Metal,
            );
            prop.damage_impulse = 0.8;
            prop.break_impulse = 1.7;
            commands
                .spawn((
                    ScenarioEntity,
                    prop,
                    RigidBody::Dynamic,
                    Mass(1.8),
                    Restitution::new(0.52),
                    Friction::new(0.28),
                    Collider::cuboid(0.34, 1.34, 0.34),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|ms| {
                    spawn_outlined_box(
                        ms,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.32, 0.05, 0.32),
                        black.clone(),
                        Transform::from_xyz(0.0, -0.64, 0.0),
                        0.008,
                    );
                    ms.spawn((
                        Mesh3d(cache.cuboid(meshes, 0.035, 1.22, 0.035)),
                        MeshMaterial3d(chrome.clone()),
                        Transform::from_xyz(0.0, -0.02, 0.0),
                    ));
                    ms.spawn((
                        Mesh3d(cache.cuboid(meshes, 0.03, 0.03, 0.44)),
                        MeshMaterial3d(chrome),
                        Transform::from_xyz(0.0, 0.56, 0.12)
                            .with_rotation(Quat::from_rotation_x(-0.35)),
                    ));
                    spawn_outlined_box(
                        ms,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.07, 0.07, 0.14),
                        black,
                        Transform::from_xyz(0.0, 0.64, 0.32),
                        0.007,
                    );
                });
        }

        PropPrefab::RingLightRig => {
            let halo = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(1.0, 0.96, 0.90),
                    emissive: LinearRgba::rgb(2.2, 1.9, 1.6),
                    unlit: true,
                    ..default()
                },
            );
            let stand = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.16, 0.18, 0.22),
                    ..default()
                },
            );
            let mut prop = DestructibleProp::tall(
                lbl("CEO APOLOGY HALO!"),
                pts(380),
                Color::srgb(0.95, 0.96, 0.98),
                Color::srgb(0.18, 0.20, 0.24),
                ImpactProfile::GlassCrystal,
                SfxMaterial::Glass,
            );
            prop.damage_impulse = 0.95;
            prop.break_impulse = 1.95;
            commands
                .spawn((
                    ScenarioEntity,
                    prop,
                    RigidBody::Dynamic,
                    Mass(2.2),
                    Restitution::new(0.48),
                    Friction::new(0.28),
                    Collider::cuboid(0.48, 1.48, 0.42),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|rl| {
                    rl.spawn((
                        Mesh3d(cache.cuboid(meshes, 0.38, 0.05, 0.38)),
                        MeshMaterial3d(stand.clone()),
                        Transform::from_xyz(0.0, -0.71, 0.0),
                    ));
                    rl.spawn((
                        Mesh3d(cache.cuboid(meshes, 0.04, 1.12, 0.04)),
                        MeshMaterial3d(stand),
                        Transform::from_xyz(0.0, -0.14, 0.0),
                    ));
                    // 4-segment pixel-art glowing ring halo
                    for (ox, oy, sx, sy) in [
                        (0.0, 0.68, 0.42, 0.06),
                        (0.0, 0.32, 0.42, 0.06),
                        (-0.18, 0.50, 0.06, 0.32),
                        (0.18, 0.50, 0.06, 0.32),
                    ] {
                        rl.spawn((
                            Mesh3d(cache.cuboid(meshes, sx, sy, 0.05)),
                            MeshMaterial3d(halo.clone()),
                            Transform::from_xyz(ox, oy, 0.0),
                        ));
                    }
                });
        }

        PropPrefab::FoldingChair => {
            let steel = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.46, 0.50, 0.58),
                    metallic: 0.55,
                    perceptual_roughness: 0.45,
                    ..default()
                },
            );
            let mut prop = DestructibleProp::tall(
                lbl("AUDIENCE SEAT!"),
                pts(190),
                Color::srgb(0.46, 0.50, 0.58),
                Color::srgb(0.24, 0.26, 0.32),
                ImpactProfile::HeavyProp,
                SfxMaterial::Metal,
            );
            prop.damage_impulse = 0.9;
            prop.break_impulse = 2.0;
            prop.debris_count = 6;
            commands
                .spawn((
                    ScenarioEntity,
                    prop,
                    RigidBody::Dynamic,
                    Mass(2.1),
                    Restitution::new(0.52),
                    Friction::new(0.26),
                    Collider::cuboid(0.46, 0.84, 0.46),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|fc| {
                    spawn_outlined_box(
                        fc,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.42, 0.04, 0.40),
                        steel.clone(),
                        Transform::from_xyz(0.0, -0.04, 0.02),
                        0.008,
                    );
                    spawn_outlined_box(
                        fc,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.42, 0.24, 0.04),
                        steel.clone(),
                        Transform::from_xyz(0.0, 0.28, -0.18),
                        0.008,
                    );
                    let back_leg_mesh = cache.cuboid(meshes, 0.035, 0.82, 0.035);
                    let front_leg_mesh = cache.cuboid(meshes, 0.035, 0.42, 0.035);
                    for lx in [-0.19_f32, 0.19_f32] {
                        fc.spawn((
                            Mesh3d(back_leg_mesh.clone()),
                            MeshMaterial3d(steel.clone()),
                            Transform::from_xyz(lx, 0.0, -0.18),
                        ));
                        fc.spawn((
                            Mesh3d(front_leg_mesh.clone()),
                            MeshMaterial3d(steel.clone()),
                            Transform::from_xyz(lx, -0.21, 0.18),
                        ));
                    }
                });
        }

        PropPrefab::BalloonColumn => {
            let purp = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.68, 0.24, 0.88),
                    perceptual_roughness: 0.22,
                    ..default()
                },
            );
            let gold = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.98, 0.78, 0.20),
                    metallic: 0.45,
                    perceptual_roughness: 0.24,
                    ..default()
                },
            );
            let mut prop = DestructibleProp::tall(
                lbl("LAYOFF BALLOON ARCH"),
                pts(340),
                Color::srgb(0.68, 0.24, 0.88),
                Color::srgb(0.98, 0.78, 0.20),
                ImpactProfile::PressurizedHazard,
                SfxMaterial::PressurizedBlast,
            );
            prop.damage_impulse = 0.75;
            prop.break_impulse = 1.45;
            commands
                .spawn((
                    ScenarioEntity,
                    prop,
                    RigidBody::Dynamic,
                    Mass(1.4),
                    Restitution::new(0.64),
                    Friction::new(0.24),
                    Collider::cuboid(0.48, 1.66, 0.48),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|bc| {
                    for i in 0..6 {
                        let y = -0.62 + i as f32 * 0.25;
                        let angle = i as f32 * 1.1;
                        let ox = angle.cos() * 0.09;
                        let oz = angle.sin() * 0.09;
                        let m = if i % 2 == 0 {
                            purp.clone()
                        } else {
                            gold.clone()
                        };
                        spawn_outlined_box(
                            bc,
                            meshes,
                            cache,
                            shared,
                            Vec3::new(0.28, 0.26, 0.28),
                            m,
                            Transform::from_xyz(ox, y, oz),
                            0.009,
                        );
                    }
                });
        }

        // ── Scene 5: Sad Corporate Breakroom Prefabs ─────────────────────────
        PropPrefab::VendingMachine => {
            let body = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.14, 0.18, 0.24),
                    perceptual_roughness: 0.55,
                    ..default()
                },
            );
            let glass = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgba(0.75, 0.94, 0.98, 0.48),
                    emissive: LinearRgba::rgb(0.35, 0.55, 0.58),
                    alpha_mode: AlphaMode::Blend,
                    ..default()
                },
            );
            let snack_a = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.94, 0.30, 0.22),
                    emissive: LinearRgba::rgb(0.35, 0.08, 0.05),
                    ..default()
                },
            );
            let snack_b = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.98, 0.78, 0.18),
                    emissive: LinearRgba::rgb(0.38, 0.28, 0.04),
                    ..default()
                },
            );
            let mut prop = DestructibleProp::tall(
                lbl("$7 VENDING RIG!"),
                pts(740),
                Color::srgb(0.76, 0.93, 1.0),
                Color::srgb(0.96, 0.76, 0.16),
                ImpactProfile::GlassCrystal,
                SfxMaterial::Glass,
            );
            prop.damage_impulse = 1.7;
            prop.break_impulse = 3.4;
            prop.debris_count = 14;
            commands
                .spawn((
                    ScenarioEntity,
                    prop,
                    RigidBody::Dynamic,
                    Mass(18.0),
                    Restitution::new(0.40),
                    Friction::new(0.36),
                    Collider::cuboid(0.92, 1.78, 0.72),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|vm| {
                    spawn_outlined_box(
                        vm,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.92, 1.78, 0.72),
                        body,
                        Transform::IDENTITY,
                        0.014,
                    );
                    let snack_mesh = cache.cuboid(meshes, 0.11, 0.16, 0.06);
                    for row in 0..3 {
                        for col in 0..3 {
                            let sx = -0.22 + col as f32 * 0.16;
                            let sy = 0.05 + row as f32 * 0.28;
                            let m = if (row + col) % 2 == 0 {
                                snack_a.clone()
                            } else {
                                snack_b.clone()
                            };
                            vm.spawn((
                                Mesh3d(snack_mesh.clone()),
                                MeshMaterial3d(m),
                                Transform::from_xyz(sx, sy, 0.34),
                            ));
                        }
                    }
                    vm.spawn((
                        GlassSurface {
                            size: Vec2::new(0.56, 0.96),
                            z_offset: 0.02,
                        },
                        Mesh3d(cache.cuboid(meshes, 0.58, 0.98, 0.02)),
                        MeshMaterial3d(glass),
                        Transform::from_xyz(-0.08, 0.28, 0.37),
                    ));
                });
        }

        PropPrefab::OfficeFridgeTall => {
            let enamel = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.88, 0.90, 0.84),
                    perceptual_roughness: 0.36,
                    ..default()
                },
            );
            let note = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.98, 0.88, 0.32),
                    ..default()
                },
            );
            let note_pink = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.98, 0.46, 0.56),
                    ..default()
                },
            );
            let mut prop = DestructibleProp::tall(
                lbl("5PM FRIDGE PURGE!"),
                pts(660),
                Color::srgb(0.88, 0.90, 0.84),
                Color::srgb(0.22, 0.68, 0.96),
                ImpactProfile::Liquid {
                    liquid_color: Color::srgb(0.22, 0.72, 0.96),
                    puddle_radius: 1.05,
                },
                SfxMaterial::Metal,
            );
            prop.damage_impulse = 1.8;
            prop.break_impulse = 3.6;
            prop.debris_count = 13;
            commands
                .spawn((
                    ScenarioEntity,
                    prop,
                    RigidBody::Dynamic,
                    Mass(17.0),
                    Restitution::new(0.42),
                    Friction::new(0.36),
                    Collider::cuboid(0.84, 1.72, 0.74),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|fr| {
                    spawn_outlined_box(
                        fr,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.84, 1.12, 0.74),
                        enamel.clone(),
                        Transform::from_xyz(0.0, -0.30, 0.0),
                        0.013,
                    );
                    spawn_outlined_box(
                        fr,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.84, 0.56, 0.74),
                        enamel,
                        Transform::from_xyz(0.0, 0.58, 0.0),
                        0.013,
                    );
                    // Passive-aggressive sticky notes plastered all over the door!
                    let note_mesh = cache.cuboid(meshes, 0.14, 0.14, 0.015);
                    for (nx, ny, pm) in [
                        (-0.16, 0.12, note.clone()),
                        (0.14, 0.04, note_pink.clone()),
                        (-0.06, 0.55, note),
                        (0.18, -0.18, note_pink),
                    ] {
                        fr.spawn((
                            Mesh3d(note_mesh.clone()),
                            MeshMaterial3d(pm),
                            Transform::from_xyz(nx, ny, 0.38),
                        ));
                    }
                });
        }

        PropPrefab::SheetCakeTable => {
            let frosting = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.98, 0.96, 0.94),
                    perceptual_roughness: 0.52,
                    ..default()
                },
            );
            let icing_trim = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.96, 0.34, 0.56),
                    ..default()
                },
            );
            let tray = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.76, 0.80, 0.86),
                    metallic: 0.65,
                    ..default()
                },
            );
            let mut prop = DestructibleProp::small(
                lbl("0% COLA SHEET CAKE!"),
                pts(460),
                Color::srgb(0.98, 0.96, 0.94),
                Color::srgb(0.96, 0.28, 0.42),
                ImpactProfile::Liquid {
                    liquid_color: Color::srgb(0.96, 0.24, 0.20),
                    puddle_radius: 0.88,
                },
                SfxMaterial::Liquid,
            );
            prop.damage_impulse = 0.75;
            prop.break_impulse = 1.5;
            commands
                .spawn((
                    ScenarioEntity,
                    prop,
                    RigidBody::Dynamic,
                    Mass(2.0),
                    Restitution::new(0.38),
                    Friction::new(0.34),
                    Collider::cuboid(0.62, 0.16, 0.42),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|ck| {
                    spawn_outlined_box(
                        ck,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.62, 0.03, 0.42),
                        tray,
                        Transform::from_xyz(0.0, -0.06, 0.0),
                        0.008,
                    );
                    spawn_outlined_box(
                        ck,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.54, 0.11, 0.34),
                        frosting,
                        Transform::from_xyz(0.0, 0.01, 0.0),
                        0.009,
                    );
                    ck.spawn((
                        Mesh3d(cache.cuboid(meshes, 0.46, 0.025, 0.26)),
                        MeshMaterial3d(icing_trim),
                        Transform::from_xyz(0.0, 0.072, 0.0),
                    ));
                });
        }

        PropPrefab::CoffeeUrn => {
            let steel = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.76, 0.80, 0.86),
                    metallic: 0.75,
                    perceptual_roughness: 0.28,
                    ..default()
                },
            );
            let black = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.14, 0.15, 0.18),
                    ..default()
                },
            );
            let mut prop = DestructibleProp::tall(
                lbl("SLUDGE COFFEE URN!"),
                pts(490),
                Color::srgb(0.76, 0.80, 0.86),
                Color::srgb(0.25, 0.12, 0.06),
                ImpactProfile::Liquid {
                    liquid_color: Color::srgb(0.25, 0.12, 0.06),
                    puddle_radius: 1.05,
                },
                SfxMaterial::Liquid,
            );
            prop.damage_impulse = 0.95;
            prop.break_impulse = 1.95;
            commands
                .spawn((
                    ScenarioEntity,
                    prop,
                    VfxEmitter {
                        timer: 0.0,
                        interval: 0.42,
                        offset: Vec3::new(0.0, 0.32, 0.0),
                        style: EmitterStyle::CoffeeSteam,
                    },
                    RigidBody::Dynamic,
                    Mass(3.4),
                    Restitution::new(0.44),
                    Friction::new(0.32),
                    Collider::cuboid(0.32, 0.56, 0.32),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|cu| {
                    let body =
                        cache.frustum_y(meshes, Vec2::splat(0.24), Vec2::splat(0.28), 0.44);
                    cu.spawn((
                        Mesh3d(body.clone()),
                        MeshMaterial3d(steel),
                        Transform::from_xyz(0.0, 0.02, 0.0),
                    ));
                    cu.spawn((
                        NotShadowCaster,
                        Mesh3d(body),
                        MeshMaterial3d(shared.ink_outline_mat.clone()),
                        Transform::from_xyz(0.0, 0.02, 0.0).with_scale(Vec3::splat(1.07)),
                    ));
                    spawn_outlined_box(
                        cu,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.26, 0.08, 0.26),
                        black,
                        Transform::from_xyz(0.0, -0.24, 0.0),
                        0.008,
                    );
                });
        }

        PropPrefab::SodaCanSixPack => {
            let cyan = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.22, 0.78, 0.92),
                    metallic: 0.55,
                    perceptual_roughness: 0.30,
                    ..default()
                },
            );
            let mut prop = DestructibleProp::small(
                lbl("WARM SELTZER PACK!"),
                pts(230),
                Color::srgb(0.22, 0.78, 0.92),
                Color::srgb(0.86, 0.90, 0.94),
                ImpactProfile::Liquid {
                    liquid_color: Color::srgb(0.22, 0.68, 0.96),
                    puddle_radius: 0.58,
                },
                SfxMaterial::Liquid,
            );
            prop.damage_impulse = 0.7;
            prop.break_impulse = 1.4;
            commands
                .spawn((
                    ScenarioEntity,
                    prop,
                    RigidBody::Dynamic,
                    Mass(1.1),
                    Restitution::new(0.50),
                    Friction::new(0.28),
                    Collider::cuboid(0.26, 0.15, 0.18),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|sp| {
                    for r in 0..2 {
                        for c in 0..3 {
                            let cx = -0.08 + c as f32 * 0.08;
                            let cz = -0.04 + r as f32 * 0.08;
                            spawn_outlined_box(
                                sp,
                                meshes,
                                cache,
                                shared,
                                Vec3::new(0.065, 0.13, 0.065),
                                cyan.clone(),
                                Transform::from_xyz(cx, 0.0, cz),
                                0.006,
                            );
                        }
                    }
                });
        }

        PropPrefab::PlasticTrophyRack => {
            let wood = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.46, 0.32, 0.20),
                    ..default()
                },
            );
            let lucite = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgba(0.72, 0.92, 1.0, 0.75),
                    emissive: LinearRgba::rgb(0.22, 0.42, 0.55),
                    alpha_mode: AlphaMode::Blend,
                    ..default()
                },
            );
            let mut prop = DestructibleProp::tall(
                lbl("5YR LUCITE PLAQUES!"),
                pts(360),
                Color::srgb(0.76, 0.93, 1.0),
                Color::srgb(0.46, 0.32, 0.20),
                ImpactProfile::GlassCrystal,
                SfxMaterial::Glass,
            );
            prop.damage_impulse = 1.1;
            prop.break_impulse = 2.3;
            commands
                .spawn((
                    ScenarioEntity,
                    prop,
                    RigidBody::Dynamic,
                    Mass(4.5),
                    Restitution::new(0.46),
                    Friction::new(0.30),
                    Collider::cuboid(0.76, 1.18, 0.34),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|tr| {
                    spawn_outlined_box(
                        tr,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.74, 1.16, 0.32),
                        wood,
                        Transform::IDENTITY,
                        0.011,
                    );
                    let plaque_mesh = cache.cuboid(meshes, 0.12, 0.22, 0.05);
                    for px in [-0.22_f32, 0.0, 0.22] {
                        for py in [-0.15_f32, 0.28_f32] {
                            tr.spawn((
                                Mesh3d(plaque_mesh.clone()),
                                MeshMaterial3d(lucite.clone()),
                                Transform::from_xyz(px, py, 0.14),
                            ));
                        }
                    }
                });
        }

        // ── Scene 6: Fulfillment Warehouse Supervisor Cage Prefabs ───────────
        PropPrefab::PalletBoxTower => {
            let pallet_wood = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.62, 0.44, 0.24),
                    perceptual_roughness: 0.85,
                    ..default()
                },
            );
            let carton = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.74, 0.56, 0.34),
                    perceptual_roughness: 0.82,
                    ..default()
                },
            );
            let tape = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.18, 0.56, 0.88),
                    ..default()
                },
            );
            let mut prop = DestructibleProp::tall(
                lbl("PRIME CRUNCH PALLET"),
                pts(540),
                Color::srgb(0.74, 0.56, 0.34),
                Color::srgb(0.62, 0.44, 0.24),
                ImpactProfile::PaperStack { is_blueprint: false },
                SfxMaterial::WoodCeramic,
            );
            prop.damage_impulse = 1.4;
            prop.break_impulse = 2.8;
            prop.debris_count = 12;
            commands
                .spawn((
                    ScenarioEntity,
                    prop,
                    RigidBody::Dynamic,
                    Mass(9.0),
                    Restitution::new(0.42),
                    Friction::new(0.36),
                    Collider::cuboid(1.06, 1.38, 0.96),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|pl| {
                    // Wooden pallet base
                    spawn_outlined_box(
                        pl,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(1.06, 0.14, 0.96),
                        pallet_wood,
                        Transform::from_xyz(0.0, -0.62, 0.0),
                        0.012,
                    );
                    // Stacked shipping cartons!
                    for (bx, by, bz, sx, sy, sz) in [
                        (-0.24, -0.28, -0.18, 0.46, 0.48, 0.44),
                        (0.25, -0.28, -0.16, 0.46, 0.48, 0.44),
                        (-0.22, -0.28, 0.24, 0.46, 0.48, 0.40),
                        (0.24, -0.28, 0.24, 0.44, 0.48, 0.40),
                        (-0.18, 0.20, 0.0, 0.48, 0.44, 0.48),
                        (0.24, 0.18, 0.04, 0.42, 0.40, 0.44),
                        (0.02, 0.54, -0.02, 0.44, 0.30, 0.40),
                    ] {
                        spawn_outlined_box(
                            pl,
                            meshes,
                            cache,
                            shared,
                            Vec3::new(sx, sy, sz),
                            carton.clone(),
                            Transform::from_xyz(bx, by, bz),
                            0.010,
                        );
                        pl.spawn((
                            Mesh3d(cache.cuboid(meshes, sx * 0.92, 0.015, 0.06)),
                            MeshMaterial3d(tape.clone()),
                            Transform::from_xyz(bx, by + sy * 0.5 + 0.005, bz),
                        ));
                    }
                });
        }

        PropPrefab::ConveyorRoller => {
            let frame = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.18, 0.46, 0.34),
                    metallic: 0.45,
                    perceptual_roughness: 0.52,
                    ..default()
                },
            );
            let roller = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.74, 0.78, 0.84),
                    metallic: 0.75,
                    perceptual_roughness: 0.28,
                    ..default()
                },
            );
            let mut prop = DestructibleProp::tall(
                lbl("NO-BREAK CONVEYOR!"),
                pts(520),
                Color::srgb(0.74, 0.78, 0.84),
                Color::srgb(0.18, 0.46, 0.34),
                ImpactProfile::HeavyProp,
                SfxMaterial::Metal,
            );
            prop.damage_impulse = 1.6;
            prop.break_impulse = 3.2;
            commands
                .spawn((
                    ScenarioEntity,
                    prop,
                    RigidBody::Dynamic,
                    Mass(11.0),
                    Restitution::new(0.44),
                    Friction::new(0.32),
                    Collider::cuboid(1.68, 0.78, 0.64),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|cv| {
                    for rz in [-0.28_f32, 0.28_f32] {
                        spawn_outlined_box(
                            cv,
                            meshes,
                            cache,
                            shared,
                            Vec3::new(1.66, 0.10, 0.06),
                            frame.clone(),
                            Transform::from_xyz(0.0, 0.32, rz),
                            0.010,
                        );
                    }
                    let roller_mesh = cache.cuboid(meshes, 0.09, 0.07, 0.52);
                    for i in 0..7 {
                        let rx = -0.66 + i as f32 * 0.22;
                        cv.spawn((
                            Mesh3d(roller_mesh.clone()),
                            MeshMaterial3d(roller.clone()),
                            Transform::from_xyz(rx, 0.34, 0.0),
                        ));
                    }
                    let leg_mesh = cache.cuboid(meshes, 0.06, 0.70, 0.06);
                    for lx in [-0.68_f32, 0.68_f32] {
                        for lz in [-0.26_f32, 0.26_f32] {
                            cv.spawn((
                                Mesh3d(leg_mesh.clone()),
                                MeshMaterial3d(frame.clone()),
                                Transform::from_xyz(lx, -0.04, lz),
                            ));
                        }
                    }
                });
        }

        PropPrefab::BarcodeScannerGun => {
            let yel = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.96, 0.76, 0.12),
                    perceptual_roughness: 0.42,
                    ..default()
                },
            );
            let laser = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(1.0, 0.20, 0.18),
                    emissive: LinearRgba::rgb(2.4, 0.15, 0.12),
                    unlit: true,
                    ..default()
                },
            );
            let mut prop = DestructibleProp::small(
                lbl("PICK-RATE SCANNER!"),
                pts(260),
                Color::srgb(0.96, 0.76, 0.12),
                Color::srgb(0.18, 0.20, 0.24),
                ImpactProfile::Electronics,
                SfxMaterial::Electronics,
            );
            prop.damage_impulse = 0.7;
            prop.break_impulse = 1.4;
            commands
                .spawn((
                    ScenarioEntity,
                    prop,
                    RigidBody::Dynamic,
                    Mass(0.8),
                    Restitution::new(0.52),
                    Friction::new(0.28),
                    Collider::cuboid(0.18, 0.22, 0.22),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|bs| {
                    spawn_outlined_box(
                        bs,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.10, 0.08, 0.18),
                        yel.clone(),
                        Transform::from_xyz(0.0, 0.05, 0.0),
                        0.006,
                    );
                    spawn_outlined_box(
                        bs,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.07, 0.14, 0.07),
                        yel,
                        Transform::from_xyz(0.0, -0.04, -0.03)
                            .with_rotation(Quat::from_rotation_x(0.28)),
                        0.006,
                    );
                    bs.spawn((
                        Mesh3d(cache.cuboid(meshes, 0.07, 0.03, 0.02)),
                        MeshMaterial3d(laser),
                        Transform::from_xyz(0.0, 0.05, 0.095),
                    ));
                });
        }

        PropPrefab::QuotaTimerSign => {
            let post = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.92, 0.72, 0.14),
                    perceptual_roughness: 0.45,
                    ..default()
                },
            );
            let screen = cache.material(
                materials,
                StandardMaterial {
                    base_color_texture: Some(textures.injury_board.clone()),
                    emissive: LinearRgba::rgb(1.15, 0.35, 0.18),
                    emissive_texture: Some(textures.injury_board.clone()),
                    ..default()
                },
            );
            let mut prop = DestructibleProp::tall(
                lbl("BATHROOM COUNTDOWN!"),
                pts(560),
                Color::srgb(0.92, 0.72, 0.14),
                Color::srgb(0.96, 0.24, 0.20),
                ImpactProfile::Electronics,
                SfxMaterial::Electronics,
            );
            prop.damage_impulse = 1.2;
            prop.break_impulse = 2.5;
            commands
                .spawn((
                    ScenarioEntity,
                    prop,
                    RigidBody::Dynamic,
                    Mass(5.2),
                    Restitution::new(0.46),
                    Friction::new(0.30),
                    Collider::cuboid(0.86, 1.54, 0.40),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|qt| {
                    spawn_outlined_box(
                        qt,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.46, 0.06, 0.38),
                        post.clone(),
                        Transform::from_xyz(0.0, -0.74, 0.0),
                        0.009,
                    );
                    qt.spawn((
                        Mesh3d(cache.cuboid(meshes, 0.07, 1.04, 0.07)),
                        MeshMaterial3d(post.clone()),
                        Transform::from_xyz(0.0, -0.20, 0.0),
                    ));
                    spawn_outlined_box(
                        qt,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.84, 0.48, 0.10),
                        post,
                        Transform::from_xyz(0.0, 0.50, 0.0),
                        0.011,
                    );
                    qt.spawn((
                        ScreenSurface,
                        Mesh3d(cache.upright_quad(meshes, 0.76, 0.40)),
                        MeshMaterial3d(screen),
                        Transform::from_xyz(0.0, 0.50, 0.056),
                    ));
                });
        }

        PropPrefab::ShrinkWrapRoll => {
            let film = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgba(0.82, 0.92, 0.98, 0.68),
                    alpha_mode: AlphaMode::Blend,
                    perceptual_roughness: 0.18,
                    ..default()
                },
            );
            let core = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.64, 0.46, 0.28),
                    ..default()
                },
            );
            let mut prop = DestructibleProp::tall(
                lbl("STRETCH-WRAP ROLL!"),
                pts(310),
                Color::srgb(0.86, 0.92, 0.96),
                Color::srgb(0.64, 0.46, 0.28),
                ImpactProfile::HeavyProp,
                SfxMaterial::Paper,
            );
            prop.damage_impulse = 1.0;
            prop.break_impulse = 2.1;
            commands
                .spawn((
                    ScenarioEntity,
                    prop,
                    RigidBody::Dynamic,
                    Mass(3.0),
                    Restitution::new(0.50),
                    Friction::new(0.28),
                    Collider::cuboid(0.32, 0.96, 0.32),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|sw| {
                    sw.spawn((
                        Mesh3d(cache.cuboid(meshes, 0.10, 0.96, 0.10)),
                        MeshMaterial3d(core),
                        Transform::IDENTITY,
                    ));
                    let roll =
                        cache.frustum_y(meshes, Vec2::splat(0.26), Vec2::splat(0.26), 0.82);
                    sw.spawn((Mesh3d(roll), MeshMaterial3d(film), Transform::IDENTITY));
                });
        }

        PropPrefab::PropaneForkliftTank => {
            let tank_yel = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.94, 0.72, 0.14),
                    metallic: 0.45,
                    perceptual_roughness: 0.38,
                    ..default()
                },
            );
            let valve = cache.material(
                materials,
                StandardMaterial {
                    base_color: Color::srgb(0.86, 0.20, 0.18),
                    ..default()
                },
            );
            let mut prop = DestructibleProp::tall(
                lbl("FORKLIFT LP TANK!"),
                pts(700),
                Color::srgb(0.94, 0.72, 0.14),
                Color::srgb(0.96, 0.24, 0.18),
                ImpactProfile::PressurizedHazard,
                SfxMaterial::PressurizedBlast,
            );
            prop.damage_impulse = 1.1;
            prop.break_impulse = 2.1;
            prop.debris_count = 12;
            commands
                .spawn((
                    ScenarioEntity,
                    prop,
                    VfxEmitter {
                        timer: 0.0,
                        interval: 0.36,
                        offset: Vec3::new(0.0, 0.42, 0.0),
                        style: EmitterStyle::PressurizedHiss,
                    },
                    RigidBody::Dynamic,
                    Mass(6.5),
                    Restitution::new(0.48),
                    Friction::new(0.32),
                    Collider::cuboid(0.38, 0.86, 0.38),
                    tf,
                    Visibility::default(),
                ))
                .with_children(|pt| {
                    let cyl =
                        cache.frustum_y(meshes, Vec2::splat(0.32), Vec2::splat(0.32), 0.68);
                    pt.spawn((
                        Mesh3d(cyl.clone()),
                        MeshMaterial3d(tank_yel.clone()),
                        Transform::from_xyz(0.0, -0.06, 0.0),
                    ));
                    pt.spawn((
                        NotShadowCaster,
                        Mesh3d(cyl),
                        MeshMaterial3d(shared.ink_outline_mat.clone()),
                        Transform::from_xyz(0.0, -0.06, 0.0).with_scale(Vec3::splat(1.07)),
                    ));
                    spawn_outlined_box(
                        pt,
                        meshes,
                        cache,
                        shared,
                        Vec3::new(0.24, 0.14, 0.24),
                        valve,
                        Transform::from_xyz(0.0, 0.34, 0.0),
                        0.008,
                    );
                });
        }
    }
}
