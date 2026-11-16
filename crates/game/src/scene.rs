use avian3d::prelude::*;
use bevy::math::Affine2;
use bevy::prelude::*;

use crate::pixel_art::ProceduralTextures;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ScenarioId {
    #[default]
    CorporateCubicle,
    BuildingSite,
}

impl ScenarioId {
    pub fn title(self) -> &'static str {
        match self {
            Self::CorporateCubicle => "CORPORATE CUBICLE",
            Self::BuildingSite => "BUILDING SITE TRAILER",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::CorporateCubicle => Self::BuildingSite,
            Self::BuildingSite => Self::CorporateCubicle,
        }
    }
}

#[derive(Resource, Default)]
pub struct CurrentScenario(pub ScenarioId);

#[derive(Component)]
pub struct ScenarioEntity;

#[derive(Component)]
pub struct PlayerDesk;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DebrisStyle {
    Generic,
    Papers,
    Water,
    Electronics,
    Ceramic,
}

#[derive(Component, Clone)]
pub struct DestructibleProp {
    pub label: &'static str,
    pub base_points: u32,
    pub can_topple: bool,
    pub toppled: bool,
    pub damaged: bool,
    pub shattered: bool,
    pub damage_impulse: f32,
    pub break_impulse: f32,
    pub debris_color: Color,
    pub debris_style: DebrisStyle,
    pub debris_count: u8,
    pub prev_velocity: Vec3,
}

impl DestructibleProp {
    pub fn small(
        label: &'static str,
        base_points: u32,
        debris_color: Color,
        debris_style: DebrisStyle,
    ) -> Self {
        Self {
            label,
            base_points,
            can_topple: false,
            toppled: false,
            damaged: false,
            shattered: false,
            damage_impulse: 1.4,
            break_impulse: 2.6,
            debris_color,
            debris_style,
            debris_count: 6,
            prev_velocity: Vec3::ZERO,
        }
    }

    pub fn tall(
        label: &'static str,
        base_points: u32,
        debris_color: Color,
        debris_style: DebrisStyle,
    ) -> Self {
        Self {
            label,
            base_points,
            can_topple: true,
            toppled: false,
            damaged: false,
            shattered: false,
            damage_impulse: 2.2,
            break_impulse: 4.5,
            debris_color,
            debris_style,
            debris_count: 8,
            prev_velocity: Vec3::ZERO,
        }
    }
}

pub fn spawn_scenario(
    id: ScenarioId,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    textures: &ProceduralTextures,
) {
    match id {
        ScenarioId::CorporateCubicle => {
            spawn_office_scenario(commands, meshes, materials, textures)
        }
        ScenarioId::BuildingSite => {
            spawn_building_site_scenario(commands, meshes, materials, textures)
        }
    }
}

fn spawn_room_bounds(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    textures: &ProceduralTextures,
    floor_mat: Handle<StandardMaterial>,
    wall_mat: Handle<StandardMaterial>,
    trim_mat: Handle<StandardMaterial>,
    window_mat: Handle<StandardMaterial>,
) {
    // Floor
    commands.spawn((
        ScenarioEntity,
        RigidBody::Static,
        Restitution::new(0.45),
        Friction::new(0.30),
        Collider::cuboid(12.0, 0.4, 12.0),
        Mesh3d(meshes.add(Cuboid::new(12.0, 0.4, 12.0))),
        MeshMaterial3d(floor_mat),
        Transform::from_xyz(0.0, -0.2, -1.0),
    ));

    // Acoustic tile ceiling
    let ceiling_mat = materials.add(StandardMaterial {
        base_color_texture: Some(textures.ceiling_tile.clone()),
        uv_transform: Affine2::from_scale(Vec2::splat(8.0)),
        perceptual_roughness: 0.9,
        ..default()
    });
    commands.spawn((
        ScenarioEntity,
        Mesh3d(meshes.add(Cuboid::new(12.0, 0.2, 12.0))),
        MeshMaterial3d(ceiling_mat),
        Transform::from_xyz(0.0, 4.35, -1.0),
    ));

    // Back wall + baseboard
    commands.spawn((
        ScenarioEntity,
        RigidBody::Static,
        Restitution::new(0.55),
        Friction::new(0.25),
        Collider::cuboid(12.0, 4.5, 0.4),
        Mesh3d(meshes.add(Cuboid::new(12.0, 4.5, 0.4))),
        MeshMaterial3d(wall_mat.clone()),
        Transform::from_xyz(0.0, 2.25, -6.5),
    ));
    commands.spawn((
        ScenarioEntity,
        Mesh3d(meshes.add(Cuboid::new(12.0, 0.18, 0.06))),
        MeshMaterial3d(trim_mat.clone()),
        Transform::from_xyz(0.0, 0.09, -6.28),
    ));

    // Left & Right walls + baseboards
    for x in [-5.5_f32, 5.5_f32] {
        commands.spawn((
            ScenarioEntity,
            RigidBody::Static,
            Restitution::new(0.55),
            Friction::new(0.25),
            Collider::cuboid(0.4, 4.5, 12.0),
            Mesh3d(meshes.add(Cuboid::new(0.4, 4.5, 12.0))),
            MeshMaterial3d(wall_mat.clone()),
            Transform::from_xyz(x, 2.25, -1.0),
        ));
        let inner_x = if x < 0.0 { x + 0.21 } else { x - 0.21 };
        commands.spawn((
            ScenarioEntity,
            Mesh3d(meshes.add(Cuboid::new(0.06, 0.18, 12.0))),
            MeshMaterial3d(trim_mat.clone()),
            Transform::from_xyz(inner_x, 0.09, -1.0),
        ));
    }

    // Back window + Venetian blind slats
    commands.spawn((
        ScenarioEntity,
        Mesh3d(meshes.add(Cuboid::new(3.8, 1.9, 0.06))),
        MeshMaterial3d(window_mat),
        Transform::from_xyz(0.0, 2.35, -6.27),
    ));
    let slat_mesh = meshes.add(Cuboid::new(3.8, 0.04, 0.08));
    for i in 0..8 {
        let sy = 1.55 + i as f32 * 0.21;
        commands.spawn((
            ScenarioEntity,
            Mesh3d(slat_mesh.clone()),
            MeshMaterial3d(trim_mat.clone()),
            Transform::from_xyz(0.0, sy, -6.22),
        ));
    }

    // Subtle sunlit floor reflection strips beneath window
    let sun_floor_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.88, 0.52, 0.16),
        emissive: LinearRgba::rgb(0.25, 0.20, 0.08),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    for i in 0..3 {
        commands.spawn((
            ScenarioEntity,
            Mesh3d(meshes.add(Cuboid::new(3.4, 0.006, 0.35))),
            MeshMaterial3d(sun_floor_mat.clone()),
            Transform::from_xyz(0.0, 0.004, -5.2 + i as f32 * 0.65),
        ));
    }

    // Wall clock above window
    let clock_face = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.94, 0.88),
        ..default()
    });
    commands.spawn((
        ScenarioEntity,
        Mesh3d(meshes.add(Cylinder::new(0.28, 0.06))),
        MeshMaterial3d(clock_face),
        Transform::from_xyz(2.8, 3.15, -6.25)
            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
    ));

    // Ceiling fluorescent light panels (glowing!)
    let fluor_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.96, 0.99, 1.0),
        emissive: LinearRgba::rgb(1.4, 1.45, 1.5),
        ..default()
    });
    for fz in [-1.2_f32, -4.0_f32] {
        commands.spawn((
            ScenarioEntity,
            Mesh3d(meshes.add(Cuboid::new(2.4, 0.06, 0.55))),
            MeshMaterial3d(fluor_mat.clone()),
            Transform::from_xyz(0.0, 4.22, fz),
        ));
    }
}

