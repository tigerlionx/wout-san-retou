//! Wout San Retou (The Road of No Return): Level 1, "The Night Before".
//!
//! A fictional game about what gang life costs. Everything here is invented.
//! Ro, 17, has refused his "baptism" and must slip through the compound's
//! yard to the house: sabotage the gang's supplies, stay out of sight of
//! patrolling guards and a sniper (they notice, hear, investigate and chase),
//! or wake the kids by name instead of fighting them.

use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;

mod ai;
mod audio;
mod autoplay;
mod character;
mod embedded;
mod level;
mod level2;
mod level3;
mod level4;
mod level5;
mod level6;
mod level7;
mod hud;
mod kaleido;
mod playtest;
mod plague;
mod player;
mod poster;
mod style;
mod weapons;

pub use ai::{AiState, Guard, GuardKind};
pub use character::{Boxes, Mover};
pub use level2::Poison;
pub use weapons::{AimOverride, Health, Loadout, Noises, Shake};

pub const START_MINUTE: f32 = 11.0 * 60.0; // 11:00 AM
pub const NIGHT_MINUTE: f32 = 18.0 * 60.0; // 6:00 PM

const DISCLAIMER: &str = "\
WOUT SAN RETOU (The Road of No Return)

This is a work of fiction. All characters, gangs, places and events are invented, and any resemblance to real people is unintentional.

It was made to show young people what gang life really costs, and that there is always another road. Haitian Vodou is a living religion and is not what this story is about. The villain here is a fraud who pretends to have spiritual power.

The game contains violence and frightening scenes. If you or someone you know is being pressured to join a gang, you are not alone. Help is listed at the end of the game.

---

Wout San Retou se yon istwa envante. Tout pèsonaj, gang, kote ak evènman yo envante, e si yo sanble ak moun reyèl, se pa fè espre. (Bouyon Kreyòl: li bezwen revizyon yon natif natal.)

Press ENTER to begin";

const PITCH: [&str; 3] = [
    "THREE MONTHS AGO\n\nA phone. New sneakers. A hot meal.\n\n\"Stay with us, Ro. Nobody touches you.\"\n- Madanm Flanm\n\n[ENTER]",
    "TONIGHT\n\nThey call it a baptism.\nProve you belong: hurt someone who owes.\n\nYou are seventeen.\n\n[ENTER]",
    "YOU SAID NO.\n\nThe gate is locked. Everyone has seen your face.\n\nGet to the house.\nSlow them down: the truck, the fuel, the food.\nStay low. Stay quiet. Do not look back.\n\n[ENTER]",
];

