use bevy::picking::prelude::*;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::audio::{RetroAudio, SoundEffect};
use crate::bridge::{HudLeaderboard, PendingRoundReset};
use crate::destruction::{PendingPopups, ScoreState};
use crate::flip::{CameraRig, FlipTracker};
use crate::render_pipeline::{PixelRenderState, SceneCamera3d};
use crate::scenario_data::{CurrentScenario, ScenarioRegistry};
use crate::GamePhase;

#[derive(Component, Clone, Copy)]
pub enum HudTextRole {
    Scenario,
    Score,
    Combo,
    ScoreFeed,
    TopScoreName(usize),
    TopScoreValue(usize),
    MyHighScore,
    PromptHint,
    Settling,
    TallyHeader,
    TallyRowLabel(usize),
    TallyRowValue(usize),
    TallyTier(u8),
    TallyTotal,
    TallyStamp,
    TallySub,
}

#[derive(Component)]
pub struct PromptOverlay;

#[derive(Component)]
pub struct SettlingBanner;

#[derive(Component)]
pub struct PowerBarFill;

#[derive(Component)]
pub struct SettledOverlay;

#[derive(Component)]
pub struct TallyBarFill;

#[derive(Component)]
pub struct FloatingPopup {
    pub origin_screen: Vec2,
    pub vel_screen: Vec2,
    pub age: f32,
    pub duration: f32,
    pub base_font_size: f32,
    pub base_color: Color,
}

#[derive(Resource, Default)]
pub struct HudState {
    pub displayed_score: f32,
    pub pixel_font: Handle<Font>,
    pub popup_seq: u32,
    pub tally_age: f32,
    pub tally_rows_slammed: u8,
    pub tally_verdict_slammed: bool,
    pub tally_tick_accum: f32,
}

impl HudState {
    pub fn reset_round(&mut self) {
        self.displayed_score = 0.0;
        self.tally_age = 0.0;
        self.tally_rows_slammed = 0;
        self.tally_verdict_slammed = false;
        self.tally_tick_accum = 0.0;
    }
}

pub fn start_settled_tally(mut hud: ResMut<HudState>) {
    hud.tally_age = 0.0;
    hud.tally_rows_slammed = 0;
    hud.tally_verdict_slammed = false;
    hud.tally_tick_accum = 0.0;
}

fn retro_shadow() -> TextShadow {
    TextShadow {
        offset: Vec2::new(2.0, 2.0),
        color: Color::srgba(0.02, 0.03, 0.05, 0.95),
    }
}

fn truncate_handle(name: &str, max_chars: usize) -> String {
    let mut chars = name.chars();
    let taken: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        let trimmed: String = taken.chars().take(max_chars.saturating_sub(2)).collect();
        format!("{trimmed}..")
    } else {
        taken
    }
}

const MI_MIN: u32 = 2_600;
const SI_MIN: u32 = 7_000;
const OI_MIN: u32 = 14_500;
const TI_MIN: u32 = u32::MAX;

fn rank_tier_info(score: u32) -> (u8, &'static str, &'static str, Color, Color) {
    if score >= TI_MIN {
        (
            4,
            "TRANSFORMATIONAL IMPACT",
            "PARADIGM OBLITERATED!! IMMEDIATE PROMOTION!!",
            Color::srgb(1.0, 0.36, 0.78),
            Color::srgb(1.0, 0.88, 0.28),
        )
    } else if score >= OI_MIN {
        (
            3,
            "OUTSTANDING IMPACT",
            "EXCEEDS ALL Q4 RESTRUCTURING KPIS!",
            Color::srgb(1.0, 0.80, 0.20),
            Color::srgb(1.0, 0.62, 0.18),
        )
    } else if score >= SI_MIN {
        (
            2,
            "SIGNIFICANT IMPACT",
            "STRONG DISRUPTION. C-SUITE IS CONCERNED.",
            Color::srgb(0.32, 0.90, 0.98),
            Color::srgb(0.32, 0.90, 0.98),
        )
    } else if score >= MI_MIN {
        (
            1,
            "MODERATE IMPACT",
            "MEETS MINIMUM TANTRUM EXPECTATIONS.",
            Color::srgb(0.82, 0.70, 0.34),
            Color::srgb(0.82, 0.70, 0.34),
        )
    } else {
        (
            0,
            "NOT ENOUGH IMPACT",
            "LOW OUTPUT. PLACED ON IMMEDIATE PIP.",
            Color::srgb(0.68, 0.26, 0.26),
            Color::srgb(0.55, 0.24, 0.24),
        )
    }
}

