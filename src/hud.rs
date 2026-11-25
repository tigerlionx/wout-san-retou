//! The interface: the game on the left, a side panel on the right.
//!
//! The 3D camera renders into the left part of the window only; a second, 2D camera draws the
//! interface over the whole window. Nothing but a small prompt and the crosshair ever sits on top
//! of the game. `update_hud` (main.rs) fills `HudData`; `apply_hud` puts it on screen.
use crate::*;
use bevy::camera::{CameraOutputMode, ClearColorConfig, Viewport};
use bevy::render::render_resource::BlendState;
use bevy::text::{Font, FontSource, LineHeight};
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::ui::IsDefaultUiCamera;

/// Width of the side panel in logical pixels.
pub const PANEL_W: f32 = 380.0;

const PANEL_BG: Color = Color::srgb(0.045, 0.05, 0.055);
// dark glass: the billboard shows between the cards and is dimmed behind them, so text stays readable
const CARD_BG: Color = Color::srgba(0.05, 0.055, 0.06, 0.8);
const AMBER: Color = Color::srgb(0.96, 0.77, 0.34);
const TEXT: Color = Color::srgb(0.94, 0.95, 0.93);
const DIM: Color = Color::srgb(0.6, 0.65, 0.65);
const BAR_BG: Color = Color::srgb(0.13, 0.14, 0.15);

#[derive(Component)]
pub struct UiCamera;

/// The poison clock at the top of the game view, and the big card that announces the stab.
#[derive(Component)]
pub struct CountdownBox;
#[derive(Component)]
pub struct CountdownTime;
#[derive(Component)]
pub struct AlertBox;

/// What each interface element shows.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    Level,
    Clock,
    HealthVal,
    HealthBar,
    WeightVal,
    WeightBar,
    NoticeVal,
    NoticeBar,
    Weapon,
    Ammo,
    Grenades,
    Goals,
    Message,
    MsgBox,
    Prompt,
    PromptPill,
    Controls,
}

/// Everything the panel shows, filled in by `update_hud`.
#[derive(Resource, Default)]
pub struct HudData {
    pub clock: String,
    pub health: f32,
    pub weight: f32,
    pub noticed: f32,
    pub notice_label: String,
    pub stance: String,
    pub weapon: String,
    pub ammo: String,
    pub grenades: u32,
    pub reloading: bool,
    pub goals: String,
    pub prompt: String,
    pub message: String,
}

/// The billboard behind the panel: lwa emblems and vèvè in lights, changing as you scroll (see `billboard`).
const SIGILS: [(&str, &str); 12] = [
    ("legba", "Papa Legba · opens the gate"),
    ("danbala", "Danbala and Ayida · the two snakes"),
    ("ogou", "Ogou · iron, war and work"),
    ("erzulie", "Erzulie Freda · love and beauty"),
    ("saint", "The saint's halo · the lwa beside the saints"),
    ("baron", "Baron Samedi · the cross at the cemetery gate"),
    ("agwe", "Agwe · king of the sea"),
    ("asson", "The asson and the drum · they call the lwa"),
    ("dantor", "Erzulie Dantò · the fierce mother"),
    ("wanga", "A wanga · a charm, bound and blessed"),
    ("simbi", "Simbi · the snake of springs and rivers"),
    ("govi", "A govi · where a lwa rests"),
];

#[derive(Component)]
pub struct BillShell;
#[derive(Component)]
pub struct BillLayer;
#[derive(Component)]
pub struct BillBase;
#[derive(Component)]
pub struct BillNext;
#[derive(Component)]
pub struct BillWipe;
#[derive(Component)]
pub struct BillBar;
#[derive(Component)]
pub struct ScrollBox;
#[derive(Component)]
pub struct Caption;

#[derive(Resource)]
pub struct Billboard {
    images: Vec<Handle<Image>>,
    idx: usize,
    /// 0..1 while a new picture is being revealed from the top down
    wipe: Option<f32>,
    auto: f32,
    accum: f32,
}

const SIGIL_ALPHA: f32 = 0.42;

