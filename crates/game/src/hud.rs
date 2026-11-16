use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::destruction::{PendingPopups, ScoreState};
use crate::flip::FlipTracker;
use crate::scene::CurrentScenario;
use crate::GamePhase;

#[derive(Component)]
pub struct ScenarioText;

#[derive(Component)]
pub struct ScoreText;

#[derive(Component)]
pub struct ComboText;

#[derive(Component)]
pub struct ScoreFeedText;

#[derive(Component)]
pub struct PromptOverlay;

#[derive(Component)]
pub struct SettlingBanner;

#[derive(Component)]
pub struct SettlingText;

#[derive(Component)]
pub struct PowerBarFill;

#[derive(Component)]
pub struct SettledOverlay;

#[derive(Component)]
pub struct SettledStatsText;

#[derive(Component)]
pub struct RetryButton;

#[derive(Component)]
pub struct NextScenarioButton;

#[derive(Component)]
pub struct FloatingPopup {
    pub screen_pos: Vec2,
    pub age: f32,
    pub duration: f32,
    pub base_font_size: f32,
    pub base_color: Color,
}

#[derive(Resource, Default)]
pub struct HudState {
    pub displayed_score: f32,
}

pub fn setup_hud(mut commands: Commands) {
    // Top-left score & combo panel
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(16.0),
                left: Val::Px(16.0),
                padding: UiRect::axes(Val::Px(14.0), Val::Px(10.0)),
                border: UiRect::all(Val::Px(2.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.06, 0.09, 0.86)),
            BorderColor::all(Color::srgb(0.95, 0.75, 0.25)),
        ))
        .with_children(|parent| {
            parent.spawn((
                ScenarioText,
                Text::new("CORPORATE CUBICLE  [1/2]"),
                TextFont {
                    font_size: FontSize::Px(11.0),
                    ..default()
                },
                TextColor(Color::srgb(0.55, 0.88, 0.96)),
            ));
            parent.spawn((
                ScoreText,
                Text::new("SCORE  00000"),
                TextFont {
                    font_size: FontSize::Px(20.0),
                    ..default()
                },
                TextColor(Color::srgb(0.98, 0.94, 0.82)),
            ));
            parent.spawn((
                ComboText,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::srgb(0.98, 0.45, 0.35)),
            ));
            parent.spawn((
                ScoreFeedText,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::srgb(0.95, 0.78, 0.28)),
            ));
        });

    // Bottom-center settling indicator (visible in Flipping)
    commands
        .spawn((
            SettlingBanner,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(24.0),
                left: Val::Percent(50.0),
                margin: UiRect::left(Val::Px(-130.0)),
                width: Val::Px(260.0),
                padding: UiRect::axes(Val::Px(14.0), Val::Px(8.0)),
                border: UiRect::all(Val::Px(2.0)),
                justify_content: JustifyContent::Center,
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.06, 0.09, 0.86)),
            BorderColor::all(Color::srgb(0.55, 0.88, 0.96)),
        ))
        .with_children(|b| {
            b.spawn((
                SettlingText,
                Text::new("CHAOS IN PROGRESS..."),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::srgb(0.55, 0.88, 0.96)),
            ));
        });

    // Bottom-center drag & flip prompt (visible in Ready)
    commands
        .spawn((
            PromptOverlay,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(24.0),
                left: Val::Percent(50.0),
                margin: UiRect::left(Val::Px(-180.0)),
                width: Val::Px(360.0),
                padding: UiRect::axes(Val::Px(16.0), Val::Px(10.0)),
                border: UiRect::all(Val::Px(2.0)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(8.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.06, 0.09, 0.86)),
            BorderColor::all(Color::srgb(0.95, 0.75, 0.25)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("CLICK & FLICK UP TO FLIP"),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::srgb(0.98, 0.94, 0.82)),
            ));

            // Live power bar track
            parent
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(8.0),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.11, 0.13, 0.18)),
                    BorderColor::all(Color::srgb(0.28, 0.33, 0.44)),
                ))
                .with_children(|track| {
                    track.spawn((
                        PowerBarFill,
                        Node {
                            width: Val::Percent(0.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.95, 0.75, 0.25)),
                    ));
                });
        });

    // Center Settled overlay (hidden until GamePhase::Settled)
    commands
        .spawn((
            SettledOverlay,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(50.0),
                left: Val::Percent(50.0),
                margin: UiRect {
                    left: Val::Px(-170.0),
                    top: Val::Px(-110.0),
                    ..default()
                },
                width: Val::Px(340.0),
                padding: UiRect::all(Val::Px(20.0)),
                border: UiRect::all(Val::Px(3.0)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(12.0),
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.06, 0.09, 0.94)),
            BorderColor::all(Color::srgb(0.95, 0.75, 0.25)),
        ))
        .with_children(|card| {
            card.spawn((
                Text::new("FLIP COMPLETE!"),
                TextFont {
                    font_size: FontSize::Px(22.0),
                    ..default()
                },
                TextColor(Color::srgb(0.95, 0.75, 0.25)),
            ));

            card.spawn((
                SettledStatsText,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::srgb(0.98, 0.94, 0.82)),
            ));

            card.spawn(Node {
                column_gap: Val::Px(10.0),
                ..default()
            })
            .with_children(|row| {
                row.spawn((
                    RetryButton,
                    Interaction::default(),
                    Node {
                        padding: UiRect::axes(Val::Px(16.0), Val::Px(10.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.95, 0.75, 0.25)),
                    BorderColor::all(Color::srgb(0.98, 0.94, 0.82)),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("RETRY [R]"),
                        TextFont {
                            font_size: FontSize::Px(14.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.05, 0.06, 0.09)),
                    ));
                });

                row.spawn((
                    NextScenarioButton,
                    Interaction::default(),
                    Node {
                        padding: UiRect::axes(Val::Px(16.0), Val::Px(10.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.14, 0.18, 0.25)),
                    BorderColor::all(Color::srgb(0.55, 0.88, 0.96)),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("NEXT ROOM [N]"),
                        TextFont {
                            font_size: FontSize::Px(14.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.55, 0.88, 0.96)),
                    ));
                });
            });
        });
}

