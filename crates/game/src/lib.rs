mod audio;
mod bridge;
mod destruction;
mod flip;
mod hud;
mod meshes;
mod palette;
mod pixel_art;
mod prefabs;
mod render_pipeline;
mod scenario_data;

use avian3d::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::pbr::DistanceFog;
use bevy::prelude::*;
use bevy::window::{PresentMode, WindowResolution};
use wasm_bindgen::prelude::*;

use audio::{flush_audio_queue, RetroAudio};
use bridge::{
    drain_bridge_commands, emit_run_complete, emit_scenario_changed, HudLeaderboard,
    PendingRoundReset,
};
use destruction::{
    check_run_settled, process_desk_breakup, process_destruction, update_debris_lifetime,
    update_fluttering_papers, update_vfx_emitters, update_vfx_particles, PendingPopups, ScoreState,
};
use flip::{handle_flip_gesture, update_follow_camera, CameraRig, FlipTracker};
use hud::{
    animate_score_popups, setup_hud, spawn_score_popups, start_settled_tally, update_hud,
    FloatingPopup, HudState,
};
use meshes::{create_shared_meshes, SharedMeshes};
use palette::{CorporateJokeRng, VfxPalette};
use pixel_art::{create_procedural_textures, ProceduralTextures};
use prefabs::{spawn_scenario_world, ScenarioEntity};
use render_pipeline::{
    apply_scenario_lighting, setup_retro_cameras, PixelRenderState, RetroPostMaterial,
    RetroRenderPlugin, SceneCamera3d,
};
use scenario_data::{CurrentScenario, ScenarioDataPlugin, ScenarioDef, ScenarioRegistry};

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
            DefaultPlugins
                .set(AssetPlugin {
                    meta_check: bevy::asset::AssetMetaCheck::Never,
                    ..default()
                })
                .set(WindowPlugin {
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
            RetroRenderPlugin,
            ScenarioDataPlugin,
        ))
        .init_state::<GamePhase>()
        .insert_resource(ClearColor(Color::srgb(0.06, 0.08, 0.12)))
        .init_resource::<FlipTracker>()
        .init_resource::<CameraRig>()
        .init_resource::<ScoreState>()
        .init_resource::<PendingPopups>()
        .init_resource::<HudState>()
        .init_resource::<RetroAudio>()
        .init_resource::<CorporateJokeRng>()
        .init_resource::<PendingRoundReset>()
        .init_resource::<HudLeaderboard>()
        .add_systems(
            OnEnter(GamePhase::Settled),
            (start_settled_tally, emit_run_complete),
        )
        .add_systems(
            Startup,
            (
                setup_shared_assets,
                setup_retro_cameras,
                setup_initial_scene
                    .after(setup_shared_assets)
                    .after(setup_retro_cameras),
                setup_hud.after(setup_retro_cameras),
                notify_js_ready.after(setup_initial_scene),
            ),
        )
        .add_systems(
            Update,
            (
                handle_flip_gesture.run_if(in_state(GamePhase::Ready)),
                (process_destruction, process_desk_breakup, check_run_settled)
                    .run_if(in_state(GamePhase::Flipping)),
                update_vfx_emitters,
                update_vfx_particles,
                update_fluttering_papers,
                update_debris_lifetime,
                update_follow_camera,
                spawn_score_popups,
                animate_score_popups,
                update_hud,
                flush_audio_queue,
                drain_bridge_commands,
                poll_dynamic_scenario_assets,
                handle_round_and_scenario_input.after(drain_bridge_commands),
            ),
        )
        .run();
}

fn setup_shared_assets(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let textures = create_procedural_textures(&mut images);
    let shared = create_shared_meshes(&mut meshes, &mut materials);
    let vfx_palette = VfxPalette::new(&mut materials, &textures);
    commands.insert_resource(textures);
    commands.insert_resource(shared);
    commands.insert_resource(vfx_palette);
}

#[allow(clippy::too_many_arguments)]
fn setup_initial_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut post_mats: ResMut<Assets<RetroPostMaterial>>,
    textures: Res<ProceduralTextures>,
    shared: Res<SharedMeshes>,
    pixel_state: Res<PixelRenderState>,
    scenario: Res<CurrentScenario>,
    registry: Res<ScenarioRegistry>,
    mut joke_rng: ResMut<CorporateJokeRng>,
    mut cam_q: Query<(&mut Camera, &mut DistanceFog), With<SceneCamera3d>>,
) {
    if let Some(def) = registry.get(&scenario.0) {
        apply_scenario_lighting(
            &def.lighting,
            &mut commands,
            &pixel_state,
            &mut post_mats,
            &mut cam_q,
        );
        spawn_scenario_world(
            def,
            &mut commands,
            &mut meshes,
            &mut materials,
            &textures,
            &shared,
            &mut joke_rng,
        );
    }
}