fn font(handle: &Handle<Font>, size: f32) -> TextFont {
    TextFont {
        font: FontSource::Handle(handle.clone()),
        font_size: FontSize::Px(size),
        ..default()
    }
}

pub fn spawn_hud(mut commands: Commands, assets: Res<AssetServer>) {
    let regular: Handle<Font> = assets.load("fonts/FiraSans-Regular.ttf");
    let bold: Handle<Font> = assets.load("fonts/FiraSans-Bold.ttf");
    let italic: Handle<Font> = assets.load("fonts/FiraSans-Italic.ttf");
    let sigils: Vec<Handle<Image>> = SIGILS.iter().map(|(n, _)| assets.load(format!("veve/{n}.png"))).collect();
    let led: Handle<Image> = assets.load("veve/led.png");
    commands.insert_resource(Billboard {
        images: sigils.clone(),
        idx: 0,
        wipe: None,
        auto: 0.0,
        accum: 0.0,
    });

    // The camera that draws all interface, over the whole window.
    commands.spawn((
        UiCamera,
        Camera2d,
        Camera {
            order: 10,
            // transparent, and written over the game with alpha blending: only the interface's own pixels land
            clear_color: ClearColorConfig::Custom(Color::NONE),
            output_mode: CameraOutputMode::Write {
                blend_state: Some(BlendState::ALPHA_BLENDING),
                clear_color: ClearColorConfig::None,
            },
            ..default()
        },
        IsDefaultUiCamera,
    ));

    commands
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Row,
                ..default()
            },
            // under the full-screen text cards, which are spawned later and cover everything
            GlobalZIndex(-5),
        ))
        .with_children(|root| {
            // ----------------------------------------------------------- the game side
            root.spawn(Node {
                flex_grow: 1.0,
                height: percent(100),
                position_type: PositionType::Relative,
                ..default()
            })
            .with_children(|game| {
                // red flash when hurt, purple when drugged
                game.spawn((
                    HurtOverlay,
                    Node {
                        position_type: PositionType::Absolute,
                        width: percent(100),
                        height: percent(100),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.7, 0.02, 0.02, 0.0)),
                ));
                // first-person crosshair
                game.spawn((
                    Crosshair,
                    Text::new("+"),
                    font(&bold, 28.0),
                    TextColor(Color::WHITE),
                    Node {
                        position_type: PositionType::Absolute,
                        left: percent(50),
                        top: percent(50),
                        margin: UiRect {
                            left: px(-8),
                            top: px(-18),
                            ..default()
                        },
                        ..default()
                    },
                    Visibility::Hidden,
                ));
                // the poison clock: top centre of the game view; white, red for the last 30 seconds
                game.spawn(Node {
                    position_type: PositionType::Absolute,
                    top: px(16),
                    left: px(0),
                    right: px(0),
                    justify_content: JustifyContent::Center,
                    ..default()
                })
                .with_children(|row| {
                    row.spawn((
                        CountdownBox,
                        Node {
                            display: Display::None,
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            padding: UiRect::axes(px(26), px(8)),
                            border: UiRect::all(px(2)),
                            border_radius: BorderRadius::all(px(12)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.74)),
                        BorderColor::all(Color::WHITE),
                        children![
                            (Text::new("POISONED"), font(&bold, 13.0), TextColor(DIM)),
                            (CountdownTime, Text::new("2:00"), font(&bold, 48.0), TextColor(Color::WHITE)),
                            (Text::new("Find the potion room"), font(&regular, 14.0), TextColor(TEXT)),
                        ],
                    ));
                });
                // the stabbed card, a few seconds after the blade goes in
                game.spawn(Node {
                    position_type: PositionType::Absolute,
                    top: percent(22),
                    left: px(0),
                    right: px(0),
                    justify_content: JustifyContent::Center,
                    ..default()
                })
                .with_children(|row| {
                    row.spawn((
                        AlertBox,
                        Node {
                            display: Display::None,
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            row_gap: px(8),
                            max_width: percent(72),
                            padding: UiRect::axes(px(34), px(22)),
                            border: UiRect::all(px(2)),
                            border_radius: BorderRadius::all(px(14)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.05, 0.0, 0.0, 0.88)),
                        BorderColor::all(Color::srgb(0.9, 0.15, 0.12)),
                        children![
                            (Text::new("YOU HAVE BEEN STABBED"), font(&bold, 30.0), TextColor(Color::srgb(1.0, 0.3, 0.25))),
                            (
                                Text::new("Granmèt Nwit's blade was poisoned.\nYou have 2 minutes to find the potion room and drink the cure.\nIf the clock reaches zero, you die."),
                                font(&regular, 20.0),
                                TextColor(TEXT),
                                TextLayout::justify(Justify::Center),
                            ),
                            (Text::new("The altar is in the north. Candles and notes will help."), font(&italic, 15.0), TextColor(DIM)),
                        ],
                    ));
                });
                // the one thing allowed on the game: what E does right now
                game.spawn(Node {
                    position_type: PositionType::Absolute,
                    bottom: px(26),
                    left: px(0),
                    right: px(0),
                    justify_content: JustifyContent::Center,
                    ..default()
                })
                .with_children(|row| {
                    row.spawn((
                        Slot::PromptPill,
                        Node {
                            display: Display::None,
                            padding: UiRect::axes(px(20), px(10)),
                            border: UiRect::all(px(1)),
                            border_radius: BorderRadius::all(px(10)),
                            max_width: percent(80),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.74)),
                        BorderColor::all(AMBER),
                        children![(Slot::Prompt, Text::new(""), font(&bold, 19.0), TextColor(TEXT))],
                    ));
                });
            });

            // ---------------------------------------------------------- the panel side
            root.spawn((
                BillShell,
                Node {
                    width: px(PANEL_W),
                    height: percent(100),
                    border: UiRect::left(px(2)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(PANEL_BG),
                BorderColor::all(Color::srgb(0.32, 0.26, 0.12)),
            ))
            .with_children(|shell| {
                // the picture on show
                shell.spawn(sigil_layer(sigils[0].clone(), BillBase));
                // the next picture, revealed from the top down behind a scanning line
                shell
                    .spawn((
                        BillWipe,
                        Node {
                            position_type: PositionType::Absolute,
                            top: px(0),
                            left: px(0),
                            width: percent(100),
                            height: percent(0),
                            overflow: Overflow::clip(),
                            ..default()
                        },
                    ))
                    .with_children(|w| {
                        w.spawn(sigil_layer(sigils[0].clone(), BillNext));
                    });
                shell.spawn((
                    BillBar,
                    Node {
                        position_type: PositionType::Absolute,
                        top: px(0),
                        left: px(0),
                        width: percent(100),
                        height: px(3),
                        display: Display::None,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(1.0, 0.86, 0.5, 0.9)),
                ));
                // the bulbs: a grid of thin dark gaps over the pictures
                shell.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        top: px(0),
                        left: px(0),
                        width: percent(100),
                        height: percent(100),
                        ..default()
                    },
                    ImageNode {
                        image: led.clone(),
                        image_mode: NodeImageMode::Tiled {
                            tile_x: true,
                            tile_y: true,
                            stretch_value: 1.0,
                        },
                        ..default()
                    },
                ));

                shell
                    .spawn((
                        ScrollBox,
                        Node {
                            position_type: PositionType::Absolute,
                            top: px(0),
                            left: px(0),
                            width: percent(100),
                            height: percent(100),
                            overflow: Overflow::scroll_y(),
                            ..default()
                        },
                        ScrollPosition::default(),
                    ))
                    .with_children(|sb| {
                        sb.spawn(Node {
                            width: percent(100),
                            min_height: percent(100),
                            flex_shrink: 0.0,
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(px(18)),
                            row_gap: px(12),
                            ..default()
                        })
                        .with_children(|panel| {
                // header: the game, the clock, where you are
                panel
                    .spawn(Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: px(2),
                        ..default()
                    })
                    .with_children(|h| {
                        h.spawn(Node {
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            ..default()
                        })
                        .with_children(|r| {
                            r.spawn((Text::new("WOUT SAN RETOU"), font(&bold, 13.0), TextColor(AMBER)));
                            r.spawn((Slot::Clock, Text::new(""), font(&bold, 15.0), TextColor(TEXT)));
                        });
                        h.spawn((Slot::Level, Text::new(""), font(&bold, 26.0), TextColor(TEXT)));
                        h.spawn((Caption, Text::new(SIGILS[0].1), font(&italic, 12.0), TextColor(AMBER)));
                    });
                divider(panel);

                // vitals
                vital(panel, &bold, &regular, "HEALTH", Slot::HealthVal, Slot::HealthBar, percent(100));
                panel
                    .spawn(Node {
                        column_gap: px(16),
                        ..default()
                    })
                    .with_children(|row| {
                        vital(row, &bold, &regular, "WEIGHT", Slot::WeightVal, Slot::WeightBar, percent(50));
                        vital(row, &bold, &regular, "NOTICED", Slot::NoticeVal, Slot::NoticeBar, percent(50));
                    });

                // weapon
                panel
                    .spawn((
                        Node {
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            padding: UiRect::axes(px(14), px(10)),
                            border_radius: BorderRadius::all(px(10)),
                            ..default()
                        },
                        BackgroundColor(CARD_BG),
                    ))
                    .with_children(|w| {
                        w.spawn(Node {
                            flex_direction: FlexDirection::Column,
                            ..default()
                        })
                        .with_children(|c| {
                            c.spawn((Slot::Weapon, Text::new(""), font(&bold, 17.0), TextColor(TEXT)));
                            c.spawn((Slot::Grenades, Text::new(""), font(&regular, 13.0), TextColor(DIM)));
                        });
                        w.spawn((Slot::Ammo, Text::new(""), font(&bold, 30.0), TextColor(AMBER)));
                    });

                // objectives
                heading(panel, &bold, "OBJECTIVES");
                panel
                    .spawn((
                        Node {
                            padding: UiRect::axes(px(12), px(10)),
                            border_radius: BorderRadius::all(px(8)),
                            ..default()
                        },
                        BackgroundColor(CARD_BG),
                    ))
                    .with_children(|g| {
                        g.spawn((Slot::Goals, Text::new(""), font(&regular, 16.0), TextColor(TEXT)));
                    });

                // the journal: what the game is telling you
                panel
                    .spawn((
                        Slot::MsgBox,
                        Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: px(4),
                            padding: UiRect::all(px(12)),
                            border: UiRect::left(px(3)),
                            border_radius: BorderRadius::all(px(8)),
                            ..default()
                        },
                        BackgroundColor(CARD_BG),
                        BorderColor::all(AMBER),
                    ))
                    .with_children(|m| {
                        m.spawn((Text::new("JOURNAL"), font(&bold, 12.0), TextColor(DIM)));
                        m.spawn((Slot::Message, Text::new(""), font(&italic, 16.0), TextColor(TEXT)));
                    });

                // pushes the controls to the bottom
                panel.spawn(Node {
                    flex_grow: 1.0,
                    ..default()
                });

                // controls
                panel
                    .spawn((
                        Slot::Controls,
                        Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: px(6),
                            padding: UiRect::axes(px(12), px(10)),
                            border_radius: BorderRadius::all(px(8)),
                            ..default()
                        },
                        BackgroundColor(CARD_BG),
                    ))
                    .with_children(|c| {
                        heading(c, &bold, "CONTROLS   (F1 hides)");
                        c.spawn(Node {
                            flex_wrap: FlexWrap::Wrap,
                            row_gap: px(4),
                            ..default()
                        })
                        .with_children(|grid| {
                            for (key, what) in [
                                ("WASD", "move"),
                                ("Shift", "run"),
                                ("C", "crouch"),
                                ("Z", "prone"),
                                ("Space", "jump"),
                                ("Mouse", "look"),
                                ("RMB", "aim"),
                                ("LMB", "fire"),
                                ("1 2 3 4", "weapons"),
                                ("R", "reload"),
                                ("G", "grenade"),
                                ("F", "knife"),
                                ("E", "use"),
                                ("V", "camera"),
                                ("H", "drink"),
                                ("F9", "note"),
                            ] {
                                grid.spawn(Node {
                                    width: percent(33.3),
                                    column_gap: px(6),
                                    align_items: AlignItems::Baseline,
                                    ..default()
                                })
                                .with_children(|item| {
                                    item.spawn((
                                        Node {
                                            min_width: px(40),
                                            ..default()
                                        },
                                        Text::new(key),
                                        font(&bold, 13.0),
                                        TextColor(AMBER),
                                    ));
                                    item.spawn((Text::new(what), font(&regular, 12.5), TextColor(DIM)));
                                });
                            }
                        });
                    });
                        });
                    });
            });
        });
}