pub fn spawn_office_scenario(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    textures: &ProceduralTextures,
) {
    // ── Room Static Geometry ───────────────────────────────────────────────
    let floor_mat = materials.add(StandardMaterial {
        base_color_texture: Some(textures.carpet.clone()),
        uv_transform: Affine2::from_scale(Vec2::splat(10.0)),
        perceptual_roughness: 0.9,
        ..default()
    });
    let wall_mat = materials.add(StandardMaterial {
        base_color_texture: Some(textures.wall_grid.clone()),
        uv_transform: Affine2::from_scale(Vec2::new(6.0, 2.5)),
        perceptual_roughness: 0.9,
        ..default()
    });
    let trim_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.20, 0.24, 0.30),
        perceptual_roughness: 0.8,
        ..default()
    });
    let window_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.86, 0.48),
        emissive: LinearRgba::rgb(0.85, 0.64, 0.25),
        ..default()
    });

    spawn_room_bounds(
        commands,
        meshes,
        materials,
        textures,
        floor_mat,
        wall_mat,
        trim_mat.clone(),
        window_mat,
    );

    // ── Player's Oak Desk ──────────────────────────────────────────────────
    let oak_mat = materials.add(StandardMaterial {
        base_color_texture: Some(textures.wood.clone()),
        uv_transform: Affine2::from_scale(Vec2::new(2.0, 1.0)),
        perceptual_roughness: 0.68,
        ..default()
    });
    let leg_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.20, 0.22, 0.27),
        perceptual_roughness: 0.5,
        ..default()
    });

    commands
        .spawn((
            ScenarioEntity,
            PlayerDesk,
            RigidBody::Dynamic,
            Mass(22.0),
            Friction::new(0.32),
            Restitution::new(0.40),
            LinearDamping(0.08),
            AngularDamping(0.08),
            Collider::compound(vec![
                (
                    Vec3::new(0.0, 0.34, 0.0),
                    Quat::IDENTITY,
                    Collider::cuboid(2.1, 0.08, 1.0),
                ),
                (
                    Vec3::new(-0.92, -0.02, -0.40),
                    Quat::IDENTITY,
                    Collider::cuboid(0.09, 0.68, 0.09),
                ),
                (
                    Vec3::new(0.92, -0.02, -0.40),
                    Quat::IDENTITY,
                    Collider::cuboid(0.09, 0.68, 0.09),
                ),
                (
                    Vec3::new(-0.92, -0.02, 0.40),
                    Quat::IDENTITY,
                    Collider::cuboid(0.09, 0.68, 0.09),
                ),
                (
                    Vec3::new(0.92, -0.02, 0.40),
                    Quat::IDENTITY,
                    Collider::cuboid(0.09, 0.68, 0.09),
                ),
            ]),
            Transform::from_xyz(0.0, 0.38, 0.85),
            Visibility::default(),
        ))
        .with_children(|desk| {
            desk.spawn((
                Mesh3d(meshes.add(Cuboid::new(2.1, 0.08, 1.0))),
                MeshMaterial3d(oak_mat.clone()),
                Transform::from_xyz(0.0, 0.34, 0.0),
            ));
            let leg_mesh = meshes.add(Cuboid::new(0.09, 0.68, 0.09));
            for (lx, lz) in [(-0.92, -0.40), (0.92, -0.40), (-0.92, 0.40), (0.92, 0.40)] {
                desk.spawn((
                    Mesh3d(leg_mesh.clone()),
                    MeshMaterial3d(leg_mat.clone()),
                    Transform::from_xyz(lx, -0.02, lz),
                ));
            }
        });

    // ── Desk Props (y ~ 0.76 tabletop) ─────────────────────────────────────
    let crt_case = materials.add(StandardMaterial {
        base_color: Color::srgb(0.88, 0.84, 0.74),
        perceptual_roughness: 0.65,
        ..default()
    });
    let crt_screen = materials.add(StandardMaterial {
        base_color_texture: Some(textures.crt_angry.clone()),
        emissive: LinearRgba::rgb(0.18, 0.55, 0.52),
        emissive_texture: Some(textures.crt_angry.clone()),
        ..default()
    });

    // CRT Monitor (compound visual: stand + main bezel + rear tube + angry screen)
    commands
        .spawn((
            ScenarioEntity,
            DestructibleProp::small(
                "CRT MONITOR",
                300,
                Color::srgb(0.20, 0.82, 0.76),
                DebrisStyle::Electronics,
            ),
            RigidBody::Dynamic,
            Mass(3.8),
            Restitution::new(0.50),
            Friction::new(0.30),
            LinearDamping(0.10),
            AngularDamping(0.10),
            Collider::cuboid(0.46, 0.44, 0.42),
            Transform::from_xyz(-0.28, 0.98, 0.70),
            Visibility::default(),
        ))
        .with_children(|crt| {
            // Stand
            crt.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.26, 0.06, 0.24))),
                MeshMaterial3d(crt_case.clone()),
                Transform::from_xyz(0.0, -0.19, 0.0),
            ));
            // Front bezel
            crt.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.46, 0.38, 0.24))),
                MeshMaterial3d(crt_case.clone()),
                Transform::from_xyz(0.0, 0.02, 0.06),
            ));
            // Rear tube taper
            crt.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.34, 0.28, 0.20))),
                MeshMaterial3d(crt_case.clone()),
                Transform::from_xyz(0.0, 0.01, -0.14),
            ));
            // Angry glowing screen
            crt.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.36, 0.28, 0.02))),
                MeshMaterial3d(crt_screen.clone()),
                Transform::from_xyz(0.0, 0.02, 0.185),
            ));
        });

    // Mechanical Keyboard + keycaps
    let keycap_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.72, 0.69, 0.62),
        perceptual_roughness: 0.7,
        ..default()
    });
    commands
        .spawn((
            ScenarioEntity,
            DestructibleProp::small(
                "KEYBOARD",
                120,
                Color::srgb(0.86, 0.83, 0.75),
                DebrisStyle::Electronics,
            ),
            RigidBody::Dynamic,
            Mass(1.0),
            Restitution::new(0.55),
            Friction::new(0.28),
            LinearDamping(0.10),
            AngularDamping(0.10),
            Collider::cuboid(0.48, 0.05, 0.18),
            Mesh3d(meshes.add(Cuboid::new(0.48, 0.035, 0.18))),
            MeshMaterial3d(crt_case.clone()),
            Transform::from_xyz(-0.02, 0.785, 1.08),
        ))
        .with_children(|kb| {
            kb.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.42, 0.02, 0.12))),
                MeshMaterial3d(keycap_mat),
                Transform::from_xyz(0.0, 0.022, -0.01),
            ));
        });

    // Coffee Mug (with handle + coffee surface)
    let mug_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.96, 0.94, 0.90),
        perceptual_roughness: 0.35,
        ..default()
    });
    let coffee_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.24, 0.13, 0.07),
        perceptual_roughness: 0.2,
        ..default()
    });
    let mut mug_prop = DestructibleProp::small(
        "COFFEE MUG",
        150,
        Color::srgb(0.95, 0.93, 0.88),
        DebrisStyle::Ceramic,
    );
    mug_prop.break_impulse = 1.9;
    commands
        .spawn((
            ScenarioEntity,
            mug_prop,
            RigidBody::Dynamic,
            Mass(0.55),
            Restitution::new(0.52),
            Friction::new(0.28),
            LinearDamping(0.10),
            AngularDamping(0.10),
            Collider::cylinder(0.065, 0.13),
            Mesh3d(meshes.add(Cylinder::new(0.065, 0.13))),
            MeshMaterial3d(mug_mat.clone()),
            Transform::from_xyz(0.32, 0.83, 0.95),
        ))
        .with_children(|mug| {
            mug.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.052, 0.015))),
                MeshMaterial3d(coffee_mat),
                Transform::from_xyz(0.0, 0.062, 0.0),
            ));
            mug.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.04, 0.075, 0.025))),
                MeshMaterial3d(mug_mat),
                Transform::from_xyz(0.072, 0.0, 0.0),
            ));
        });

    // Red Stapler (base + top lever)
    let stapler_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.90, 0.20, 0.20),
        perceptual_roughness: 0.4,
        ..default()
    });
    commands
        .spawn((
            ScenarioEntity,
            DestructibleProp::small(
                "RED STAPLER!",
                200,
                Color::srgb(0.92, 0.24, 0.24),
                DebrisStyle::Generic,
            ),
            RigidBody::Dynamic,
            Mass(0.65),
            Restitution::new(0.60),
            Friction::new(0.26),
            LinearDamping(0.10),
            AngularDamping(0.10),
            Collider::cuboid(0.16, 0.06, 0.07),
            Mesh3d(meshes.add(Cuboid::new(0.16, 0.025, 0.07))),
            MeshMaterial3d(stapler_mat.clone()),
            Transform::from_xyz(-0.68, 0.79, 1.02),
        ))
        .with_children(|st| {
            st.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.15, 0.03, 0.06))),
                MeshMaterial3d(stapler_mat),
                Transform::from_xyz(0.0, 0.025, 0.0).with_rotation(Quat::from_rotation_z(0.14)),
            ));
        });

    // Potted Cactus (cute terracotta pot + green cactus + arms)
    let pot_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.82, 0.42, 0.24),
        perceptual_roughness: 0.8,
        ..default()
    });
    let cactus_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.22, 0.68, 0.34),
        perceptual_roughness: 0.75,
        ..default()
    });
    commands
        .spawn((
            ScenarioEntity,
            DestructibleProp::small(
                "POOR CACTUS!",
                180,
                Color::srgb(0.28, 0.72, 0.36),
                DebrisStyle::Ceramic,
            ),
            RigidBody::Dynamic,
            Mass(0.8),
            Restitution::new(0.50),
            Friction::new(0.30),
            LinearDamping(0.10),
            AngularDamping(0.10),
            Collider::cylinder(0.08, 0.22),
            Transform::from_xyz(-0.78, 0.87, 0.72),
            Visibility::default(),
        ))
        .with_children(|cac| {
            cac.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.075, 0.11))),
                MeshMaterial3d(pot_mat),
                Transform::from_xyz(0.0, -0.055, 0.0),
            ));
            cac.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.042, 0.15))),
                MeshMaterial3d(cactus_mat.clone()),
                Transform::from_xyz(0.0, 0.045, 0.0),
            ));
            cac.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.06, 0.035, 0.035))),
                MeshMaterial3d(cactus_mat.clone()),
                Transform::from_xyz(0.045, 0.055, 0.0),
            ));
            cac.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.05, 0.035, 0.035))),
                MeshMaterial3d(cactus_mat),
                Transform::from_xyz(-0.042, 0.035, 0.0),
            ));
        });

    // TPS Report Paper Stacks
    let paper_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.96, 0.96, 0.92),
        perceptual_roughness: 0.9,
        ..default()
    });
    for (px, pz, h) in [(0.62, 0.78, 0.14), (0.76, 0.96, 0.10)] {
        let mut paper_prop = DestructibleProp::small(
            "TPS REPORTS",
            100,
            Color::srgb(0.96, 0.96, 0.93),
            DebrisStyle::Papers,
        );
        paper_prop.break_impulse = 1.6;
        paper_prop.debris_count = 10;
        commands.spawn((
            ScenarioEntity,
            paper_prop,
            RigidBody::Dynamic,
            Mass(0.7),
            Restitution::new(0.45),
            Friction::new(0.30),
            LinearDamping(0.10),
            AngularDamping(0.10),
            Collider::cuboid(0.24, h, 0.30),
            Mesh3d(meshes.add(Cuboid::new(0.24, h, 0.30))),
            MeshMaterial3d(paper_mat.clone()),
            Transform::from_xyz(px, 0.76 + h * 0.5, pz),
        ));
    }

    // ── Room Props (Target Zone) ───────────────────────────────────────────
    let partition_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.28, 0.60, 0.56),
        perceptual_roughness: 0.9,
        ..default()
    });
    let cat_poster_mat = materials.add(StandardMaterial {
        base_color_texture: Some(textures.cat_poster.clone()),
        perceptual_roughness: 0.6,
        ..default()
    });

    for (idx, (cx, cz)) in [(-1.85, -1.55), (1.85, -1.75)].iter().enumerate() {
        commands
            .spawn((
                ScenarioEntity,
                DestructibleProp::tall(
                    "CUBICLE WALL",
                    350,
                    Color::srgb(0.32, 0.64, 0.58),
                    DebrisStyle::Generic,
                ),
                RigidBody::Dynamic,
                Mass(12.0),
                Restitution::new(0.45),
                Friction::new(0.32),
                LinearDamping(0.10),
                AngularDamping(0.10),
                Collider::cuboid(1.6, 1.55, 0.12),
                Mesh3d(meshes.add(Cuboid::new(1.6, 1.55, 0.12))),
                MeshMaterial3d(partition_mat.clone()),
                Transform::from_xyz(*cx, 0.78, *cz),
            ))
            .with_children(|part| {
                // Top metal rail
                part.spawn((
                    Mesh3d(meshes.add(Cuboid::new(1.64, 0.06, 0.15))),
                    MeshMaterial3d(trim_mat.clone()),
                    Transform::from_xyz(0.0, 0.77, 0.0),
                ));
                // Crooked cat poster on the left cubicle partition
                if idx == 0 {
                    part.spawn((
                        Mesh3d(meshes.add(Cuboid::new(0.42, 0.42, 0.03))),
                        MeshMaterial3d(cat_poster_mat.clone()),
                        Transform::from_xyz(0.15, 0.25, 0.075)
                            .with_rotation(Quat::from_rotation_z(-0.08)),
                    ));
                }
            });
    }

    // Tall Filing Cabinets (with 3 drawer fronts & handles)
    let cabinet_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.46, 0.50, 0.42),
        perceptual_roughness: 0.6,
        ..default()
    });
    let drawer_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.38, 0.42, 0.35),
        perceptual_roughness: 0.55,
        ..default()
    });
    for (fx, fz) in [(-1.25, -3.2), (1.45, -3.5)] {
        commands
            .spawn((
                ScenarioEntity,
                DestructibleProp::tall(
                    "FILING CABINET",
                    450,
                    Color::srgb(0.48, 0.52, 0.45),
                    DebrisStyle::Papers,
                ),
                RigidBody::Dynamic,
                Mass(15.0),
                Restitution::new(0.48),
                Friction::new(0.30),
                LinearDamping(0.10),
                AngularDamping(0.10),
                Collider::cuboid(0.65, 1.50, 0.65),
                Mesh3d(meshes.add(Cuboid::new(0.65, 1.50, 0.65))),
                MeshMaterial3d(cabinet_mat.clone()),
                Transform::from_xyz(fx, 0.75, fz),
            ))
            .with_children(|cab| {
                for dy in [-0.42_f32, 0.0, 0.42] {
                    cab.spawn((
                        Mesh3d(meshes.add(Cuboid::new(0.54, 0.34, 0.03))),
                        MeshMaterial3d(drawer_mat.clone()),
                        Transform::from_xyz(0.0, dy, 0.33),
                    ));
                }
            });
    }

    // Water Cooler
    let cooler_body_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.90, 0.92, 0.95),
        perceptual_roughness: 0.45,
        ..default()
    });
    let cooler_jug_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.24, 0.70, 0.96),
        emissive: LinearRgba::rgb(0.04, 0.14, 0.24),
        perceptual_roughness: 0.2,
        ..default()
    });
    commands
        .spawn((
            ScenarioEntity,
            DestructibleProp::tall(
                "WATER COOLER",
                500,
                Color::srgb(0.32, 0.76, 0.98),
                DebrisStyle::Water,
            ),
            RigidBody::Dynamic,
            Mass(10.0),
            Restitution::new(0.55),
            Friction::new(0.28),
            LinearDamping(0.10),
            AngularDamping(0.10),
            Collider::cuboid(0.45, 1.05, 0.45),
            Mesh3d(meshes.add(Cuboid::new(0.45, 1.05, 0.45))),
            MeshMaterial3d(cooler_body_mat),
            Transform::from_xyz(0.0, 0.53, -2.55),
        ))
        .with_children(|cooler| {
            cooler.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.18, 0.42))),
                MeshMaterial3d(cooler_jug_mat),
                Transform::from_xyz(0.0, 0.72, 0.0),
            ));
        });

    // Swivel Chair near coworker's desk
    let chair_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.18, 0.20, 0.25),
        perceptual_roughness: 0.75,
        ..default()
    });
    commands
        .spawn((
            ScenarioEntity,
            DestructibleProp::tall(
                "SWIVEL CHAIR",
                220,
                Color::srgb(0.22, 0.25, 0.32),
                DebrisStyle::Generic,
            ),
            RigidBody::Dynamic,
            Mass(6.0),
            Restitution::new(0.60),
            Friction::new(0.22),
            LinearDamping(0.08),
            AngularDamping(0.08),
            Collider::cuboid(0.52, 0.90, 0.52),
            Transform::from_xyz(-0.75, 0.46, -2.1),
            Visibility::default(),
        ))
        .with_children(|ch| {
            ch.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.26, 0.06))),
                MeshMaterial3d(leg_mat.clone()),
                Transform::from_xyz(0.0, -0.40, 0.0),
            ));
            ch.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.04, 0.36))),
                MeshMaterial3d(leg_mat),
                Transform::from_xyz(0.0, -0.20, 0.0),
            ));
            ch.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.48, 0.08, 0.46))),
                MeshMaterial3d(chair_mat.clone()),
                Transform::from_xyz(0.0, 0.0, 0.0),
            ));
            ch.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.44, 0.44, 0.08))),
                MeshMaterial3d(chair_mat),
                Transform::from_xyz(0.0, 0.24, -0.20),
            ));
        });

    // Coworker Desk + CRT in the back
    commands.spawn((
        ScenarioEntity,
        DestructibleProp::tall(
            "COWORKER DESK",
            400,
            Color::srgb(0.62, 0.38, 0.19),
            DebrisStyle::Generic,
        ),
        RigidBody::Dynamic,
        Mass(16.0),
        Restitution::new(0.45),
        Friction::new(0.32),
        LinearDamping(0.10),
        AngularDamping(0.10),
        Collider::cuboid(1.8, 0.75, 0.85),
        Mesh3d(meshes.add(Cuboid::new(1.8, 0.75, 0.85))),
        MeshMaterial3d(oak_mat),
        Transform::from_xyz(0.15, 0.38, -4.45),
    ));
    commands
        .spawn((
            ScenarioEntity,
            DestructibleProp::small(
                "COWORKER CRT",
                350,
                Color::srgb(0.20, 0.82, 0.76),
                DebrisStyle::Electronics,
            ),
            RigidBody::Dynamic,
            Mass(3.8),
            Restitution::new(0.50),
            Friction::new(0.30),
            LinearDamping(0.10),
            AngularDamping(0.10),
            Collider::cuboid(0.46, 0.42, 0.40),
            Mesh3d(meshes.add(Cuboid::new(0.46, 0.42, 0.40))),
            MeshMaterial3d(crt_case),
            Transform::from_xyz(0.15, 0.98, -4.45),
        ))
        .with_children(|crt| {
            crt.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.36, 0.28, 0.03))),
                MeshMaterial3d(crt_screen),
                Transform::from_xyz(0.0, 0.02, 0.20),
            ));
        });

    // Office Printer / Copier Station (left-midground)
    let printer_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.84, 0.84, 0.80),
        perceptual_roughness: 0.55,
        ..default()
    });
    let led_green = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.95, 0.45),
        emissive: LinearRgba::rgb(0.3, 1.5, 0.5),
        ..default()
    });
    commands
        .spawn((
            ScenarioEntity,
            DestructibleProp::tall(
                "PC LOAD LETTER!",
                420,
                Color::srgb(0.86, 0.86, 0.82),
                DebrisStyle::Electronics,
            ),
            RigidBody::Dynamic,
            Mass(11.0),
            Restitution::new(0.50),
            Friction::new(0.28),
            LinearDamping(0.09),
            AngularDamping(0.09),
            Collider::cuboid(0.68, 0.92, 0.60),
            Mesh3d(meshes.add(Cuboid::new(0.68, 0.52, 0.60))),
            MeshMaterial3d(printer_mat.clone()),
            Transform::from_xyz(-2.35, 0.55, -2.85),
        ))
        .with_children(|pr| {
            pr.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.56, 0.40, 0.48))),
                MeshMaterial3d(trim_mat.clone()),
                Transform::from_xyz(0.0, -0.36, 0.0),
            ));
            pr.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.06, 0.04, 0.04))),
                MeshMaterial3d(led_green),
                Transform::from_xyz(0.22, 0.16, 0.30),
            ));
        });

    // Office Bookshelf with colorful manuals (back-right)
    let shelf_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.52, 0.34, 0.18),
        perceptual_roughness: 0.75,
        ..default()
    });
    let book_colors = [
        Color::srgb(0.85, 0.25, 0.25),
        Color::srgb(0.22, 0.55, 0.85),
        Color::srgb(0.92, 0.72, 0.20),
    ];
    commands
        .spawn((
            ScenarioEntity,
            DestructibleProp::tall(
                "SYNERGY MANUALS!",
                480,
                Color::srgb(0.62, 0.40, 0.22),
                DebrisStyle::Papers,
            ),
            RigidBody::Dynamic,
            Mass(14.0),
            Restitution::new(0.45),
            Friction::new(0.30),
            LinearDamping(0.09),
            AngularDamping(0.09),
            Collider::cuboid(1.1, 1.65, 0.42),
            Mesh3d(meshes.add(Cuboid::new(1.1, 1.65, 0.42))),
            MeshMaterial3d(shelf_mat),
            Transform::from_xyz(2.65, 0.83, -4.65),
        ))
        .with_children(|bs| {
            for (row_idx, by) in [-0.35_f32, 0.15, 0.55].iter().enumerate() {
                let bm = materials.add(StandardMaterial {
                    base_color: book_colors[row_idx % book_colors.len()],
                    perceptual_roughness: 0.7,
                    ..default()
                });
                bs.spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.86, 0.24, 0.28))),
                    MeshMaterial3d(bm),
                    Transform::from_xyz(0.0, *by, 0.08),
                ));
            }
        });

    // Potted Office Fern (right-midground)
    let fern_pot = materials.add(StandardMaterial {
        base_color: Color::srgb(0.82, 0.42, 0.24),
        perceptual_roughness: 0.8,
        ..default()
    });
    let fern_leaf = materials.add(StandardMaterial {
        base_color: Color::srgb(0.20, 0.66, 0.32),
        perceptual_roughness: 0.75,
        ..default()
    });
    commands
        .spawn((
            ScenarioEntity,
            DestructibleProp::tall(
                "OFFICE FERN!",
                240,
                Color::srgb(0.24, 0.68, 0.34),
                DebrisStyle::Ceramic,
            ),
            RigidBody::Dynamic,
            Mass(4.0),
            Restitution::new(0.52),
            Friction::new(0.28),
            LinearDamping(0.09),
            AngularDamping(0.09),
            Collider::cylinder(0.22, 0.85),
            Transform::from_xyz(2.45, 0.44, -2.55),
            Visibility::default(),
        ))
        .with_children(|pl| {
            pl.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.20, 0.38))),
                MeshMaterial3d(fern_pot),
                Transform::from_xyz(0.0, -0.22, 0.0),
            ));
            pl.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.48, 0.42, 0.48))),
                MeshMaterial3d(fern_leaf),
                Transform::from_xyz(0.0, 0.18, 0.0),
            ));
        });
}

