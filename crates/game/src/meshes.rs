use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::Face;

/// Shared dark ink outline material (inverted hull `CullMode::Front`, unlit) for crisp 1px pixel-art silhouettes.
#[derive(Resource, Clone)]
pub struct SharedMeshes {
    pub unit_cube: Handle<Mesh>,
    pub shard_wedge: Handle<Mesh>,
    pub shard_plate: Handle<Mesh>,
    pub paper_sheet: Handle<Mesh>,
    pub floor_quad: Handle<Mesh>,
    pub ink_outline_mat: Handle<StandardMaterial>,
}

pub fn create_shared_meshes(
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> SharedMeshes {
    let ink_outline_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.055, 0.070, 0.105),
        unlit: true,
        cull_mode: Some(Face::Front),
        ..default()
    });

    SharedMeshes {
        unit_cube: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        shard_wedge: meshes.add(make_shard_wedge_mesh(Vec3::new(0.16, 0.05, 0.14))),
        shard_plate: meshes.add(make_shard_wedge_mesh(Vec3::new(0.24, 0.025, 0.19))),
        paper_sheet: meshes.add(make_paper_sheet_mesh(0.25, 0.32)),
        floor_quad: meshes.add(make_floor_quad_mesh(1.0, 1.0)),
        ink_outline_mat,
    }
}

/// Computes uniform expansion scale for an inverted-hull outline around a box of `size`.
pub fn outline_scale_for(size: Vec3, thickness: f32) -> Vec3 {
    let t2 = thickness * 2.0;
    Vec3::new(
        (size.x + t2) / size.x.max(0.02),
        (size.y + t2) / size.y.max(0.02),
        (size.z + t2) / size.z.max(0.02),
    )
}

/// Builds a vertical (+Y) tapered rectangular frustum (great for pots, bins, pedestals, lampshades, keycaps).
pub fn make_frustum_y_mesh(bottom_xz: Vec2, top_xz: Vec2, height: f32) -> Mesh {
    let hy = height * 0.5;
    let bx = bottom_xz.x * 0.5;
    let bz = bottom_xz.y * 0.5;
    let tx = top_xz.x * 0.5;
    let tz = top_xz.y * 0.5;

    let b0 = Vec3::new(-bx, -hy, bz);
    let b1 = Vec3::new(bx, -hy, bz);
    let b2 = Vec3::new(bx, -hy, -bz);
    let b3 = Vec3::new(-bx, -hy, -bz);

    let t0 = Vec3::new(-tx, hy, tz);
    let t1 = Vec3::new(tx, hy, tz);
    let t2 = Vec3::new(tx, hy, -tz);
    let t3 = Vec3::new(-tx, hy, -tz);

    build_six_quad_solid([b0, b1, b2, b3], [t0, t1, t2, t3])
}

/// Builds a front-to-back (-Z) tapered frustum (ideal for bulky 90s CRT rear housings!).
pub fn make_frustum_z_mesh(front_wh: Vec2, back_wh: Vec2, depth: f32) -> Mesh {
    let hz = depth * 0.5;
    let fx = front_wh.x * 0.5;
    let fy = front_wh.y * 0.5;
    let rx = back_wh.x * 0.5;
    let ry = back_wh.y * 0.5;

    let b0 = Vec3::new(-fx, -fy, hz);
    let b1 = Vec3::new(fx, -fy, hz);
    let b2 = Vec3::new(rx, -ry, -hz);
    let b3 = Vec3::new(-rx, -ry, -hz);

    let t0 = Vec3::new(-fx, fy, hz);
    let t1 = Vec3::new(fx, fy, hz);
    let t2 = Vec3::new(rx, ry, -hz);
    let t3 = Vec3::new(-rx, ry, -hz);

    build_six_quad_solid([b0, b1, b2, b3], [t0, t1, t2, t3])
}

fn build_six_quad_solid(b: [Vec3; 4], t: [Vec3; 4]) -> Mesh {
    let mut positions = Vec::with_capacity(24);
    let mut normals = Vec::with_capacity(24);
    let mut uvs = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);

    // Each quad is (bottom_left, bottom_right, top_right, top_left) viewed from outside -> CCW winding.
    // UVs have v=0 at top, v=1 at bottom so pixel-art textures are right-side-up!
    let faces = [
        (b[0], b[1], t[1], t[0]), // Front (+Z)
        (b[2], b[3], t[3], t[2]), // Back (-Z)
        (b[1], b[2], t[2], t[1]), // Right (+X)
        (b[3], b[0], t[0], t[3]), // Left (-X)
        (t[0], t[1], t[2], t[3]), // Top (+Y)
        (b[3], b[2], b[1], b[0]), // Bottom (-Y)
    ];

    for (p0, p1, p2, p3) in faces {
        let base = positions.len() as u32;
        let n = (p1 - p0).cross(p3 - p0).normalize_or_zero();
        positions.extend_from_slice(&[p0.to_array(), p1.to_array(), p2.to_array(), p3.to_array()]);
        normals.extend_from_slice(&[n.to_array(); 4]);
        uvs.extend_from_slice(&[[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]);
        indices.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 3, base]);
    }

    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))
}