/// Maps score across the 5 labeled zones: NEI (0..20%), MI (20..45%), SI (45..70%), OI (70..92%), TI (92..100%).
fn score_to_bar_percent(score: u32) -> f32 {
    let s = score as f32;
    if score == 0 {
        2.0
    } else if score < MI_MIN {
        2.8 + (s / MI_MIN as f32) * 17.2
    } else if score < SI_MIN {
        20.0 + ((s - MI_MIN as f32) / (SI_MIN - MI_MIN) as f32) * 25.0
    } else if score < OI_MIN {
        45.0 + ((s - SI_MIN as f32) / (OI_MIN - SI_MIN) as f32) * 25.0
    } else {
        let excess = (s - OI_MIN as f32).max(0.0);
        70.0 + 21.2 * (1.0 - (-excess / 15_500.0).exp())
    }
}

pub fn setup_hud(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut hud_state: ResMut<HudState>,
) {
    let font: Handle<Font> = asset_server.load("fonts/arcade_pixel.ttf");
    hud_state.pixel_font = font.clone();

    // Top-left column: Live Score box + Top 5 High Scores & My High Score box underneath
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            top: Val::Px(14.0),
            left: Val::Px(14.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(8.0),
            align_items: AlignItems::FlexStart,
            ..default()
        })
        .with_children(|col| {
            // 1. Live score & combo arcade panel
            col.spawn((
                Node {
                    min_width: Val::Px(232.0),
                    padding: UiRect::axes(Val::Px(14.0), Val::Px(10.0)),
                    border: UiRect::all(Val::Px(3.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(6.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.04, 0.05, 0.08, 0.88)),
                BorderColor::all(Color::srgb(0.95, 0.75, 0.25)),
            ))
            .with_children(|parent| {
                parent.spawn((
                    HudTextRole::Scenario,
                    Text::new("CORPORATE CUBICLE  [1-6]"),
                    TextFont {
                        font: font.clone().into(),
                        font_size: FontSize::Px(10.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.55, 0.88, 0.96)),
                    retro_shadow(),
                ));
                parent.spawn((
                    HudTextRole::Score,
                    Text::new("SCORE 00000"),
                    TextFont {
                        font: font.clone().into(),
                        font_size: FontSize::Px(18.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.98, 0.94, 0.82)),
                    retro_shadow(),
                ));
                parent.spawn((
                    HudTextRole::Combo,
                    Text::new(""),
                    TextFont {
                        font: font.clone().into(),
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(Color::srgb(1.0, 0.45, 0.34)),
                    retro_shadow(),
                ));
                parent.spawn((
                    HudTextRole::ScoreFeed,
                    Text::new(""),
                    TextFont {
                        font: font.clone().into(),
                        font_size: FontSize::Px(10.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.96, 0.80, 0.30)),
                    retro_shadow(),
                ));
            });

            // 2. Top 5 High Scores + My High Score panel underneath
            col.spawn((
                Node {
                    min_width: Val::Px(232.0),
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(9.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(5.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.04, 0.05, 0.08, 0.86)),
                BorderColor::all(Color::srgb(0.95, 0.75, 0.25)),
            ))
            .with_children(|board| {
                board.spawn((
                    Text::new("TOP 5 HIGH SCORES"),
                    TextFont {
                        font: font.clone().into(),
                        font_size: FontSize::Px(9.5),
                        ..default()
                    },
                    TextColor(Color::srgb(0.95, 0.75, 0.25)),
                    retro_shadow(),
                ));

                for idx in 0..5 {
                    board
                        .spawn(Node {
                            width: Val::Percent(100.0),
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(10.0),
                            ..default()
                        })
                        .with_children(|row| {
                            row.spawn((
                                HudTextRole::TopScoreName(idx),
                                Text::new(format!("{}. ---", idx + 1)),
                                TextFont {
                                    font: font.clone().into(),
                                    font_size: FontSize::Px(9.0),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.42, 0.46, 0.56)),
                                retro_shadow(),
                            ));
                            row.spawn((
                                HudTextRole::TopScoreValue(idx),
                                Text::new("-----"),
                                TextFont {
                                    font: font.clone().into(),
                                    font_size: FontSize::Px(9.0),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.42, 0.46, 0.56)),
                                retro_shadow(),
                            ));
                        });
                }

                // Divider before My High Score
                board.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(1.0),
                        margin: UiRect::vertical(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.32, 0.38, 0.50, 0.65)),
                ));

                board
                    .spawn(Node {
                        width: Val::Percent(100.0),
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(10.0),
                        ..default()
                    })
                    .with_children(|my_row| {
                        my_row.spawn((
                            Text::new("MY HIGH SCORE"),
                            TextFont {
                                font: font.clone().into(),
                                font_size: FontSize::Px(9.5),
                                ..default()
                            },
                            TextColor(Color::srgb(0.55, 0.88, 0.96)),
                            retro_shadow(),
                        ));
                        my_row.spawn((
                            HudTextRole::MyHighScore,
                            Text::new("00000"),
                            TextFont {
                                font: font.clone().into(),
                                font_size: FontSize::Px(10.5),
                                ..default()
                            },
                            TextColor(Color::srgb(1.0, 0.86, 0.30)),
                            retro_shadow(),
                        ));
                    });
            });
        });

    // Bottom-center settling indicator (Flipping)
    commands
        .spawn((
            SettlingBanner,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(22.0),
                left: Val::Percent(50.0),
                margin: UiRect::left(Val::Px(-145.0)),
                width: Val::Px(290.0),
                padding: UiRect::axes(Val::Px(14.0), Val::Px(8.0)),
                border: UiRect::all(Val::Px(2.0)),
                justify_content: JustifyContent::Center,
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.04, 0.05, 0.08, 0.88)),
            BorderColor::all(Color::srgb(0.55, 0.88, 0.96)),
        ))
        .with_children(|b| {
            b.spawn((
                HudTextRole::Settling,
                Text::new("CHAOS IN PROGRESS..."),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Px(11.0),
                    ..default()
                },
                TextColor(Color::srgb(0.55, 0.88, 0.96)),
                retro_shadow(),
            ));
        });

    // Bottom-center drag & directional flip prompt (Ready)
    commands
        .spawn((
            PromptOverlay,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(22.0),
                left: Val::Percent(50.0),
                margin: UiRect::left(Val::Px(-205.0)),
                width: Val::Px(410.0),
                padding: UiRect::axes(Val::Px(16.0), Val::Px(10.0)),
                border: UiRect::all(Val::Px(3.0)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(8.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.04, 0.05, 0.08, 0.88)),
            BorderColor::all(Color::srgb(0.95, 0.75, 0.25)),
        ))
        .with_children(|parent| {
            parent.spawn((
                HudTextRole::PromptHint,
                Text::new("SWIPE UP + ANGLE TO AIM & FLIP!"),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Px(11.5),
                    ..default()
                },
                TextColor(Color::srgb(0.98, 0.94, 0.82)),
                retro_shadow(),
            ));

            parent
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(10.0),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.10, 0.12, 0.17)),
                    BorderColor::all(Color::srgb(0.30, 0.36, 0.48)),
                ))
                .with_children(|track| {
                    track.spawn((
                        PowerBarFill,
                        Node {
                            width: Val::Percent(0.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.96, 0.76, 0.24)),
                    ));
                });
        });

    // ── Slam-Animated Performance Review Tally Card (Settled) ──────────────
    commands
        .spawn((
            SettledOverlay,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(50.0),
                left: Val::Percent(50.0),
                margin: UiRect {
                    left: Val::Px(-235.0),
                    top: Val::Px(-180.0),
                    ..default()
                },
                width: Val::Px(470.0),
                padding: UiRect::axes(Val::Px(22.0), Val::Px(18.0)),
                border: UiRect::all(Val::Px(4.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(9.0),
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.04, 0.05, 0.08, 0.95)),
            BorderColor::all(Color::srgb(0.95, 0.75, 0.25)),
        ))
        .with_children(|card| {
            card.spawn((
                HudTextRole::TallyHeader,
                Text::new("ANNUAL IMPACT CALIBRATION"),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::srgb(0.95, 0.75, 0.25)),
                retro_shadow(),
            ));

            // 4 Slam Breakdown Rows
            card.spawn(Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(7.0),
                padding: UiRect::vertical(Val::Px(6.0)),
                border: UiRect::vertical(Val::Px(1.0)),
                ..default()
            })
            .insert(BorderColor::all(Color::srgba(0.32, 0.38, 0.50, 0.55)))
            .with_children(|rows| {
                for idx in 0..4 {
                    rows.spawn(Node {
                        width: Val::Percent(100.0),
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        min_height: Val::Px(16.0),
                        ..default()
                    })
                    .with_children(|r| {
                        r.spawn((
                            HudTextRole::TallyRowLabel(idx),
                            Text::new(""),
                            TextFont {
                                font: font.clone().into(),
                                font_size: FontSize::Px(9.5),
                                ..default()
                            },
                            TextColor(Color::srgb(0.86, 0.90, 0.95)),
                            retro_shadow(),
                        ));
                        r.spawn((
                            HudTextRole::TallyRowValue(idx),
                            Text::new(""),
                            TextFont {
                                font: font.clone().into(),
                                font_size: FontSize::Px(11.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.98, 0.82, 0.28)),
                            retro_shadow(),
                        ));
                    });
                }
            });

            // Total Score Line
            card.spawn(Node {
                width: Val::Percent(100.0),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|tot| {
                tot.spawn((
                    Text::new("CALIBRATED SCORE"),
                    TextFont {
                        font: font.clone().into(),
                        font_size: FontSize::Px(11.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.98, 0.94, 0.82)),
                    retro_shadow(),
                ));
                tot.spawn((
                    HudTextRole::TallyTotal,
                    Text::new("00000"),
                    TextFont {
                        font: font.clone().into(),
                        font_size: FontSize::Px(18.0),
                        ..default()
                    },
                    TextColor(Color::srgb(1.0, 0.86, 0.28)),
                    retro_shadow(),
                ));
            });

            // Labeled Rating Scale Row: NEI ── MI ── SI ── OI ── TI
            card.spawn(Node {
                width: Val::Percent(100.0),
                height: Val::Px(13.0),
                position_type: PositionType::Relative,
                ..default()
            })
            .with_children(|lbls| {
                let markers: [(u8, &str, f32); 5] = [
                    (0, "NEI", 2.0),
                    (1, "MI", 21.5),
                    (2, "SI", 46.5),
                    (3, "OI", 71.5),
                    (4, "TI", 92.5),
                ];
                for (tier_id, code, left_pct) in markers {
                    lbls.spawn((
                        HudTextRole::TallyTier(tier_id),
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Percent(left_pct),
                            top: Val::Px(0.0),
                            ..default()
                        },
                        Text::new(code),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::Px(9.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.48, 0.54, 0.66)),
                        retro_shadow(),
                    ));
                }
            });

            // Segmented Impact Meter Bar aligned with NEI | MI | SI | OI | TI ticks!
            card.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(18.0),
                    border: UiRect::all(Val::Px(2.0)),
                    position_type: PositionType::Relative,
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.08, 0.10, 0.14)),
                BorderColor::all(Color::srgb(0.32, 0.38, 0.50)),
            ))
            .with_children(|bar| {
                bar.spawn((
                    TallyBarFill,
                    Node {
                        width: Val::Percent(0.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.68, 0.26, 0.26)),
                ));
                for pct in [20.0_f32, 45.0, 70.0, 92.0] {
                    bar.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Percent(pct),
                            top: Val::Px(0.0),
                            width: Val::Px(2.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.90, 0.94, 1.0, 0.45)),
                    ));
                }
            });

            // Verdict Rating Stamp + Corporate Subtitle!
            card.spawn(Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(5.0),
                min_height: Val::Px(40.0),
                justify_content: JustifyContent::Center,
                ..default()
            })
            .with_children(|v| {
                v.spawn((
                    HudTextRole::TallyStamp,
                    Text::new(""),
                    TextFont {
                        font: font.clone().into(),
                        font_size: FontSize::Px(15.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.95, 0.75, 0.25)),
                    retro_shadow(),
                ));
                v.spawn((
                    HudTextRole::TallySub,
                    Text::new(""),
                    TextFont {
                        font: font.clone().into(),
                        font_size: FontSize::Px(9.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.76, 0.80, 0.88)),
                    retro_shadow(),
                ));
            });

            // Action Buttons
            card.spawn(Node {
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                column_gap: Val::Px(12.0),
                margin: UiRect::top(Val::Px(3.0)),
                ..default()
            })
            .with_children(|row| {
                row.spawn((
                    Node {
                        padding: UiRect::axes(Val::Px(16.0), Val::Px(10.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.95, 0.75, 0.25)),
                    BorderColor::all(Color::srgb(0.98, 0.94, 0.82)),
                ))
                .observe(|_: On<PointerClick>, mut pending: ResMut<PendingRoundReset>| {
                    pending.reset_current = true;
                })
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("FLIP AGAIN [R]"),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::Px(10.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.05, 0.06, 0.09)),
                    ));
                });

                row.spawn((
                    Node {
                        padding: UiRect::axes(Val::Px(16.0), Val::Px(10.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.14, 0.18, 0.26)),
                    BorderColor::all(Color::srgb(0.55, 0.88, 0.96)),
                ))
                .observe(|_: On<PointerClick>, mut pending: ResMut<PendingRoundReset>| {
                    pending.trigger_next = true;
                })
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("NEXT ROOM [N]"),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::Px(10.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.55, 0.88, 0.96)),
                    ));
                });
            });
        });
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn update_hud(
    time: Res<Time>,
    phase: Res<State<GamePhase>>,
    scenario: Res<CurrentScenario>,
    registry: Res<ScenarioRegistry>,
    score: Res<ScoreState>,
    tracker: Res<FlipTracker>,
    leaderboard: Res<HudLeaderboard>,
    mut hud_state: ResMut<HudState>,
    mut audio: ResMut<RetroAudio>,
    mut camera_rig: ResMut<CameraRig>,
    mut text_q: Query<(&mut Text, &mut TextFont, &mut TextColor, &HudTextRole)>,
    mut node_q: Query<(
        &mut Node,
        &mut BackgroundColor,
        &mut BorderColor,
        Option<&PromptOverlay>,
        Option<&SettlingBanner>,
        Option<&SettledOverlay>,
        Option<&PowerBarFill>,
        Option<&TallyBarFill>,
    )>,
) {
    let dt = time.delta_secs();
    let target = score.total_score as f32;
    hud_state.displayed_score += (target - hud_state.displayed_score) * (1.0 - (-14.0 * dt).exp());
    if (target - hud_state.displayed_score).abs() < 0.8 {
        hud_state.displayed_score = target;
    }

    let is_settled = *phase.get() == GamePhase::Settled;
    if is_settled {
        hud_state.tally_age += dt;
    }

    let tally_age = hud_state.tally_age;
    let row_times = [0.06_f32, 0.19, 0.32, 0.45];
    let verdict_time = 0.62_f32;

    if is_settled {
        while (hud_state.tally_rows_slammed as usize) < row_times.len()
            && tally_age >= row_times[hud_state.tally_rows_slammed as usize]
        {
            let idx = hud_state.tally_rows_slammed;
            hud_state.tally_rows_slammed += 1;
            audio.push(SoundEffect::TallySlam(idx));
            camera_rig.add_trauma(0.09);
        }

        if tally_age < verdict_time && score.total_score > 0 {
            hud_state.tally_tick_accum += dt;
            if hud_state.tally_tick_accum >= 0.055 {
                hud_state.tally_tick_accum = 0.0;
                let progress = (tally_age / verdict_time).clamp(0.0, 1.0);
                audio.push(SoundEffect::TallyTick(progress));
            }
        }

        if !hud_state.tally_verdict_slammed && tally_age >= verdict_time {
            hud_state.tally_verdict_slammed = true;
            let (tier, _, _, _, _) = rank_tier_info(score.total_score);
            audio.push(SoundEffect::RankVerdict(tier));
            let shake = match tier {
                4 => 0.48,
                3 => 0.32,
                2 => 0.18,
                1 => 0.08,
                _ => 0.04,
            };
            camera_rig.add_trauma(shake);
        }
    }

    let bar_progress = if is_settled {
        (tally_age / 0.58).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let animated_total =
        (score.total_score as f32 * (1.0 - (1.0 - bar_progress).powi(2))).round() as u32;
    let (live_tier, _, _, live_bar_col, _) = rank_tier_info(animated_total);
    let (final_tier, rank_name, rank_sub, rank_col, accent_col) = rank_tier_info(score.total_score);

    let row_defs: [(String, String, Color); 4] = [
        (
            format!("LIQUIDATED ASSETS ({}X)", score.items_destroyed),
            format!("+{}", score.destruction_points),
            Color::srgb(0.98, 0.86, 0.34),
        ),
        (
            format!("DOWNSIZED FURNITURE ({}X)", score.items_toppled),
            format!("+{}", score.topple_points),
            Color::srgb(0.52, 0.90, 0.98),
        ),
        (
            format!("SYNERGY CHAIN (X{})", score.max_combo.max(1)),
            if score.combo_bonus > 0 {
                format!("+{}", score.combo_bonus)
            } else {
                "+0".to_string()
            },
            if score.combo_bonus > 0 {
                Color::srgb(1.0, 0.56, 0.32)
            } else {
                Color::srgb(0.52, 0.44, 0.44)
            },
        ),
        (
            score.feat_label.clone(),
            format!("+{}", score.feat_bonus),
            if score.feat_bonus > 0 {
                Color::srgb(0.45, 0.98, 0.62)
            } else {
                Color::srgb(0.66, 0.34, 0.34)
            },
        ),
    ];

    let scen_title = registry.title_for(&scenario.0);
    let pulse_flash = (time.elapsed_secs() * 12.0).sin() > 0.0;

    for (mut text, mut font, mut color, role) in &mut text_q {
        match *role {
            HudTextRole::Scenario => {
                text.0 = format!("{scen_title}  [1-6]");
            }
            HudTextRole::Score => {
                text.0 = format!("SCORE {:05}", hud_state.displayed_score.round() as u32);
                font.font_size = FontSize::Px(18.0 + score.score_pulse * 4.0);
                color.0 = if score.score_pulse > 0.25 {
                    Color::srgb(1.0, 0.86, 0.30)
                } else {
                    Color::srgb(0.98, 0.94, 0.82)
                };
            }
            HudTextRole::Combo => {
                text.0 = if score.combo > 1 {
                    format!("CHAIN x{}", score.combo)
                } else {
                    String::new()
                };
            }
            HudTextRole::ScoreFeed => {
                text.0 = score
                    .recent_feed
                    .iter()
                    .map(|e| e.text.as_str())
                    .collect::<Vec<_>>()
                    .join("\n");
            }
            HudTextRole::TopScoreName(idx) => {
                if let Some(row) = leaderboard.top_scores.get(idx) {
                    let handle = truncate_handle(&row.username, 14);
                    text.0 = format!("{}. {}", idx + 1, handle);
                    color.0 = if row.is_me {
                        Color::srgb(0.55, 0.88, 0.96)
                    } else {
                        Color::srgb(0.92, 0.90, 0.84)
                    };
                } else if idx == 0 && leaderboard.loading {
                    text.0 = "LOADING...".to_string();
                    color.0 = Color::srgb(0.55, 0.62, 0.74);
                } else {
                    text.0 = format!("{}. ---", idx + 1);
                    color.0 = Color::srgb(0.42, 0.46, 0.56);
                }
            }
            HudTextRole::TopScoreValue(idx) => {
                if let Some(row) = leaderboard.top_scores.get(idx) {
                    text.0 = format!("{:05}", row.score);
                    color.0 = if row.is_me {
                        Color::srgb(0.55, 0.88, 0.96)
                    } else {
                        Color::srgb(0.98, 0.82, 0.28)
                    };
                } else {
                    text.0 = "-----".to_string();
                    color.0 = Color::srgb(0.42, 0.46, 0.56);
                }
            }
            HudTextRole::MyHighScore => {
                let live_round = hud_state.displayed_score.round() as u32;
                let best = leaderboard.my_high_score.max(live_round);
                text.0 = format!("{:05}", best);
                color.0 = if live_round > leaderboard.my_high_score && live_round > 0 {
                    Color::srgb(0.45, 0.98, 0.62)
                } else {
                    Color::srgb(1.0, 0.86, 0.30)
                };
            }
            HudTextRole::PromptHint => {
                if tracker.dragging && tracker.live_power > 0.04 {
                    let a = tracker.live_aim;
                    text.0 = if a < -0.45 {
                        "<<< HARD LEFT FLANK!".to_string()
                    } else if a < -0.16 {
                        "<< AIMING LEFT".to_string()
                    } else if a > 0.45 {
                        "HARD RIGHT FLANK! >>>".to_string()
                    } else if a > 0.16 {
                        "AIMING RIGHT >>".to_string()
                    } else {
                        "^ STRAIGHT DOWN CENTER ^".to_string()
                    };
                } else {
                    text.0 = "SWIPE UP + ANGLE TO AIM & FLIP!".to_string();
                }
            }
            HudTextRole::Settling => {
                text.0 = if score.settle_timer > 0.05 || score.elapsed_since_flip > 2.0 {
                    "CALIBRATING...".to_string()
                } else {
                    "RESTRUCTURING IN PROGRESS...".to_string()
                };
            }
            HudTextRole::TallyHeader => {
                if is_settled {
                    if hud_state.tally_verdict_slammed && final_tier == 0 {
                        text.0 = "HR PERFORMANCE PIP // FAIL".to_string();
                        color.0 = Color::srgb(0.76, 0.34, 0.34);
                    } else {
                        text.0 = "ANNUAL IMPACT CALIBRATION".to_string();
                        color.0 = Color::srgb(0.95, 0.75, 0.25);
                    }
                }
            }
            HudTextRole::TallyRowLabel(idx) => {
                if is_settled && tally_age >= row_times[idx] {
                    let local_t = ((tally_age - row_times[idx]) / 0.10).clamp(0.0, 1.0);
                    let slam_scale = 1.40 - 0.40 * local_t;
                    text.0 = row_defs[idx].0.clone();
                    font.font_size = FontSize::Px(9.5 * slam_scale);
                    color.0 = if final_tier == 0 && idx >= 2 {
                        Color::srgb(0.62, 0.50, 0.50)
                    } else {
                        Color::srgb(0.88, 0.92, 0.96)
                    };
                } else {
                    text.0.clear();
                }
            }
            HudTextRole::TallyRowValue(idx) => {
                if is_settled && tally_age >= row_times[idx] {
                    let local_t = ((tally_age - row_times[idx]) / 0.10).clamp(0.0, 1.0);
                    let slam_scale = 1.55 - 0.55 * local_t;
                    text.0 = row_defs[idx].1.clone();
                    font.font_size = FontSize::Px(11.0 * slam_scale);
                    color.0 = row_defs[idx].2;
                } else {
                    text.0.clear();
                }
            }
            HudTextRole::TallyTier(tier_id) => {
                if is_settled {
                    let active = tier_id == live_tier;
                    font.font_size = FontSize::Px(if active { 10.5 } else { 8.5 });
                    color.0 = if active {
                        live_bar_col
                    } else if tier_id < live_tier {
                        Color::srgb(0.76, 0.80, 0.88)
                    } else {
                        Color::srgb(0.42, 0.46, 0.56)
                    };
                }
            }
            HudTextRole::TallyTotal => {
                if is_settled {
                    text.0 = format!("{:05}", animated_total);
                    color.0 = if final_tier == 0 && hud_state.tally_verdict_slammed {
                        Color::srgb(0.72, 0.36, 0.36)
                    } else {
                        live_bar_col
                    };
                }
            }
            HudTextRole::TallyStamp => {
                if is_settled && hud_state.tally_verdict_slammed {
                    let local_t = ((tally_age - verdict_time) / 0.12).clamp(0.0, 1.0);
                    let slam_scale = 1.85 - 0.85 * local_t;
                    let base_px = if final_tier == 0 { 13.5 } else { 15.0 };
                    text.0 = rank_name.to_string();
                    font.font_size = FontSize::Px(base_px * slam_scale);
                    color.0 = if final_tier == 4 && pulse_flash {
                        accent_col
                    } else {
                        rank_col
                    };
                } else {
                    text.0.clear();
                }
            }
            HudTextRole::TallySub => {
                if is_settled && hud_state.tally_verdict_slammed {
                    text.0 = rank_sub.to_string();
                    color.0 = if final_tier == 0 {
                        Color::srgb(0.68, 0.48, 0.48)
                    } else {
                        Color::srgb(0.84, 0.88, 0.94)
                    };
                } else {
                    text.0.clear();
                }
            }
        }
    }

    for (mut node, mut bg, mut border, is_prompt, is_settling, is_settled_card, is_power, is_t_bar) in
        &mut node_q
    {
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
        } else if is_settled_card.is_some() {
            node.display = if is_settled {
                Display::Flex
            } else {
                Display::None
            };
            if is_settled {
                if hud_state.tally_verdict_slammed {
                    if final_tier == 0 {
                        *bg = BackgroundColor(Color::srgba(0.08, 0.05, 0.06, 0.96));
                        *border = BorderColor::all(Color::srgb(0.52, 0.22, 0.24));
                    } else if final_tier == 4 {
                        *bg = BackgroundColor(Color::srgba(0.05, 0.04, 0.10, 0.96));
                        *border = BorderColor::all(if pulse_flash { rank_col } else { accent_col });
                    } else {
                        *bg = BackgroundColor(Color::srgba(0.04, 0.05, 0.08, 0.95));
                        *border = BorderColor::all(rank_col);
                    }
                } else {
                    *bg = BackgroundColor(Color::srgba(0.04, 0.05, 0.08, 0.95));
                    *border = BorderColor::all(Color::srgb(0.95, 0.75, 0.25));
                }
            }
        } else if is_power.is_some() {
            let pct = (tracker.live_power * 100.0).clamp(0.0, 100.0);
            node.width = Val::Percent(pct);
            *bg = BackgroundColor(if tracker.live_power < 0.20 {
                Color::srgb(0.72, 0.34, 0.30)
            } else if tracker.live_power < 0.60 {
                Color::srgb(0.96, 0.76, 0.24)
            } else {
                Color::srgb(0.36, 0.95, 0.64)
            });
        } else if is_t_bar.is_some() && is_settled {
            let bar_pct = score_to_bar_percent(animated_total);
            node.width = Val::Percent(bar_pct);
            *bg = BackgroundColor(if live_tier == 4 && pulse_flash {
                Color::srgb(1.0, 0.88, 0.28)
            } else {
                live_bar_col
            });
        }
    }
}