/// A full-height layer with one square picture in the middle, tagged so the billboard can find it.
fn sigil_layer(img: Handle<Image>, tag: impl Bundle) -> impl Bundle {
    (
        BillLayer,
        Node {
            position_type: PositionType::Absolute,
            top: px(0),
            left: px(0),
            width: percent(100),
            height: px(800),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        children![(
            tag,
            Node {
                width: percent(100),
                aspect_ratio: Some(1.0),
                ..default()
            },
            ImageNode::new(img).with_color(Color::srgba(1.0, 1.0, 1.0, SIGIL_ALPHA)),
        )],
    )
}

fn divider(parent: &mut ChildSpawnerCommands) {
    parent.spawn((
        Node {
            height: px(1),
            width: percent(100),
            ..default()
        },
        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.09)),
    ));
}

fn heading(parent: &mut ChildSpawnerCommands, bold: &Handle<Font>, text: &str) {
    parent.spawn((Text::new(text), font(bold, 12.0), TextColor(DIM)));
}

fn vital(parent: &mut ChildSpawnerCommands, bold: &Handle<Font>, regular: &Handle<Font>, name: &str, val: Slot, bar: Slot, width: Val) {
    parent
        .spawn(Node {
            width,
            flex_direction: FlexDirection::Column,
            row_gap: px(5),
            ..default()
        })
        .with_children(|v| {
            v.spawn(Node {
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Baseline,
                column_gap: px(8),
                ..default()
            })
            .with_children(|r| {
                r.spawn((Text::new(name), font(bold, 12.0), TextColor(DIM)));
                r.spawn((val, Text::new(""), font(regular, 15.0), TextColor(TEXT)));
            });
            v.spawn((
                Node {
                    height: px(9),
                    width: percent(100),
                    border_radius: BorderRadius::all(px(5)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(BAR_BG),
            ))
            .with_children(|b| {
                b.spawn((
                    bar,
                    Node {
                        height: percent(100),
                        width: percent(0),
                        ..default()
                    },
                    BackgroundColor(TEXT),
                ));
            });
        });
}

/// The game fills everything left of the panel.
pub fn layout_viewport(
    ui: Single<&Camera, (With<UiCamera>, Without<GameCamera>)>,
    mut game: Single<&mut Camera, (With<GameCamera>, Without<UiCamera>)>,
) {
    if crate::poster::on() {
        return; // key art: the game fills the whole picture
    }
    let (Some(size), Some(scale)) = (ui.physical_target_size(), ui.target_scaling_factor()) else {
        return;
    };
    let panel = (PANEL_W * scale).round() as u32;
    let size = UVec2::new(size.x.saturating_sub(panel).max(64), size.y);
    if game.viewport.as_ref().is_none_or(|v| v.physical_size != size) {
        game.viewport = Some(Viewport {
            physical_position: UVec2::ZERO,
            physical_size: size,
            ..default()
        });
    }
}

fn level_title(state: &AppState) -> &'static str {
    match state {
        AppState::Playing => "Level 1 · The Yard",
        AppState::Level2 => "Level 2 · The House",
        AppState::Level3 => "Level 3 · The Party Wing",
        AppState::Level4 => "Level 4 · The Gallery",
        AppState::Level5 => "Level 5 · The Hall",
        AppState::Level6 => "Level 6 · Nwit's Quarters",
        AppState::Level7 => "Level 7 · The Rooftop",
        _ => "",
    }
}

fn ramp(t: f32, low: Color, mid: Color, high: Color) -> Color {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        low.mix(&mid, t * 2.0)
    } else {
        mid.mix(&high, (t - 0.5) * 2.0)
    }
}

