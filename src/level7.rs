//! Level 7: the rooftop. Granmèt Nwit, weakened, on all fours beside his
//! candle ring. Ro's first move is a trap whichever way he chooses: hand him a
//! bottle (a fake death, then a lunge at the neck: keep moving), shoot him
//! (he disarms Ro and throws the gun across the roof), or rush to handcuff
//! him (he counter-stabs). Then a real duel: he circles, telegraphs and
//! lunges; a miss leaves him exposed. At a fifth of his health he drops the
//! knife and kneels, offering money. Four endings: hand him over to a
//! helicopter, take his money and his place, kill him, or walk away.

use crate::ai::{face, walk_to, AiState, Guard, GuardKind};
use crate::character::{resolve, spawn_humanoid, Box3, Boxes, Mover};
use crate::level::Ctx;
use crate::level2::candle;
use crate::level3::{DOOR_W, WALL_H};
use crate::level5::Bleed;
use crate::style::{glow, grow_box, ink, matte, Ink};
use crate::weapons::{dust_puff, Fx, Health, Loadout, Rng, Shake};
use crate::*;
use bevy::ecs::system::SystemParam;

pub const ARRIVAL: Vec3 = Vec3::new(20.0, 0.0, 0.0);
const NWIT_START: Vec3 = Vec3::new(-6.0, 0.0, 0.0);
const DOOR: Vec3 = Vec3::new(24.0, 1.2, 0.0);
const NWIT_HP: f32 = 500.0;
const HELI_FAR: Vec3 = Vec3::new(-90.0, 18.0, 0.0);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ending {
    /// Hand him over: the helicopter lifts him away.
    Justice,
    /// Take his money and his place.
    Throne,
    /// Walk away down the stairs.
    Road,
    /// Kill him.
    Blood,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    /// On all fours, waiting for Ro to choose.
    Prone,
    /// He drank the bottle and is "dying".
    FakeDeath,
    /// The knife glints: he is about to lunge.
    LungeTell,
    Lunge,
    /// Exposed after a lunge.
    Recover,
    /// Counter-stabbing a rush.
    Counter,
    /// Just disarmed Ro; drinking his own bottle.
    Disarm,
    Duel,
    Kneel,
    Cuffed,
    Dead,
}

#[derive(Resource)]
pub struct Level7 {
    pub phase: Phase,
    pub timer: f32,
    pub stabs: u32,
    pub from: Vec3,
    pub to: Vec3,
    pub lunge_dmg: f32,
    pub neck: bool,
    pub strafe: f32,
    pub strafe_dir: f32,
    pub attack_cd: f32,
    pub taunt_cd: f32,
    pub cuff: f32,
    pub gun_pos: Vec3,
    pub door_open: bool,
    pub last_caught: u32,
    pub force_reset: bool,
    pub ending: Option<Ending>,
    pub ending_timer: f32,
    /// How the opening went (for the tally on screen).
    pub opening: &'static str,
    pub heli_sound: bool,
}

#[derive(Component)]
pub struct Nwit7;
#[derive(Component)]
pub struct NwitBlade;
#[derive(Component)]
pub struct FloorKnife;
#[derive(Component)]
pub struct Money;
#[derive(Component)]
pub struct GunPickup;
#[derive(Component)]
pub struct Heli;
#[derive(Component)]
pub struct Spin(pub f32);

#[derive(SystemParam)]
pub struct Gear<'w> {
    health: ResMut<'w, Health>,
    poison: ResMut<'w, Poison>,
    loadout: ResMut<'w, Loadout>,
    bleed: ResMut<'w, Bleed>,
    shake: ResMut<'w, Shake>,
    msg: ResMut<'w, Message>,
    stats: Res<'w, Stats>,
    keys: Res<'w, ButtonInput<KeyCode>>,
}

fn flat_dist(a: Vec3, b: Vec3) -> f32 {
    Vec2::new(a.x - b.x, a.z - b.z).length()
}

// ------------------------------------------------------------------- setup

