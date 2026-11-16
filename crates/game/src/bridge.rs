use std::sync::Mutex;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::audio::RetroAudio;
use crate::destruction::ScoreState;
use crate::scenario_data::{CurrentScenario, ScenarioRegistry};

#[derive(Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HudLeaderboardRow {
    pub username: String,
    pub score: u32,
    #[serde(default)]
    pub is_me: bool,
}

#[derive(Resource, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HudLeaderboard {
    #[serde(default)]
    pub my_high_score: u32,
    #[serde(default)]
    pub top_scores: Vec<HudLeaderboardRow>,
    #[serde(default)]
    pub loading: bool,
}

#[derive(Clone)]
pub enum BridgeCommand {
    LoadScenario(String),
    ResetRound,
    SetMuted(bool),
    SetLeaderboard(HudLeaderboard),
}

static COMMAND_QUEUE: Mutex<Vec<BridgeCommand>> = Mutex::new(Vec::new());

#[wasm_bindgen]
pub fn bridge_load_scenario(id: &str) {
    if let Ok(mut q) = COMMAND_QUEUE.lock() {
        q.push(BridgeCommand::LoadScenario(id.trim().to_string()));
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

#[wasm_bindgen]
pub fn bridge_set_leaderboard(json: &str) {
    if let Ok(payload) = serde_json::from_str::<HudLeaderboard>(json)
        && let Ok(mut q) = COMMAND_QUEUE.lock()
    {
        q.push(BridgeCommand::SetLeaderboard(payload));
    }
}

pub fn drain_bridge_commands(
    mut audio: ResMut<RetroAudio>,
    mut pending_reset: ResMut<PendingRoundReset>,
    mut leaderboard: ResMut<HudLeaderboard>,
) {
    let Ok(mut q) = COMMAND_QUEUE.lock() else {
        return;
    };
    for cmd in q.drain(..) {
        match cmd {
            BridgeCommand::LoadScenario(id) => pending_reset.target = Some(id),
            BridgeCommand::ResetRound => pending_reset.reset_current = true,
            BridgeCommand::SetMuted(muted) => audio.muted = muted,
            BridgeCommand::SetLeaderboard(next) => *leaderboard = next,
        }
    }
}

#[derive(Resource, Default)]
pub struct PendingRoundReset {
    pub target: Option<String>,
    pub reset_current: bool,
    pub trigger_next: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RunCompletePayload<'a> {
    scenario_id: &'a str,
    scenario_title: &'a str,
    total_score: u32,
    items_destroyed: u32,
    items_toppled: u32,
    max_combo: u32,
}

pub fn emit_run_complete(
    scenario: Res<CurrentScenario>,
    registry: Res<ScenarioRegistry>,
    score: Res<ScoreState>,
) {
    let payload = RunCompletePayload {
        scenario_id: &scenario.0,
        scenario_title: registry.title_for(&scenario.0),
        total_score: score.total_score,
        items_destroyed: score.items_destroyed,
        items_toppled: score.items_toppled,
        max_combo: score.max_combo.max(1),
    };
    if let Ok(json) = serde_json::to_string(&payload) {
        dispatch_window_event("flipthetable:run_complete", &json);
    }
}

pub fn emit_scenario_changed(id: &str) {
    dispatch_window_event("flipthetable:scenario_changed", id);
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