pub const OBJECTIVES: [&str; 3] = [
    "Slash the supply truck's tires",
    "Open the fuel store's valves",
    "Spoil the food store",
];

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum AppState {
    #[default]
    Disclaimer,
    Pitch,
    Playing,
    Complete,
    Level2,
    Complete2,
    Level3,
    Complete3,
    Level4,
    Complete4,
    Level5,
    Complete5,
    Level6,
    Complete6,
    Level7,
    Ending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LostStatus {
    Standing,
    Woken,
    Fallen,
}

// ---------------------------------------------------------------- components

#[derive(Component)]
pub struct LevelEntity;
#[derive(Component)]
pub struct Player;
#[derive(Component)]
pub struct GameCamera;
#[derive(Component)]
pub struct Sun;
#[derive(Component)]
pub struct Hud;
#[derive(Component)]
pub struct ScreenUi;
#[derive(Component)]
pub struct ScreenText;
#[derive(Component)]
pub struct HurtOverlay;
#[derive(Component)]
pub struct Crosshair;

#[derive(Component)]
pub struct SabotageTarget {
    pub idx: usize,
    pub prompt: &'static str,
    pub done_msg: &'static str,
    pub pos: Vec3,
    pub secs: f32,
    pub beacon: Handle<StandardMaterial>,
}

// ----------------------------------------------------------------- resources

#[derive(Resource)]
pub struct GameClock {
    pub minute: f32,
    pub night_fell: bool,
    /// Game minutes per real second (the day lasts 7 real minutes).
    pub speed: f32,
}

#[derive(Resource, Default)]
pub struct Weight(pub f32); // 0..=100

#[derive(Resource, Default)]
pub struct Message(pub String);

#[derive(Resource, Default)]
pub struct Prompt(pub String);

#[derive(Resource, Default)]
pub struct Objectives {
    pub done: [bool; 3],
    pub progress: [f32; 3],
    pub discovered: [bool; 3],
    pub door_open: bool,
}

#[derive(Resource, Default)]
pub struct Stats {
    pub caught: u32,
    pub woken: u32,
    pub kills: u32,
    pub elapsed: f32,
}

#[derive(Resource, Default)]
pub struct Caught(pub Option<String>);

#[derive(Resource)]
pub struct Checkpoint(pub Vec3);

#[derive(Resource)]
pub struct CamRig {
    pub yaw: f32,
    pub pitch_third: f32,
    pub pitch_first: f32,
    /// Temporary muzzle climb from firing; recovers by itself.
    pub recoil: f32,
    pub first: bool,
    /// Aiming down the sights (first person, right mouse button): 0 to 1, eased.
    pub aim: f32,
}

#[derive(Resource)]
pub struct DoorMat(pub Handle<StandardMaterial>);

/// Debug/testing: when set, only the named guard thinks; the rest stand idle.
#[derive(Resource, Default)]
pub struct AiOnly(pub Option<&'static str>);

#[derive(Resource, Default)]
pub struct PitchStep(pub usize);

/// Running totals across the whole game (the per-level Stats reset).
#[derive(Resource, Default)]
pub struct Tally {
    pub woken: u32,
    pub kills: u32,
    last_woken: u32,
    last_kills: u32,
}

fn tally(stats: Res<Stats>, mut t: ResMut<Tally>) {
    t.woken += stats.woken.saturating_sub(t.last_woken);
    t.kills += stats.kills.saturating_sub(t.last_kills);
    t.last_woken = stats.woken;
    t.last_kills = stats.kills;
}

#[derive(Resource, Default)]
pub struct EndingKind(pub Option<level7::Ending>);

/// Read-only inputs the HUD needs (grouped to stay under Bevy's parameter limit).
#[derive(bevy::ecs::system::SystemParam)]
struct HudRes<'w> {
    weight: Res<'w, Weight>,
    msg: Res<'w, Message>,
    prompt: Res<'w, Prompt>,
    stats: Res<'w, Stats>,
    l3: Option<Res<'w, level3::Level3>>,
    l4: Option<Res<'w, level4::Level4>>,
    l5: Option<Res<'w, level5::Level5>>,
    l6: Option<Res<'w, level6::Level6>>,
    l7: Option<Res<'w, level7::Level7>>,
    bleed: Res<'w, level5::Bleed>,
}

fn main() {
    let mut app = App::new();
    // WSR_AUTOPLAY=1 runs a scripted playtest that saves screenshots and exits.
    if std::env::var_os("WSR_AUTOPLAY").is_some() {
        app.add_plugins(autoplay::AutoplayPlugin);
    }
    // In autoplay the window stays on top: a window hidden behind other apps
    // is not presented, and its screenshots come out black.
    let autoplay = std::env::var_os("WSR_AUTOPLAY").is_some();
    // Assets: an `assets` folder next to the executable (a packaged build), else the one beside Cargo.toml.
    let assets = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("assets")))
        .filter(|p| p.is_dir())
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| concat!(env!("CARGO_MANIFEST_DIR"), "/assets").into());
    // The assets are packed into the executable (see build.rs); the folder is the fallback for anything not packed.
    app.add_plugins(embedded::EmbeddedAssetsPlugin { folder: assets.clone() });
    app.add_plugins(DefaultPlugins.set(AssetPlugin {
        // Audio lives in assets/audio (see src/audio.rs).
        file_path: assets,
        ..default()
    }).set(WindowPlugin {
        primary_window: Some({
            let mut window = Window {
            title: "Wout San Retou".into(),
            // The web build draws into <canvas id="bevy"> and fills its parent.
            canvas: Some("#bevy".into()),
            fit_canvas_to_parent: true,
            prevent_default_event_handling: true,
            window_level: if autoplay {
                bevy::window::WindowLevel::AlwaysOnTop
            } else {
                bevy::window::WindowLevel::Normal
            },
            ..default()
            };
            // The browser draws at CSS pixels, not Retina pixels: a quarter of the work on a laptop screen.
            #[cfg(target_arch = "wasm32")]
            window.resolution.set_scale_factor_override(Some(1.0));
            #[cfg(not(target_arch = "wasm32"))]
            let _ = &mut window;
            window
        }),
        ..default()
    }))
    .init_state::<AppState>()
    .insert_resource(ClearColor(Color::srgb(0.62, 0.66, 0.66)))
    .insert_resource(GameClock {
        minute: START_MINUTE,
        night_fell: false,
        speed: 1.0,
    })
    .insert_resource(Checkpoint(Vec3::new(0.0, 0.0, 26.0)))
    .insert_resource(CamRig {
        yaw: 0.0,
        pitch_third: -0.95,
        pitch_first: 0.0,
        recoil: 0.0,
        first: false,
        aim: 0.0,
    })
    .init_resource::<Weight>()
    .init_resource::<weapons::HitMark>()
    .insert_resource(playtest::SessionLog::open())
    .add_systems(Update, (playtest::record, playtest::fps_report))
    .init_resource::<Message>()
    .init_resource::<Prompt>()
    .init_resource::<Objectives>()
    .init_resource::<Stats>()
    .init_resource::<Caught>()
    .init_resource::<AiOnly>()
    .init_resource::<PitchStep>()
    .init_resource::<Boxes>()
    .init_resource::<plague::Plague>()
    .init_resource::<Loadout>()
    .init_resource::<weapons::LoadoutSave>()
    .add_systems(Update, weapons::snapshot_loadout.before(level::handle_caught))
    .init_resource::<Health>()
    .init_resource::<Noises>()
    .init_resource::<Shake>()
    .init_resource::<Poison>()
    .init_resource::<level3::Grab>()
    .init_resource::<level3::Story>()
    .init_resource::<level5::Bleed>()
    .init_resource::<Tally>()
    .init_resource::<audio::Intensity>()
    .init_resource::<EndingKind>()
    .init_resource::<AimOverride>()
    .add_systems(Startup, (style::setup_camera, hud::spawn_hud, weapons::setup_fx, character::setup_rig, jump_to_start_level))
    .init_resource::<hud::HudData>()
    .add_systems(Update, (hud::apply_hud, hud::layout_viewport, hud::style_cards, hud::billboard, audio::intro_music, kaleido::attach, kaleido::animate, hud::fit_cards, kaleido::music_button, kaleido::music_button_look))
    .add_systems(Update, player::cursor_mode)
    // Disclaimer
    .add_systems(OnEnter(AppState::Disclaimer), |c: Commands| {
        show_screen(c, DISCLAIMER)
    })
    .add_systems(Update, disclaimer_input.run_if(in_state(AppState::Disclaimer)))
    .add_systems(OnExit(AppState::Disclaimer), hide_screen)
    // Pitch cards
    .add_systems(OnEnter(AppState::Pitch), |c: Commands, mut s: ResMut<PitchStep>| {
        s.0 = 0;
        show_screen(c, PITCH[0]);
    })
    .add_systems(Update, pitch_input.run_if(in_state(AppState::Pitch)))
    .add_systems(OnExit(AppState::Pitch), hide_screen)
    // Level 1
    .add_systems(OnEnter(AppState::Playing), level::setup_level)
    .add_systems(OnEnter(AppState::Level3), level3::setup_level3)
    .add_systems(OnExit(AppState::Level3), (cleanup_level, cleanup_level3))
    .add_systems(
        Update,
        (
            level3::lost_ai,
            level3::interact3,
        )
            .chain()
            .after(ai::think)
            .before(level::handle_caught)
            .run_if(in_lost_level),
    )
    .add_systems(
        Update,
        (level3::stairs3, level3::disco)
            .after(level3::interact3)
            .before(level::handle_caught)
            .run_if(in_state(AppState::Level3)),
    )
    .add_systems(OnEnter(AppState::Level7), level7::setup_level7)
    .add_systems(OnExit(AppState::Level7), (cleanup_level, cleanup_level7))
    .add_systems(
        Update,
        (level7::nwit7, level7::interact7, level7::goal7)
            .chain()
            .after(ai::think)
            .before(level::handle_caught)
            .run_if(in_state(AppState::Level7)),
    )
    .add_systems(OnEnter(AppState::Ending), (show_ending, audio::stop_music))
    .add_systems(OnEnter(AppState::Pitch), audio::stop_music)
    .add_systems(
        Update,
        (audio::audio_test, audio::music).run_if(|| std::env::var_os("WSR_AUDIOTEST").is_some()),
    )
    .add_systems(Update, ending_input.run_if(in_state(AppState::Ending)))
    .add_systems(OnExit(AppState::Ending), hide_screen)
    .add_systems(
        Update,
        player::phone_light.run_if(|state: Res<State<AppState>>| {
            matches!(state.get(), AppState::Level2 | AppState::Level3 | AppState::Level4 | AppState::Level5 | AppState::Level6)
        }),
    )
    .add_systems(OnEnter(AppState::Level6), level6::setup_level6)
    .add_systems(OnExit(AppState::Level6), (cleanup_level, cleanup_level6))
    .add_systems(
        Update,
        (
            level6::nwit6,
            level6::interact6,
            level6::night6,
            level6::goal6,
        )
            .chain()
            .after(ai::think)
            .before(level::handle_caught)
            .run_if(in_state(AppState::Level6)),
    )
    .add_systems(OnEnter(AppState::Level5), level5::setup_level5)
    .add_systems(OnExit(AppState::Level5), (cleanup_level, cleanup_level5))
    .add_systems(
        Update,
        level5::sword5
            .before(ai::think)
            .run_if(in_state(AppState::Level5)),
    )
    .add_systems(
        Update,
        (
            level5::interact5,
            level5::traps5,
            level5::bleed_tick,
            level5::goal5,
        )
            .chain()
            .after(level3::interact3)
            .before(level::handle_caught)
            .run_if(in_state(AppState::Level5)),
    )
    .add_systems(OnEnter(AppState::Level4), level4::setup_level4)
    .add_systems(OnExit(AppState::Level4), (cleanup_level, cleanup_level4))
    .add_systems(
        Update,
        (level4::glass4, level4::fall4, level4::stairs4)
            .chain()
            .after(level3::interact3)
            .before(level::handle_caught)
            .run_if(in_state(AppState::Level4)),
    )
    .add_systems(OnEnter(AppState::Level2), level2::setup_level2)
    .add_systems(OnExit(AppState::Level2), cleanup_level)
    .add_systems(
        Update,
        (
            level2::nwit_ai,
            level2::poison_tick,
            level2::interact2,
            level2::goal2,
            level2::flicker,
        )
            .chain()
            .after(ai::think)
            .before(level::handle_caught)
            .run_if(in_state(AppState::Level2)),
    )
    .add_systems(
        Update,
        level::check_goal
            .after(level::handle_caught)
            .run_if(in_state(AppState::Playing)),
    )
    .add_systems(
        Update,
        (
            (
                advance_clock,
                player::control,
                weapons::select_and_reload,
                weapons::shoot,
                weapons::hitmark_ui,
                weapons::melee,
                weapons::throw_grenade,
                weapons::update_grenades,
                weapons::resolve_deaths,
                level::interact,
                ai::think,
            )
                .chain(),
            (
                weapons::recover,
                tally,
                audio::music,
                audio::hurt_sfx,
                weapons::use_bottle,
                level::handle_caught,
                character::reskin,
                character::attach_bodies,
                character::animate,
                ai::fall_animation,
                weapons::held_models,
                weapons::grip_weapons,
                weapons::update_particles,
                ai::visuals,
                player::camera,
                update_lighting,
                update_hud,
            )
                .chain(),
        )
            .chain()
            .run_if(in_game),
    )
    .add_systems(
        Update,
        plague::plague.after(ai::think).before(level::handle_caught).run_if(in_game),
    )
    .add_systems(OnExit(AppState::Playing), cleanup_level)
    // Complete
    .add_systems(OnEnter(AppState::Complete), show_complete)
    .add_systems(Update, complete_input.run_if(in_state(AppState::Complete)))
    .add_systems(OnExit(AppState::Complete), hide_screen)
    .add_systems(OnEnter(AppState::Complete2), show_complete2)
    .add_systems(Update, complete2_input.run_if(in_state(AppState::Complete2)))
    .add_systems(OnExit(AppState::Complete2), hide_screen)
    .add_systems(OnEnter(AppState::Complete3), show_complete3)
    .add_systems(Update, complete3_input.run_if(in_state(AppState::Complete3)))
    .add_systems(OnExit(AppState::Complete3), hide_screen)
    .add_systems(OnEnter(AppState::Complete4), show_complete4)
    .add_systems(Update, complete4_input.run_if(in_state(AppState::Complete4)))
    .add_systems(OnExit(AppState::Complete4), hide_screen)
    .add_systems(OnEnter(AppState::Complete5), show_complete5)
    .add_systems(Update, complete5_input.run_if(in_state(AppState::Complete5)))
    .add_systems(OnExit(AppState::Complete5), hide_screen)
    .add_systems(Update, hud::poison_hud)
    .add_systems(OnEnter(AppState::Complete6), show_complete6)
    .add_systems(Update, complete6_input.run_if(in_state(AppState::Complete6)))
    .add_systems(OnExit(AppState::Complete6), hide_screen)
    .add_systems(
        Update,
        (
            poster::spawn_poster,
            character::poster_pose.after(character::animate),
            poster::poster_camera.after(player::camera),
            poster::hide_bun,
            poster::poster_lighting.after(update_lighting),
        )
            .run_if(|| poster::on()),
    );
    #[cfg(target_arch = "wasm32")]
    app.add_systems(Update, style::web_lite);
    app.run();
}

