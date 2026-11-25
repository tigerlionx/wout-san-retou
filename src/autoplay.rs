//! Scripted playtests. Run with `WSR_AUTOPLAY=1 ./target/debug/wout-san-retou`.
//!
//! * default: movement, stealth, bots, sabotage and the door (full level)
//! * `WSR_COMBAT=1`: weapons (AK-47, pistol, reload, grenade, knife) and return fire
//! * `WSR_QUICK=1`: a short smoke script
//!
//! Injects key and mouse presses (one step per frame), teleports the player
//! and guards into test situations, logs the state after each check, saves
//! screenshots to `playtest/` and exits. It renders to an offscreen image, so
//! screenshots work even when the display is asleep or the window is hidden.

use crate::level2::{Level2, Nwit, NwitState, VialRole};
use crate::level3::{Level3, Lost, LostState, Story};
use crate::weapons::{Health, Loadout};
use crate::*;
use bevy::app::AppExit;
use bevy::asset::RenderAssetUsages;
use bevy::camera::RenderTarget;
use bevy::ecs::system::SystemParam;
use bevy::image::Image;
use bevy::input::InputSystems;
use bevy::render::render_resource::TextureFormat;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};

#[derive(Resource)]
struct OffscreenTarget(Handle<Image>);

/// UI follows the main camera, which now renders offscreen: point every
/// top-level UI node at it so screenshots include the HUD.
fn retarget_ui(
    mut commands: Commands,
    camera: Single<Entity, With<crate::hud::UiCamera>>,
    roots: Query<Entity, (With<Node>, Without<ChildOf>, Without<UiTargetCamera>)>,
) {
    for e in &roots {
        commands.entity(e).insert(UiTargetCamera(*camera));
    }
}

fn use_offscreen_target(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    camera: Single<Entity, With<GameCamera>>,
    ui_camera: Single<Entity, With<crate::hud::UiCamera>>,
) {
    // WSR_SIZE=2400x3600 for key art
    let (w, h) = std::env::var("WSR_SIZE")
        .ok()
        .and_then(|v| {
            let (a, b) = v.split_once('x')?;
            Some((a.parse().ok()?, b.parse().ok()?))
        })
        .unwrap_or((1600u32, 900u32));
    let mut img = Image::new_target_texture(w, h, TextureFormat::Rgba8UnormSrgb, None);
    img.asset_usage = RenderAssetUsages::RENDER_WORLD;
    let handle = images.add(img);
    commands
        .entity(*camera)
        .insert(RenderTarget::from(handle.clone()));
    commands
        .entity(*ui_camera)
        .insert(RenderTarget::from(handle.clone()));
    commands.insert_resource(OffscreenTarget(handle));
}

