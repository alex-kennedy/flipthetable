use bevy::prelude::*;
use bevy::window::{PresentMode, WindowResolution};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn start_game(canvas_selector: &str) {
    console_error_panic_hook::set_once();

    let selector = canvas_selector.to_string();

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Flip the Table".into(),
                canvas: Some(selector),
                resolution: WindowResolution::new(960, 540),
                fit_canvas_to_parent: true,
                prevent_default_event_handling: true,
                present_mode: PresentMode::AutoVsync,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.08, 0.09, 0.12)))
        .add_systems(Startup, (setup_scene, setup_hud, notify_js_ready))
        .add_systems(Update, spin_preview_desk)
        .run();
}

#[derive(Component)]
struct PreviewDesk;

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Simple office floor
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(12.0, 12.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.22, 0.24, 0.28),
            perceptual_roughness: 0.9,
            ..default()
        })),
    ));

    // Placeholder low-poly desk
    commands.spawn((
        PreviewDesk,
        Mesh3d(meshes.add(Cuboid::new(2.0, 0.12, 1.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.62, 0.41, 0.24),
            perceptual_roughness: 0.7,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.85, 0.0),
    ));

    // Key light
    commands.spawn((
        DirectionalLight {
            illuminance: 8_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(3.0, 6.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Seated PoV camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 1.65, 2.4).looking_at(Vec3::new(0.0, 0.9, -1.0), Vec3::Y),
    ));
}

fn setup_hud(mut commands: Commands) {
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            top: Val::Px(16.0),
            left: Val::Px(16.0),
            padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        })
        .insert(BackgroundColor(Color::srgba(0.05, 0.06, 0.09, 0.85)))
        .insert(BorderColor::all(Color::srgb(0.95, 0.75, 0.25)))
        .with_children(|parent| {
            parent.spawn((
                Text::new("FLIP THE TABLE // WASM + BEVY READY"),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(Color::srgb(0.98, 0.94, 0.82)),
            ));
        });
}

fn spin_preview_desk(time: Res<Time>, mut query: Query<&mut Transform, With<PreviewDesk>>) {
    for mut transform in &mut query {
        transform.rotate_y(time.delta_secs() * 0.45);
    }
}

fn notify_js_ready() {
    if let Some(window) = web_sys::window() {
        let init = web_sys::CustomEventInit::new();
        init.set_detail(&JsValue::from_str("ready"));
        if let Ok(event) = web_sys::CustomEvent::new_with_event_init_dict("flipthetable:ready", &init) {
            let _ = window.dispatch_event(&event);
        }
    }
}