// ------------------------------------------------------------ text screens

fn show_screen(mut commands: Commands, text: &str) {
    commands.spawn((
        ScreenUi,
        Node {
            width: percent(100),
            height: percent(100),
            padding: UiRect::all(px(48)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgb(0.04, 0.045, 0.045)),
        children![(
            ScreenText,
            Text::new(text),
            TextFont {
                font_size: FontSize::Px(21.0),
                ..default()
            },
            TextColor(Color::srgb(0.86, 0.88, 0.86)),
        )],
    ));
}

fn hide_screen(mut commands: Commands, ui: Query<Entity, With<ScreenUi>>) {
    for e in &ui {
        commands.entity(e).despawn();
    }
}

fn pressed_enter(keys: &ButtonInput<KeyCode>) -> bool {
    keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space)
}

fn disclaimer_input(keys: Res<ButtonInput<KeyCode>>, mut next: ResMut<NextState<AppState>>) {
    if pressed_enter(&keys) {
        next.set(AppState::Pitch);
    }
}

fn pitch_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut step: ResMut<PitchStep>,
    mut next: ResMut<NextState<AppState>>,
    mut text: Single<&mut Text, With<ScreenText>>,
) {
    if !pressed_enter(&keys) {
        return;
    }
    step.0 += 1;
    if step.0 >= PITCH.len() {
        next.set(AppState::Playing);
    } else {
        text.0 = PITCH[step.0].to_string();
    }
}