enum Step {
    Shot(&'static str),
    Tap(KeyCode),
    Hold(KeyCode),
    Release(KeyCode),
    MouseTap,
    MouseHold,
    MouseRelease,
    Aim(Option<Vec3>),
    Teleport(Vec3),
    /// Place a guard: name, position, yaw. Gives it full health and a fresh patrol.
    PlaceGuard(&'static str, Vec3, f32),
    AiOnly(Option<&'static str>),
    SetMinute(f32),
    GotoLevel2,
    GotoLevel3,
    GotoLevel4,
    GotoLevel5,
    GotoLevel6,
    GotoLevel7,
    /// Start the rooftop opening again.
    Reset7,
    /// Force the rooftop phase.
    SetPhase7(crate::level7::Phase),
    /// Stand beside the thrown gun.
    GotoGun,
    /// Set Granmèt Nwit's health.
    SetNwitHp(f32),
    /// Level 6: empty Nwit's health-bottle stash, so he can bleed.
    DrainReserve,
    /// Pretend all bosses were killed and the Sword Man was not freed.
    SetStoryAll,
    /// Stand 1.2 m from the Sword Man.
    GotoNearSword,
    /// Full health and no bleed (test convenience).
    Heal,
    /// Ro goes down (health 0): he should come back at the save point with the ammunition he had there.
    Kill,
    /// Build the poster scene (WSR_POSTER=1).
    Poster,
    /// Set the poison clock to this many seconds (the poison must be active).
    SetPoison(f32),
    /// Stand the whole cast in a row in front of the camera (WSR_CAST=1).
    Lineup(f32, usize, usize),
    /// Stand 1.2 m from the nearest tranced Lost.
    GotoNearestLost,
    /// Make Granmèt Nwit start an ambush now.
    Ambush,
    SetAttempts(u32),
    TrackNwit(bool),
    /// Keep aiming at Tèt Wòch.
    TrackBoss(bool),
    /// Stand beside the vial with this role.
    GotoVial(VialRole),
    Log(&'static str),
    Exit,
}

fn v(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

fn intro() -> Vec<(f32, Step)> {
    use Step::*;
    vec![
        (0.8, Tap(KeyCode::Enter)),
        (1.1, Tap(KeyCode::Enter)),
        (1.4, Tap(KeyCode::Enter)),
        (1.7, Tap(KeyCode::Enter)),
        (3.0, AiOnly(Some("nobody"))),
    ]
}

fn combat_script() -> Vec<(f32, Step)> {
    use Step::*;
    let south = std::f32::consts::PI;
    let mut s = intro();
    s.extend(vec![
        // --- AK-47: a burst into a standing target 8 m away.
        (3.2, PlaceGuard("Kendy", v(5.0, 0.0, -6.0), south)),
        (3.3, Teleport(v(5.0, 0.0, 2.0))),
        (3.4, Tap(KeyCode::Digit3)),
        (3.5, Aim(Some(v(5.0, 1.1, -6.0)))),
        (3.9, Log("AK selected, 30/120")),
        (4.0, MouseHold),
        (4.15, Shot("c01_ak_firing")),
        (4.3, Shot("c02_ak_firing")),
        (4.6, MouseRelease),
        (4.8, Log("after a burst: ammo down, Kendy damaged")),
        (4.9, Shot("c03_ak_hit")),
        (5.0, MouseHold),
        (5.8, MouseRelease),
        (6.0, Log("second burst: Kendy should be Fallen, weight +20")),
        (6.2, Shot("c04_fallen")),
        // --- Pistol: one headshot.
        (6.6, PlaceGuard("Kendy", v(5.0, 0.0, -6.0), south)),
        (6.7, Tap(KeyCode::Digit2)),
        (6.9, Aim(Some(v(5.0, 1.55, -6.0)))),
        (7.2, MouseTap),
        (7.5, Log("pistol headshot: Kendy Fallen from one round")),
        // --- Reload.
        (7.6, Tap(KeyCode::KeyR)),
        (7.8, Log("reloading the pistol")),
        (9.6, Log("reload done: 15 in the magazine")),
        // --- Return fire: Roody hunts the player in the open and shoots.
        (9.8, Aim(None)),
        (9.9, AiOnly(Some("Roody"))),
        (10.0, PlaceGuard("Roody", v(0.0, 0.0, -6.0), south)),
        (10.1, Teleport(v(0.0, 0.0, 6.0))),
        (13.0, Shot("c05_return_fire")),
        (14.5, Log("Roody saw him and fired: health below 100")),
        (16.0, Shot("c06_return_fire")),
        (16.2, Log("still under fire")),
        // --- Grenade at the fuel store.
        (16.4, AiOnly(Some("nobody"))),
        (16.5, PlaceGuard("Frantz", v(20.0, 0.0, -6.0), south)),
        (16.6, Teleport(v(12.0, 0.0, -3.0))),
        (16.7, Aim(Some(v(18.0, 0.1, -8.2)))),
        (16.8, Tap(KeyCode::Digit4)),
        (17.2, MouseTap),
        (17.8, Shot("c07_grenade_flying")),
        (19.9, Shot("c08_blast")),
        (20.1, Shot("c09_blast")),
        (20.6, Shot("c10_smoke")),
        (21.5, Log("grenade: fuel store done, Frantz hurt or down, 4 grenades left")),
        // --- Knife from behind.
        (21.7, PlaceGuard("Dieuseul", v(-6.0, 0.0, -3.0), south)),
        (21.8, Teleport(v(-6.0, 0.0, -4.2))),
        (21.9, Tap(KeyCode::Digit1)),
        (22.2, Tap(KeyCode::KeyF)),
        (22.5, Shot("c11_knife")),
        (23.0, Log("knife from behind: Dieuseul Fallen")),
        // --- Night: the fallen rise again.
        (23.2, SetMinute(17.0 * 60.0 + 59.0)),
        (25.2, Shot("c12_risen")),
        (25.5, Log("after 6:00 PM: the fallen are standing again")),
        (27.5, Exit),
    ]);
    s
}

fn level_script() -> Vec<(f32, Step)> {
    use Step::*;
    let south = std::f32::consts::PI;
    vec![
        (0.8, Shot("01_disclaimer")),
        (1.2, Tap(KeyCode::Enter)),
        (1.8, Shot("02_pitch")),
        (2.0, Tap(KeyCode::Enter)),
        (2.4, Tap(KeyCode::Enter)),
        (2.8, Tap(KeyCode::Enter)),
        (3.6, Log("level started")),
        (3.8, AiOnly(Some("nobody"))),
        (4.0, Shot("03_level_start")),
        // --- Mobility.
        (4.5, Hold(KeyCode::KeyW)),
        (5.5, Release(KeyCode::KeyW)),
        (5.6, Log("walked 1s (z drop ~4)")),
        (5.8, Hold(KeyCode::ShiftLeft)),
        (5.85, Hold(KeyCode::KeyW)),
        (6.85, Release(KeyCode::KeyW)),
        (6.9, Release(KeyCode::ShiftLeft)),
        (7.0, Log("sprinted 1s (z drop ~7)")),
        (7.2, Hold(KeyCode::KeyC)),
        (7.3, Hold(KeyCode::KeyW)),
        (8.3, Release(KeyCode::KeyW)),
        (8.4, Log("crouched 1s (z drop ~2)")),
        (8.5, Release(KeyCode::KeyC)),
        (9.0, Tap(KeyCode::Space)),
        (9.3, Log("jump apex (y ~1)")),
        (9.4, Shot("04_jump")),
        (10.2, Log("landed (y 0)")),
        (10.4, Teleport(v(-4.0, 0.0, 13.6))),
        (10.6, Hold(KeyCode::KeyS)),
        (10.7, Tap(KeyCode::Space)),
        (11.2, Log("mid-vault over the 1 m crate (y > 0)")),
        (12.0, Release(KeyCode::KeyS)),
        (12.2, Log("after vaulting")),
        (12.4, Tap(KeyCode::KeyV)),
        (12.7, Shot("05_first_person")),
        (12.8, Tap(KeyCode::KeyV)),
        (13.0, Hold(KeyCode::ArrowRight)),
        (13.4, Release(KeyCode::ArrowRight)),
        (13.5, Shot("06_turned_camera")),
        // --- Recognition.
        (13.6, Teleport(v(0.0, 0.0, 26.0))),
        (13.7, AiOnly(Some("Dieuseul"))),
        (13.8, PlaceGuard("Dieuseul", v(0.0, 0.0, 17.0), 0.0)),
        (13.9, Teleport(v(0.0, 0.0, 12.0))),
        (14.6, Log("guard 5 m ahead, standing in the open: noticing")),
        (14.7, Shot("07_noticed")),
        (20.0, Log("he has chased and shot at the player")),
        // --- Hiding behind a bush.
        (20.4, PlaceGuard("Dieuseul", v(-3.0, 0.0, 2.0), south)),
        (20.5, Teleport(v(-3.0, 0.0, 10.4))),
        (20.6, Hold(KeyCode::KeyC)),
        (22.4, Log("crouched behind the bush: awareness stays 0")),
        (22.5, Shot("08_hiding")),
        (22.6, Release(KeyCode::KeyC)),
        (24.4, Log("standing behind the same bush: awareness rises")),
        // --- Wake by name.
        (24.6, AiOnly(Some("nobody"))),
        (24.7, PlaceGuard("Dieuseul", v(5.0, 0.0, 0.0), south)),
        (24.8, Teleport(v(5.0, 0.0, -1.3))),
        (25.0, Tap(KeyCode::KeyE)),
        (25.3, Log("woke Dieuseul from behind: woken 1")),
        // --- Sniper.
        (25.5, AiOnly(Some("Mackenson"))),
        (25.6, Teleport(v(-10.0, 0.0, -6.0))),
        (28.0, Shot("09_sniper")),
        (30.0, Log("sniper has fired at the player")),
        // --- Sabotage then the door.
        (30.2, AiOnly(Some("nobody"))),
        (30.3, Teleport(v(-16.0, 0.0, 2.6))),
        (30.5, Hold(KeyCode::KeyE)),
        (31.5, Shot("10_sabotage")),
        (33.3, Release(KeyCode::KeyE)),
        (33.4, Log("truck sabotaged")),
        (33.5, Teleport(v(17.0, 0.0, -5.6))),
        (33.7, Hold(KeyCode::KeyE)),
        (36.4, Release(KeyCode::KeyE)),
        (36.5, Log("fuel sabotaged")),
        (36.6, Teleport(v(14.0, 0.0, 13.9))),
        (36.8, Hold(KeyCode::KeyE)),
        (39.5, Release(KeyCode::KeyE)),
        (39.6, Log("food sabotaged: door unlocks")),
        (39.8, Teleport(v(0.0, 0.0, -14.0))),
        (40.0, Shot("11_door_open")),
        (40.1, Teleport(v(0.0, 0.0, -17.6))),
        (41.0, Log("at the door: Complete")),
        (41.5, Shot("12_complete")),
        (42.0, Exit),
    ]
}

fn level7_script(kind: &str) -> Vec<(f32, Step)> {
    use crate::level7::Phase;
    use Step::*;
    let mut s = intro();
    s.extend(vec![(3.2, GotoLevel7), (4.5, Log("on the roof")), (4.6, Shot("r01_arrival"))]);
    if kind == "openings" {
        s.extend(vec![
            // --- Path A: hand him a bottle (fake death, lunge: keep moving).
            (5.0, Teleport(v(-2.5, 0.0, 0.0))),
            (5.2, Tap(KeyCode::KeyE)),
            (5.5, Log("bottle: fake death, one bottle gone, Nwit hp 650")),
            (6.7, Shot("r02_fake_death")),
            (7.7, Hold(KeyCode::ShiftLeft)),
            (7.75, Hold(KeyCode::KeyD)),
            (9.5, Release(KeyCode::KeyD)),
            (9.55, Release(KeyCode::ShiftLeft)),
            (9.6, Log("kept moving: the lunge should have missed")),
            (9.7, Shot("r03_missed")),
            (10.8, Reset7),
            (10.9, Heal),
            // --- Path B: shoot him (disarmed).
            (11.2, Teleport(v(-3.0, 0.0, 0.0))),
            (11.3, Tap(KeyCode::Digit3)),
            (11.5, Aim(Some(v(-6.0, 1.0, 0.0)))),
            (11.7, MouseTap),
            (12.3, Log("shot: disarmed, cut, bleeding, gun across the roof")),
            (12.4, Shot("r04_disarmed")),
            (12.6, Aim(None)),
            (12.8, GotoGun),
            (13.0, Tap(KeyCode::KeyE)),
            (13.4, Log("gun recovered: not disarmed")),
            (13.6, Reset7),
            (13.7, Heal),
            // --- Path C: rush him (counter-stabs).
            (14.0, Teleport(v(-4.6, 0.0, 0.0))),
            (14.9, Shot("r05_counter_stab")),
            (17.2, Log("rushed: three stabs, about 75 health gone")),
            (17.3, Heal),
            // --- The duel and the kneel.
            (17.5, Teleport(v(0.0, 0.0, 3.0))),
            (17.6, Tap(KeyCode::Digit3)),
            (17.8, TrackNwit(true)),
            (18.0, MouseHold),
            (21.0, MouseRelease),
            (21.1, TrackNwit(false)),
            (21.2, Log("the duel: Nwit hurt, still lunging")),
            (21.3, Shot("r06_duel")),
            (21.5, Heal),
            (21.6, SetNwitHp(90.0)),
            (22.2, Log("a fifth left: he kneels, the door is unlocked")),
            (22.3, Shot("r07_kneel")),
            (22.8, Exit),
        ]);
        return s;
    }
    // Short routes straight to each ending.
    s.extend(vec![
        (5.0, SetPhase7(Phase::Duel)),
        (5.1, SetNwitHp(90.0)),
        (5.8, Log("kneeling")),
        (5.9, Shot("e01_kneel")),
    ]);
    match kind {
        "justice" => s.extend(vec![
            (6.2, Teleport(v(-4.6, 0.0, 0.2))),
            (6.4, Hold(KeyCode::KeyE)),
            (8.0, Release(KeyCode::KeyE)),
            (8.5, Shot("e02_helicopter_in")),
            (10.7, Shot("e03_lifted")),
            (12.5, Shot("e04_gone")),
            (15.5, Log("ending screen: the hand-over")),
            (15.7, Shot("e05_ending")),
            (16.2, Exit),
        ]),
        "throne" => s.extend(vec![
            (6.2, Teleport(v(-7.4, 0.0, 0.9))),
            (6.5, Tap(KeyCode::KeyE)),
            (8.5, Log("ending screen: the candle passes")),
            (8.7, Shot("e05_ending")),
            (9.2, Exit),
        ]),
        "road" => s.extend(vec![
            (6.2, Teleport(v(23.7, 0.0, 0.0))),
            (7.5, Log("ending screen: the road")),
            (7.7, Shot("e05_ending")),
            (8.2, Exit),
        ]),
        _ => s.extend(vec![
            // blood: shoot him dead
            (6.2, Teleport(v(-1.5, 0.0, 0.0))),
            (6.3, Tap(KeyCode::Digit3)),
            (6.5, TrackNwit(true)),
            (6.7, MouseHold),
            (8.0, MouseRelease),
            (8.1, TrackNwit(false)),
            (10.7, Log("ending screen: the same road")),
            (10.9, Shot("e05_ending")),
            (11.4, Exit),
        ]),
    }
    s
}

fn level6_script() -> Vec<(f32, Step)> {
    use Step::*;
    let mut s = intro();
    s.extend(vec![
        (3.2, GotoLevel6),
        (3.4, AiOnly(Some("nobody"))),
        (4.5, Log("inside his quarters")),
        (4.6, Shot("n01_antechamber")),
        // --- A tour: the Jacuzzi, the ritual chamber, the gallery, the bedroom.
        (5.0, Teleport(v(12.0, 0.0, 5.5))),
        (5.5, Shot("n02_jacuzzi")),
        (5.7, Teleport(v(0.0, 0.0, 4.5))),
        (6.2, Shot("n03_ritual")),
        (6.4, Teleport(v(-10.5, 0.0, 3.0))),
        (6.6, Tap(KeyCode::KeyE)),
        (6.9, Log("read the gallery note")),
        (7.0, Shot("n04_gallery")),
        (7.2, Teleport(v(-24.0, 0.0, 4.0))),
        (7.7, Shot("n05_bedroom")),
        // --- The hunt: three ambushes, shoot him each time he steps out.
        (7.9, Teleport(v(0.0, 0.0, 13.0))),
        (8.0, Tap(KeyCode::Digit3)),
        (8.2, Ambush),
        (8.6, Log("ambush 1: the cue says which wall")),
        (9.8, Shot("n06_stepping_out")),
        (10.0, TrackNwit(true)),
        (10.1, MouseHold),
        (11.4, MouseRelease),
        (11.6, Log("shot him: Nwit hp down, he vanished (Woy)")),
        (11.7, TrackNwit(false)),
        (11.8, Heal),
        (11.9, Ambush),
        (13.6, TrackNwit(true)),
        (13.7, MouseHold),
        (15.0, MouseRelease),
        (15.2, Log("second window: hp down more")),
        (15.3, TrackNwit(false)),
        (15.4, Heal),
        // --- He cannot be reached while hidden; a graze costs him nothing: 1 hit then flee at half.
        (15.4, Tap(KeyCode::KeyR)),
        (15.45, DrainReserve),
        (15.5, SetNwitHp(340.0)), // under the flee line: the stash is drained, so he runs for the roof
        (15.6, Ambush),
        (17.3, TrackNwit(true)),
        (17.4, MouseHold),
        (18.4, MouseRelease),
        (18.6, TrackNwit(false)),
        (18.7, Log("Nwit fled at half health: stairs open")),
        (18.8, Shot("n07_fled")),
        // --- 6:00 PM: the ones left behind rise.
        (19.0, SetStoryAll),
        (19.1, SetMinute(17.0 * 60.0 + 59.7)),
        (21.0, Log("6 PM: Madanm Flanm, Tèt Wòch and the Sword Man have risen")),
        (21.1, Shot("n08_night")),
        // --- Out by the roof stairs.
        (21.3, Teleport(v(27.0, 0.0, -16.5))),
        (22.3, Log("Complete6")),
        (22.8, Exit),
    ]);
    s
}

fn level5_script() -> Vec<(f32, Step)> {
    use Step::*;
    let mut s = intro();
    s.extend(vec![
        (3.2, GotoLevel5),
        (4.5, Log("arrived on the second floor")),
        (4.6, Shot("s01_arrival")),
        // --- The armory: the M50.
        (5.0, Teleport(v(-25.0, 0.0, 15.2))),
        (5.2, Tap(KeyCode::KeyE)),
        (5.5, Log("took the M50")),
        (5.6, Tap(KeyCode::Digit5)),
        (6.0, Aim(Some(v(-25.0, 1.2, 8.0)))),
        (6.2, MouseHold),
        (6.5, Shot("s02_m50")),
        (7.4, MouseRelease),
        (7.6, Log("M50 burst: ammo well below 200")),
        // --- A spike trap: stand on it through a full cycle.
        (7.8, Aim(None)),
        (8.0, Teleport(v(15.0, 0.0, 8.0))),
        (9.5, Shot("s03_trap_a")),
        (11.0, Shot("s03_trap_b")),
        (12.5, Shot("s03_trap_c")),
        (13.5, Log("stood on the spikes: health lost")),
        (13.6, Heal),
        // --- The hall: the swarm and the Sword Man appear; bottles and the ledger.
        (13.8, AiOnly(Some("nobody"))),
        (14.0, Teleport(v(0.0, 0.0, -8.0))),
        (14.5, Log("in the hall: the Sword Man has appeared")),
        (14.6, Shot("s04_hall")),
        (14.8, Teleport(v(-2.5, 0.0, -9.2))),
        (15.0, Tap(KeyCode::KeyE)),
        (15.2, Teleport(v(-1.5, 0.0, -9.2))),
        (15.4, Tap(KeyCode::KeyE)),
        (15.6, Teleport(v(-0.5, 0.0, -9.2))),
        (15.8, Tap(KeyCode::KeyE)),
        (16.0, Teleport(v(4.6, 0.0, -9.2))),
        (16.2, Tap(KeyCode::KeyE)),
        (16.5, Log("3 bottles and the ledger: name known")),
        // --- Bullets and grenades barely touch him (shot from a clear line while he is frozen).
        (16.6, Teleport(v(7.0, 0.0, -17.0))),
        (16.7, Tap(KeyCode::Digit3)),
        (16.9, TrackBoss(true)),
        (17.1, MouseHold),
        (19.1, MouseRelease),
        (19.3, Log("AK at the Sword Man: ~20 off 1000, a little faster")),
        (19.4, Tap(KeyCode::Digit4)),
        (19.7, MouseTap),
        (22.8, Log("grenade: Sword Man hp unchanged")),
        (22.9, TrackBoss(false)),
        // --- The fight: he cuts, you bleed, a bottle stops it.
        (23.0, Teleport(v(0.0, 0.0, -4.5))),
        (23.1, AiOnly(None)),
        (25.8, Shot("s05_slashed")),
        (26.0, Log("the blade cut: bleeding")),
        (26.2, Tap(KeyCode::KeyH)),
        (26.5, Log("bottle used: bleed 0")),
        (26.6, Shot("s06_sword_man")),
        // --- Say his name.
        (27.2, AiOnly(Some("nobody"))),
        (27.3, Heal),
        (27.4, GotoNearSword),
        (27.6, Hold(KeyCode::KeyE)),
        (29.6, Release(KeyCode::KeyE)),
        (29.8, Log("said his name: freed")),
        (29.9, Shot("s07_freed")),
        // --- The hidden reserve room.
        (30.1, Teleport(v(-20.0, 0.0, -5.8))),
        (30.3, Tap(KeyCode::KeyE)),
        (30.6, Teleport(v(-27.0, 0.0, -16.6))),
        (30.8, Tap(KeyCode::KeyE)),
        (31.1, Teleport(v(-27.0, 0.0, -13.1))),
        (31.3, Tap(KeyCode::KeyE)),
        (31.6, Shot("s08_reserve")),
        (31.7, Log("reserve room: unlimited bottle, tether cut")),
        // --- Break the binding and leave.
        (31.9, Teleport(v(0.0, 0.0, -8.0))),
        (32.1, Hold(KeyCode::KeyE)),
        // The zombies crowd Ro while he works, and a name said to one of them takes the E key: hold longer.
        (38.6, Release(KeyCode::KeyE)),
        (38.8, Log("binding broken: stairs open")),
        (38.9, Shot("s09_stairs")),
        (39.0, Teleport(v(27.0, 0.0, -16.5))),
        (40.0, Log("Complete5")),
        (41.0, Exit),
    ]);
    s
}

fn level4_script() -> Vec<(f32, Step)> {
    use Step::*;
    let west = std::f32::consts::FRAC_PI_2; // faces -X
    let mut s = intro();
    s.extend(vec![
        (3.2, GotoLevel4),
        (4.5, Log("arrived on the first floor")),
        (4.6, Shot("g01_arrival")),
        // --- The breaker calms the Lost.
        (5.0, Teleport(v(-14.0, 0.0, -16.2))),
        (5.2, Hold(KeyCode::KeyE)),
        (8.6, Release(KeyCode::KeyE)),
        (8.7, Log("breaker thrown: trance off")),
        // --- Tèt Wòch: bullets are slow, the knife does nothing.
        (9.0, AiOnly(Some("Tèt Wòch"))),
        (9.1, PlaceGuard("Tèt Wòch", v(19.0, 0.0, -8.0), west)),
        (9.2, Teleport(v(15.5, 0.0, -8.0))),
        (9.4, Tap(KeyCode::Digit1)),
        (9.7, Tap(KeyCode::KeyF)),
        (10.0, Log("knife on armor: hp unchanged (300)")),
        (10.1, Shot("g02_knife_skids")),
        (10.3, Tap(KeyCode::Digit3)),
        (10.5, Aim(Some(v(19.0, 1.1, -8.0)))),
        (10.7, MouseHold),
        (11.5, MouseRelease),
        (11.7, Log("AK burst on armor: hp well above 200")),
        // --- The glass: two grenades from the north walkway while he stands on it.
        (11.9, AiOnly(Some("nobody"))),
        (12.0, PlaceGuard("Tèt Wòch", v(6.0, 0.0, 8.0), west)),
        (12.1, Teleport(v(0.0, 0.0, -16.0))),
        (12.2, Teleport(v(-11.0, 0.0, -4.0))),
        (12.25, Shot("g02b_glass_intact")),
        (12.28, Teleport(v(0.0, 0.0, -16.0))),
        (12.3, Aim(Some(v(-5.0, 0.1, -8.0)))),
        (12.4, Tap(KeyCode::Digit4)),
        (12.8, MouseTap),
        (15.8, Shot("g03_blast1")),
        (16.2, Log("first blast: glass cracked, boss unharmed")),
        (16.3, Shot("g04_cracked")),
        (16.35, Teleport(v(-11.0, 0.0, -4.0))),
        (16.45, Shot("g04b_cracked_wide")),
        (16.5, Teleport(v(0.0, 0.0, -16.0))),
        (16.6, MouseTap),
        (19.6, Shot("g05_collapse")),
        (20.0, Shot("g06_falling")),
        (20.1, Teleport(v(-11.0, 0.0, -4.0))),
        (20.5, Shot("g06b_pit_wide")),
        (20.6, Teleport(v(0.0, 0.0, -16.0))),
        (21.0, Log("glass collapsed: boss in the pit, stunned, hp about 150")),
        (21.1, Shot("g07_pit")),
        // --- Finish him from above.
        (21.3, Teleport(v(0.0, 0.0, -13.2))),
        (21.4, Tap(KeyCode::Digit3)),
        (21.6, Aim(None)),
        (21.7, TrackBoss(true)),
        (22.0, MouseHold),
        (24.2, MouseRelease),
        (24.3, TrackBoss(false)),
        (24.5, Log("boss down: stairs open")),
        (24.6, Shot("g08_boss_down")),
        // --- Ro falls into the pit himself and climbs out.
        (24.8, Teleport(v(4.0, 0.0, 4.0))),
        (26.2, Log("fell into the pit: below -5, hurt")),
        (26.3, Shot("g09_ro_in_pit")),
        (26.5, Teleport(v(2.0, -6.0, -10.8))),
        (26.7, Hold(KeyCode::KeyA)),
        (31.5, Release(KeyCode::KeyA)),
        (31.7, Log("walked west along the pit stairs (y should rise)")),
        (32.0, Teleport(v(27.0, 0.0, -16.5))),
        (33.0, Log("stairs: Complete4")),
        (33.5, Exit),
    ]);
    s
}

fn level3_script() -> Vec<(f32, Step)> {
    use Step::*;
    let south = std::f32::consts::PI;
    let mut s = intro();
    s.extend(vec![
        (3.2, GotoLevel3),
        (4.5, Log("arrived in the party wing")),
        (4.6, Shot("p01_arrival")),
        // --- The Lost come for Ro in the pool hall.
        (5.0, Teleport(v(-6.0, 0.0, 17.0))),
        (5.2, Shot("p02_pool")),
        (9.0, Log("Lost drifting in: some may be clinging")),
        (9.1, Shot("p03_swarm")),
        // --- Wake one by name.
        (9.3, GotoNearestLost),
        (9.5, Hold(KeyCode::KeyE)),
        (10.9, Release(KeyCode::KeyE)),
        (11.0, Log("woke one Lost by name: woken 1, Weight lower")),
        // --- Cut the music at the DJ booth.
        (11.2, Teleport(v(17.0, 0.0, -15.0))),
        (11.4, Hold(KeyCode::KeyE)),
        (12.6, Shot("p04_booth")),
        (14.8, Release(KeyCode::KeyE)),
        (14.9, Log("music cut")),
        (15.0, Shot("p05_silent")),
        // --- A gift with a price.
        (15.2, Teleport(v(-5.0, 0.0, -12.6))),
        (15.4, Tap(KeyCode::KeyE)),
        (15.7, Log("took a gift: more ammo, Weight +10")),
        // --- Madanm Flanm.
        (15.9, AiOnly(Some("Madanm Flanm"))),
        (16.0, PlaceGuard("Madanm Flanm", v(19.0, 0.0, -12.0), south)),
        (16.1, Teleport(v(19.0, 0.0, -2.0))),
        (16.2, Tap(KeyCode::Digit3)),
        (16.3, Aim(Some(v(19.0, 1.1, -12.0)))),
        (16.6, MouseHold),
        (18.0, Shot("p06_boss")),
        (20.5, Log("firefight with Madanm Flanm")),
        (22.5, Log("firefight")),
        (24.0, MouseRelease),
        (24.2, Log("she should be down, stairs open")),
        (24.3, Shot("p07_stairs")),
        (24.5, Teleport(v(27.0, 0.0, -16.5))),
        (25.5, Log("stairs: Complete3")),
        (26.0, Exit),
    ]);
    s
}

fn level2_script() -> Vec<(f32, Step)> {
    use Step::*;
    let mut s = intro();
    s.extend(vec![
        (3.2, GotoLevel2),
        (4.5, Log("inside the house")),
        (4.6, Shot("h01_foyer")),
        // --- Notes: the clue to the cure.
        (4.8, Teleport(v(3.0, 0.0, 15.0))),
        (5.0, Hold(KeyCode::KeyD)),
        (6.2, Release(KeyCode::KeyD)),
        (6.3, Log("walking in the foyer")),
        (6.4, Teleport(v(13.0, 0.0, 15.8))), // beside the pantry note at (13,15)
        (6.6, Tap(KeyCode::KeyE)),
        (6.8, Log("read the pantry note")),
        (6.9, Shot("h02_note")),
        // --- Ambush 1: shoot him, he vanishes.
        (7.2, Teleport(v(-5.0, 0.0, -2.0))),
        (7.3, Ambush),
        (7.5, Shot("h03_telegraph")),
        (9.2, TrackNwit(true)),
        (9.3, Shot("h04_charging")),
        (9.4, MouseTap),
        (9.7, MouseTap),
        (10.0, Log("shot at him: he should have vanished (Woy)")),
        (10.1, TrackNwit(false)),
        // --- Ambushes 2 and 3: he reaches Ro, the second a slash, then the stab.
        (10.3, Aim(None)),
        (10.4, Ambush),
        (13.0, Log("ambush 2 reached Ro: slash, health 75")),
        (13.2, SetAttempts(2)),
        (13.3, Ambush),
        (15.8, Shot("h05_stab")),
        (16.0, Log("the stab: poisoned, ~120 s")),
        (17.0, Shot("h05b_stab_card")),
        (17.1, SetPoison(25.0)),
        (17.5, Shot("h05c_last_30s_red")),
        // --- Run to the altar and test the vials: drug first, then the cure.
        (17.7, Teleport(v(-5.0, 0.0, -12.5))),
        (17.8, Shot("h06_altar")),
        (17.9, GotoVial(VialRole::Drug)),
        (18.1, Log("beside the drug vial: smell prompt")),
        (18.2, Tap(KeyCode::KeyE)),
        (18.5, Hold(KeyCode::KeyW)),
        (19.7, Release(KeyCode::KeyW)),
        (19.8, Shot("h07_drugged")),
        (19.9, Log("drugged: purple pulse, steering drifts")),
        (20.0, GotoVial(VialRole::Cure)),
        (20.2, Log("beside the cure vial: smell prompt")),
        (20.3, Tap(KeyCode::KeyE)),
        (20.7, Log("cured: poison off, 5 bottle uses, saved, door open")),
        (20.8, Shot("h08_cured")),
        // --- Bottle, then the death vial, then out.
        (21.0, Tap(KeyCode::KeyH)),
        (21.2, Log("used the bottle (only if hurt)")),
        (21.3, GotoVial(VialRole::Death)),
        (21.5, Log("beside the death vial: sharp smell")),
        (21.6, Tap(KeyCode::KeyE)),
        (22.0, Log("wrong vial: back at the save point at full health")),
        (22.2, Teleport(v(-5.0, 0.0, -18.5))),
        (22.7, Shot("h09_door")),
        (22.8, Teleport(v(-5.0, 0.0, -19.6))),
        (23.7, Log("through the north door: Complete2")),
        (25.0, Exit),
    ]);
    s
}

fn script() -> Vec<(f32, Step)> {
    use Step::*;
    if std::env::var_os("WSR_COMBAT").is_some() {
        return combat_script();
    }
    if std::env::var_os("WSR_MOVES").is_some() {
        // The new movement clips: lie down, crawl, get up, crouch, aim, leap (third person, WSR_CAMDIST helps).
        let mut s = intro();
        s.extend(vec![
            (3.2, Teleport(v(5.0, 0.0, 2.0))),
            (3.4, Tap(KeyCode::Digit3)),
            (4.0, Shot("m00_stand")),
            (4.2, Tap(KeyCode::KeyZ)),
            (4.5, Shot("m01_going_down")),
            (5.6, Shot("m02_prone_idle")),
            (5.8, Hold(KeyCode::KeyW)),
            (6.8, Shot("m03_crawl")),
            (7.6, Release(KeyCode::KeyW)),
            (7.7, Tap(KeyCode::KeyZ)),
            (8.0, Shot("m04_getting_up")),
            (9.2, Shot("m05_standing")),
            (9.3, Hold(KeyCode::KeyC)),
            (10.3, Shot("m06_crouch_aim")),
            (10.4, Hold(KeyCode::KeyW)),
            (11.3, Shot("m07_crouch_walk")),
            (11.5, Release(KeyCode::KeyW)),
            (11.6, Release(KeyCode::KeyC)),
            (11.7, Teleport(v(5.0, 0.0, 14.0))),
            (12.0, Hold(KeyCode::ShiftLeft)),
            (12.1, Hold(KeyCode::KeyW)),
            (12.5, Tap(KeyCode::Space)),
            (12.75, Shot("m08_run_jump")),
            (13.0, Shot("m09_run_jump_late")),
            (13.5, Release(KeyCode::KeyW)),
            (13.6, Release(KeyCode::ShiftLeft)),
            (13.8, Log("moves done")),
            (14.0, Exit),
        ]);
        return s;
    }
    if std::env::var_os("WSR_SWORDTEST").is_some() {
        let mut s = intro();
        s.extend(vec![
            (3.2, GotoLevel5),
            (4.0, AiOnly(Some("nobody"))),
            (4.2, Teleport(v(0.0, 0.0, -8.0))),
            (4.8, Teleport(v(7.0, 0.0, -17.0))),
            (5.2, Log("sword man out (frozen)")),
            (5.3, Tap(KeyCode::Digit3)),
            (5.5, TrackBoss(true)),
            (5.7, MouseHold),
            (7.7, MouseRelease),
            (7.9, Log("after 2s of AK at the frozen Sword Man")),
            (8.0, Exit),
        ]);
        return s;
    }
    if let Some((from, count)) = std::env::var("WSR_CASTSEL").ok().and_then(|v| {
        let mut it = v.split(',').filter_map(|x| x.parse::<usize>().ok());
        Some((it.next()?, it.next()?))
    }) {
        // A few of the cast up close (WSR_CASTSEL=from,count with WSR_CAMDIST=4.5).
        let mut s = intro();
        s.extend(vec![
            (3.4, AiOnly(Some("nobody"))),
            (3.45, Teleport(v(0.0, 0.0, 2.0))),
            (3.5, Lineup(0.0, from, count)),
            (6.0, Shot("s01")),
            (8.0, Exit),
        ]);
        return s;
    }
    if std::env::var_os("WSR_CAST").is_some() {
        // The whole cast in a row (run with WSR_CAMDIST=7 WSR_CAMPITCH=-0.15).
        let mut s = intro();
        s.extend(vec![
            (3.4, AiOnly(Some("nobody"))),
            (3.45, Teleport(v(0.0, 0.0, 2.0))),
            (3.5, Lineup(0.0, 0, 10)),
            (6.0, Shot("k01_idle")),
            (6.5, Shot("k02_idle")),
            (7.0, Exit),
        ]);
        return s;
    }
    if std::env::var_os("WSR_CLOSEUP").is_some() {
        // Characters up close (run with WSR_CAMDIST=4 WSR_CAMPITCH=-0.35).
        let mut s = intro();
        s.extend(vec![
            (3.4, AiOnly(Some("nobody"))),
            (3.8, Shot("c01_idle")),
            (4.0, Hold(KeyCode::KeyW)),
            (4.9, Shot("c02_walk_a")),
            (5.05, Shot("c03_walk_b")),
            (5.2, Hold(KeyCode::ShiftLeft)),
            (6.2, Shot("c04_run_a")),
            (6.32, Shot("c05_run_b")),
            (6.5, Release(KeyCode::ShiftLeft)),
            (6.55, Release(KeyCode::KeyW)),
            (7.3, Hold(KeyCode::KeyC)),
            (8.4, Shot("c06_crouch_still")),
            (8.5, Hold(KeyCode::KeyW)),
            (9.6, Shot("c07_crouch_walk")),
            (9.7, Release(KeyCode::KeyW)),
            (9.75, Release(KeyCode::KeyC)),
            (10.5, Tap(KeyCode::Space)),
            (10.8, Shot("c08_jump")),
            (11.8, Tap(KeyCode::Digit3)),
            (12.4, Shot("c09_rifle_ready")),
            (12.5, Tap(KeyCode::Digit1)),
            (12.9, Tap(KeyCode::KeyF)),
            (13.0, Shot("c10_slash")),
            (13.5, Exit),
        ]);
        return s;
    }
    if let Ok(kind) = std::env::var("WSR_LEVEL7") {
        return level7_script(&kind);
    }
    if std::env::var_os("WSR_LEVEL6").is_some() {
        return level6_script();
    }
    if std::env::var_os("WSR_LEVEL5").is_some() {
        return level5_script();
    }
    if std::env::var_os("WSR_LEVEL4").is_some() {
        return level4_script();
    }
    if std::env::var_os("WSR_LEVEL3").is_some() {
        return level3_script();
    }
    if std::env::var_os("WSR_LEVEL2").is_some() {
        return level2_script();
    }
    if std::env::var_os("WSR_PLAGUE").is_some() {
        // The gang everywhere: an overview at the start and again after reinforcements
        // have arrived (run with WSR_CAMDIST=60 WSR_CAMPITCH=-1.2). Stand still.
        let mut s = intro();
        // WSR_PLAGUE=2..6 jumps to that level first.
        let goto = match std::env::var("WSR_PLAGUE").ok().as_deref() {
            Some("2") => Some(GotoLevel2),
            Some("3") => Some(GotoLevel3),
            Some("4") => Some(GotoLevel4),
            Some("5") => Some(GotoLevel5),
            Some("6") => Some(GotoLevel6),
            _ => None,
        };
        if let Some(g) = goto {
            s.push((3.05, g));
        }
        s.extend(vec![
            (3.1, AiOnly(None)),
            (4.0, Shot("p01_seed")),
            (4.2, Log("patrols placed")),
            (16.0, Log("after 16 s")),
            (30.0, Shot("p02_arrivals")),
            (30.2, Log("after reinforcements")),
            (38.0, Exit),
        ]);
        return s;
    }
    if std::env::var_os("WSR_POSTER").is_some() {
        let mut s = intro();
        s.extend(vec![
            (3.4, AiOnly(Some("nobody"))),
            (3.6, Poster),
            (9.0, Shot("poster_raw")),
            (10.0, Exit),
        ]);
        return s;
    }
    if std::env::var_os("WSR_DEATH").is_some() {
        // fire the pistol, go down, and check the ammunition comes back to what it was at the start
        let mut s = intro();
        s.extend(vec![
            (3.8, AiOnly(Some("nobody"))),
            (4.0, Log("at the start: pistol 15/45, grenades 5")),
            (4.3, Tap(KeyCode::Digit2)),
            (4.5, MouseTap),
            (4.8, MouseTap),
            (5.1, MouseTap),
            (5.4, MouseTap),
            (5.7, Tap(KeyCode::KeyG)),
            (6.2, Log("after 4 shots and a grenade: ammo is down")),
            (6.4, Kill),
            (7.0, Log("after going down: ammo is back")),
            (7.5, Exit),
        ]);
        return s;
    }
    if std::env::var_os("WSR_INTRO").is_some() {
        // the introduction cards, left alone to watch the kaleidoscope
        use Step::*;
        return vec![
            (3.0, Shot("i01_disclaimer")),
            (9.0, Shot("i02_disclaimer")),
            (10.0, Tap(KeyCode::KeyM)),
            (11.0, Shot("i02b_music_off")),
            (14.0, Tap(KeyCode::Enter)),
            (17.0, Shot("i03_pitch")),
            (24.0, Shot("i04_pitch")),
            (25.0, Exit),
        ];
    }
    if std::env::var_os("WSR_QUICK").is_some() {
        let mut s = intro();
        s.extend(vec![(4.0, Shot("q00")), (5.0, Exit)]);
        return s;
    }
    level_script()
}

pub struct AutoplayPlugin;

impl Plugin for AutoplayPlugin {
    fn build(&self, app: &mut App) {
        // WSR_WINDOW=1 screenshots the real window (Retina scale, real viewport) instead of an offscreen image.
        if std::env::var_os("WSR_WINDOW").is_none() {
            app.add_systems(PostStartup, use_offscreen_target)
                .add_systems(Update, retarget_ui);
        }
        app.add_systems(PreUpdate, run_script.after(InputSystems));
    }
}

/// Read-only state printed by `Log` steps.
#[derive(SystemParam)]
struct Probe<'w, 's> {
    weight: Res<'w, Weight>,
    msg: Res<'w, Message>,
    stats: Res<'w, Stats>,
    objectives: Res<'w, Objectives>,
    state: Res<'w, State<AppState>>,
    loadout: Res<'w, Loadout>,
    poison: ResMut<'w, crate::Poison>,
    prompt: Res<'w, Prompt>,
    l2: Option<Res<'w, Level2>>,
    l3: Option<Res<'w, Level3>>,
    story: ResMut<'w, Story>,
    l7: Option<ResMut<'w, crate::level7::Level7>>,
    l6: Option<ResMut<'w, crate::level6::Level6>>,
    lost: Query<'w, 's, (&'static Lost, &'static Transform), (Without<Player>, Without<Guard>)>,
}

#[derive(Default)]
struct Runner {
    next: usize,
    tapped: Vec<KeyCode>,
    mouse_tapped: bool,
    steps: Vec<(f32, Step)>,
    track: bool,
    track_boss: bool,
}

#[allow(clippy::too_many_arguments)]
fn run_script(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut r: Local<Runner>,
    mut only: ResMut<AiOnly>,
    mut aim: ResMut<AimOverride>,
    mut clock: ResMut<GameClock>,
    mut next_state: ResMut<NextState<AppState>>,
    mut health: ResMut<Health>,
    mut bleed: ResMut<crate::level5::Bleed>,
    mut nwits: Query<&mut Nwit>,
    mut players: Query<(&mut Transform, &mut Mover), (With<Player>, Without<Guard>)>,
    mut guards: Query<(&mut Guard, &mut Transform), Without<Player>>,
    mut probe: Probe,
    target: Option<Res<OffscreenTarget>>,
) {
    let r = &mut *r;
    for k in r.tapped.drain(..) {
        keys.release(k);
    }
    if r.mouse_tapped {
        mouse.release(MouseButton::Left);
        r.mouse_tapped = false;
    }
    if r.steps.is_empty() {
        r.steps = script();
    }
    let t = time.elapsed_secs();

    // Keep aiming at Granmèt Nwit while tracking.
    if r.track {
        for (g, tf) in &guards {
            if g.name == "Granmèt Nwit" && tf.translation.y > -10.0 {
                aim.0 = Some(tf.translation + Vec3::Y * 1.0);
            }
        }
    }

    if r.track_boss {
        for (g, tf) in &guards {
            if g.name == "Tèt Wòch" || g.name == "Evens" {
                aim.0 = Some(tf.translation + Vec3::Y * 1.0);
            }
        }
    }

    // One step per frame so each step sees the effects of the one before.
    if r.next >= r.steps.len() || r.steps[r.next].0 > t {
        return;
    }
    match &r.steps[r.next].1 {
        Step::Shot(name) => {
            let shot = match &target {
                Some(t) => Screenshot::image(t.0.clone()),
                None => Screenshot::primary_window(),
            };
            commands
                .spawn(shot)
                .observe(save_to_disk(format!("playtest/{name}.png")));
            println!("[autoplay {t:.1}s] screenshot {name}");
        }
        Step::Tap(k) => {
            keys.press(*k);
            r.tapped.push(*k);
        }
        Step::Hold(k) => keys.press(*k),
        Step::Release(k) => keys.release(*k),
        Step::MouseTap => {
            mouse.press(MouseButton::Left);
            r.mouse_tapped = true;
        }
        Step::MouseHold => mouse.press(MouseButton::Left),
        Step::MouseRelease => mouse.release(MouseButton::Left),
        Step::Aim(p) => aim.0 = *p,
        Step::Teleport(p) => {
            if let Ok((mut tf, mut mv)) = players.single_mut() {
                tf.translation = *p;
                mv.vel = Vec3::ZERO;
            }
        }
        Step::PlaceGuard(name, p, yaw) => {
            for (mut g, mut tf) in &mut guards {
                if g.name == *name {
                    tf.translation = *p;
                    tf.rotation = Quat::from_rotation_y(*yaw);
                    g.state = AiState::Patrol;
                    g.awareness = 0.0;
                    g.timer = 0.0;
                    g.unseen = 0.0;
                    g.aim = 0.0;
                    g.hp = g.max_hp;
                    g.home_y = 0.0;
                    g.stagger = 0.0;
                    g.fall = 0.0;
                    g.fall_yaw = *yaw;
                    g.stagger = 0.0;
                    g.status = LostStatus::Standing;
                    g.route = vec![*p]; // stand still on his post
                    g.wp = 0;
                }
            }
        }
        Step::AiOnly(n) => only.0 = *n,
        Step::SetMinute(m) => clock.minute = *m,
        Step::GotoLevel2 => next_state.set(AppState::Level2),
        Step::GotoLevel3 => next_state.set(AppState::Level3),
        Step::GotoLevel4 => next_state.set(AppState::Level4),
        Step::GotoLevel5 => next_state.set(AppState::Level5),
        Step::GotoLevel6 => next_state.set(AppState::Level6),
        Step::GotoLevel7 => next_state.set(AppState::Level7),
        Step::Reset7 => {
            if let Some(l7) = probe.l7.as_mut() {
                l7.force_reset = true;
            }
        }
        Step::SetPhase7(ph) => {
            if let Some(l7) = probe.l7.as_mut() {
                l7.phase = *ph;
            }
        }
        Step::GotoGun => {
            let gp = probe.l7.as_ref().map(|l| l.gun_pos);
            if let (Some(gp), Ok((mut tf, mut mv))) = (gp, players.single_mut()) {
                tf.translation = Vec3::new(gp.x + 1.0, 0.0, gp.z);
                mv.vel = Vec3::ZERO;
            }
        }
        Step::DrainReserve => {
            if let Some(l6) = probe.l6.as_mut() {
                l6.reserve = 0;
            }
        }
        Step::SetNwitHp(hp) => {
            for (mut g, _) in &mut guards {
                if g.name == "Granmèt Nwit" {
                    g.hp = *hp;
                }
            }
        }
        Step::SetStoryAll => {
            probe.story.flanm_killed = true;
            probe.story.tet_woch_killed = true;
            probe.story.evens_freed = false;
        }
        Step::Poster => {
            commands.insert_resource(crate::poster::PosterRequest);
        }
        Step::SetPoison(t) => {
            probe.poison.remaining = *t;
        }
        Step::Kill => {
            health.hp = 0.0;
        }
        Step::Heal => {
            health.hp = 100.0;
            bleed.0 = 0.0;
        }
        Step::Lineup(speed, from, count) => {
            use crate::character::{Cast, Skin};
            let base = players.iter().next().map(|(t, _)| t.translation).unwrap_or(Vec3::ZERO);
            let cast = [
                Cast::Hero, Cast::GangA, Cast::GangB, Cast::Nwit, Cast::Flanm,
                Cast::Woch, Cast::Manchet, Cast::Brute, Cast::Yaku, Cast::Prisoner,
            ];
            let sel: Vec<Cast> = cast.into_iter().skip(*from).take(*count).collect();
            let n = sel.len() as f32;
            for (i, c) in sel.into_iter().enumerate() {
                let zombie = matches!(c, Cast::Brute | Cast::Yaku | Cast::Prisoner);
                commands.spawn((
                    Transform::from_translation(base + Vec3::new((i as f32 - (n - 1.0) / 2.0) * 1.15, 0.0, -3.0))
                        .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
                    Visibility::default(),
                    Mover { grounded: true, speed: *speed, zombie, ..default() },
                    Skin(c),
                    LevelEntity,
                ));
            }
        }
        Step::GotoNearSword => {
            let at = guards
                .iter()
                .find(|(g, _)| g.name == "Evens")
                .map(|(_, tf)| tf.translation);
            if let (Some(at), Ok((mut tf, mut mv))) = (at, players.single_mut()) {
                tf.translation = Vec3::new(at.x + 1.2, 0.0, at.z);
                mv.vel = Vec3::ZERO;
            }
        }
        Step::GotoNearestLost => {
            if let Ok((mut tf, mut mv)) = players.single_mut() {
                let here = tf.translation;
                let best = probe
                    .lost
                    .iter()
                    .filter(|(l, _)| l.state == LostState::Tranced)
                    .min_by(|a, b| a.1.translation.distance(here).total_cmp(&b.1.translation.distance(here)));
                if let Some((_, lt)) = best {
                    tf.translation = Vec3::new(lt.translation.x + 1.2, 0.0, lt.translation.z);
                    mv.vel = Vec3::ZERO;
                }
            }
        }
        Step::Ambush => {
            for mut n in &mut nwits {
                n.state = NwitState::Lurk;
                n.timer = 0.01;
                n.enabled = true;
            }
        }
        Step::SetAttempts(a) => {
            for mut n in &mut nwits {
                n.attempts = *a;
            }
        }
        Step::TrackBoss(on) => {
            r.track_boss = *on;
            if !*on {
                aim.0 = None;
            }
        }
        Step::TrackNwit(on) => {
            r.track = *on;
            if !*on {
                aim.0 = None;
            }
        }
        Step::GotoVial(role) => {
            if let (Some(l2), Ok((mut tf, mut mv))) = (&probe.l2, players.single_mut()) {
                if let Some(slot) = l2.roles.iter().position(|x| x == role) {
                    let p = crate::level2::VIAL_POS_PUB[slot];
                    tf.translation = Vec3::new(p.x, 0.0, p.z + 1.0);
                    mv.vel = Vec3::ZERO;
                }
            }
        }
        Step::Log(label) => {
            let (pos, crouch) = players
                .single()
                .map(|(t, m)| (t.translation, m.crouch))
                .unwrap_or((Vec3::NAN, 0.0));
            let l = &probe.loadout;
            println!(
                "[autoplay {t:.1}s] {label}\n    state={:?} clock={:.0} health={:.0} weight={:.0} pos=({:.1},{:.2},{:.1}) crouch={crouch:.1} caught={} woken={} kills={} done={:?} door={}\n    gun={:?} pistol={}/{} ak={}/{} m50={}/{} grenades={} reload={:.1}\n    poison: active={} left={:.0} drugged={:.1} bottles={}\n    prompt=\"{}\"\n    msg=\"{}\"",
                probe.state.get(), clock.minute, health.hp, probe.weight.0, pos.x, pos.y, pos.z,
                probe.stats.caught, probe.stats.woken, probe.stats.kills, probe.objectives.done,
                probe.objectives.door_open, l.current, l.pistol_mag, l.pistol_reserve, l.ak_mag,
                l.ak_reserve, l.m50_mag, l.m50_reserve, l.grenades, l.reload, probe.poison.active, probe.poison.remaining,
                probe.poison.drugged, probe.poison.bottles, probe.prompt.0, probe.msg.0
            );
            if let Some(l7) = &probe.l7 {
                println!("      level7: phase={:?} door_open={} ending={:?} opening={}", l7.phase, l7.door_open, l7.ending, l7.opening);
            }
            if let Some(l3) = &probe.l3 {
                let tranced = probe.lost.iter().filter(|(l, _)| l.state == LostState::Tranced).count();
                println!("      lost: {} tranced, music={} stairs_open={}", tranced, l3.music, l3.stairs_open);
            }
            for (g, tf) in &guards {
                if g.kind_is_phantom() {
                    let n = nwits.iter().next();
                    println!(
                        "      {:<12} hp={:.0}/{:.0} at=({:.1},{:.1},{:.1}) state={:?} attempts={} enabled={}",
                        g.name, g.hp, g.max_hp, tf.translation.x, tf.translation.y, tf.translation.z,
                        n.as_ref().map(|n| n.state), n.as_ref().map(|n| n.attempts).unwrap_or(0),
                        n.as_ref().map(|n| n.enabled).unwrap_or(false)
                    );
                } else {
                    println!(
                        "      {:<9} {:?} {:<13} hp={:.0} awareness={:.2} sees={} speed={:.2} at=({:.0},{:.0})",
                        g.name, g.status, g.state.label(), g.hp, g.awareness, g.sees, g.speed_mul,
                        tf.translation.x, tf.translation.z
                    );
                }
            }
        }
        Step::Exit => {
            println!("[autoplay {t:.1}s] done");
            commands.write_message(AppExit::Success);
        }
    }
    r.next += 1;
}
