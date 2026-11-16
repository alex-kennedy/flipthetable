use bevy::prelude::*;

use crate::destruction::{PendingPopups, ScoreState};
use crate::flip::FlipTracker;
use crate::GamePhase;

#[derive(Component)]
pub struct ScoreText;

#[derive(Component)]
pub struct ComboText;

#[derive(Component)]
pub struct PromptOverlay;

#[derive(Component)]
pub struct PowerBarFill;

#[derive(Component)]
pub struct SettledOverlay;

#[derive(Component)]
pub struct SettledStatsText;

#[derive(Component)]
pub struct RetryButton;

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
                Text::new("DESK FLIPPED!"),
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

            card.spawn((
                RetryButton,
                Interaction::default(),
                Node {
                    padding: UiRect::axes(Val::Px(20.0), Val::Px(10.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.95, 0.75, 0.25)),
                BorderColor::all(Color::srgb(0.98, 0.94, 0.82)),
            ))
            .with_children(|btn| {
                btn.spawn((
                    Text::new("FLIP AGAIN [R]"),
                    TextFont {
                        font_size: FontSize::Px(15.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.05, 0.06, 0.09)),
                ));
            });
        });
}

pub fn update_hud(
    time: Res<Time>,
    phase: Res<State<GamePhase>>,
    score: Res<ScoreState>,
    tracker: Res<FlipTracker>,
    mut hud_state: ResMut<HudState>,
    mut score_q: Query<&mut Text, (With<ScoreText>, Without<ComboText>, Without<SettledStatsText>)>,
    mut combo_q: Query<&mut Text, (With<ComboText>, Without<ScoreText>, Without<SettledStatsText>)>,
    mut stats_q: Query<&mut Text, (With<SettledStatsText>, Without<ScoreText>, Without<ComboText>)>,
    mut prompt_q: Query<&mut Node, (With<PromptOverlay>, Without<SettledOverlay>, Without<PowerBarFill>)>,
    mut settled_q: Query<&mut Node, (With<SettledOverlay>, Without<PromptOverlay>, Without<PowerBarFill>)>,
    mut power_q: Query<&mut Node, (With<PowerBarFill>, Without<PromptOverlay>, Without<SettledOverlay>)>,
) {
    let dt = time.delta_secs();
    let target = score.total_score as f32;
    hud_state.displayed_score += (target - hud_state.displayed_score) * (1.0 - (-12.0 * dt).exp());
    if (target - hud_state.displayed_score).abs() < 0.8 {
        hud_state.displayed_score = target;
    }

    if let Ok(mut text) = score_q.single_mut() {
        text.0 = format!("SCORE  {:05}", hud_state.displayed_score.round() as u32);
    }

    if let Ok(mut combo) = combo_q.single_mut() {
        if score.combo > 1 {
            combo.0 = format!("COMBO  x{}", score.combo);
        } else {
            combo.0.clear();
        }
    }

    if let Ok(mut prompt_node) = prompt_q.single_mut() {
        prompt_node.display = if *phase.get() == GamePhase::Ready {
            Display::Flex
        } else {
            Display::None
        };
    }

    if let Ok(mut power_node) = power_q.single_mut() {
        power_node.width = Val::Percent((tracker.live_power * 100.0).clamp(0.0, 100.0));
    }

    if let Ok(mut settled_node) = settled_q.single_mut() {
        settled_node.display = if *phase.get() == GamePhase::Settled {
            Display::Flex
        } else {
            Display::None
        };
    }

    if *phase.get() == GamePhase::Settled {
        if let Ok(mut stats) = stats_q.single_mut() {
            stats.0 = format!(
                "SCORE: {}\nDESTROYED: {}   TOPPLED: {}\nMAX COMBO: x{}",
                score.total_score,
                score.items_destroyed,
                score.items_toppled,
                score.max_combo.max(1),
            );
        }
    }
}

pub fn spawn_score_popups(
    mut commands: Commands,
    mut pending: ResMut<PendingPopups>,
    cam_q: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
) {
    if pending.0.is_empty() {
        return;
    }
    let Ok((camera, cam_tf)) = cam_q.single() else {
        pending.0.clear();
        return;
    };

    for item in pending.0.drain(..) {
        if let Ok(screen_pos) = camera.world_to_viewport(cam_tf, item.world_pos) {
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