#[allow(clippy::too_many_arguments)]
pub fn setup_level7(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    ink_res: Res<Ink>,
    mut rng: ResMut<Rng>,
    mut checkpoint: ResMut<Checkpoint>,
    mut msg: ResMut<Message>,
    mut health: ResMut<Health>,
    mut poison: ResMut<Poison>,
    mut loadout: ResMut<Loadout>,
    mut bleed: ResMut<Bleed>,
    mut rig: ResMut<CamRig>,
    mut objectives: ResMut<Objectives>,
    mut stats: ResMut<Stats>,
    mut ending: ResMut<EndingKind>,
    fx: Res<Fx>,
) {
    let ink_res = ink_res.clone();
    checkpoint.0 = ARRIVAL;
    fx.ambience(&mut commands, 0.12); // the city below
    health.hp = 100.0;
    health.hurt = 0.0;
    bleed.0 = 0.0;
    loadout.disarmed = false;
    loadout.cramped = false;
    // "The level is locked": make sure there are bottles for a long duel.
    poison.bottles = poison.bottles.max(3);
    poison.active = false;
    poison.drugged = 0.0;
    *objectives = Objectives::default();
    *stats = Stats::default();
    ending.0 = None;
    rig.yaw = std::f32::consts::FRAC_PI_2 * 0.0;
    rig.first = false;
    msg.0 = "The roof. The wind. On all fours in a ring of candles, a man mutters: \"Mwen se chandèl, van pa tenyen m...\"".into();

    let floor = matte(&mut mats, 0.2);
    let parapet = matte(&mut mats, 0.34);
    let crate_m = matte(&mut mats, 0.4);
    let metal = matte(&mut mats, 0.5);
    let dark = matte(&mut mats, 0.08);
    let city = matte(&mut mats, 0.1);

    let mut cx = Ctx {
        commands: &mut commands,
        meshes: &mut meshes,
        ink: &ink_res,
        boxes: Vec::new(),
    };
    let plane = cx.meshes.add(Plane3d::default().mesh().size(49.0, 33.0));
    cx.commands.spawn((Mesh3d(plane), MeshMaterial3d(floor), LevelEntity));

    // --- Parapet all round, with a gap on the east side for the stair door.
    cx.block(&parapet, 0.0, 0.0, -16.4, Vec3::new(49.6, 1.2, 0.8), true);
    cx.block(&parapet, 0.0, 0.0, 16.4, Vec3::new(49.6, 1.2, 0.8), true);
    cx.block(&parapet, -24.4, 0.0, 0.0, Vec3::new(0.8, 1.2, 33.6), true);
    let seg = 16.4 - DOOR_W / 2.0;
    cx.block(&parapet, 24.4, 0.0, -(DOOR_W / 2.0 + seg / 2.0), Vec3::new(0.8, 1.2, seg), true);
    cx.block(&parapet, 24.4, 0.0, DOOR_W / 2.0 + seg / 2.0, Vec3::new(0.8, 1.2, seg), true);
    let door_m = glow(&mut mats, Vec3::new(1.0, 0.15, 0.1), 2.0);
    cx.block(&door_m, DOOR.x, 0.0, DOOR.z, Vec3::new(0.4, WALL_H - 0.4, DOOR_W), true);

    // --- Roof clutter: a water tank, vents, units, crates, an antenna mast.
    let tank = cx.meshes.add(Cylinder::new(2.2, 3.0));
    cx.boxes.push(Box3 { c: Vec3::new(-12.0, 1.5, -10.0), h: Vec3::new(2.2, 1.5, 2.2) });
    cx.commands.spawn((
        ink(tank, metal.clone(), cx.ink, Transform::from_xyz(-12.0, 1.5, -10.0), Vec3::new(1.04, 1.03, 1.04)),
        LevelEntity,
    ));
    for (x, z, sx, sy, sz) in [
        (12.0_f32, -10.0_f32, 3.0_f32, 1.6_f32, 2.0_f32),
        (14.0, 10.0, 3.0, 1.6, 2.0),
        (4.0, 9.0, 1.6, 1.0, 1.6),
        (6.0, -8.0, 1.6, 1.0, 1.6),
        (-16.0, 8.0, 2.0, 1.4, 1.2),
        (-4.0, -12.0, 1.4, 1.0, 1.4),
    ] {
        cx.block(&crate_m, x, 0.0, z, Vec3::new(sx, sy, sz), true);
    }
    cx.block(&metal, -20.0, 0.0, 0.0, Vec3::new(0.3, 9.0, 0.3), false);
    let red = glow(&mut mats, Vec3::new(1.0, 0.1, 0.1), 7.0);
    cx.block(&red, -20.0, 9.0, 0.0, Vec3::new(0.4, 0.4, 0.4), false);
    // A neon candle sign on the tank: the Brigad's colours.
    let pink = glow(&mut mats, Vec3::new(1.0, 0.2, 0.6), 7.0);
    let cyan = glow(&mut mats, Vec3::new(0.2, 0.9, 1.0), 7.0);
    cx.block(&pink, -12.0, 3.4, -10.0, Vec3::new(3.0, 0.2, 0.1), false);
    cx.block(&cyan, -12.0, 3.0, -10.0, Vec3::new(2.4, 0.15, 0.1), false);
    for (c, x) in [(Vec3::new(1.0, 0.2, 0.6), -13.0_f32), (Vec3::new(0.2, 0.9, 1.0), -11.0)] {
        cx.commands.spawn((
            PointLight {
                color: Color::srgb(c.x, c.y, c.z),
                intensity: 1_200_000.0,
                range: 11.0,
                ..default()
            },
            Transform::from_xyz(x, 3.4, -8.4),
            LevelEntity,
        ));
    }
    // --- A ring of candles round the spot where he waits.
    for i in 0..10 {
        let a = i as f32 / 10.0 * std::f32::consts::TAU;
        candle(&mut cx, &mut mats, NWIT_START + Vec3::new(a.cos() * 2.6, 0.0, a.sin() * 2.6), Vec3::new(1.0, 0.7, 0.3), &mut rng);
    }
    // --- A stain on the roof where he has been waiting.
    let stain = matte(&mut mats, 0.05);
    let sm = cx.meshes.add(Cuboid::new(1.0, 0.012, 0.7));
    cx.commands.spawn((Mesh3d(sm), MeshMaterial3d(stain), Transform::from_xyz(-7.0, 0.012, 0.6), LevelEntity));

    // --- The city beyond: dark towers with a few lit windows.
    let window_warm = glow(&mut mats, Vec3::new(1.0, 0.75, 0.35), 2.5);
    let window_cool = glow(&mut mats, Vec3::new(0.5, 0.75, 1.0), 2.5);
    for i in 0..26 {
        let a = i as f32 / 26.0 * std::f32::consts::TAU + rng.next() * 0.2;
        let r = 38.0 + rng.next() * 30.0;
        let (w, d) = (7.0 + rng.next() * 7.0, 7.0 + rng.next() * 7.0);
        let top = -10.0 + rng.next() * 22.0;
        let (x, z) = (a.cos() * r, a.sin() * r * 0.8);
        cx.block(&city, x, -60.0, z, Vec3::new(w, top + 60.0, d), false);
        // Windows on the face toward the roof.
        let to_roof = Vec3::new(-x, 0.0, -z).normalize();
        for _ in 0..7 {
            let wm = if rng.next() > 0.5 { window_warm.clone() } else { window_cool.clone() };
            let m = cx.meshes.add(Cuboid::new(0.9, 1.1, 0.1));
            let off = Vec3::new(rng.signed() * w * 0.4, top - 1.5 - rng.next() * 12.0, rng.signed() * d * 0.4);
            cx.commands.spawn((
                Mesh3d(m),
                MeshMaterial3d(wm),
                Transform::from_xyz(x + to_roof.x * (w * 0.5 + 0.1) + off.x * to_roof.z.abs(), off.y, z + to_roof.z * (d * 0.5 + 0.1) + off.z * to_roof.x.abs()),
                LevelEntity,
            ));
        }
    }

    // --- The extraction helicopter, parked far off until it is called.
    let hb = cx.meshes.add(Cuboid::new(3.2, 1.6, 1.8));
    let tail = cx.meshes.add(Cuboid::new(4.2, 0.35, 0.35));
    let skid = cx.meshes.add(Cuboid::new(3.6, 0.1, 0.12));
    let rotor = cx.meshes.add(Cuboid::new(8.0, 0.05, 0.3));
    let trotor = cx.meshes.add(Cuboid::new(0.12, 1.4, 0.1));
    let heli_body = matte(&mut mats, 0.14);
    let lamp = cx.commands.spawn((
        PointLight {
            color: Color::srgb(1.0, 1.0, 0.95),
            intensity: 6_000_000.0,
            range: 26.0,
            ..default()
        },
        Transform::from_xyz(0.0, -1.0, 0.0),
    )).id();
    let nav_r = glow(&mut mats, Vec3::new(1.0, 0.1, 0.1), 8.0);
    let nav_g = glow(&mut mats, Vec3::new(0.1, 1.0, 0.2), 8.0);
    let nav_m = cx.meshes.add(Sphere::new(0.12));
    let heli = cx
        .commands
        .spawn((
            Heli,
            ink(hb, heli_body.clone(), cx.ink, Transform::from_translation(HELI_FAR), Vec3::splat(1.05)),
            LevelEntity,
        ))
        .id();
    cx.commands.entity(heli).add_child(lamp);
    cx.commands.entity(heli).with_child(ink(tail, heli_body.clone(), cx.ink, Transform::from_xyz(-3.0, 0.3, 0.0), grow_box(Vec3::new(4.2, 0.35, 0.35), 0.03)));
    cx.commands.entity(heli).with_child(ink(skid.clone(), dark.clone(), cx.ink, Transform::from_xyz(0.0, -1.0, 0.7), Vec3::splat(1.1)));
    cx.commands.entity(heli).with_child(ink(skid, dark.clone(), cx.ink, Transform::from_xyz(0.0, -1.0, -0.7), Vec3::splat(1.1)));
    cx.commands.entity(heli).with_child((Spin(26.0), Mesh3d(rotor), MeshMaterial3d(dark.clone()), Transform::from_xyz(0.0, 1.0, 0.0)));
    cx.commands.entity(heli).with_child((Spin(30.0), Mesh3d(trotor), MeshMaterial3d(dark.clone()), Transform::from_xyz(-5.1, 0.4, 0.2).with_rotation(Quat::from_rotation_z(0.0))));
    cx.commands.entity(heli).with_child((Mesh3d(nav_m.clone()), MeshMaterial3d(nav_r), Transform::from_xyz(0.0, 0.2, 1.0)));
    cx.commands.entity(heli).with_child((Mesh3d(nav_m), MeshMaterial3d(nav_g), Transform::from_xyz(0.0, 0.2, -1.0)));

    cx.commands.spawn((
        Sun,
        DirectionalLight {
            illuminance: 1200.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.9, 0.5, 0.0)),
        LevelEntity,
    ));
    let boxes = std::mem::take(&mut cx.boxes);
    commands.insert_resource(Boxes(boxes));
    commands.insert_resource(crate::level3::Grab(0));
    commands.insert_resource(Level7 {
        phase: Phase::Prone,
        timer: 0.0,
        stabs: 0,
        from: Vec3::ZERO,
        to: Vec3::ZERO,
        lunge_dmg: 35.0,
        neck: false,
        strafe: 0.0,
        strafe_dir: 1.0,
        attack_cd: 1.5,
        taunt_cd: 6.0,
        cuff: 0.0,
        gun_pos: Vec3::ZERO,
        door_open: false,
        last_caught: 0,
        force_reset: false,
        ending: None,
        ending_timer: 0.0,
        opening: "",
        heli_sound: false,
    });

    // --- Ro, and Granmèt Nwit.
    let ro = spawn_humanoid(&mut commands, &mut meshes, &mut mats, &ink_res, ARRIVAL, std::f32::consts::FRAC_PI_2, 0.62, 0.22, 0.34);
    commands.entity(ro).insert((Player, crate::character::Skin(crate::character::Cast::Hero)));
    crate::weapons::attach_player_weapons(&mut commands, &mut meshes, &mut mats, &ink_res, ro);

    let nwit = spawn_humanoid(&mut commands, &mut meshes, &mut mats, &ink_res, NWIT_START, -std::f32::consts::FRAC_PI_2, 0.08, 0.06, 0.28);
    commands.entity(nwit).insert(crate::character::Skin(crate::character::Cast::Nwit));
    commands
        .entity(nwit)
        .insert((
            Nwit7,
            Guard {
                hp: NWIT_HP,
                max_hp: NWIT_HP,
                mortal: true,
                state: AiState::Patrol,
                range: 12.0,
                fall_yaw: -std::f32::consts::FRAC_PI_2,
                ..Guard::base("Granmèt Nwit", 52, GuardKind::Phantom)
            },
        ));
}

