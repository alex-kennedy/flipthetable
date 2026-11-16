use avian3d::prelude::*;
use bevy::prelude::*;

#[derive(Component)]
pub struct ScenarioEntity;

#[derive(Component)]
pub struct PlayerDesk;

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
    pub debris_count: u8,
    pub prev_velocity: Vec3,
}

impl DestructibleProp {
    pub fn small(label: &'static str, base_points: u32, debris_color: Color) -> Self {
        Self {
            label,
            base_points,
            can_topple: false,
            toppled: false,
            damaged: false,
            shattered: false,
            damage_impulse: 2.2,
            break_impulse: 4.5,
            debris_color,
            debris_count: 5,
            prev_velocity: Vec3::ZERO,
        }
    }

    pub fn tall(label: &'static str, base_points: u32, debris_color: Color) -> Self {
        Self {
            label,
            base_points,
            can_topple: true,
            toppled: false,
            damaged: false,
            shattered: false,
            damage_impulse: 3.5,
            break_impulse: 7.5,
            debris_color,
            debris_count: 7,
            prev_velocity: Vec3::ZERO,
        }
    }
}

pub fn spawn_office_scenario(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    // ── Room Static Geometry ───────────────────────────────────────────────
    let floor_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.20, 0.23, 0.28),
        perceptual_roughness: 0.92,
        ..default()
    });
    let wall_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.78, 0.76, 0.70),
        perceptual_roughness: 0.9,
        ..default()
    });
    let trim_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.25, 0.29, 0.35),
        perceptual_roughness: 0.85,
        ..default()
    });

    // Floor
    commands.spawn((
        ScenarioEntity,
        RigidBody::Static,
        Collider::cuboid(12.0, 0.4, 12.0),
        Mesh3d(meshes.add(Cuboid::new(12.0, 0.4, 12.0))),
        MeshMaterial3d(floor_mat),
        Transform::from_xyz(0.0, -0.2, -1.0),
    ));

    // Back wall
    commands.spawn((
        ScenarioEntity,
        RigidBody::Static,
        Collider::cuboid(12.0, 4.5, 0.4),
        Mesh3d(meshes.add(Cuboid::new(12.0, 4.5, 0.4))),
        MeshMaterial3d(wall_mat.clone()),
        Transform::from_xyz(0.0, 2.25, -6.5),
    ));

    // Left & Right walls
    for x in [-5.5_f32, 5.5_f32] {
        commands.spawn((
            ScenarioEntity,
            RigidBody::Static,
            Collider::cuboid(0.4, 4.5, 12.0),
            Mesh3d(meshes.add(Cuboid::new(0.4, 4.5, 12.0))),
            MeshMaterial3d(wall_mat.clone()),
            Transform::from_xyz(x, 2.25, -1.0),
        ));
    }

    // Back window frame accent (warm sunlit blinds)
    let window_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.98, 0.85, 0.48),
        emissive: LinearRgba::rgb(0.65, 0.50, 0.20),
        ..default()
    });
    commands.spawn((
        ScenarioEntity,
        Mesh3d(meshes.add(Cuboid::new(3.4, 1.8, 0.08))),
        MeshMaterial3d(window_mat),
        Transform::from_xyz(0.0, 2.3, -6.26),
    ));
    commands.spawn((
        ScenarioEntity,
        Mesh3d(meshes.add(Cuboid::new(3.6, 0.12, 0.12))),
        MeshMaterial3d(trim_mat),
        Transform::from_xyz(0.0, 3.25, -6.24),
    ));

    // ── Player's Oak Desk ──────────────────────────────────────────────────
    let oak_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.62, 0.38, 0.19),
        perceptual_roughness: 0.72,
        ..default()
    });
    let leg_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.24, 0.26, 0.30),
        perceptual_roughness: 0.6,
        ..default()
    });

    // Desk top at y = 0.76, thickness 0.08; 4 legs underneath
    commands
        .spawn((
            ScenarioEntity,
            PlayerDesk,
            RigidBody::Dynamic,
            Mass(24.0),
            Friction::new(0.45),
            Restitution::new(0.15),
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
            // Tabletop mesh
            desk.spawn((
                Mesh3d(meshes.add(Cuboid::new(2.1, 0.08, 1.0))),
                MeshMaterial3d(oak_mat.clone()),
                Transform::from_xyz(0.0, 0.34, 0.0),
            ));
            // 4 legs
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
    // CRT Monitor
    let crt_case = materials.add(StandardMaterial {
        base_color: Color::srgb(0.86, 0.82, 0.72),
        perceptual_roughness: 0.7,
        ..default()
    });
    let crt_screen = materials.add(StandardMaterial {
        base_color: Color::srgb(0.10, 0.48, 0.46),
        emissive: LinearRgba::rgb(0.08, 0.42, 0.40),
        ..default()
    });
    commands
        .spawn((
            ScenarioEntity,
            DestructibleProp::small("CRT MONITOR", 300, Color::srgb(0.20, 0.75, 0.72)),
            RigidBody::Dynamic,
            Mass(4.5),
            Collider::cuboid(0.46, 0.42, 0.40),
            Mesh3d(meshes.add(Cuboid::new(0.46, 0.42, 0.40))),
            MeshMaterial3d(crt_case.clone()),
            Transform::from_xyz(-0.32, 0.98, 0.72),
        ))
        .with_children(|crt| {
            crt.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.36, 0.28, 0.03))),
                MeshMaterial3d(crt_screen.clone()),
                Transform::from_xyz(0.0, 0.02, 0.20),
            ));
        });

    // Mechanical Keyboard
    commands.spawn((
        ScenarioEntity,
        DestructibleProp::small("KEYBOARD", 120, Color::srgb(0.85, 0.82, 0.74)),
        RigidBody::Dynamic,
        Mass(1.1),
        Collider::cuboid(0.48, 0.05, 0.18),
        Mesh3d(meshes.add(Cuboid::new(0.48, 0.05, 0.18))),
        MeshMaterial3d(crt_case.clone()),
        Transform::from_xyz(0.0, 0.79, 1.08),
    ));

    // Coffee Mug
    let mug_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.96, 0.94, 0.90),
        perceptual_roughness: 0.4,
        ..default()
    });
    let mut mug_prop = DestructibleProp::small("COFFEE MUG", 150, Color::srgb(0.95, 0.93, 0.88));
    mug_prop.break_impulse = 2.8; // Extra fragile!
    commands.spawn((
        ScenarioEntity,
        mug_prop,
        RigidBody::Dynamic,
        Mass(0.6),
        Collider::cylinder(0.065, 0.13),
        Mesh3d(meshes.add(Cylinder::new(0.065, 0.13))),
        MeshMaterial3d(mug_mat),
        Transform::from_xyz(0.28, 0.83, 0.92),
    ));

    // Red Stapler
    let stapler_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.88, 0.20, 0.20),
        perceptual_roughness: 0.45,
        ..default()
    });
    commands.spawn((
        ScenarioEntity,
        DestructibleProp::small("RED STAPLER!", 200, Color::srgb(0.90, 0.22, 0.22)),
        RigidBody::Dynamic,
        Mass(0.7),
        Collider::cuboid(0.16, 0.06, 0.07),
        Mesh3d(meshes.add(Cuboid::new(0.16, 0.06, 0.07))),
        MeshMaterial3d(stapler_mat),
        Transform::from_xyz(-0.68, 0.80, 1.02),
    ));

    // TPS Report Paper Stacks
    let paper_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.95, 0.92),
        perceptual_roughness: 0.9,
        ..default()
    });
    for (px, pz, h) in [(0.58, 0.78, 0.14), (0.74, 0.95, 0.10)] {
        let mut paper_prop =
            DestructibleProp::small("TPS REPORTS", 100, Color::srgb(0.96, 0.96, 0.93));
        paper_prop.break_impulse = 2.5;
        commands.spawn((
            ScenarioEntity,
            paper_prop,
            RigidBody::Dynamic,
            Mass(0.8),
            Collider::cuboid(0.24, h, 0.30),
            Mesh3d(meshes.add(Cuboid::new(0.24, h, 0.30))),
            MeshMaterial3d(paper_mat.clone()),
            Transform::from_xyz(px, 0.76 + h * 0.5, pz),
        ));
    }

    // ── Room Props (Target Zone) ───────────────────────────────────────────
    // Cubicle Partitions
    let partition_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.28, 0.56, 0.54),
        perceptual_roughness: 0.9,
        ..default()
    });
    for (cx, cz, label) in [(-1.85, -1.55, "CUBICLE WALL"), (1.85, -1.75, "CUBICLE WALL")] {
        commands.spawn((
            ScenarioEntity,
            DestructibleProp::tall(label, 350, Color::srgb(0.32, 0.62, 0.58)),
            RigidBody::Dynamic,
            Mass(14.0),
            Collider::cuboid(1.6, 1.55, 0.12),
            Mesh3d(meshes.add(Cuboid::new(1.6, 1.55, 0.12))),
            MeshMaterial3d(partition_mat.clone()),
            Transform::from_xyz(cx, 0.78, cz),
        ));
    }

    // Tall Filing Cabinets
    let cabinet_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.45, 0.49, 0.42),
        perceptual_roughness: 0.65,
        ..default()
    });
    for (fx, fz) in [(-1.25, -3.2), (1.45, -3.5)] {
        commands.spawn((
            ScenarioEntity,
            DestructibleProp::tall("FILING CABINET", 450, Color::srgb(0.48, 0.52, 0.45)),
            RigidBody::Dynamic,
            Mass(18.0),
            Collider::cuboid(0.65, 1.50, 0.65),
            Mesh3d(meshes.add(Cuboid::new(0.65, 1.50, 0.65))),
            MeshMaterial3d(cabinet_mat.clone()),
            Transform::from_xyz(fx, 0.75, fz),
        ));
    }

    // Water Cooler
    let cooler_body_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.88, 0.90, 0.92),
        perceptual_roughness: 0.5,
        ..default()
    });
    let cooler_jug_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.25, 0.68, 0.95),
        perceptual_roughness: 0.25,
        ..default()
    });
    commands
        .spawn((
            ScenarioEntity,
            DestructibleProp::tall("WATER COOLER", 500, Color::srgb(0.35, 0.75, 0.98)),
            RigidBody::Dynamic,
            Mass(12.0),
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

    // Coworker Desk + Monitor in the back
    commands.spawn((
        ScenarioEntity,
        DestructibleProp::tall("COWORKER DESK", 400, Color::srgb(0.62, 0.38, 0.19)),
        RigidBody::Dynamic,
        Mass(20.0),
        Collider::cuboid(1.8, 0.75, 0.85),
        Mesh3d(meshes.add(Cuboid::new(1.8, 0.75, 0.85))),
        MeshMaterial3d(oak_mat),
        Transform::from_xyz(0.15, 0.38, -4.45),
    ));
    commands
        .spawn((
            ScenarioEntity,
            DestructibleProp::small("COWORKER CRT", 350, Color::srgb(0.20, 0.75, 0.72)),
            RigidBody::Dynamic,
            Mass(4.5),
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
}
