//! The kaleidoscope behind the introduction and level-complete cards: the sigils of the lwa (assets/veve) turn slowly in rings,
//! mirrored like the pieces in a kaleidoscope, dissolving one into the next. It is dim and slow on purpose:
//! it should draw the eye and make the room feel alive, never fight the text (which sits on a dark glass card).
use crate::*;
use bevy::math::Rot2;
use bevy::picking::hover::Hovered;
use bevy::text::{FontSource};
use bevy::ui::{Pressed, UiTransform, Val2};
use bevy::ui_widgets::{observe, Activate, Button};

/// (copies in the ring, ring radius and piece size as fractions of the stage, spin in radians/s,
///  seconds each sigil stays, first sigil, brightness)
const RINGS: [(usize, f32, f32, f32, f32, usize, f32); 4] = [
    (12, 0.40, 0.30, 0.030, 17.0, 0, 0.20),
    (8, 0.235, 0.22, -0.050, 13.0, 4, 0.20),
    (5, 0.11, 0.13, 0.085, 11.0, 7, 0.17),
    (1, 0.0, 0.52, 0.018, 23.0, 2, 0.14),
];

const SIGILS: [&str; 12] = [
    "legba", "danbala", "ogou", "erzulie", "saint", "baron", "agwe", "asson", "dantor", "wanga", "simbi", "govi",
];

#[derive(Component)]
pub struct Stage;

#[derive(Component)]
pub struct Petal {
    ring: usize,
    k: usize,
    last: usize,
}

/// Put a kaleidoscope behind each new introduction card.
pub fn attach(
    mut commands: Commands,
    assets: Res<AssetServer>,
    state: Res<State<AppState>>,
    cards: Query<Entity, Added<ScreenUi>>,
) {
    if !matches!(
        state.get(),
        AppState::Disclaimer
            | AppState::Pitch
            | AppState::Complete
            | AppState::Complete2
            | AppState::Complete3
            | AppState::Complete4
            | AppState::Complete5
            | AppState::Complete6
    ) {
        return;
    }
    let images: Vec<Handle<Image>> = SIGILS.iter().map(|n| assets.load(format!("veve/{n}.png"))).collect();
    for card in &cards {
        let root = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0),
                    top: px(0),
                    width: percent(100),
                    height: percent(100),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    overflow: Overflow::clip(),
                    ..default()
                },
                // behind the text, which is the card's other child
                ZIndex(-1),
            ))
            .with_children(|r| {
                r.spawn((
                    Stage,
                    Node {
                        width: Val::VMax(135.0),
                        aspect_ratio: Some(1.0),
                        flex_shrink: 0.0,
                        ..default()
                    },
                ))
                .with_children(|stage| {
                    for (ring, (n, _, size, _, _, first, _)) in RINGS.iter().enumerate() {
                        for k in 0..*n {
                            stage.spawn((
                                Petal { ring, k, last: usize::MAX },
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: percent(50.0 - size * 50.0),
                                    top: percent(50.0 - size * 50.0),
                                    width: percent(size * 100.0),
                                    aspect_ratio: Some(1.0),
                                    ..default()
                                },
                                ImageNode::new(images[*first].clone()).with_color(Color::srgba(1.0, 1.0, 1.0, 0.0)),
                                UiTransform::default(),
                            ));
                        }
                    }
                });
            })
            .id();
        commands.entity(card).add_child(root);
    }
}

