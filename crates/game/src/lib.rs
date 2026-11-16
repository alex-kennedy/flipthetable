mod audio;
mod destruction;
mod flip;
mod hud;
mod pixel_art;
mod scene;

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::window::{PresentMode, WindowResolution};
use wasm_bindgen::prelude::*;

use audio::{flush_audio_queue, RetroAudio};
use destruction::{
    check_run_settled, process_destruction, update_debris_lifetime, PendingPopups, ScoreState,
};
use flip::{handle_flip_gesture, update_follow_camera, CameraRig, FlipTracker};
use hud::{
    animate_score_popups, setup_hud, spawn_score_popups, update_hud, FloatingPopup, HudState,
    NextScenarioButton, RetryButton,
};
use pixel_art::{create_procedural_textures, ProceduralTextures};
use scene::{spawn_scenario, CurrentScenario, ScenarioEntity, ScenarioId};

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
        .init_resource::<RetroAudio>()
        .init_resource::<CurrentScenario>()
        .add_systems(
            Startup,
            (
                setup_pixel_pipeline_and_camera,
                setup_initial_scene.after(setup_pixel_pipeline_and_camera),
                setup_hud.after(setup_pixel_pipeline_and_camera),
                notify_js_ready,
            ),
        )
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
                flush_audio_queue,
                handle_retry_input,
            ),
        )
        .run();
}

fn setup_pixel_pipeline_and_camera(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    rig: Res<CameraRig>,
) {
    let textures = create_procedural_textures(&mut images);
    commands.insert_resource(textures);

    // Warm office ambient bounce light so shadows are rich, never pitch black
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.65, 0.72, 0.85),
        brightness: 420.0,
        ..default()
    });

    // Warm sunlit directional key light
    commands.spawn((
        DirectionalLight {
            illuminance: 13_500.0,
            color: Color::srgb(1.0, 0.92, 0.74),
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(3.2, 6.8, 3.0).looking_at(Vec3::new(0.0, 0.6, -2.0), Vec3::Y),
    ));

    // Subtle cool cubicle fill light
    commands.spawn((
        PointLight {
            color: Color::srgb(0.55, 0.85, 0.95),
            intensity: 80_000.0,
            range: 14.0,
            ..default()
        },
        Transform::from_xyz(-2.8, 3.2, -1.0),
    ));

    // 3D scene camera renders directly to the window with Msaa::Off for crisp retro edges
    commands.spawn((
        Camera3d::default(),
        Msaa::Off,
        Transform::from_translation(rig.seated_pos).looking_at(rig.seated_look, Vec3::Y),
    ));
}

fn setup_initial_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    textures: Res<ProceduralTextures>,
    scenario: Res<CurrentScenario>,
) {
    spawn_scenario(scenario.0, &mut commands, &mut meshes, &mut materials, &textures);
}

#[allow(deprecated)]
fn handle_retry_input(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    btn_q: Query<
        (&Interaction, Option<&RetryButton>, Option<&NextScenarioButton>),
        Changed<Interaction>,
    >,
    scenario_q: Query<Entity, With<ScenarioEntity>>,
    popup_q: Query<Entity, With<FloatingPopup>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    textures: Res<ProceduralTextures>,
    mut current_scenario: ResMut<CurrentScenario>,
    mut tracker: ResMut<FlipTracker>,
    mut rig: ResMut<CameraRig>,
    mut score: ResMut<ScoreState>,
    mut popups: ResMut<PendingPopups>,
    mut hud_state: ResMut<HudState>,
    mut virtual_time: ResMut<Time<Virtual>>,
    mut next_state: ResMut<NextState<GamePhase>>,
) {
    let mut clicked_retry = false;
    let mut clicked_next = false;
    for (interaction, is_retry, is_next) in &btn_q {
        if *interaction == Interaction::Pressed {
            if is_retry.is_some() {
                clicked_retry = true;
            }
            if is_next.is_some() {
                clicked_next = true;
            }
        }
    }

    let mut target_scenario = None;
    if keys.just_pressed(KeyCode::Digit1) {
        target_scenario = Some(ScenarioId::CorporateCubicle);
    } else if keys.just_pressed(KeyCode::Digit2) {
        target_scenario = Some(ScenarioId::BuildingSite);
    } else if keys.just_pressed(KeyCode::KeyN) || clicked_next {
        target_scenario = Some(current_scenario.0.next());
    } else if keys.just_pressed(KeyCode::KeyR) || clicked_retry {
        target_scenario = Some(current_scenario.0);
    }

    let Some(next_id) = target_scenario else {
        return;
    };
    current_scenario.0 = next_id;

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

    spawn_scenario(next_id, &mut commands, &mut meshes, &mut materials, &textures);
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
