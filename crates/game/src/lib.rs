mod destruction;
mod flip;
mod hud;
mod scene;

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::window::{PresentMode, WindowResolution};
use wasm_bindgen::prelude::*;

use destruction::{
    check_run_settled, process_destruction, update_debris_lifetime, PendingPopups, ScoreState,
};
use flip::{handle_flip_gesture, update_follow_camera, CameraRig, FlipTracker};
use hud::{
    animate_score_popups, setup_hud, spawn_score_popups, update_hud, FloatingPopup, HudState,
    RetryButton,
};
use scene::{spawn_office_scenario, ScenarioEntity};

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GamePhase {
    #[default]
    Ready,
    Flipping,
    Settled,
}

#[wasm_bindgen]
pub fn start_game(canvas_selector: &str) {
    console_error_panic_hook::set_once();

    let selector = canvas_selector.to_string();

    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
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
            }),
            PhysicsPlugins::default(),
        ))
        .init_state::<GamePhase>()
        .insert_resource(ClearColor(Color::srgb(0.08, 0.09, 0.12)))
        .init_resource::<FlipTracker>()
        .init_resource::<CameraRig>()
        .init_resource::<ScoreState>()
        .init_resource::<PendingPopups>()
        .init_resource::<HudState>()
        .add_systems(Startup, (setup_camera_and_lights, setup_initial_scene, setup_hud, notify_js_ready))
        .add_systems(
            Update,
            (
                handle_flip_gesture.run_if(in_state(GamePhase::Ready)),
                (process_destruction, check_run_settled).run_if(in_state(GamePhase::Flipping)),
                update_debris_lifetime,
                update_follow_camera,
                spawn_score_popups,
                animate_score_popups,
                update_hud,
                handle_retry_input,
            ),
        )
        .run();
}

fn setup_camera_and_lights(mut commands: Commands, rig: Res<CameraRig>) {
    // Warm sunlit directional light
    commands.spawn((
        DirectionalLight {
            illuminance: 10_500.0,
            color: Color::srgb(1.0, 0.94, 0.80),
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(3.5, 6.5, 3.5).looking_at(Vec3::new(0.0, 0.6, -2.0), Vec3::Y),
    ));

    // Seated PoV camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(rig.seated_pos).looking_at(rig.seated_look, Vec3::Y),
    ));
}

fn setup_initial_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    spawn_office_scenario(&mut commands, &mut meshes, &mut materials);
}

#[allow(deprecated)]
fn handle_retry_input(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    btn_q: Query<&Interaction, (Changed<Interaction>, With<RetryButton>)>,
    scenario_q: Query<Entity, With<ScenarioEntity>>,
    popup_q: Query<Entity, With<FloatingPopup>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut tracker: ResMut<FlipTracker>,
    mut rig: ResMut<CameraRig>,
    mut score: ResMut<ScoreState>,
    mut popups: ResMut<PendingPopups>,
    mut hud_state: ResMut<HudState>,
    mut virtual_time: ResMut<Time<Virtual>>,
    mut next_state: ResMut<NextState<GamePhase>>,
) {
    let clicked_retry = btn_q.iter().any(|i| *i == Interaction::Pressed);
    if !keys.just_pressed(KeyCode::KeyR) && !clicked_retry {
        return;
    }

    for entity in &scenario_q {
        commands.entity(entity).despawn();
    }
    for entity in &popup_q {
        commands.entity(entity).despawn();
    }

    tracker.reset();
    rig.reset();
    score.reset();
    popups.0.clear();
    hud_state.displayed_score = 0.0;
    virtual_time.set_relative_speed(1.0);

    spawn_office_scenario(&mut commands, &mut meshes, &mut materials);
    next_state.set(GamePhase::Ready);
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
