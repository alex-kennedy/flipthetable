use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{ClearColorConfig, RenderTarget};
use bevy::image::ImageSampler;
use bevy::light::{CascadeShadowConfigBuilder, ShadowFilteringMethod};
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, Extent3d, ShaderType, TextureDimension, TextureFormat, TextureUsages,
};
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dPlugin, MeshMaterial2d};
use bevy::window::PrimaryWindow;

use crate::flip::CameraRig;
use crate::prefabs::ScenarioEntity;
use crate::scenario_data::LightingDef;

pub const PIXEL_TARGET_HEIGHT: u32 = 270;

pub struct RetroRenderPlugin;

impl Plugin for RetroRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<RetroPostMaterial>::default())
            .add_systems(Update, sync_pixel_canvas_aspect);
    }
}

#[derive(Component)]
pub struct SceneCamera3d;

#[derive(Component)]
pub struct UpscaleQuad;

#[derive(Resource, Clone)]
pub struct PixelRenderState {
    pub target_image: Handle<Image>,
    pub post_material: Handle<RetroPostMaterial>,
    pub pixel_width: u32,
    pub pixel_height: u32,
}

#[derive(ShaderType, Debug, Clone, Copy)]
pub struct RetroPostUniform {
    pub params: Vec4,
    pub shadow_tint: Vec4,
    pub highlight_tint: Vec4,
}

#[derive(AsBindGroup, Debug, Clone, Asset, TypePath)]
pub struct RetroPostMaterial {
    #[uniform(0)]
    pub uniform: RetroPostUniform,
    #[texture(1)]
    #[sampler(2)]
    pub scene_tex: Handle<Image>,
}

impl Material2d for RetroPostMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/retro_post.wesl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Opaque
    }
}

pub fn make_offscreen_pixel_image(width: u32, height: u32) -> Image {
    let size = Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let mut image = Image::new_fill(
        size,
        TextureDimension::D2,
        &[16, 20, 28, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;
    image.sampler = ImageSampler::nearest();
    image
}

pub fn setup_retro_cameras(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut post_materials: ResMut<Assets<RetroPostMaterial>>,
    rig: Res<CameraRig>,
) {
    let pixel_w = 480_u32;
    let pixel_h = PIXEL_TARGET_HEIGHT;
    let target_image = images.add(make_offscreen_pixel_image(pixel_w, pixel_h));

    let post_material = post_materials.add(RetroPostMaterial {
        uniform: RetroPostUniform {
            params: Vec4::new(pixel_w as f32, pixel_h as f32, 30.0, 0.85),
            shadow_tint: Vec4::new(0.10, 0.14, 0.24, 0.48),
            highlight_tint: Vec4::new(1.06, 0.95, 0.78, 0.42),
        },
        scene_tex: target_image.clone(),
    });

    commands.insert_resource(PixelRenderState {
        target_image: target_image.clone(),
        post_material: post_material.clone(),
        pixel_width: pixel_w,
        pixel_height: pixel_h,
    });

    // 1. Offscreen low-res 3D camera (Layer 0)
    commands.spawn((
        SceneCamera3d,
        Camera3d::default(),
        Camera {
            order: 0,
            clear_color: ClearColorConfig::Custom(Color::srgb(0.07, 0.09, 0.13)),
            ..default()
        },
        RenderTarget::Image(target_image.into()),
        Msaa::Off,
        ShadowFilteringMethod::Hardware2x2,
        Projection::Perspective(PerspectiveProjection {
            fov: 54.0_f32.to_radians(),
            near: 0.12,
            far: 32.0,
            ..default()
        }),
        DistanceFog {
            color: Color::srgba(0.09, 0.12, 0.18, 1.0),
            directional_light_color: Color::srgba(1.0, 0.80, 0.44, 0.28),
            directional_light_exponent: 18.0,
            falloff: FogFalloff::Linear {
                start: 5.5,
                end: 16.5,
            },
        },
        Transform::from_translation(rig.seated_pos).looking_at(rig.seated_look, Vec3::Y),
    ));

    // 2. Native-resolution 2D composite + UI camera (Layer 1)
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        Msaa::Off,
        RenderLayers::layer(1),
        IsDefaultUiCamera,
    ));

    // Full-viewport quad on Layer 1 running `retro_post.wesl`
    commands.spawn((
        UpscaleQuad,
        Mesh2d(meshes.add(Rectangle::new(1.0, 1.0))),
        MeshMaterial2d(post_material),
        Transform::from_scale(Vec3::new(960.0, 540.0, 1.0)),
        RenderLayers::layer(1),
    ));
}