fn poll_dynamic_scenario_assets(
    mut registry: ResMut<ScenarioRegistry>,
    scenario_assets: Res<Assets<ScenarioDef>>,
    mut pending_reset: ResMut<PendingRoundReset>,
) {
    let mut loaded_ids = Vec::new();
    for (slug, handle) in &registry.pending_dynamic {
        if let Some(def) = scenario_assets.get(handle) {
            loaded_ids.push((slug.clone(), def.clone()));
        }
    }
    for (slug, def) in loaded_ids {
        registry.pending_dynamic.remove(&slug);
        if !registry.order.contains(&slug) {
            registry.order.push(slug.clone());
        }
        registry.defs.insert(slug.clone(), def);
        pending_reset.target = Some(slug);
    }
}

#[derive(SystemParam)]
struct ResetState<'w> {
    current_scenario: ResMut<'w, CurrentScenario>,
    registry: ResMut<'w, ScenarioRegistry>,
    tracker: ResMut<'w, FlipTracker>,
    rig: ResMut<'w, CameraRig>,
    score: ResMut<'w, ScoreState>,
    popups: ResMut<'w, PendingPopups>,
    hud_state: ResMut<'w, HudState>,
    joke_rng: ResMut<'w, CorporateJokeRng>,
    virtual_time: ResMut<'w, Time<Virtual>>,
    next_state: ResMut<'w, NextState<GamePhase>>,
    pending_reset: ResMut<'w, PendingRoundReset>,
}

#[allow(clippy::too_many_arguments)]
fn handle_round_and_scenario_input(
    mut commands: Commands,
    time: Res<Time<Real>>,
    keys: Res<ButtonInput<KeyCode>>,
    asset_server: Res<AssetServer>,
    scenario_q: Query<Entity, With<ScenarioEntity>>,
    popup_q: Query<Entity, With<FloatingPopup>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut post_mats: ResMut<Assets<RetroPostMaterial>>,
    textures: Res<ProceduralTextures>,
    shared: Res<SharedMeshes>,
    pixel_state: Res<PixelRenderState>,
    mut cam_q: Query<(&mut Camera, &mut DistanceFog), With<SceneCamera3d>>,
    mut rs: ResetState,
) {
    let mut target_id = rs.pending_reset.target.take();

    if target_id.is_none() && rs.pending_reset.trigger_next {
        rs.pending_reset.trigger_next = false;
        target_id = Some(rs.registry.next_after(&rs.current_scenario.0));
    }
    if target_id.is_none() && rs.pending_reset.reset_current {
        rs.pending_reset.reset_current = false;
        target_id = Some(rs.current_scenario.0.clone());
    }
    if target_id.is_none() {
        if keys.just_pressed(KeyCode::Digit1) {
            target_id = rs.registry.by_index(0).map(str::to_string);
        } else if keys.just_pressed(KeyCode::Digit2) {
            target_id = rs.registry.by_index(1).map(str::to_string);
        } else if keys.just_pressed(KeyCode::Digit3) {
            target_id = rs.registry.by_index(2).map(str::to_string);
        } else if keys.just_pressed(KeyCode::Digit4) {
            target_id = rs.registry.by_index(3).map(str::to_string);
        } else if keys.just_pressed(KeyCode::Digit5) {
            target_id = rs.registry.by_index(4).map(str::to_string);
        } else if keys.just_pressed(KeyCode::Digit6) {
            target_id = rs.registry.by_index(5).map(str::to_string);
        } else if keys.just_pressed(KeyCode::KeyN) {
            target_id = Some(rs.registry.next_after(&rs.current_scenario.0));
        } else if keys.just_pressed(KeyCode::KeyR) {
            target_id = Some(rs.current_scenario.0.clone());
        }
    }

    let Some(next_id) = target_id else {
        return;
    };

    let Some(def) = rs.registry.get(&next_id).cloned() else {
        if !rs.registry.pending_dynamic.contains_key(&next_id) {
            let handle = asset_server.load(format!("scenarios/{next_id}.json"));
            rs.registry.pending_dynamic.insert(next_id, handle);
        }
        return;
    };

    rs.current_scenario.0 = next_id.clone();
    emit_scenario_changed(&next_id);

    for entity in &scenario_q {
        commands.entity(entity).despawn();
    }
    for entity in &popup_q {
        commands.entity(entity).despawn();
    }

    rs.tracker.reset();
    rs.rig.reset();
    rs.score.reset();
    rs.popups.0.clear();
    rs.hud_state.reset_round();
    rs.joke_rng
        .advance_round((time.elapsed_secs_f64() * 1_000_000.0) as u64);
    rs.virtual_time.set_relative_speed(1.0);

    apply_scenario_lighting(
        &def.lighting,
        &mut commands,
        &pixel_state,
        &mut post_mats,
        &mut cam_q,
    );
    spawn_scenario_world(
        &def,
        &mut commands,
        &mut meshes,
        &mut materials,
        &textures,
        &shared,
        &mut rs.joke_rng,
    );
    rs.next_state.set(GamePhase::Ready);
}

fn notify_js_ready() {
    if let Some(window) = web_sys::window() {
        let init = web_sys::CustomEventInit::new();
        init.set_detail(&JsValue::from_str("ready"));
        if let Ok(event) =
            web_sys::CustomEvent::new_with_event_init_dict("flipthetable:ready", &init)
        {
            let _ = window.dispatch_event(&event);
        }
    }
}