pub fn spawn_score_popups(
    mut commands: Commands,
    mut pending: ResMut<PendingPopups>,
    mut hud_state: ResMut<HudState>,
    pixel_state: Res<PixelRenderState>,
    cam_q: Query<(&Camera, &GlobalTransform), With<SceneCamera3d>>,
    win_q: Query<&Window, With<PrimaryWindow>>,
) {
    if pending.0.is_empty() {
        return;
    }
    let Ok((camera, cam_tf)) = cam_q.single() else {
        pending.0.clear();
        return;
    };
    let (win_w, win_h) = win_q
        .single()
        .map(|w| (w.width().max(320.0), w.height().max(240.0)))
        .unwrap_or((960.0, 540.0));

    let scale_x = win_w / (pixel_state.pixel_width.max(1) as f32);
    let scale_y = win_h / (pixel_state.pixel_height.max(1) as f32);
    let max_x = (win_w - 220.0).max(80.0);
    let max_y = (win_h - 64.0).max(80.0);

    for (batch_idx, item) in pending.0.drain(..).enumerate() {
        if let Ok(low_pos) = camera.world_to_viewport(cam_tf, item.world_pos) {
            hud_state.popup_seq = hud_state.popup_seq.wrapping_add(1);
            let stagger_y = (batch_idx as f32) * -24.0;
            let screen_pos = Vec2::new(
                (low_pos.x * scale_x).clamp(48.0, max_x),
                (low_pos.y * scale_y + stagger_y).clamp(76.0, max_y),
            );
            let dir_x = if hud_state.popup_seq % 2 == 0 { 1.0 } else { -1.0 };
            let vx = dir_x * (18.0 + (hud_state.popup_seq % 4) as f32 * 8.0);
            let vy = if item.big { -128.0 } else { -98.0 };
            let font_size = if item.big { 14.0 } else { 10.5 };

            commands.spawn((
                FloatingPopup {
                    origin_screen: screen_pos,
                    vel_screen: Vec2::new(vx, vy),
                    age: 0.0,
                    duration: 1.05,
                    base_font_size: font_size,
                    base_color: item.color,
                },
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(screen_pos.x - 70.0),
                    top: Val::Px(screen_pos.y - 12.0),
                    ..default()
                },
                Text::new(item.text),
                TextFont {
                    font: hud_state.pixel_font.clone().into(),
                    font_size: FontSize::Px(font_size),
                    ..default()
                },
                TextColor(item.color),
                retro_shadow(),
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
        let a = popup.age;
        let t = (a / popup.duration).clamp(0.0, 1.0);

        let pos_x = popup.origin_screen.x + popup.vel_screen.x * a;
        let pos_y = popup.origin_screen.y + popup.vel_screen.y * a + 0.5 * 145.0 * a * a;
        node.left = Val::Px(pos_x - 70.0);
        node.top = Val::Px(pos_y - 12.0);

        let pop_scale = if t < 0.15 {
            0.75 + (t / 0.15) * 0.50
        } else {
            1.25 - ((t - 0.15) / 0.85) * 0.25
        };
        font.font_size = FontSize::Px(popup.base_font_size * pop_scale);

        let alpha = if t > 0.68 {
            1.0 - ((t - 0.68) / 0.32)
        } else {
            1.0
        };
        color.0 = popup.base_color.with_alpha(alpha.clamp(0.0, 1.0));

        if popup.age >= popup.duration {
            commands.entity(entity).despawn();
        }
    }
}