/// Turn the rings, breathe the whole stage, dissolve each sigil into the next.
pub fn animate(
    time: Res<Time>,
    assets: Res<AssetServer>,
    mut stage: Query<&mut UiTransform, (With<Stage>, Without<Petal>)>,
    mut petals: Query<(&mut Petal, &mut UiTransform, &mut ImageNode)>,
) {
    let t = time.elapsed_secs();
    for mut s in &mut stage {
        let z = 1.0 + 0.04 * (t * std::f32::consts::TAU / 19.0).sin();
        s.scale = Vec2::splat(z);
        s.rotation = Rot2::radians(-0.012 * t);
    }
    for (mut p, mut tf, mut img) in &mut petals {
        let (n, radius, size, spin, period, first, bright) = RINGS[p.ring];
        // each ring is a little out of step with the others
        let u = t / period + p.ring as f32 * 0.37;
        let phase = u.fract();
        let step = u.floor() as usize;
        let sigil = (first + step * (3 + p.ring)) % SIGILS.len();
        if sigil != p.last {
            p.last = sigil;
            img.image = assets.load(format!("veve/{}.png", SIGILS[sigil]));
        }
        // dissolve: dark at the swap, full in between
        let fade = (phase * std::f32::consts::PI).sin().powi(2);
        let breath = 0.85 + 0.15 * (t * std::f32::consts::TAU / 10.0).sin();
        img.color = Color::srgba(1.0, 0.95, 0.85, bright * fade * breath);

        let a = std::f32::consts::TAU * p.k as f32 / n as f32 + spin * t;
        if radius > 0.0 {
            tf.translation = Val2::percent(a.cos() * radius / size * 100.0, a.sin() * radius / size * 100.0);
        }
        // pieces point outward; every other one is the mirror image
        tf.rotation = Rot2::radians(a + std::f32::consts::FRAC_PI_2 + if radius > 0.0 { 0.0 } else { spin * t * 3.0 });
        tf.scale = Vec2::new(if p.k % 2 == 1 { -1.0 } else { 1.0 }, 1.0);
    }
}

#[derive(Component)]
pub struct MusicLabel;

/// The "Music: on / off" button in the corner of the introduction cards (the key M does the same).
pub fn music_button(
    mut commands: Commands,
    assets: Res<AssetServer>,
    state: Res<State<AppState>>,
    cards: Query<Entity, Added<ScreenUi>>,
) {
    if !matches!(state.get(), AppState::Disclaimer | AppState::Pitch) {
        return;
    }
    for card in &cards {
        let button = commands
            .spawn((
                Button,
                Hovered::default(),
                Node {
                    position_type: PositionType::Absolute,
                    top: px(18),
                    right: px(22),
                    padding: UiRect::axes(px(16), px(8)),
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(px(20)),
                    ..default()
                },
                BorderColor::all(Color::srgba(0.96, 0.77, 0.34, 0.7)),
                BackgroundColor(Color::srgba(0.02, 0.02, 0.025, 0.7)),
                children![(
                    MusicLabel,
                    Text::new("Music: on   (M)"),
                    TextFont {
                        font: FontSource::Handle(assets.load("fonts/FiraSans-Bold.ttf")),
                        font_size: FontSize::Px(15.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.96, 0.77, 0.34)),
                )],
                observe(|_: On<Activate>, mut intensity: ResMut<crate::audio::Intensity>| {
                    intensity.music_on = !intensity.music_on;
                }),
            ))
            .id();
        commands.entity(card).add_child(button);
    }
}

/// Keep the button's words and look in step with the music, and light it when the pointer is over it.
pub fn music_button_look(
    intensity: Res<crate::audio::Intensity>,
    mut labels: Query<&mut Text, With<MusicLabel>>,
    mut buttons: Query<(&Hovered, Has<Pressed>, &mut BackgroundColor), With<Button>>,
) {
    let want = if intensity.music_on { "Music: on   (M)" } else { "Music: off   (M)" };
    for mut t in &mut labels {
        if t.0 != want {
            t.0 = want.to_string();
        }
    }
    for (hovered, pressed, mut bg) in &mut buttons {
        bg.0 = if pressed {
            Color::srgba(0.3, 0.22, 0.06, 0.9)
        } else if hovered.get() {
            Color::srgba(0.16, 0.12, 0.04, 0.85)
        } else {
            Color::srgba(0.02, 0.02, 0.025, 0.7)
        };
    }
}