// -------------------------------------------------------------- the duel

fn reset_nwit(
    l7: &mut Level7,
    g: &mut Guard,
    tf: &mut Transform,
    mv: &mut Mover,
    blade: &mut Query<&mut Visibility, (With<NwitBlade>, Without<Nwit7>, Without<FloorKnife>)>,
    knife: &mut Query<&mut Visibility, (With<FloorKnife>, Without<Nwit7>, Without<NwitBlade>)>,
) {
    l7.phase = Phase::Prone;
    l7.timer = 0.0;
    l7.heli_sound = false;
    l7.stabs = 0;
    l7.cuff = 0.0;
    l7.attack_cd = 1.5;
    g.hp = NWIT_HP;
    g.max_hp = NWIT_HP;
    g.pain = 0.0;
    g.status = LostStatus::Standing;
    g.fall = 0.0;
    tf.translation = NWIT_START;
    tf.rotation = Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2);
    mv.speed = 0.0;
    for mut v in blade.iter_mut() {
        *v = Visibility::Hidden;
    }
    for mut v in knife.iter_mut() {
        *v = Visibility::Inherited;
    }
}

#[allow(clippy::too_many_arguments)]
pub fn nwit7(
    time: Res<Time>,
    boxes: Res<Boxes>,
    mut l7: ResMut<Level7>,
    mut commands: Commands,
    fx: Res<Fx>,
    mut rng: ResMut<Rng>,
    mut gear: Gear,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    ink_res: Res<Ink>,
    player: Single<(&Transform, &Mover), (With<Player>, Without<Nwit7>)>,
    nwit: Single<(&mut Guard, &mut Transform, &mut Mover), (With<Nwit7>, Without<Player>, Without<Heli>)>,
    mut blade: Query<&mut Visibility, (With<NwitBlade>, Without<Nwit7>, Without<FloorKnife>)>,
    mut knife: Query<&mut Visibility, (With<FloorKnife>, Without<Nwit7>, Without<NwitBlade>)>,
    mut heli: Query<&mut Transform, (With<Heli>, Without<Nwit7>, Without<Player>)>,
    guns: Query<Entity, With<GunPickup>>,
) {
    let dt = time.delta_secs().min(0.1);
    let (pt, pm) = player.into_inner();
    let (mut g, mut tf, mut mv) = nwit.into_inner();
    let ppos = pt.translation;
    let dist = flat_dist(ppos, tf.translation);

    // Ro went down, or a forced reset: start the opening again.
    if gear.stats.caught != l7.last_caught || l7.force_reset {
        l7.last_caught = gear.stats.caught;
        l7.force_reset = false;
        reset_nwit(&mut l7, &mut g, &mut tf, &mut mv, &mut blade, &mut knife);
        gear.loadout.disarmed = false;
        for e in &guns {
            commands.entity(e).despawn();
        }
        gear.msg.0 = "Again. He is on all fours in the candle ring, muttering, as if nothing happened.".into();
        return;
    }
    if l7.ending.is_some() {
        return;
    }
    // Killed: a helicopter still comes for him.
    if l7.phase != Phase::Dead && g.hp <= 0.0 {
        l7.phase = Phase::Dead;
        l7.timer = 0.0;
        gear.msg.0 = "He goes down and does not rise. The wind is the only sound. You did what he did. Far off, rotors: someone is coming to take the body.".into();
    }
    if l7.phase == Phase::Dead {
        l7.timer += dt;
        let t = l7.timer;
        let done = fly_heli(
            t,
            &mut l7.heli_sound,
            &mut tf,
            &mut heli,
            &fx,
            &mut commands,
            &mut gear.msg.0,
            "The body is hoisted off the roof on a line, and the rotors fade into the dark.",
        );
        if done {
            l7.ending = Some(Ending::Blood);
            l7.ending_timer = 0.3;
        }
        return;
    }

    // A fifth of his health left: his fists drop and he kneels.
    if matches!(l7.phase, Phase::Duel | Phase::Recover | Phase::LungeTell | Phase::Lunge | Phase::Counter | Phase::Disarm | Phase::FakeDeath)
        && g.hp <= g.max_hp * 0.2
    {
        l7.phase = Phase::Kneel;
        for mut v in blade.iter_mut() {
            *v = Visibility::Hidden;
        }
        l7.door_open = true;
        // Money, and the way down unlocks (the level was locked).
        let m = glow(&mut mats, Vec3::new(1.0, 0.8, 0.2), 4.0);
        let mm = meshes.add(Cuboid::new(0.7, 0.5, 0.5));
        let at = tf.translation + Vec3::new(-1.8, 0.0, 0.6); // behind him, away from the stairs side
        commands.spawn((
            Money,
            ink(mm, m, &ink_res, Transform::from_xyz(at.x, 0.25, at.z), Vec3::splat(1.1)),
            LevelEntity,
        ));
        commands.insert_resource(MoneyAt(at));
        gear.msg.0 = "His fists open and he kneels, shaking: \"Pa touye m. Mwen gen lajan. Anpil lajan.\" (Don't kill me. I have money. A lot.) The stair door unlocks. Choose.".into();
        return;
    }

    match l7.phase {
        Phase::Prone => {
            mv.crouch = 1.0;
            mv.speed = 0.0;
            let to = ppos - tf.translation;
            face(&mut tf, to, 4.0, dt);
            let e = gear.keys.just_pressed(KeyCode::KeyE);
            if g.pain > 0.0 || pm.noise >= 35.0 || gear.loadout.disarmed {
                // --- The shot: he disarms Ro and throws the gun across the roof.
                l7.opening = "shot";
                l7.phase = Phase::Disarm;
                l7.timer = 2.2;
                let dir = (tf.translation - ppos).with_y(0.0).normalize_or_zero();
                tf.translation = ppos + dir * 1.1;
                gear.health.hp -= 25.0;
                gear.health.hurt = 1.0;
                gear.shake.0 = 0.8;
                gear.loadout.disarmed = true;
                gear.loadout.current = crate::weapons::WeaponKind::Knife;
                // The gun skids to the farthest corner from Ro.
                let spots = [Vec3::new(-18.0, 0.0, 12.0), Vec3::new(-18.0, 0.0, -12.0), Vec3::new(10.0, 0.0, -13.0), Vec3::new(10.0, 0.0, 13.0)];
                let far = *spots.iter().max_by(|a, b| flat_dist(**a, ppos).total_cmp(&flat_dist(**b, ppos))).unwrap();
                l7.gun_pos = far;
                let gm = matte(&mut mats, 0.12);
                let gb = meshes.add(Cuboid::new(0.7, 0.12, 0.2));
                commands.spawn((
                    GunPickup,
                    ink(gb, gm, &ink_res, Transform::from_xyz(far.x, 0.08, far.z), Vec3::splat(1.2)),
                    LevelEntity,
                ));
                gear.bleed.0 = 8.0;
                g.hp = g.max_hp; // he drinks his own bottle
                for mut v in blade.iter_mut() {
                    *v = Visibility::Inherited;
                }
                for mut v in knife.iter_mut() {
                    *v = Visibility::Hidden;
                }
                gear.msg.0 = "He is on you before you blink. The gun is gone, spinning across the roof. A blow to the ribs: you are bleeding soul. He drinks from his own bottle and smiles. Fight with your fists, or get your gun back.".into();
            } else if dist < 1.5 {
                // --- The rush: he was waiting for it.
                l7.opening = "rush";
                l7.phase = Phase::Counter;
                l7.timer = 0.15;
                l7.stabs = 0;
                for mut v in blade.iter_mut() {
                    *v = Visibility::Inherited;
                }
                for mut v in knife.iter_mut() {
                    *v = Visibility::Hidden;
                }
                gear.msg.0 = "You rush him. His fist is already coming.".into();
            } else if e && (1.6..=4.6).contains(&dist) {
                if gear.poison.bottles == 0 {
                    gear.msg.0 = "You have no bottle to give him.".into();
                } else {
                    // --- The bottle: a fake death, then the lunge.
                    l7.opening = "bottle";
                    if gear.poison.bottles < 900 {
                        gear.poison.bottles -= 1;
                    }
                    g.max_hp = NWIT_HP * 1.3; // "extra longevity"
                    g.hp = g.max_hp;
                    l7.phase = Phase::FakeDeath;
                    l7.timer = 2.6;
                    for mut v in knife.iter_mut() {
                        *v = Visibility::Hidden;
                    }
                    gear.msg.0 = "He drinks, convulses, claws at his throat: \"Pwazon! Ou touye m!\" He is dying. Keep your distance, and watch his hands.".into();
                }
            }
        }
        Phase::FakeDeath => {
            l7.timer -= dt;
            mv.crouch = 1.0;
            // Writhing.
            let t = time.elapsed_secs();
            tf.rotation = Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2) * Quat::from_rotation_z((t * 14.0).sin() * 0.25);
            if g.pain > 0.0 || l7.timer <= 0.0 {
                tf.rotation = Quat::from_rotation_y(tf.rotation.to_euler(EulerRot::YXZ).0);
                l7.phase = Phase::LungeTell;
                l7.timer = 0.8;
                l7.lunge_dmg = 70.0;
                l7.neck = true;
                for mut v in blade.iter_mut() {
                    *v = Visibility::Inherited;
                }
                gear.msg.0 = "He is on his feet and he is not dying. NOW: keep moving: sprint, jump, anything!".into();
            }
        }
        Phase::LungeTell => {
            mv.crouch = 0.0;
            mv.speed = 0.0;
            let to = ppos - tf.translation;
            face(&mut tf, to, 14.0, dt);
            l7.timer -= dt;
            if l7.timer <= 0.0 {
                l7.from = tf.translation;
                l7.to = Vec3::new(ppos.x, 0.0, ppos.z);
                l7.phase = Phase::Lunge;
                l7.timer = 0.45;
            }
        }
        Phase::Lunge => {
            let t = 1.0 - (l7.timer / 0.45).clamp(0.0, 1.0);
            tf.translation = l7.from.lerp(l7.to, t);
            mv.speed = 9.0;
            l7.timer -= dt;
            if l7.timer <= 0.0 {
                mv.speed = 0.0;
                mv.slash = 1.0;
                mv.swing = crate::character::Swing::Punch;
                if flat_dist(ppos, tf.translation) < 1.3 {
                    gear.health.hp -= l7.lunge_dmg;
                    gear.health.hurt = 1.0;
                    gear.shake.0 = 1.0;
                    if l7.neck {
                        gear.bleed.0 = 10.0;
                        gear.msg.0 = "A fist like a hammer, to the throat. Everything goes thin. Drink (H) if you can.".into();
                    } else {
                        gear.msg.0 = "He clips you with a fist as he passes.".into();
                    }
                } else {
                    gear.msg.0 = "He misses and crashes onto the concrete. He is exposed: hit him!".into();
                    dust_puff(&mut commands, &fx, tf.translation + Vec3::Y * 0.3, Vec3::Y, 0.4, 0.8);
                }
                l7.neck = false;
                l7.phase = Phase::Recover;
                l7.timer = 1.6;
            }
        }
        Phase::Recover => {
            mv.crouch = 0.6;
            l7.timer -= dt;
            if l7.timer <= 0.0 {
                mv.crouch = 0.0;
                let d = tf.translation - ppos;
                l7.strafe = d.z.atan2(d.x);
                l7.strafe_dir = if rng.next() > 0.5 { 1.0 } else { -1.0 };
                l7.attack_cd = 1.3;
                l7.phase = Phase::Duel;
            }
        }
        Phase::Counter => {
            mv.crouch = 0.0;
            let to = ppos - tf.translation;
            face(&mut tf, to, 14.0, dt);
            l7.timer -= dt;
            if l7.timer <= 0.0 {
                if dist < 2.4 {
                    mv.slash = 1.0;
                    mv.swing = crate::character::Swing::Punch;
                    gear.health.hp -= 25.0; // a quarter of Ro's health
                    gear.health.hurt = 1.0;
                    gear.shake.0 = 0.8;
                    l7.stabs += 1;
                    gear.msg.0 = format!("Blow {}. He punches, steps back, punches again. Get away from him. Drink (H).", l7.stabs);
                    l7.timer = 0.6;
                    if l7.stabs >= 3 {
                        l7.phase = Phase::Recover;
                        l7.timer = 0.8;
                    }
                } else {
                    gear.msg.0 = "You back away. He rises slowly, fists ready.".into();
                    l7.phase = Phase::Recover;
                    l7.timer = 0.6;
                }
            }
        }
        Phase::Disarm => {
            mv.crouch = 0.0;
            let to = ppos - tf.translation;
            face(&mut tf, to, 10.0, dt);
            l7.timer -= dt;
            if l7.timer <= 0.0 {
                l7.phase = Phase::Recover;
                l7.timer = 0.6;
            }
        }
        Phase::Duel => {
            mv.crouch = 0.0;
            // Circle Ro at a few metres, then telegraph and lunge.
            l7.strafe += 0.9 * dt * l7.strafe_dir;
            let target = ppos + Vec3::new(l7.strafe.cos(), 0.0, l7.strafe.sin()) * 4.6;
            let target = Vec3::new(target.x.clamp(-22.0, 22.0), 0.0, target.z.clamp(-14.0, 14.0));
            walk_to(&mut tf, &mut mv, target, 4.2, dt, &boxes.0);
            let to = ppos - tf.translation;
            face(&mut tf, to, 12.0, dt);
            tf.translation = resolve(tf.translation, 0.4, &boxes.0);
            // Angrier as he is hurt.
            let frac = (g.hp / g.max_hp).clamp(0.0, 1.0);
            l7.attack_cd -= dt;
            if l7.attack_cd <= 0.0 {
                l7.phase = Phase::LungeTell;
                l7.timer = 0.55;
                l7.lunge_dmg = 35.0;
                l7.neck = false;
                l7.attack_cd = (1.6 + rng.next() * 1.2) * (0.6 + 0.4 * frac);
            }
            l7.taunt_cd -= dt;
            if l7.taunt_cd <= 0.0 {
                l7.taunt_cd = 7.0;
                gear.msg.0 = "\"Plip, plip! Mwen se chandèl, van pa tenyen m!\"".into();
            }
        }
        Phase::Kneel => {
            mv.crouch = 1.0;
            mv.speed = 0.0;
            let to = ppos - tf.translation;
            face(&mut tf, to, 4.0, dt);
        }
        Phase::Cuffed => {
            mv.crouch = 0.0;
            mv.speed = 0.0;
            l7.timer += dt;
            let t = l7.timer;
            let done = fly_heli(
                t,
                &mut l7.heli_sound,
                &mut tf,
                &mut heli,
                &fx,
                &mut commands,
                &mut gear.msg.0,
                "He is lifted off the roof, kicking, shouting down at you: \"Mwen se Granmèt Nwit! Mwen se chandèl, van pa tenyen m!\"",
            );
            if done {
                l7.ending = Some(Ending::Justice);
                l7.ending_timer = 0.3;
            }
        }
        Phase::Dead => {}
    }
}