/// Upright +Z quad with top-left UV = (0, 0) so pixel art (CRT screens, posters, whiteboards, memos) is right-side up.
pub fn make_upright_quad_mesh(width: f32, height: f32) -> Mesh {
    let hw = width * 0.5;
    let hh = height * 0.5;
    let positions = vec![
        [-hw, -hh, 0.0],
        [hw, -hh, 0.0],
        [hw, hh, 0.0],
        [-hw, hh, 0.0],
    ];
    let normals = vec![[0.0, 0.0, 1.0]; 4];
    let uvs = vec![[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
    let indices = Indices::U32(vec![0, 1, 2, 2, 3, 0]);

    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(indices)
}

/// Horizontal +Y quad for floor decals (spills, paint splatters, scorch marks, sunbeam floor strips).
pub fn make_floor_quad_mesh(width: f32, depth: f32) -> Mesh {
    let hw = width * 0.5;
    let hd = depth * 0.5;
    let positions = vec![
        [-hw, 0.0, hd],
        [hw, 0.0, hd],
        [hw, 0.0, -hd],
        [-hw, 0.0, -hd],
    ];
    let normals = vec![[0.0, 1.0, 0.0]; 4];
    let uvs = vec![[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
    let indices = Indices::U32(vec![0, 1, 2, 2, 3, 0]);

    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(indices)
}

/// Double-sided ultra-thin slightly bowed sheet mesh for fluttering TPS reports & blueprints.
pub fn make_paper_sheet_mesh(width: f32, length: f32) -> Mesh {
    let hw = width * 0.5;
    let hl = length * 0.5;
    let bow = 0.016;
    let positions = vec![
        // Top side
        [-hw, 0.0, hl],
        [0.0, bow, hl],
        [hw, 0.0, hl],
        [-hw, 0.0, -hl],
        [0.0, bow, -hl],
        [hw, 0.0, -hl],
        // Bottom side (reversed normals)
        [-hw, -0.003, hl],
        [0.0, bow - 0.003, hl],
        [hw, -0.003, hl],
        [-hw, -0.003, -hl],
        [0.0, bow - 0.003, -hl],
        [hw, -0.003, -hl],
    ];
    let normals = vec![
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
    ];
    let uvs = vec![
        [0.0, 1.0],
        [0.5, 1.0],
        [1.0, 1.0],
        [0.0, 0.0],
        [0.5, 0.0],
        [1.0, 0.0],
        [0.0, 1.0],
        [0.5, 1.0],
        [1.0, 1.0],
        [0.0, 0.0],
        [0.5, 0.0],
        [1.0, 0.0],
    ];
    let indices = Indices::U32(vec![
        0, 1, 4, 4, 3, 0, // left half top
        1, 2, 5, 5, 4, 1, // right half top
        6, 9, 10, 10, 7, 6, // left half bottom
        7, 10, 11, 11, 8, 7, // right half bottom
    ]);

    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(indices)
}

/// Asymmetric angular triangular wedge prism for jagged glass, ceramic, PCB, and wood splinters.
pub fn make_shard_wedge_mesh(size: Vec3) -> Mesh {
    let hx = size.x * 0.5;
    let hy = size.y * 0.5;
    let hz = size.z * 0.5;

    let b0 = Vec3::new(-hx, -hy, hz * 0.9);
    let b1 = Vec3::new(hx, -hy, hz * 0.25);
    let b2 = Vec3::new(-hx * 0.25, -hy, -hz);

    let t0 = Vec3::new(-hx * 0.85, hy, hz * 0.8);
    let t1 = Vec3::new(hx * 0.9, hy, hz * 0.2);
    let t2 = Vec3::new(-hx * 0.2, hy, -hz * 0.9);

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    // Top & bottom triangles
    let tris = [(t0, t1, t2), (b2, b1, b0)];
    for (p0, p1, p2) in tris {
        let base = positions.len() as u32;
        let n = (p1 - p0).cross(p2 - p0).normalize_or_zero();
        positions.extend_from_slice(&[p0.to_array(), p1.to_array(), p2.to_array()]);
        normals.extend_from_slice(&[n.to_array(); 3]);
        uvs.extend_from_slice(&[[0.0, 0.0], [1.0, 0.0], [0.5, 1.0]]);
        indices.extend_from_slice(&[base, base + 1, base + 2]);
    }

    // 3 side quads
    let quads = [(b0, b1, t1, t0), (b1, b2, t2, t1), (b2, b0, t0, t2)];
    for (p0, p1, p2, p3) in quads {
        let base = positions.len() as u32;
        let n = (p1 - p0).cross(p3 - p0).normalize_or_zero();
        positions.extend_from_slice(&[p0.to_array(), p1.to_array(), p2.to_array(), p3.to_array()]);
        normals.extend_from_slice(&[n.to_array(); 4]);
        uvs.extend_from_slice(&[[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]);
        indices.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 3, base]);
    }

    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))
}