pub fn update_hud(
    time: Res<Time>,
    phase: Res<State<GamePhase>>,
    scenario: Res<CurrentScenario>,
    score: Res<ScoreState>,
    tracker: Res<FlipTracker>,
    mut hud_state: ResMut<HudState>,
    mut text_q: Query<(
        &mut Text,
        &mut TextFont,
        &mut TextColor,
        Option<&ScenarioText>,
        Option<&ScoreText>,
        Option<&ComboText>,
        Option<&ScoreFeedText>,
        Option<&SettlingText>,
        Option<&SettledStatsText>,
    )>,
    mut node_q: Query<(
        &mut Node,
        Option<&PromptOverlay>,
        Option<&SettlingBanner>,
        Option<&SettledOverlay>,
        Option<&PowerBarFill>,
    )>,
) {
    let dt = time.delta_secs();
    let target = score.total_score as f32;
    hud_state.displayed_score += (target - hud_state.displayed_score) * (1.0 - (-12.0 * dt).exp());
    if (target - hud_state.displayed_score).abs() < 0.8 {
        hud_state.displayed_score = target;
    }

    let rank = match score.total_score {
        s if s >= 6000 => "S  // PURE MENACE",
        s if s >= 3500 => "A  // EXECUTIVE RAGE",
        s if s >= 1800 => "B  // DESK DISASTER",
        s if s >= 600 => "C  // MILD TANTRUM",
        _ => "D  // POLITE NUDGE",
    };

    for (mut text, mut font, mut color, is_scen, is_score, is_combo, is_feed, is_settle, is_stats) in
        &mut text_q
    {
        if is_scen.is_some() {
            text.0 = format!("{}  [1/2]", scenario.0.title());
        } else if is_score.is_some() {
            text.0 = format!("SCORE  {:05}", hud_state.displayed_score.round() as u32);
            font.font_size = FontSize::Px(20.0 + score.score_pulse * 4.0);
            color.0 = if score.score_pulse > 0.25 {
                Color::srgb(1.0, 0.86, 0.30)
            } else {
                Color::srgb(0.98, 0.94, 0.82)
            };
        } else if is_combo.is_some() {
            text.0 = if score.combo > 1 {
                format!("COMBO  x{}", score.combo)
            } else {
                String::new()
            };
        } else if is_feed.is_some() {
            text.0 = score
                .recent_feed
                .iter()
                .map(|e| e.text.as_str())
                .collect::<Vec<_>>()
                .join("\n");
        } else if is_settle.is_some() {
            text.0 = if score.settle_timer > 0.1 || score.elapsed_since_flip > 3.0 {
                "SETTLING...".to_string()
            } else {
                "CHAOS IN PROGRESS...".to_string()
            };
        } else if is_stats.is_some() && *phase.get() == GamePhase::Settled {
            text.0 = format!(
                "RANK:  {}\nSCORE: {}\nDESTROYED: {}   TOPPLED: {}\nMAX COMBO: x{}",
                rank,
                score.total_score,
                score.items_destroyed,
                score.items_toppled,
                score.max_combo.max(1),
            );
        }
    }

    for (mut node, is_prompt, is_settling, is_settled, is_power) in &mut node_q {
        if is_prompt.is_some() {
            node.display = if *phase.get() == GamePhase::Ready {
                Display::Flex
            } else {
                Display::None
            };
        } else if is_settling.is_some() {
            node.display = if *phase.get() == GamePhase::Flipping {
                Display::Flex
            } else {
                Display::None
            };
        } else if is_settled.is_some() {
            node.display = if *phase.get() == GamePhase::Settled {
                Display::Flex
            } else {
                Display::None
            };
        } else if is_power.is_some() {
            node.width = Val::Percent((tracker.live_power * 100.0).clamp(0.0, 100.0));
        }
    }
}