/// The helicopter arrives, hoists him (alive or not) and flies off. True when it is gone.
#[allow(clippy::too_many_arguments)]
fn fly_heli(
    t: f32,
    started: &mut bool,
    tf: &mut Transform,
    heli: &mut Query<&mut Transform, (With<Heli>, Without<Nwit7>, Without<Player>)>,
    fx: &Fx,
    commands: &mut Commands,
    msg: &mut String,
    lifted: &str,
) -> bool {
    if !*started {
        *started = true;
        fx.heli_loop(commands, 0.7);
    }
    if let Ok(mut ht) = heli.single_mut() {
        let hover = Vec3::new(tf.translation.x, 8.0, tf.translation.z);
        if t < 3.0 {
            let k = (t / 3.0).clamp(0.0, 1.0);
            let k = k * k * (3.0 - 2.0 * k);
            ht.translation = HELI_FAR.lerp(hover, k);
        } else if t < 5.5 {
            ht.translation = hover;
            let k = ((t - 3.0) / 2.5).clamp(0.0, 1.0);
            tf.translation.y = k * 6.0; // hoisted on a line
        } else {
            let k = ((t - 5.5) / 3.0).clamp(0.0, 1.0);
            ht.translation = hover.lerp(Vec3::new(45.0, 28.0, -25.0), k * k);
            tf.translation.y = ht.translation.y - 2.0;
        }
        ht.rotation = Quat::from_rotation_y(-0.2);
    }
    if t > 3.2 && t < 3.4 {
        *msg = lifted.to_string();
    }
    t >= 8.5
}

