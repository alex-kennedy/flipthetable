use std::sync::Mutex;

use bevy::prelude::*;
use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::audio::RetroAudio;
use crate::destruction::ScoreState;
use crate::scene::{CurrentScenario, ScenarioId};

#[derive(Clone, Copy)]
pub enum BridgeCommand {
    LoadScenario(ScenarioId),
    ResetRound,
    SetMuted(bool),
}

static COMMAND_QUEUE: Mutex<Vec<BridgeCommand>> = Mutex::new(Vec::new());

pub fn parse_scenario_id(id: &str) -> ScenarioId {
    match id {
        "building_site" => ScenarioId::BuildingSite,
        _ => ScenarioId::CorporateCubicle,
    }
}

pub fn scenario_slug(id: ScenarioId) -> &'static str {
    match id {
        ScenarioId::CorporateCubicle => "corporate",
        ScenarioId::BuildingSite => "building_site",
    }
}

#[wasm_bindgen]
pub fn bridge_load_scenario(id: &str) {
    if let Ok(mut q) = COMMAND_QUEUE.lock() {
        q.push(BridgeCommand::LoadScenario(parse_scenario_id(id)));
    }
}

#[wasm_bindgen]
pub fn bridge_reset_round() {
    if let Ok(mut q) = COMMAND_QUEUE.lock() {
        q.push(BridgeCommand::ResetRound);
    }
}

#[wasm_bindgen]
pub fn bridge_set_muted(muted: bool) {
    if let Ok(mut q) = COMMAND_QUEUE.lock() {
        q.push(BridgeCommand::SetMuted(muted));
    }
}

pub fn drain_bridge_commands(
    mut audio: ResMut<RetroAudio>,
    mut pending_reset: ResMut<PendingRoundReset>,
) {
    let Ok(mut q) = COMMAND_QUEUE.lock() else {
        return;
    };
    for cmd in q.drain(..) {
        match cmd {
            BridgeCommand::LoadScenario(id) => pending_reset.target = Some(id),
            BridgeCommand::ResetRound => pending_reset.reset_current = true,
            BridgeCommand::SetMuted(muted) => audio.muted = muted,
        }
    }
}

#[derive(Resource, Default)]
pub struct PendingRoundReset {
    pub target: Option<ScenarioId>,
    pub reset_current: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RunCompletePayload {
    scenario_id: &'static str,
    scenario_title: &'static str,
    total_score: u32,
    items_destroyed: u32,
    items_toppled: u32,
    max_combo: u32,
}

pub fn emit_run_complete(scenario: Res<CurrentScenario>, score: Res<ScoreState>) {
    let payload = RunCompletePayload {
        scenario_id: scenario_slug(scenario.0),
        scenario_title: scenario.0.title(),
        total_score: score.total_score,
        items_destroyed: score.items_destroyed,
        items_toppled: score.items_toppled,
        max_combo: score.max_combo.max(1),
    };
    if let Ok(json) = serde_json::to_string(&payload) {
        dispatch_window_event("flipthetable:run_complete", &json);
    }
}

pub fn emit_scenario_changed(id: ScenarioId) {
    dispatch_window_event("flipthetable:scenario_changed", scenario_slug(id));
}

fn dispatch_window_event(name: &str, detail: &str) {
    if let Some(window) = web_sys::window() {
        let init = web_sys::CustomEventInit::new();
        init.set_detail(&JsValue::from_str(detail));
        if let Ok(event) = web_sys::CustomEvent::new_with_event_init_dict(name, &init) {
            let _ = window.dispatch_event(&event);
        }
    }
}