/// Check boxes stay ASCII (Fira Sans has no check-mark glyphs); the leading spaces go, and the level
/// status lines that run several facts together with wide gaps become one fact per line.
fn tidy_goals(goals: &str) -> String {
    goals
        .lines()
        .flat_map(|l| l.split("   "))
        .map(|l| {
            let l = l.trim();
            if let Some(rest) = l.strip_prefix("[x] ") {
                format!("[x]  {rest}")
            } else if let Some(rest) = l.strip_prefix("[ ] ") {
                format!("[  ]  {rest}")
            } else {
                l.to_string()
            }
        })
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn apply_hud(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    data: Res<HudData>,
    state: Res<State<AppState>>,
    mut texts: Query<(&Slot, &mut Text)>,
    mut nodes: Query<(&Slot, &mut Node, &mut BackgroundColor), Without<Text>>,
    mut last_msg: Local<String>,
    mut flash: Local<f32>,
    mut controls_hidden: Local<bool>,
) {
    if keys.just_pressed(KeyCode::F1) {
        *controls_hidden = !*controls_hidden;
    }
    if data.message != *last_msg {
        *last_msg = data.message.clone();
        *flash = 1.0;
    }
    *flash = (*flash - time.delta_secs() * 0.5).max(0.0);

    let weapon = if data.reloading { format!("{}  ·  reloading", data.weapon) } else { data.weapon.clone() };
    let vals = |slot: Slot| -> String {
        match slot {
            Slot::Level => level_title(state.get()).to_string(),
            Slot::Clock => data.clock.clone(),
            Slot::HealthVal => format!("{:.0}", data.health),
            Slot::WeightVal => format!("{:.0} / 100", data.weight),
            Slot::NoticeVal => format!("{:.0}% {}", data.noticed * 100.0, data.notice_label),
            Slot::Weapon => weapon.clone(),
            Slot::Ammo => data.ammo.clone(),
            Slot::Grenades => format!("Grenades {}   ·   {}", data.grenades, data.stance),
            Slot::Goals => tidy_goals(&data.goals),
            Slot::Message => data.message.clone(),
            Slot::Prompt => data.prompt.clone(),
            _ => String::new(),
        }
    };
    for (slot, mut text) in &mut texts {
        let s = vals(*slot);
        if text.0 != s {
            text.0 = s;
        }
    }
    for (slot, mut node, mut bg) in &mut nodes {
        match slot {
            Slot::HealthBar => {
                let t = (data.health / 100.0).clamp(0.0, 1.0);
                node.width = percent(t * 100.0);
                bg.0 = ramp(1.0 - t, Color::srgb(0.45, 0.82, 0.52), Color::srgb(0.95, 0.76, 0.3), Color::srgb(0.9, 0.2, 0.2));
            }
            Slot::WeightBar => {
                let t = (data.weight / 100.0).clamp(0.0, 1.0);
                node.width = percent(t * 100.0);
                bg.0 = ramp(t, Color::srgb(0.62, 0.55, 0.9), Color::srgb(0.75, 0.45, 0.85), Color::srgb(0.9, 0.25, 0.4));
            }
            Slot::NoticeBar => {
                let t = data.noticed.clamp(0.0, 1.0);
                node.width = percent(t * 100.0);
                bg.0 = ramp(t, Color::srgb(0.55, 0.65, 0.65), Color::srgb(0.95, 0.76, 0.3), Color::srgb(0.95, 0.2, 0.2));
            }
            Slot::MsgBox => {
                bg.0 = CARD_BG.mix(&Color::srgba(0.96, 0.77, 0.34, 0.2), *flash);
            }
            Slot::PromptPill => {
                node.display = if data.prompt.is_empty() { Display::None } else { Display::Flex };
            }
            Slot::Controls => {
                node.display = if *controls_hidden { Display::None } else { Display::Flex };
            }
            _ => {}
        }
    }
}

/// The full-screen story cards (pitch, level complete, ending) read better in a proportional font,
/// a bit larger, with more air between the lines, in a column that does not run edge to edge.
pub fn style_cards(assets: Res<AssetServer>, mut commands: Commands, cards: Query<Entity, Added<crate::ScreenText>>) {
    for e in &cards {
        commands.entity(e).insert((
            TextFont {
                font: FontSource::Handle(assets.load("fonts/FiraSans-Regular.ttf")),
                font_size: FontSize::Px(28.0),
                ..default()
            },
            LineHeight::RelativeToFont(1.45),
            TextColor(TEXT),
            // dark glass under the words, so the kaleidoscope behind never fights them
            Node {
                max_width: px(820),
                padding: UiRect::axes(px(36), px(28)),
                border_radius: BorderRadius::all(px(18)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.02, 0.025, 0.62)),
        ));
    }
}

/// The billboard: a picture of the lwa and their signs glows behind the panel and changes the way a
/// lit sign does, a bright line sweeping down and painting the next one. Scrolling the panel with the
/// mouse wheel (or waiting nine seconds) brings the next picture; scrolling back brings the last.
#[allow(clippy::too_many_arguments)]
pub fn billboard(
    time: Res<Time>,
    mut wheel: MessageReader<MouseWheel>,
    mut bb: ResMut<Billboard>,
    shell: Single<Entity, With<BillShell>>,
    scrollbox: Single<Entity, With<ScrollBox>>,
    wipe: Single<Entity, With<BillWipe>>,
    bar: Single<Entity, With<BillBar>>,
    base: Single<Entity, With<BillBase>>,
    next: Single<Entity, With<BillNext>>,
    caption: Single<Entity, With<Caption>>,
    layers: Query<Entity, With<BillLayer>>,
    mut nodes: Query<&mut Node>,
    computed: Query<&ComputedNode>,
    mut scroll: Query<&mut ScrollPosition>,
    mut images: Query<&mut ImageNode>,
    mut texts: Query<&mut Text>,

) {
    let dt = time.delta_secs().min(0.1);
    let h = computed.get(*shell).map(|c| c.size().y * c.inverse_scale_factor()).unwrap_or(800.0);
    if h < 8.0 {
        return;
    }
    for l in &layers {
        if let Ok(mut n) = nodes.get_mut(l) {
            if n.height != px(h) {
                n.height = px(h);
            }
        }
    }

    // scroll the panel, and count the wheel
    let max_off = computed
        .get(*scrollbox)
        .map(|c| ((c.content_size().y - c.size().y) * c.inverse_scale_factor()).max(0.0))
        .unwrap_or(0.0);
    let mut dir = 0i32;
    for ev in wheel.read() {
        let mut dy = -ev.y;
        if ev.unit == MouseScrollUnit::Line {
            dy *= 40.0;
        }
        if let Ok(mut sp) = scroll.get_mut(*scrollbox) {
            sp.y = (sp.y + dy).clamp(0.0, max_off);
        }
        bb.accum += dy;
        if bb.accum.abs() > 90.0 {
            dir = bb.accum.signum() as i32;
            bb.accum = 0.0;
        }
    }
    bb.auto += dt;
    if bb.auto > 9.0 && dir == 0 {
        dir = 1;
    }

    let n = bb.images.len();
    if dir != 0 {
        bb.auto = 0.0;
        if bb.wipe.take().is_some() {
            // a new request while one is painting: the last one lands at once
            if let Ok(mut i) = images.get_mut(*base) {
                i.image = bb.images[bb.idx].clone();
            }
        }
        bb.idx = (bb.idx + n).wrapping_add_signed(dir as isize) % n;
        if let Ok(mut i) = images.get_mut(*next) {
            i.image = bb.images[bb.idx].clone();
        }
        if let Ok(mut t) = texts.get_mut(*caption) {
            t.0 = SIGILS[bb.idx].1.to_string();
        }
        bb.wipe = Some(0.0);
    }

    if let Some(t) = bb.wipe {
        let t = t + dt / 1.1;
        if t >= 1.0 {
            if let Ok(mut i) = images.get_mut(*base) {
                i.image = bb.images[bb.idx].clone();
            }
            if let Ok(mut w) = nodes.get_mut(*wipe) {
                w.height = percent(0);
            }
            if let Ok(mut b) = nodes.get_mut(*bar) {
                b.display = Display::None;
            }
            bb.wipe = None;
        } else {
            let e = t * t * (3.0 - 2.0 * t);
            if let Ok(mut w) = nodes.get_mut(*wipe) {
                w.height = px(e * h);
            }
            if let Ok(mut b) = nodes.get_mut(*bar) {
                b.display = Display::Flex;
                b.top = px((e * h - 3.0).max(0.0));
            }
            bb.wipe = Some(t);
        }
    }

    // a lit sign never holds perfectly steady
    let s = time.elapsed_secs();
    let flick = SIGIL_ALPHA * (1.0 + 0.07 * (s * 9.0).sin() + 0.04 * (s * 23.0).sin());
    if let Ok(mut i) = images.get_mut(*base) {
        i.color = Color::srgba(1.0, 1.0, 1.0, flick);
    }
}

/// A card taller than the window (the disclaimer, in two languages) shrinks its type until it fits.
pub fn fit_cards(
    cam: Single<&Camera, With<UiCamera>>,
    mut cards: Query<(Ref<Text>, &ComputedNode, &mut TextFont), With<crate::ScreenText>>,
) {
    let Some(window) = cam.logical_target_size() else {
        return;
    };
    for (text, node, mut font) in &mut cards {
        let FontSize::Px(now) = font.font_size else {
            continue;
        };
        if text.is_changed() {
            font.font_size = FontSize::Px(28.0); // a new card starts at full size
            continue;
        }
        let tall = node.size().y * node.inverse_scale_factor();
        if tall > window.y - 24.0 && now > 12.0 {
            font.font_size = FontSize::Px((now * 0.94).max(12.0));
        }
    }
}

/// The poison clock and the stabbed card. The clock is white, and turns red and pulses for the last 30 seconds.
pub fn poison_hud(
    time: Res<Time>,
    poison: Res<crate::Poison>,
    mut clock: Query<(&mut Node, &mut BorderColor), (With<CountdownBox>, Without<AlertBox>)>,
    mut alert: Query<&mut Node, (With<AlertBox>, Without<CountdownBox>)>,
    mut digits: Query<(&mut Text, &mut TextColor), With<CountdownTime>>,
) {
    for (mut node, mut border) in &mut clock {
        let want = if poison.active { Display::Flex } else { Display::None };
        if node.display != want {
            node.display = want;
        }
        if poison.active {
            let red = poison.remaining <= 30.0;
            let pulse = if red { 0.5 + 0.5 * (time.elapsed_secs() * if poison.remaining <= 10.0 { 9.0 } else { 5.0 }).sin() } else { 0.0 };
            *border = BorderColor::all(if red { Color::srgb(1.0, 0.2 + 0.2 * pulse, 0.15) } else { Color::WHITE });
        }
    }
    for mut node in &mut alert {
        let want = if poison.active && poison.alert > 0.0 { Display::Flex } else { Display::None };
        if node.display != want {
            node.display = want;
        }
    }
    for (mut text, mut color) in &mut digits {
        let left = poison.remaining.max(0.0).ceil() as u32;
        let s = format!("{}:{:02}", left / 60, left % 60);
        if text.0 != s {
            text.0 = s;
        }
        let red = poison.active && poison.remaining <= 30.0;
        let pulse = if red { 0.5 + 0.5 * (time.elapsed_secs() * if poison.remaining <= 10.0 { 9.0 } else { 5.0 }).sin() } else { 0.0 };
        color.0 = if red { Color::srgb(1.0, 0.12 + 0.3 * pulse, 0.1 + 0.2 * pulse) } else { Color::WHITE };
    }
}