/// Keeps pixels strictly 1:1 square across any desktop/ultrawide/mobile viewport aspect ratio!
fn sync_pixel_canvas_aspect(
    win_q: Query<&Window, With<PrimaryWindow>>,
    mut state: ResMut<PixelRenderState>,
    mut images: ResMut<Assets<Image>>,
    mut post_mats: ResMut<Assets<RetroPostMaterial>>,
    mut quad_q: Query<&mut Transform, With<UpscaleQuad>>,
) {
    let Ok(win) = win_q.single() else {
        return;
    };
    let w = win.width().max(64.0);
    let h = win.height().max(64.0);

    let desired_scale = Vec3::new(w, h, 1.0);
    for mut tf in &mut quad_q {
        if tf.scale != desired_scale {
            tf.scale = desired_scale;
        }
    }

    let aspect = w / h;
    let desired_pw = ((PIXEL_TARGET_HEIGHT as f32 * aspect).round() as u32).clamp(200, 680);
    if desired_pw != state.pixel_width {
        state.pixel_width = desired_pw;
        if let Some(mut img) = images.get_mut(&state.target_image) {
            img.resize(Extent3d {
                width: desired_pw,
                height: PIXEL_TARGET_HEIGHT,
                depth_or_array_layers: 1,
            });
        }
        if let Some(mut mat) = post_mats.get_mut(&state.post_material) {
            mat.uniform.params.x = desired_pw as f32;
            mat.uniform.params.y = PIXEL_TARGET_HEIGHT as f32;
        }
    }
}

/// Applies scenario-specific lighting, shadow cascades, atmospheric fog, and split-tone post shader uniforms.
pub fn apply_scenario_lighting(
    lighting: &LightingDef,
    commands: &mut Commands,
    pixel_state: &PixelRenderState,
    post_mats: &mut Assets<RetroPostMaterial>,
    cam_q: &mut Query<(&mut Camera, &mut DistanceFog), With<SceneCamera3d>>,
) {
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(
            lighting.ambient_color[0],
            lighting.ambient_color[1],
            lighting.ambient_color[2],
        ),
        brightness: lighting.ambient_brightness,
        ..default()
    });

    if let Ok((mut cam, mut fog)) = cam_q.single_mut() {
        cam.clear_color = ClearColorConfig::Custom(Color::srgb(
            lighting.clear_color[0],
            lighting.clear_color[1],
            lighting.clear_color[2],
        ));
        fog.color = Color::srgba(
            lighting.fog_color[0],
            lighting.fog_color[1],
            lighting.fog_color[2],
            lighting.fog_color[3],
        );
        fog.directional_light_color = Color::srgba(
            lighting.sun_color[0],
            lighting.sun_color[1],
            lighting.sun_color[2],
            0.26,
        );
        fog.falloff = FogFalloff::Linear {
            start: lighting.fog_start,
            end: lighting.fog_end,
        };
    }

    if let Some(mut mat) = post_mats.get_mut(&pixel_state.post_material) {
        mat.uniform.shadow_tint = Vec4::new(
            lighting.shadow_tint[0],
            lighting.shadow_tint[1],
            lighting.shadow_tint[2],
            lighting.bloom_strength,
        );
        mat.uniform.highlight_tint = Vec4::new(
            lighting.highlight_tint[0],
            lighting.highlight_tint[1],
            lighting.highlight_tint[2],
            0.42,
        );
    }

    // Tight 2-cascade shadow map focused right over the desk & room so small desk props cast crisp pixel shadows!
    let cascades = CascadeShadowConfigBuilder {
        num_cascades: 2,
        minimum_distance: 0.15,
        first_cascade_far_bound: 3.8,
        maximum_distance: 13.5,
        overlap_proportion: 0.15,
    }
    .build();

    commands.spawn((
        ScenarioEntity,
        DirectionalLight {
            illuminance: lighting.sun_illuminance,
            color: Color::srgb(
                lighting.sun_color[0],
                lighting.sun_color[1],
                lighting.sun_color[2],
            ),
            shadow_maps_enabled: true,
            ..default()
        },
        cascades,
        Transform::from_xyz(lighting.sun_pos[0], lighting.sun_pos[1], lighting.sun_pos[2])
            .looking_at(
                Vec3::new(lighting.sun_look[0], lighting.sun_look[1], lighting.sun_look[2]),
                Vec3::Y,
            ),
    ));

    // Room bounce / cool fill light
    commands.spawn((
        ScenarioEntity,
        PointLight {
            color: Color::srgb(
                lighting.fill_color[0],
                lighting.fill_color[1],
                lighting.fill_color[2],
            ),
            intensity: lighting.fill_intensity,
            range: 14.0,
            ..default()
        },
        Transform::from_xyz(lighting.fill_pos[0], lighting.fill_pos[1], lighting.fill_pos[2]),
    ));

    // Diegetic local accent light (e.g. cyan phosphor glow casting onto keyboard & walnut desk!)
    commands.spawn((
        ScenarioEntity,
        PointLight {
            color: Color::srgb(
                lighting.accent_color[0],
                lighting.accent_color[1],
                lighting.accent_color[2],
            ),
            intensity: lighting.accent_intensity,
            range: 3.4,
            ..default()
        },
        Transform::from_xyz(
            lighting.accent_pos[0],
            lighting.accent_pos[1],
            lighting.accent_pos[2],
        ),
    ));

    // Warm overhead desk key light (no 6-face cubemap shadow pass -> 5x faster GPU frame time!)
    commands.spawn((
        ScenarioEntity,
        PointLight {
            color: Color::srgb(1.0, 0.91, 0.76),
            intensity: 68_000.0,
            range: 5.6,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_xyz(-0.75, 2.35, 1.65),
    ));
}