pub fn spawn_building_site_scenario(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    textures: &ProceduralTextures,
) {
    // ── Portable Site Trailer Room ─────────────────────────────────────────
    let floor_mat = materials.add(StandardMaterial {
        base_color_texture: Some(textures.wood.clone()),
        uv_transform: Affine2::from_scale(Vec2::splat(6.0)),
        perceptual_roughness: 0.85,
        ..default()
    });
    let wall_mat = materials.add(StandardMaterial {
        base_color_texture: Some(textures.wall_grid.clone()),
        uv_transform: Affine2::from_scale(Vec2::new(6.0, 2.5)),
        base_color: Color::srgb(0.75, 0.80, 0.78),
        perceptual_roughness: 0.78,
        ..default()
    });
    let hazard_trim_mat = materials.add(StandardMaterial {
        base_color_texture: Some(textures.hazard_stripe.clone()),
        uv_transform: Affine2::from_scale(Vec2::new(24.0, 1.0)),
        perceptual_roughness: 0.6,
        ..default()
    });
    let window_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.58, 0.84, 0.98),
        emissive: LinearRgba::rgb(0.35, 0.62, 0.85),
        ..default()
    });

    spawn_room_bounds(
        commands,
        meshes,
        materials,
        textures,
        floor_mat,
        wall_mat,
        hazard_trim_mat.clone(),
        window_mat,
    );

    // ── Folding Site Table (lighter, flies further!) ───────────────────────
    let table_top_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.48, 0.52, 0.56),
        perceptual_roughness: 0.55,
        ..default()
    });
    let steel_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.22, 0.24, 0.28),
        perceptual_roughness: 0.45,
        ..default()
    });

    commands
        .spawn((
            ScenarioEntity,
            PlayerDesk,
            RigidBody::Dynamic,
            Mass(17.0),
            Friction::new(0.28),
            Restitution::new(0.45),
            LinearDamping(0.08),
            AngularDamping(0.08),
            Collider::compound(vec![
                (
                    Vec3::new(0.0, 0.34, 0.0),
                    Quat::IDENTITY,
                    Collider::cuboid(2.0, 0.06, 0.95),
                ),
                (
                    Vec3::new(-0.88, -0.02, -0.38),
                    Quat::IDENTITY,
                    Collider::cuboid(0.07, 0.68, 0.07),
                ),
                (
                    Vec3::new(0.88, -0.02, -0.38),
                    Quat::IDENTITY,
                    Collider::cuboid(0.07, 0.68, 0.07),
                ),
                (
                    Vec3::new(-0.88, -0.02, 0.38),
                    Quat::IDENTITY,
                    Collider::cuboid(0.07, 0.68, 0.07),
                ),
                (
                    Vec3::new(0.88, -0.02, 0.38),
                    Quat::IDENTITY,
                    Collider::cuboid(0.07, 0.68, 0.07),
                ),
            ]),
            Transform::from_xyz(0.0, 0.38, 0.85),
            Visibility::default(),
        ))
        .with_children(|desk| {
            desk.spawn((
                Mesh3d(meshes.add(Cuboid::new(2.0, 0.06, 0.95))),
                MeshMaterial3d(table_top_mat),
                Transform::from_xyz(0.0, 0.34, 0.0),
            ));
            let leg_mesh = meshes.add(Cuboid::new(0.07, 0.68, 0.07));
            for (lx, lz) in [(-0.88, -0.38), (0.88, -0.38), (-0.88, 0.38), (0.88, 0.38)] {
                desk.spawn((
                    Mesh3d(leg_mesh.clone()),
                    MeshMaterial3d(steel_mat.clone()),
                    Transform::from_xyz(lx, -0.02, lz),
                ));
            }
        });

    // ── Site Desk Props ────────────────────────────────────────────────────
    // Yellow Hard Hat
    let hardhat_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.98, 0.78, 0.12),
        perceptual_roughness: 0.35,
        ..default()
    });
    commands
        .spawn((
            ScenarioEntity,
            DestructibleProp::small(
                "OSHA VIOLATION!",
                240,
                Color::srgb(0.98, 0.78, 0.14),
                DebrisStyle::Generic,
            ),
            RigidBody::Dynamic,
            Mass(0.7),
            Restitution::new(0.62),
            Friction::new(0.26),
            LinearDamping(0.09),
            AngularDamping(0.09),
            Collider::cylinder(0.14, 0.12),
            Mesh3d(meshes.add(Cylinder::new(0.14, 0.11))),
            MeshMaterial3d(hardhat_mat),
            Transform::from_xyz(-0.52, 0.82, 0.82),
        ))
        .with_children(|hat| {
            hat.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.17, 0.025))),
                MeshMaterial3d(hazard_trim_mat.clone()),
                Transform::from_xyz(0.0, -0.045, 0.0),
            ));
        });

    // Rolled Blueprints
    let blueprint_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.20, 0.52, 0.90),
        perceptual_roughness: 0.8,
        ..default()
    });
    for (bx, bz) in [(0.15, 0.84), (0.28, 0.94)] {
        let mut bp = DestructibleProp::small(
            "BLUEPRINTS RUINED!",
            160,
            Color::srgb(0.32, 0.66, 0.96),
            DebrisStyle::Papers,
        );
        bp.break_impulse = 1.6;
        bp.debris_count = 9;
        commands.spawn((
            ScenarioEntity,
            bp,
            RigidBody::Dynamic,
            Mass(0.6),
            Restitution::new(0.50),
            Friction::new(0.28),
            LinearDamping(0.09),
            AngularDamping(0.09),
            Collider::cylinder(0.055, 0.52),
            Mesh3d(meshes.add(Cylinder::new(0.055, 0.52))),
            MeshMaterial3d(blueprint_mat.clone()),
            Transform::from_xyz(bx, 0.82, bz)
                .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
        ));
    }

    // Dented Metal Thermos
    let thermos_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.78, 0.22, 0.18),
        perceptual_roughness: 0.4,
        ..default()
    });
    commands
        .spawn((
            ScenarioEntity,
            DestructibleProp::small(
                "HOT THERMOS!",
                180,
                Color::srgb(0.82, 0.26, 0.20),
                DebrisStyle::Generic,
            ),
            RigidBody::Dynamic,
            Mass(0.9),
            Restitution::new(0.58),
            Friction::new(0.28),
            LinearDamping(0.09),
            AngularDamping(0.09),
            Collider::cylinder(0.06, 0.24),
            Mesh3d(meshes.add(Cylinder::new(0.06, 0.24))),
            MeshMaterial3d(thermos_mat),
            Transform::from_xyz(0.64, 0.88, 0.95),
        ))
        .with_children(|th| {
            th.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.052, 0.05))),
                MeshMaterial3d(steel_mat.clone()),
                Transform::from_xyz(0.0, 0.14, 0.0),
            ));
        });

    // Walkie-Talkie
    let radio_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.18, 0.20, 0.24),
        perceptual_roughness: 0.6,
        ..default()
    });
    commands
        .spawn((
            ScenarioEntity,
            DestructibleProp::small(
                "WALKIE-TALKIE!",
                200,
                Color::srgb(0.95, 0.72, 0.18),
                DebrisStyle::Electronics,
            ),
            RigidBody::Dynamic,
            Mass(0.65),
            Restitution::new(0.56),
            Friction::new(0.28),
            LinearDamping(0.09),
            AngularDamping(0.09),
            Collider::cuboid(0.08, 0.18, 0.06),
            Mesh3d(meshes.add(Cuboid::new(0.08, 0.18, 0.06))),
            MeshMaterial3d(radio_mat),
            Transform::from_xyz(-0.18, 0.85, 1.04),
        ))
        .with_children(|rad| {
            rad.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.01, 0.12))),
                MeshMaterial3d(steel_mat.clone()),
                Transform::from_xyz(0.025, 0.14, 0.0),
            ));
        });

    // ── Room Targets ───────────────────────────────────────────────────────
    // Whiteboard on wheels (with doodle texture!)
    let board_mat = materials.add(StandardMaterial {
        base_color_texture: Some(textures.whiteboard_doodle.clone()),
        perceptual_roughness: 0.4,
        ..default()
    });
    commands
        .spawn((
            ScenarioEntity,
            DestructibleProp::tall(
                "SITE WHITEBOARD!",
                420,
                Color::srgb(0.94, 0.96, 0.98),
                DebrisStyle::Generic,
            ),
            RigidBody::Dynamic,
            Mass(11.0),
            Restitution::new(0.50),
            Friction::new(0.28),
            LinearDamping(0.09),
            AngularDamping(0.09),
            Collider::cuboid(1.5, 1.45, 0.12),
            Mesh3d(meshes.add(Cuboid::new(1.5, 1.15, 0.08))),
            MeshMaterial3d(board_mat),
            Transform::from_xyz(-1.35, 0.95, -2.0),
        ))
        .with_children(|wb| {
            wb.spawn((
                Mesh3d(meshes.add(Cuboid::new(1.56, 0.08, 0.12))),
                MeshMaterial3d(steel_mat.clone()),
                Transform::from_xyz(0.0, -0.60, 0.0),
            ));
        });

    // Mini-Fridge
    let fridge_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.86, 0.88, 0.90),
        perceptual_roughness: 0.45,
        ..default()
    });
    commands.spawn((
        ScenarioEntity,
        DestructibleProp::tall(
            "LUNCH RUINED!",
            500,
            Color::srgb(0.85, 0.88, 0.92),
            DebrisStyle::Generic,
        ),
        RigidBody::Dynamic,
        Mass(16.0),
        Restitution::new(0.45),
        Friction::new(0.32),
        LinearDamping(0.09),
        AngularDamping(0.09),
        Collider::cuboid(0.65, 0.95, 0.65),
        Mesh3d(meshes.add(Cuboid::new(0.65, 0.95, 0.65))),
        MeshMaterial3d(fridge_mat),
        Transform::from_xyz(1.45, 0.48, -2.4),
    ));

    // Orange Traffic Cones
    let cone_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.98, 0.42, 0.10),
        perceptual_roughness: 0.5,
        ..default()
    });
    for (cx, cz) in [(-0.4, -1.6), (0.55, -2.8)] {
        commands.spawn((
            ScenarioEntity,
            DestructibleProp::small(
                "SAFETY CONE!",
                160,
                Color::srgb(0.98, 0.45, 0.12),
                DebrisStyle::Generic,
            ),
            RigidBody::Dynamic,
            Mass(1.5),
            Restitution::new(0.68),
            Friction::new(0.24),
            LinearDamping(0.08),
            AngularDamping(0.08),
            Collider::cone(0.18, 0.55),
            Mesh3d(meshes.add(Cone::new(0.18, 0.55))),
            MeshMaterial3d(cone_mat.clone()),
            Transform::from_xyz(cx, 0.28, cz),
        ));
    }

    // Heavy Toolboxes & Paint Cans in the back
    let toolbox_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.84, 0.16, 0.16),
        perceptual_roughness: 0.45,
        ..default()
    });
    for (tx, tz) in [(-0.95, -3.7), (0.85, -3.9)] {
        commands.spawn((
            ScenarioEntity,
            DestructibleProp::tall(
                "TOOLBOX SMASH!",
                450,
                Color::srgb(0.86, 0.20, 0.20),
                DebrisStyle::Generic,
            ),
            RigidBody::Dynamic,
            Mass(10.0),
            Restitution::new(0.50),
            Friction::new(0.28),
            LinearDamping(0.09),
            AngularDamping(0.09),
            Collider::cuboid(0.68, 0.42, 0.36),
            Mesh3d(meshes.add(Cuboid::new(0.68, 0.42, 0.36))),
            MeshMaterial3d(toolbox_mat.clone()),
            Transform::from_xyz(tx, 0.22, tz),
        ));
    }

    // Paint Cans & Step Ladder
    let paint_colors = [
        Color::srgb(0.22, 0.62, 0.96),
        Color::srgb(0.96, 0.32, 0.22),
        Color::srgb(0.98, 0.80, 0.18),
    ];
    for (idx, (px, pz)) in [(-2.0, -2.5), (-1.7, -2.65), (2.1, -3.1)].iter().enumerate() {
        let pm = materials.add(StandardMaterial {
            base_color: paint_colors[idx % paint_colors.len()],
            perceptual_roughness: 0.4,
            ..default()
        });
        commands.spawn((
            ScenarioEntity,
            DestructibleProp::small(
                "PAINT SPLATTER!",
                190,
                paint_colors[idx % paint_colors.len()],
                DebrisStyle::Water,
            ),
            RigidBody::Dynamic,
            Mass(2.2),
            Restitution::new(0.58),
            Friction::new(0.26),
            LinearDamping(0.08),
            AngularDamping(0.08),
            Collider::cylinder(0.14, 0.28),
            Mesh3d(meshes.add(Cylinder::new(0.14, 0.28))),
            MeshMaterial3d(pm),
            Transform::from_xyz(*px, 0.15, *pz),
        ));
    }

    // Aluminum Step Ladder
    commands.spawn((
        ScenarioEntity,
        DestructibleProp::tall(
            "STEP LADDER!",
            340,
            Color::srgb(0.75, 0.78, 0.82),
            DebrisStyle::Generic,
        ),
        RigidBody::Dynamic,
        Mass(7.0),
        Restitution::new(0.52),
        Friction::new(0.26),
        LinearDamping(0.08),
        AngularDamping(0.08),
        Collider::cuboid(0.55, 1.35, 0.55),
        Mesh3d(meshes.add(Cuboid::new(0.55, 1.35, 0.14))),
        MeshMaterial3d(steel_mat),
        Transform::from_xyz(2.35, 0.68, -4.15).with_rotation(Quat::from_rotation_y(-0.4)),
    ));
}