fn show_complete(commands: Commands, stats: Res<Stats>, weight: Res<Weight>) {
    let text = format!(
        "LEVEL 1 COMPLETE: THE NIGHT BEFORE\n\n\
         You made it to the house.\n\n\
         Time: {:.0}s    Caught: {}    Kids woken by name: {}    Weight: {:.0}\n\n\
         The corridor ahead is dark. Granmèt Nwit knows you are coming.\n\
         [ENTER to go inside]",
        stats.elapsed, stats.caught, stats.woken, weight.0
    );
    show_screen(commands, &text);
}

fn complete_input(keys: Res<ButtonInput<KeyCode>>, mut next: ResMut<NextState<AppState>>) {
    if pressed_enter(&keys) {
        next.set(AppState::Level2);
    }
}

fn show_complete2(commands: Commands, stats: Res<Stats>, weight: Res<Weight>, poison: Res<Poison>) {
    let text = format!(
        "LEVEL 2 COMPLETE: INSIDE THE HOUSE\n\n\
         You drank the cure. The first save point is yours.\n\n\
         Time: {:.0}s    Deaths: {}    Weight: {:.0}    Bottle uses left: {}\n\n\
         Below, a swimming pool echoes with music. Madanm Flanm is waiting.\n\
         [ENTER to go down to the party]",
        stats.elapsed, stats.caught, weight.0, poison.bottles
    );
    show_screen(commands, &text);
}

fn complete2_input(keys: Res<ButtonInput<KeyCode>>, mut next: ResMut<NextState<AppState>>) {
    if pressed_enter(&keys) {
        next.set(AppState::Level3);
    }
}

fn show_complete3(commands: Commands, stats: Res<Stats>, weight: Res<Weight>) {
    let text = format!(
        "LEVEL 3 COMPLETE: THE PARTY WING\n\n\
         Madanm Flanm is down and the stairs are open.\n\n\
         Time: {:.0}s    Falls: {}    Lost woken by name: {}    Weight: {:.0}\n\n\
         Above, two penthouses look down on the pool through a floor of glass. Tèt Wòch is waiting.\n\
         [ENTER to go up]",
        stats.elapsed, stats.caught, stats.woken, weight.0
    );
    show_screen(commands, &text);
}

fn complete3_input(keys: Res<ButtonInput<KeyCode>>, mut next: ResMut<NextState<AppState>>) {
    if pressed_enter(&keys) {
        next.set(AppState::Level4);
    }
}

fn show_complete4(commands: Commands, stats: Res<Stats>, weight: Res<Weight>) {
    let text = format!(
        "LEVEL 4 COMPLETE: THE GLASS FLOOR\n\n\
         Tèt Wòch is down and the stair door is open.\n\n\
         Time: {:.0}s    Falls: {}    Lost woken by name: {}    Weight: {:.0}\n\n\
         The second floor is a maze of rooms and traps. The Sword Man is waiting.\n\
         [ENTER to go up]",
        stats.elapsed, stats.caught, stats.woken, weight.0
    );
    show_screen(commands, &text);
}

fn complete4_input(keys: Res<ButtonInput<KeyCode>>, mut next: ResMut<NextState<AppState>>) {
    if pressed_enter(&keys) {
        next.set(AppState::Level5);
    }
}