pub fn spawn_score_popups(
    mut commands: Commands,
    mut pending: ResMut<PendingPopups>,
    cam_q: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    win_q: Query<&Window, With<PrimaryWindow>>,
) {
    if pending.0.is_empty() {
        return;
    }
    let Ok((camera, cam_tf)) = cam_q.single() else {
        pending.0.clear();
        return;
    };
    let (max_x, max_y) = win_q
        .single()
        .map(|w| ((w.width() - 180.0).max(80.0), (w.height() - 60.0).max(80.0)))
        .unwrap_or((780.0, 480.0));

    for item in pending.0.drain(..) {
        if let Ok(raw_pos) = camera.world_to_viewport(cam_tf, item.world_pos) {
            let screen_pos = Vec2::new(
                raw_pos.x.clamp(48.0, max_x),
                raw_pos.y.clamp(72.0, max_y),
            );
            let font_size = if item.big { 18.0 } else { 14.0 };
            commands.spawn((
                FloatingPopup {
                    screen_pos,
                    age: 0.0,
                    duration: 1.05,
                    base_font_size: font_size,
                    base_color: item.color,
                },
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(screen_pos.x - 60.0),
                    top: Val::Px(screen_pos.y - 12.0),
                    ..default()
                },
                Text::new(item.text),
                TextFont {
                    font_size: FontSize::Px(font_size),
                    ..default()
                },
                TextColor(item.color),
            ));
        }
    }
}

pub fn animate_score_popups(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(Entity, &mut FloatingPopup, &mut Node, &mut TextFont, &mut TextColor)>,
) {
    let dt = time.delta_secs();
    for (entity, mut popup, mut node, mut font, mut color) in &mut q {
        popup.age += dt;
        let t = (popup.age / popup.duration).clamp(0.0, 1.0);

        // Ease-out upward drift + slight pop-in scale
        let y_offset = -52.0 * (1.0 - (1.0 - t) * (1.0 - t));
        node.left = Val::Px(popup.screen_pos.x - 60.0);
        node.top = Val::Px(popup.screen_pos.y - 12.0 + y_offset);

        let pop_scale = if t < 0.18 {
            0.8 + (t / 0.18) * 0.45
        } else {
            1.25 - ((t - 0.18) / 0.82) * 0.25
        };
        font.font_size = FontSize::Px(popup.base_font_size * pop_scale);

        let alpha = if t > 0.65 {
            1.0 - ((t - 0.65) / 0.35)
        } else {
            1.0
        };
        color.0 = popup.base_color.with_alpha(alpha.clamp(0.0, 1.0));

        if popup.age >= popup.duration {
            commands.entity(entity).despawn();
        }
    }
}