/// Money on the roof: where it is, so the choice can find it.
#[derive(Resource)]
pub struct MoneyAt(pub Vec3);

#[allow(clippy::too_many_arguments)]
pub fn interact7(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut prompt: ResMut<Prompt>,
    mut msg: ResMut<Message>,
    mut l7: ResMut<Level7>,
    mut loadout: ResMut<Loadout>,
    money: Option<Res<MoneyAt>>,
    mut boxes: ResMut<Boxes>,
    mut commands: Commands,
    player: Single<&Transform, With<Player>>,
    nwit: Single<&Transform, (With<Nwit7>, Without<Player>)>,
    guns: Query<(Entity, &Transform), With<GunPickup>>,
) {
    let dt = time.delta_secs().min(0.1);
    prompt.0.clear();
    let pp = player.translation;
    let e = keys.just_pressed(KeyCode::KeyE);
    let holding = keys.pressed(KeyCode::KeyE);

    // Get the gun back.
    for (ent, gt) in &guns {
        if flat_dist(pp, gt.translation) < 1.8 {
            prompt.0 = "E: pick up your gun".into();
            if e {
                loadout.disarmed = false;
                loadout.current = crate::weapons::WeaponKind::Pistol;
                commands.entity(ent).despawn();
                msg.0 = "Your gun. He threw it well, but not well enough.".into();
            }
        }
    }

    // The stair door stays shut until he kneels: the roof is the whole world.
    if l7.door_open {
        boxes.0.retain(|b| (b.c - Vec3::new(DOOR.x, 0.0, DOOR.z)).length() > 1.5);
    }

    let nt = nwit.translation;
    let d = flat_dist(pp, nt);
    match l7.phase {
        Phase::Prone => {
            if (1.6..=4.6).contains(&d) {
                prompt.0 = "E: hand him a bottle (he is asking for one)".into();
            }
        }
        Phase::Kneel => {
            let gold = money.as_ref().map(|m| m.0);
            if gold.is_some_and(|m| flat_dist(pp, m) < 1.5) {
                prompt.0 = "E: take the money, and his place".into();
                if e {
                    l7.ending = Some(Ending::Throne);
                    l7.ending_timer = 1.5;
                    msg.0 = "You pick it up. It is heavier than it looks.".into();
                }
            } else if d < 1.9 {
                prompt.0 = format!("Hold E: handcuff him ({:.0}%)   |   shoot him   |   the gold   |   the stairs, east", l7.cuff * 100.0);
                if holding {
                    l7.cuff += dt / 1.2;
                    if l7.cuff >= 1.0 {
                        l7.phase = Phase::Cuffed;
                        l7.timer = 0.0;
                        msg.0 = "Click. You call it in. A helicopter answers out of the dark.".into();
                    }
                } else {
                    l7.cuff = (l7.cuff - dt).max(0.0);
                }
            } else {
                prompt.0 = "Choose: handcuff him, shoot him, take the gold, or walk out the stairs (east).".into();
            }
        }
        _ => {}
    }
}

/// Spin rotors; run the ending clock; walking out by the stairs.
pub fn goal7(
    time: Res<Time>,
    mut l7: ResMut<Level7>,
    mut ending: ResMut<EndingKind>,
    mut next: ResMut<NextState<AppState>>,
    mut spins: Query<(&Spin, &mut Transform), Without<Player>>,
    player: Single<&Transform, (With<Player>, Without<Spin>)>,
) {
    let dt = time.delta_secs();
    for (s, mut tf) in &mut spins {
        tf.rotate_local_y(s.0 * dt);
    }
    let p = player.translation;
    if l7.ending.is_none() && l7.door_open && l7.phase == Phase::Kneel && p.x > 23.4 && p.z.abs() < 2.0 {
        l7.ending = Some(Ending::Road);
        l7.ending_timer = 0.3;
    }
    if let Some(kind) = l7.ending {
        l7.ending_timer -= dt;
        if l7.ending_timer <= 0.0 {
            ending.0 = Some(kind);
            next.set(AppState::Ending);
        }
    }
}