fn show_complete5(commands: Commands, stats: Res<Stats>, weight: Res<Weight>) {
    let text = format!(
        "LEVEL 5 COMPLETE: THE SWORD MAN\n\n\
         The binding is broken and the stairs are open.\n\n\
         Time: {:.0}s    Falls: {}    Woken by name: {}    Weight: {:.0}\n\n\
         Above: Granmèt Nwit's own rooms, a Jacuzzi, secret passages, and the man himself.\n\
         [ENTER to enter his quarters]",
        stats.elapsed, stats.caught, stats.woken, weight.0
    );
    show_screen(commands, &text);
}

fn complete5_input(keys: Res<ButtonInput<KeyCode>>, mut next: ResMut<NextState<AppState>>) {
    if pressed_enter(&keys) {
        next.set(AppState::Level6);
    }
}

fn show_complete6(commands: Commands, stats: Res<Stats>, weight: Res<Weight>) {
    let text = format!(
        "LEVEL 6 COMPLETE: THE QUARTERS\n\n\
         Granmèt Nwit is hurt and running. The stairs to the roof are open.\n\n\
         Time: {:.0}s    Falls: {}    Weight: {:.0}\n\n\
         On the roof the wind is cold. He is on all fours beside his own knife.\n\
         [ENTER to climb to the roof]",
        stats.elapsed, stats.caught, weight.0
    );
    show_screen(commands, &text);
}

fn complete6_input(keys: Res<ButtonInput<KeyCode>>, mut next: ResMut<NextState<AppState>>) {
    if pressed_enter(&keys) {
        next.set(AppState::Level7);
    }
}

fn show_ending(commands: Commands, kind: Res<EndingKind>, tally: Res<Tally>, weight: Res<Weight>) {
    use level7::Ending;
    let (title, body) = match kind.0 {
        Some(Ending::Justice) => (
            "THE HAND-OVER",
            "He rises into the dark on a line, shouting his own name at the wind, and for the first time nobody below is afraid of it.\n\n\
             There will be a trial. There will be years. It will not be clean and it will not be quick, and the Brigad will find someone else to light candles for. \
             But a man who ruled by fear is just a man on a rope, and the kids you woke will remember that.",
        ),
        Some(Ending::Throne) => (
            "THE CANDLE PASSES",
            "The money is heavy. Down below, the music starts again.\n\n\
             A boy of twelve is handed a phone, a pair of shoes and a hot meal, and says yes to someone who has your face. \
             The Road of No Return does not end. It only changes hands.",
        ),
        Some(Ending::Road) => (
            "THE ROAD",
            "You walk down the stairs, and you do not look back. Behind you, in twos and threes, the kids you woke are following, quietly, as if afraid the spell is still on the floor.\n\n\
             Outside it is raining a little. There is a school that takes late students. A workshop that will teach a trade. A football pitch, a music hall, a counsellor, a place to eat. \
             None of it is a miracle. All of it is a road.\n\n             \"Gen yon lòt wout.\"  (There is another road.)  [Kreyòl: needs native review]",
        ),
        Some(Ending::Blood) => (
            "THE SAME ROAD",
            "He is dead. The house is quiet in a way it has never been.\n\n\
             You told yourself it was the only way. Tomorrow someone will pick up the candle, and the next boy will say yes to the next phone. \
             The road you walked tonight is the road everyone walks.",
        ),
        None => ("THE END", ""),
    };
    let text = format!(
        "ENDING: {title}\n\n{body}\n\n\
         ----\n\
         Kids woken by name: {}    People you put down: {}    Weight you carry: {:.0}\n\n\
         This is a made-up story about made-up people. If you or someone you know is being pushed to join a gang, you are not alone.\n\
         [Help lines and organisations: to be added by the project team before release]\n\n\
         [ENTER to play again]",
        tally.woken, tally.kills, weight.0
    );
    show_screen(commands, &text);
}

fn ending_input(keys: Res<ButtonInput<KeyCode>>, mut next: ResMut<NextState<AppState>>) {
    if pressed_enter(&keys) {
        next.set(AppState::Pitch);
    }
}

/// Testing: `WSR_START=1..7` skips the title cards and starts on that level,
/// with the M50 and a few bottles so later levels are playable on their own.
fn jump_to_start_level(
    mut next: ResMut<NextState<AppState>>,
    mut loadout: ResMut<Loadout>,
    mut poison: ResMut<Poison>,
) {
    let Some(n) = std::env::var("WSR_START").ok().and_then(|v| v.parse::<u32>().ok()) else {
        return;
    };
    let state = match n {
        1 => AppState::Playing,
        2 => AppState::Level2,
        3 => AppState::Level3,
        4 => AppState::Level4,
        5 => AppState::Level5,
        6 => AppState::Level6,
        7 => AppState::Level7,
        _ => return,
    };
    if n >= 2 {
        poison.bottles = 5;
    }
    if n >= 5 {
        loadout.has_m50 = true;
    }
    next.set(state);
}

fn in_game(state: Res<State<AppState>>) -> bool {
    matches!(
        state.get(),
        AppState::Playing
            | AppState::Level2
            | AppState::Level3
            | AppState::Level4
            | AppState::Level5
            | AppState::Level6
            | AppState::Level7
    )
}

fn in_lost_level(state: Res<State<AppState>>) -> bool {
    matches!(
        state.get(),
        AppState::Level3 | AppState::Level4 | AppState::Level5
    )
}

fn cleanup_level7(mut commands: Commands, mut loadout: ResMut<Loadout>) {
    commands.remove_resource::<level7::Level7>();
    commands.remove_resource::<level7::MoneyAt>();
    loadout.disarmed = false;
}

fn cleanup_level6(mut commands: Commands, mut loadout: ResMut<Loadout>) {
    commands.remove_resource::<level6::Level6>();
    loadout.cramped = false;
}

fn cleanup_level5(mut commands: Commands) {
    commands.remove_resource::<level5::Level5>();
}

fn cleanup_level4(mut commands: Commands) {
    commands.remove_resource::<level3::Water>();
    commands.remove_resource::<level4::Holes>();
    commands.remove_resource::<level4::GlassRect>();
    commands.remove_resource::<weapons::Blasts>();
    commands.remove_resource::<level4::Level4>();
}

fn cleanup_level3(mut commands: Commands) {
    commands.remove_resource::<level3::Water>();
}

fn cleanup_level(mut commands: Commands, q: Query<Entity, With<LevelEntity>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

// --------------------------------------------------------- clock and lighting

fn advance_clock(
    time: Res<Time>,
    mut clock: ResMut<GameClock>,
    mut msg: ResMut<Message>,
    mut commands: Commands,
    mut guards: Query<(Entity, &mut Guard, &mut Visibility, &mut character::Mover)>,
) {
    clock.minute += time.delta_secs() * clock.speed;

    // At 6:00 PM the kids you struck (rather than woke) rise again.
    if !clock.night_fell && clock.minute >= NIGHT_MINUTE {
        clock.night_fell = true;
        let mut risen = 0;
        for (e, mut g, mut vis, mut mv) in &mut guards {
            if g.status == LostStatus::Fallen {
                g.status = LostStatus::Standing;
                g.hp = 100.0;
                g.state = AiState::Search;
                g.awareness = 0.5;
                g.timer = 0.0;
                *vis = Visibility::Inherited;
                // The ordinary dead come back as the dead: a Brute (12 bullets), a Yaku (4) or a prisoner (2),
                // with their own bodies and their bare hands. The named bosses rise as themselves.
                if g.kind == GuardKind::Walker && !g.is_boss() && !g.melee {
                    let (cast, bullets, hit) = match risen % 10 {
                        0 | 1 => (character::Cast::Brute, 12.0, 10.0),
                        2..=4 => (character::Cast::Yaku, 4.0, 5.0),
                        _ => (character::Cast::Prisoner, 2.0, 1.0),
                    };
                    g.hp = bullets;
                    g.max_hp = bullets;
                    g.bullet_hp = 1.0;
                    g.melee = true;
                    g.zombie = true;
                    g.dmg = hit;
                    g.fire_gap = 1.0;
                    g.range = 30.0;
                    g.awareness = 1.0;
                    g.state = AiState::Chase;
                    mv.zombie = true;
                    mv.armed = false;
                    commands.entity(e).insert(character::ReskinTo(cast));
                }
                risen += 1;
            }
        }
        msg.0 = format!(
            "6:00 PM. Night falls. {risen} you struck have risen again. Those you woke stay free."
        );
    }
}

fn update_lighting(
    clock: Res<GameClock>,
    mut sun: Single<&mut DirectionalLight, With<Sun>>,
    mut ambient: ResMut<GlobalAmbientLight>,
    mut clear: ResMut<ClearColor>,
    mut fog: Single<&mut DistanceFog, With<GameCamera>>,
    state: Res<State<AppState>>,
) {
    if *state.get() == AppState::Level6 {
        // His quarters: candle smoke and violet dark.
        sun.color = Color::srgb(0.7, 0.55, 0.9);
        sun.illuminance = 2200.0;
        ambient.brightness = 270.0; // dim, but readable: players hate not seeing
        let dark = Color::srgb(0.02, 0.012, 0.03);
        clear.0 = dark;
        fog.color = dark;
        fog.falloff = FogFalloff::Linear { start: 18.0, end: 55.0 };
        return;
    }
    if *state.get() == AppState::Level5 {
        // The second floor: nearly black, with the candles and the drum.
        sun.color = Color::srgb(0.8, 0.5, 0.5);
        sun.illuminance = 2200.0;
        ambient.brightness = 240.0;
        let dark = Color::srgb(0.03, 0.012, 0.015);
        clear.0 = dark;
        fog.color = dark;
        fog.falloff = FogFalloff::Linear { start: 18.0, end: 55.0 };
        return;
    }
    if matches!(state.get(), AppState::Level3 | AppState::Level4) {
        // The party wing: dim, with the disco and pool doing the lighting.
        sun.color = Color::srgb(0.7, 0.75, 1.0);
        sun.illuminance = 2300.0;
        ambient.brightness = 290.0; // the darkest levels were too dark to see Ro in (screenshots of 2026-11-25)
        let dark = Color::srgb(0.03, 0.025, 0.05);
        clear.0 = dark;
        fog.color = dark;
        fog.falloff = FogFalloff::Linear { start: 18.0, end: 55.0 };
        return;
    }
    if *state.get() == AppState::Level7 {
        // The rooftop at dusk: a low warm sun over an indigo city, so the flat concrete has colour and shadow.
        sun.color = Color::srgb(1.0, 0.86, 0.72);
        sun.illuminance = 9_500.0;
        ambient.color = Color::srgb(0.70, 0.76, 1.0);
        ambient.brightness = 480.0;
        let sky = Color::srgb(0.16, 0.14, 0.26);
        clear.0 = sky;
        fog.color = Color::srgb(0.30, 0.22, 0.30);
        fog.falloff = FogFalloff::Linear { start: 30.0, end: 95.0 };
        return;
    }
    if *state.get() == AppState::Level2 {
        // Inside the house: nearly dark, the candles do the lighting.
        sun.color = Color::srgb(0.6, 0.65, 0.9);
        sun.illuminance = 1900.0;
        ambient.brightness = 250.0;
        let dark = Color::srgb(0.015, 0.015, 0.02);
        clear.0 = dark;
        fog.color = dark;
        fog.falloff = FogFalloff::Linear { start: 18.0, end: 52.0 };
        return;
    }
    ambient.color = Color::srgb(0.85, 0.9, 0.9);
    fog.falloff = FogFalloff::Linear { start: 22.0, end: 70.0 };
    // 0.0 at 11:00 AM, 1.0 at 6:00 PM, then darker still.
    let t = ((clock.minute - START_MINUTE) / (NIGHT_MINUTE - START_MINUTE)).clamp(0.0, 1.0);
    let dusk = t * t;
    sun.color = Color::srgb(0.95, 0.97 - 0.42 * dusk, 0.97 - 0.7 * dusk);
    sun.illuminance = 9_000.0 * (1.0 - 0.8 * dusk);
    ambient.brightness = 320.0 * (1.0 - 0.7 * dusk);
    let sky = Color::srgb(0.62 - 0.30 * dusk, 0.66 - 0.45 * dusk, 0.66 - 0.55 * dusk);
    clear.0 = sky;
    fog.color = sky;
}

// ----------------------------------------------------------------------- HUD

fn update_hud(
    clock: Res<GameClock>,
    res: HudRes,
    objectives: Res<Objectives>,
    rig: Res<CamRig>,
    loadout: Res<Loadout>,
    health: Res<Health>,
    poison: Res<Poison>,
    state: Res<State<AppState>>,
    guards: Query<&Guard>,
    player: Single<&Mover, With<Player>>,
    mut data: ResMut<hud::HudData>,
    mut hurt: Single<&mut BackgroundColor, With<HurtOverlay>>,
    mut cross: Single<&mut Visibility, With<Crosshair>>,
) {
    let HudRes { weight, msg, prompt, stats, l3, l4, l5, l6, l7, bleed } = res;
    let total = clock.minute as u32;
    let (h24, m) = (total / 60, total % 60);
    let (h12, ampm) = match h24 {
        0 => (12, "AM"),
        1..=11 => (h24, "AM"),
        12 => (12, "PM"),
        _ => (h24 - 12, "PM"),
    };
    let stance = if player.prone_on {
        "prone"
    } else if player.crouch > 0.5 {
        "crouched"
    } else if player.sprinting {
        "sprinting"
    } else if !player.grounded {
        "airborne"
    } else {
        "walking"
    };
    // How close is the most alert guard to recognizing you?
    let mut worst = (0.0_f32, "unseen");
    for g in &guards {
        // A guard still on patrol cannot be at 100%: that was a guard stunned or hit while unaware (the Noticed bar
        // once read "100% patrolling"). Patrol counts for at most 30%.
        let a = if g.state == ai::AiState::Patrol { g.awareness.min(0.3) } else { g.awareness };
        if g.status == LostStatus::Standing && a > worst.0 {
            worst = (a, g.state.label());
        }
    }
    let mut goals = String::new();
    for (i, name) in OBJECTIVES.iter().enumerate() {
        let mark = if objectives.done[i] { "x" } else { " " };
        let pct = if !objectives.done[i] && objectives.progress[i] > 0.0 {
            format!("  {:.0}%", objectives.progress[i] * 100.0)
        } else {
            String::new()
        };
        goals += &format!("  [{mark}] {name}{pct}\n");
    }
    let door = if objectives.door_open {
        "  The house door is unlocked. Go."
    } else {
        "  The house door is locked until all three are done."
    };
    let in_house = *state.get() == AppState::Level2;
    let in_party = *state.get() == AppState::Level3;
    let in_gallery = *state.get() == AppState::Level4;
    let in_floor2 = *state.get() == AppState::Level5;
    let in_quarters = *state.get() == AppState::Level6;
    let on_roof = *state.get() == AppState::Level7;
    let party_goals;
    let (goals, door) = if on_roof {
        use level7::Phase;
        let nwit = guards
            .iter()
            .find(|g| g.name == "Granmèt Nwit")
            .map(|g| format!("{:.0}%", (g.hp / g.max_hp * 100.0).clamp(0.0, 100.0)))
            .unwrap_or_default();
        let hint = match l7.as_ref().map(|l| l.phase) {
            Some(Phase::Prone) => "He is on all fours. What you do first is a choice with a price.",
            Some(Phase::FakeDeath) => "He is \"dying\". Keep your distance.",
            Some(Phase::LungeTell) | Some(Phase::Lunge) => "The knife glints: MOVE.",
            Some(Phase::Recover) => "He is exposed: hit him!",
            Some(Phase::Disarm) | Some(Phase::Counter) => "Bleeding? Drink (H). Get away from the knife.",
            Some(Phase::Duel) => "Dodge the lunge, punish the miss. At 20% he will kneel.",
            Some(Phase::Kneel) => "He kneels. Handcuff him, shoot him, take the gold, or walk out east.",
            Some(Phase::Cuffed) => "The helicopter is coming.",
            Some(Phase::Dead) => "...",
            None => "",
        };
        let hurt = if bleed.0 > 0.0 { format!("   BLEEDING {:.0}s: drink (H)", bleed.0) } else { String::new() };
        party_goals = format!("  Granmèt Nwit: {nwit}    Bottles: {}{hurt}\n  {hint}", poison.bottles);
        (party_goals, "")
    } else if in_quarters {
        let nwit = guards
            .iter()
            .find(|g| g.name == "Granmèt Nwit")
            .map(|g| format!("{:.0}%", (g.hp / g.max_hp * 100.0).clamp(0.0, 100.0)))
            .unwrap_or_default();
        let open = l6.as_ref().is_some_and(|l| l.stairs_open);
        let m50 = if loadout.cramped && loadout.current == weapons::WeaponKind::M50 {
            "   (the M50 is unwieldy in these rooms)"
        } else {
            ""
        };
        party_goals = format!(
            "  Granmèt Nwit: {nwit} (he flees at 50%)    Roof stairs: {}{m50}\n  Listen for \"Plip, plip!\": it says which wall. Hit him when he steps out.",
            if open { "OPEN" } else { "sealed" }
        );
        (party_goals, "")
    } else if in_floor2 {
        let (woken, total, drum) = l3
            .as_ref()
            .map(|l| (stats.woken, l.total_lost, l.music))
            .unwrap_or((0, 0, true));
        let sword = guards
            .iter()
            .find(|g| g.name == "Evens")
            .map(|g| {
                if g.status == LostStatus::Woken {
                    "freed".to_string()
                } else {
                    format!("speed x{:.1}", g.speed_mul)
                }
            })
            .unwrap_or_default();
        let (name, binding, stairs) = l5
            .as_ref()
            .map(|l| (l.knows_name, l.binding, l.stairs_open))
            .unwrap_or((false, 0.0, false));
        let bleeding = if bleed.0 > 0.0 {
            format!("   BLEEDING {:.0}s: drink (H)", bleed.0)
        } else {
            String::new()
        };
        party_goals = format!(
            "  Sword Man: {sword}{bleeding}    Bottles: {}    His name: {}\n  Binding: {:.0}%    Stairs: {}    Drum: {}    Woken: {woken}/{total}",
            poison.bottles,
            if name { "known" } else { "unknown (find the ledger)" },
            binding * 100.0,
            if stairs { "OPEN" } else { "sealed" },
            if drum { "ON" } else { "still" },
        );
        (party_goals, "")
    } else if in_gallery {
        let (woken, total, trance, stairs) = l3
            .as_ref()
            .map(|l| (stats.woken, l.total_lost, l.music, l.stairs_open))
            .unwrap_or((0, 0, true, false));
        let _ = stairs;
        let (glass, open) = l4
            .as_ref()
            .map(|l| {
                (
                    if l.collapsed { "COLLAPSED" } else if l.cracks > 0 { "CRACKED: one more blast" } else { "intact (bullets cannot break it)" },
                    l.stairs_open,
                )
            })
            .unwrap_or(("-", false));
        let boss = guards
            .iter()
            .find(|g| g.name == "Tèt Wòch")
            .map(|g| format!("{:.0}%", (g.hp / 3.0).clamp(0.0, 100.0)))
            .unwrap_or_default();
        let enraged = l4.as_ref().is_some_and(|l| l.rage);
        let boss = if enraged { format!("{boss} (enraged)") } else { boss };
        party_goals = format!(
            "  Glass: {glass}    Tèt Wòch: {boss}    Stairs: {}\n  Lost woken by name: {woken}/{total}    Trance: {}",
            if open { "OPEN" } else { "sealed" },
            if trance { "ON (breaker in the north-west)" } else { "off" }
        );
        (party_goals, "")
    } else if in_party {
        let (woken, total, music, stairs) = l3
            .as_ref()
            .map(|l| (stats.woken, l.total_lost, l.music, l.stairs_open))
            .unwrap_or((0, 0, true, false));
        party_goals = format!(
            "  Lost woken by name: {woken}/{total}    Music: {}    Stairs: {}",
            if music { "ON (cut it at the DJ booth)" } else { "off" },
            if stairs { "OPEN" } else { "sealed until Madanm Flanm falls" }
        );
        (party_goals, "")
    } else if in_house {
        let m = poison.remaining as u32;
        let status = if poison.active {
            format!("  POISONED: {}:{:02} left. Find the potion room (the altar) in the north.", m / 60, m % 60)
        } else if poison.bottles > 0 {
            format!("  Cured. Bottle: {} uses (H).", poison.bottles)
        } else {
            "  Find the altar in the north. Candles and notes will help.".to_string()
        };
        let drug = if poison.drugged > 0.0 { "  The room swims." } else { "" };
        (status, drug)
    } else {
        (goals, door)
    };
    use weapons::WeaponKind;
    let ammo = match loadout.current {
        WeaponKind::Ak47 => format!("{}/{}", loadout.ak_mag, loadout.ak_reserve),
        WeaponKind::M50 => format!("{}/{}", loadout.m50_mag, loadout.m50_reserve),
        WeaponKind::Pistol => format!("{}/{}", loadout.pistol_mag, loadout.pistol_reserve),
        WeaponKind::Grenade => format!("{} left", loadout.grenades),
        WeaponKind::Knife => "-".to_string(),
    };
    data.clock = format!("{h12}:{m:02} {ampm}");
    data.health = health.hp.max(0.0);
    data.weight = weight.0;
    data.noticed = worst.0;
    data.notice_label = worst.1.to_string();
    data.stance = stance.to_string();
    data.weapon = format!("[{}]  {}", loadout.current as u8 + 1, loadout.current.label());
    data.ammo = ammo;
    data.reloading = loadout.reload > 0.0;
    data.grenades = loadout.grenades;
    data.goals = format!("{goals}\n{door}");
    data.prompt = prompt.0.clone();
    data.message = msg.0.clone();
    hurt.0 = if poison.drugged > 0.0 {
        // The drug: a slow purple pulse over everything.
        let pulse = 0.28 + 0.14 * (poison.drugged * 2.0).sin();
        Color::srgba(0.45, 0.1, 0.7, pulse.max(health.hurt * 0.4))
    } else {
        Color::srgba(0.7, 0.02, 0.02, health.hurt * 0.4)
    };
    **cross = if rig.first { Visibility::Inherited } else { Visibility::Hidden };
}
