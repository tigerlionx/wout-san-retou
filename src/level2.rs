//! Level 2: Inside the House. A dark maze of rooms lit by candles. Granmèt
//! Nwit, a fraud who plays at spirit-work, ambushes Ro through secret
//! passages. Shooting or cutting him only makes him vanish ("Woy!"). Twice he
//! slashes; the third time he stabs Ro with a poisoned blade: two minutes to
//! reach the altar, where three vials wait: one cures, one kills, one drugs.
//! Notes in the house say which smell is the cure. Drinking it stabilizes Ro,
//! saves the game (the first save point) and unlocks the way out.

use crate::ai::{walk_to, Guard, GuardKind};
use crate::character::{segment_blocked, spawn_humanoid, Boxes, Mover};
use crate::level::Ctx;
use crate::style::{glow, grow_box, ink, matte, Ink};
use crate::weapons::{dust_puff, Fx, Health, Rng, Shake};
use crate::*;

const POISON_SECONDS: f32 = 120.0;
/// How long the big stabbed card stays on the screen.
const POISON_ALERT_SECONDS: f32 = 9.0;
const CELL: f32 = 10.0;
const WALL_H: f32 = 2.6;

// ------------------------------------------------------------------ state

#[derive(Resource, Default)]
pub struct Poison {
    pub active: bool,
    pub remaining: f32,
    /// Seconds spent drugged (0 = clear). Cleared by the cure.
    pub drugged: f32,
    pub bottles: u32,
    /// Seconds left of the big "you have been stabbed" card (shown by `hud::poison_hud`).
    pub alert: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VialRole {
    Cure,
    Death,
    Drug,
}

impl VialRole {
    fn smell(self) -> &'static str {
        match self {
            VialRole::Cure => "herbs and lemongrass",
            VialRole::Death => "something sharp, like bitter almonds",
            VialRole::Drug => "cane syrup and smoke",
        }
    }
}

#[derive(Resource)]
pub struct Level2 {
    pub roles: [VialRole; 3],
    pub drunk: [bool; 3],
    pub door_open: bool,
    pub door_mat: Handle<StandardMaterial>,
    pub passages: Vec<Vec3>,
}

#[derive(Component)]
pub struct Vial {
    pub slot: usize,
    pub pos: Vec3,
}

#[derive(Component)]
pub struct Note {
    pub pos: Vec3,
    pub text: &'static str,
}

#[derive(Component)]
pub struct Flicker {
    pub base: f32,
    pub phase: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NwitState {
    /// Hidden in the walls, waiting.
    Lurk,
    /// His voice, a flicker of the lights, a panel about to open.
    Telegraph,
    /// Out of the wall and running at Ro with the knife.
    Charge,
}

#[derive(Component)]
pub struct Nwit {
    pub state: NwitState,
    pub timer: f32,
    /// Times he has reached Ro. Two slashes, then the poisoned stab.
    pub attempts: u32,
    pub enabled: bool,
    pub at: Vec3,
}

// ------------------------------------------------------------------ layout

/// Doorways between neighbouring cells (row, col) of the 4 by 4 grid.
const OPEN: &[((i32, i32), (i32, i32))] = &[
    ((3, 2), (3, 3)),
    ((3, 2), (2, 2)),
    ((2, 2), (2, 1)),
    ((2, 2), (1, 2)),
    ((2, 1), (2, 0)),
    ((2, 1), (3, 1)),
    ((3, 1), (3, 0)),
    ((1, 2), (1, 3)),
    ((1, 2), (0, 2)),
    ((1, 2), (1, 1)),
    ((1, 1), (0, 1)),
    ((0, 2), (0, 1)),
    ((0, 2), (0, 3)),
    ((1, 1), (1, 0)),
    ((2, 0), (1, 0)),
    ((1, 3), (2, 3)),
    ((0, 0), (1, 0)),
];

fn is_open(a: (i32, i32), b: (i32, i32)) -> bool {
    OPEN.iter().any(|&(x, y)| (x == a && y == b) || (x == b && y == a))
}

fn cell_center(r: i32, c: i32) -> Vec3 {
    Vec3::new(-15.0 + CELL * c as f32, 0.0, -15.0 + CELL * r as f32)
}

pub const START: Vec3 = Vec3::new(5.0, 0.0, 15.0);
pub const ALTAR_SPAWN: Vec3 = Vec3::new(-5.0, 0.0, -12.5);
pub const VIAL_POS_PUB: [Vec3; 3] = VIAL_POS;
const VIAL_POS: [Vec3; 3] = [
    Vec3::new(-6.4, 1.0, -16.6),
    Vec3::new(-5.0, 1.0, -16.6),
    Vec3::new(-3.6, 1.0, -16.6),
];

// ------------------------------------------------------------------- setup

#[allow(clippy::too_many_arguments)]
pub fn setup_level2(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    ink_res: Res<Ink>,
    mut rng: ResMut<Rng>,
    mut checkpoint: ResMut<Checkpoint>,
    mut msg: ResMut<Message>,
    mut health: ResMut<Health>,
    mut poison: ResMut<Poison>,
    mut rig: ResMut<CamRig>,
    mut objectives: ResMut<Objectives>,
    mut stats: ResMut<Stats>,
) {
    let ink_res = ink_res.clone();
    checkpoint.0 = START;
    msg.0 = "You are inside. The house is dark. Candles, and something that watches.".into();
    health.hp = 100.0;
    health.hurt = 0.0;
    *poison = Poison::default();
    *objectives = Objectives::default();
    *stats = Stats::default();
    rig.yaw = 0.0;
    rig.first = false;

    let floor = matte(&mut mats, 0.09);
    let wall = matte(&mut mats, 0.30);
    let trim = matte(&mut mats, 0.55);
    let table = matte(&mut mats, 0.20);
    let bone = matte(&mut mats, 0.80);
    let altar_m = matte(&mut mats, 0.14);
    let rug = matte(&mut mats, 0.16);

    let mut cx = Ctx {
        commands: &mut commands,
        meshes: &mut meshes,
        ink: &ink_res,
        boxes: Vec::new(),
    };

    // Floor.
    let plane = cx.meshes.add(Plane3d::default().mesh().size(44.0, 44.0));
    cx.commands.spawn((Mesh3d(plane), MeshMaterial3d(floor), LevelEntity));

    // --- Walls: outer ring, then every edge of the grid with a doorway where open.
    let gap = 2.4;
    let seg = (CELL - gap) / 2.0;
    for k in 0..=4 {
        let line = -20.0 + CELL * k as f32;
        // Vertical lines (constant x), one piece per row.
        for r in 0..4 {
            let zc = -15.0 + CELL * r as f32;
            let internal = (1..=3).contains(&k);
            let open = internal && is_open((r, k - 1), (r, k));
            if open {
                for s in [-1.0_f32, 1.0] {
                    cx.block(&wall, line, 0.0, zc + s * (gap / 2.0 + seg / 2.0), Vec3::new(0.5, WALL_H, seg), true);
                }
            } else {
                cx.block(&wall, line, 0.0, zc, Vec3::new(0.5, WALL_H, CELL), true);
            }
        }
        // Horizontal lines (constant z), one piece per column.
        for c in 0..4 {
            let xc = -15.0 + CELL * c as f32;
            let internal = (1..=3).contains(&k);
            if internal && is_open((k - 1, c), (k, c)) {
                for s in [-1.0_f32, 1.0] {
                    cx.block(&wall, xc + s * (gap / 2.0 + seg / 2.0), 0.0, line, Vec3::new(seg, WALL_H, 0.5), true);
                }
            } else if k == 0 && c == 1 {
                // The way out: a doorway in the north wall of the altar room.
                cx.block(&wall, -8.0, 0.0, line, Vec3::new(4.0, WALL_H, 0.5), true);
                cx.block(&wall, -2.0, 0.0, line, Vec3::new(4.0, WALL_H, 0.5), true);
            } else {
                cx.block(&wall, xc, 0.0, line, Vec3::new(CELL, WALL_H, 0.5), true);
            }
        }
    }

    // --- Candles and furniture in every room.
    let candle_cols = [
        Vec3::new(1.0, 0.75, 0.35),
        Vec3::new(1.0, 0.75, 0.35),
        Vec3::new(1.0, 0.2, 0.15),
        Vec3::new(0.3, 1.0, 0.5),
        Vec3::new(0.35, 0.5, 1.0),
    ];
    let mut n = 0;
    for r in 0..4 {
        for c in 0..4 {
            let ctr = cell_center(r, c);
            for (i, off) in [Vec3::new(-3.6, 0.0, -3.6), Vec3::new(3.6, 0.0, 3.6)].into_iter().enumerate() {
                let col = candle_cols[(n + i) % candle_cols.len()];
                candle(&mut cx, &mut mats, ctr + off, col, &mut rng);
            }
            n += 1;
            // A table to take cover behind, in most rooms.
            if (r + c) % 2 == 0 && !(r == 0 && c == 1) {
                cx.block(&table, ctr.x + 1.5, 0.0, ctr.z - 1.2, Vec3::new(2.0, 0.9, 0.9), true);
            }
        }
    }

    // --- The bone room (east, row 1) and the skull hall: skulls on shelves, bones on the floor.
    for ctr in [cell_center(1, 3), cell_center(1, 1)] {
        cx.block(&table, ctr.x, 0.0, ctr.z - 4.4, Vec3::new(7.0, 1.2, 0.6), false);
        for i in 0..6 {
            skull(&mut cx, &mut mats, &bone, Vec3::new(ctr.x - 3.0 + i as f32 * 1.2, 1.2, ctr.z - 4.4));
        }
        for _ in 0..14 {
            let a = rng.next() * 6.28;
            let m = cx.meshes.add(Cuboid::new(0.55, 0.06, 0.07));
            cx.commands.spawn((
                Mesh3d(m),
                MeshMaterial3d(bone.clone()),
                Transform::from_xyz(ctr.x + rng.signed() * 3.5, 0.04, ctr.z + rng.signed() * 3.5)
                    .with_rotation(Quat::from_rotation_y(a)),
                LevelEntity,
            ));
        }
    }
    // A rug in the foyer.
    let rug_mesh = cx.meshes.add(Cuboid::new(5.0, 0.02, 3.0));
    cx.commands.spawn((
        Mesh3d(rug_mesh),
        MeshMaterial3d(rug),
        Transform::from_xyz(5.0, 0.012, 14.0),
        LevelEntity,
    ));

    // --- Notes (the clue to the cure).
    for (pos, text) in [
        (
            cell_center(3, 3) + Vec3::new(-2.0, 0.0, 0.0),
            "A note: \"Granmèt Nwit's own medicine smells of herbs and lemongrass. Never drink what smells sharp. Never drink what smells sweet.\"",
        ),
        (
            cell_center(2, 0) + Vec3::new(2.0, 0.0, 2.0),
            "A note: \"Ti remèd la gen sant fèy ak sitwon. Li pa kapab sipòte yo frape l: li disparèt.\" (The cure smells of leaf and lemon. He cannot bear being hit: he vanishes.)",
        ),
    ] {
        let paper = glow(&mut mats, Vec3::new(0.95, 0.9, 0.6), 1.5);
        let m = cx.meshes.add(Cuboid::new(0.5, 0.04, 0.38));
        cx.commands.spawn((
            Note { pos, text },
            Mesh3d(m),
            MeshMaterial3d(paper),
            Transform::from_xyz(pos.x, 0.05, pos.z),
            LevelEntity,
        ));
    }

    // --- The altar room: a long table, three vials, the shuffled truth.
    let altar = cell_center(0, 1);
    cx.block(&altar_m, altar.x, 0.0, -16.6, Vec3::new(5.4, 1.0, 1.2), true);
    for (i, x) in [-8.2_f32, -1.8].into_iter().enumerate() {
        let _ = i;
        cx.block(&trim, x, 0.0, -16.6, Vec3::new(0.3, 2.0, 0.3), false);
    }
    let mut roles = [VialRole::Cure, VialRole::Death, VialRole::Drug];
    let mut tints = [
        Vec3::new(0.25, 1.0, 0.45),
        Vec3::new(1.0, 0.15, 0.2),
        Vec3::new(0.7, 0.3, 1.0),
    ];
    for i in (1..3).rev() {
        roles.swap(i, (rng.next() * (i as f32 + 1.0)) as usize % (i + 1));
        tints.swap(i, (rng.next() * (i as f32 + 1.0)) as usize % (i + 1));
    }
    for slot in 0..3 {
        let mat = glow(&mut mats, tints[slot], 2.5);
        let m = cx.meshes.add(Cylinder::new(0.09, 0.28));
        let p = VIAL_POS[slot];
        cx.commands.spawn((
            Vial { slot, pos: p },
            ink(m, mat, cx.ink, Transform::from_xyz(p.x, p.y + 0.14, p.z), Vec3::new(1.25, 1.1, 1.25)),
            LevelEntity,
        ));
    }
    // Candles on the altar table (skull and candle).
    for x in [-7.2_f32, -2.8] {
        candle_on(&mut cx, &mut mats, Vec3::new(x, 1.0, -16.6), Vec3::new(1.0, 0.7, 0.3), &mut rng);
    }
    skull(&mut cx, &mut mats, &bone, Vec3::new(-8.2, 2.05, -16.6));
    skull(&mut cx, &mut mats, &bone, Vec3::new(-1.8, 2.05, -16.6));

    // --- The way out: a door that stays shut until the cure.
    let door_mat = glow(&mut mats, Vec3::new(1.0, 0.15, 0.1), 2.0);
    cx.block(&door_mat, -5.0, 0.0, -20.0, Vec3::new(2.0, WALL_H - 0.2, 0.4), true); // last box pushed: removed on cure

    // --- Secret passages: spots Nwit steps out of the walls.
    let mut passages = Vec::new();
    for r in 0..4 {
        for c in 0..4 {
            let ctr = cell_center(r, c);
            passages.push(ctr + Vec3::new(0.0, 0.0, -3.9));
            passages.push(ctr + Vec3::new(0.0, 0.0, 3.9));
            passages.push(ctr + Vec3::new(-3.9, 0.0, 0.0));
            passages.push(ctr + Vec3::new(3.9, 0.0, 0.0));
        }
    }

    // --- Dim light. The candles do most of the work.
    cx.commands.spawn((
        Sun,
        DirectionalLight {
            illuminance: 500.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.2, 0.4, 0.0)),
        LevelEntity,
    ));

    let boxes = std::mem::take(&mut cx.boxes);
    commands.insert_resource(Boxes(boxes));
    commands.insert_resource(Level2 {
        roles,
        drunk: [false; 3],
        door_open: false,
        door_mat,
        passages,
    });

    // --- Ro, and Granmèt Nwit hiding in the walls.
    let ro = spawn_humanoid(&mut commands, &mut meshes, &mut mats, &ink_res, START, 0.0, 0.62, 0.22, 0.34);
    commands.entity(ro).insert((Player, crate::character::Skin(crate::character::Cast::Hero)));
    crate::weapons::attach_player_weapons(&mut commands, &mut meshes, &mut mats, &ink_res, ro);

    let nwit = spawn_humanoid(&mut commands, &mut meshes, &mut mats, &ink_res, Vec3::new(0.0, -50.0, 0.0), 0.0, 0.08, 0.06, 0.28);
    commands.entity(nwit).insert(crate::character::Skin(crate::character::Cast::Nwit));
    // His poisoned blade, glowing green.
    let blade_mat = glow(&mut mats, Vec3::new(0.3, 1.0, 0.4), 4.0);
    let blade = meshes.add(Cuboid::new(0.03, 0.05, 0.4));
    commands.entity(nwit).with_child((
        Mesh3d(blade),
        MeshMaterial3d(blade_mat),
        Transform::from_xyz(0.2, 1.1, -0.45),
    ));
    commands.entity(nwit).insert((
        Guard {
            name: "Granmèt Nwit",
            age: 52,
            kind: GuardKind::Phantom,
            status: LostStatus::Standing,
            state: AiState::Patrol,
            hp: 100.0,
            shot_cd: 0.0,
            stagger: 0.0,
            pain: 0.0,
            dmg: 15.0,
            fire_gap: 1.0,
            armor: 1.0,
                mortal: false,
                melee: false,
                max_hp: 100.0,
                speed_mul: 1.0,
                bullet_hp: 0.0,
                head_kills: false,
                boss: false,
                zombie: false,
                bleeds: false,
                hp_floor: 0.0,
            home_y: 0.0,
            fall: 0.0,
            fall_yaw: 0.0,
            awareness: 0.0,
            last_known: Vec3::ZERO,
            timer: 0.0,
            unseen: 0.0,
            sees: false,
            aim: 0.0,
            route: Vec::new(),
            wp: 0,
            home_yaw: 0.0,
            fov: 1.0,
            range: 10.0,
            eye: 1.5,
            marker: Handle::default(),
        },
        Nwit {
            state: NwitState::Lurk,
            timer: 7.0,
            attempts: 0,
            enabled: true,
            at: Vec3::ZERO,
        },
        Visibility::Hidden,
    ));
}

pub fn candle(cx: &mut Ctx, mats: &mut Assets<StandardMaterial>, at: Vec3, col: Vec3, rng: &mut Rng) {
    candle_on(cx, mats, Vec3::new(at.x, 0.0, at.z), col, rng);
}

pub fn candle_on(cx: &mut Ctx, mats: &mut Assets<StandardMaterial>, at: Vec3, col: Vec3, rng: &mut Rng) {
    let flame = glow(mats, col, 6.0);
    let stick = matte(mats, 0.8);
    let m = cx.meshes.add(Cylinder::new(0.05, 0.3));
    let f = cx.meshes.add(Sphere::new(0.045));
    cx.commands.spawn((
        Mesh3d(m),
        MeshMaterial3d(stick),
        Transform::from_xyz(at.x, at.y + 0.15, at.z),
        LevelEntity,
        children![
            (
                Mesh3d(f),
                MeshMaterial3d(flame),
                Transform::from_xyz(0.0, 0.2, 0.0),
            ),
            (
                PointLight {
                    color: Color::srgb(col.x, col.y, col.z),
                    intensity: 220_000.0,
                    range: 9.0,
                    ..default()
                },
                Flicker {
                    base: 220_000.0,
                    phase: rng.next() * 6.28,
                },
                Transform::from_xyz(0.0, 0.5, 0.0),
            )
        ],
    ));
}

pub fn skull(cx: &mut Ctx, mats: &mut Assets<StandardMaterial>, bone: &Handle<StandardMaterial>, at: Vec3) {
    let dark = matte(mats, 0.02);
    let head = cx.meshes.add(Sphere::new(0.16));
    let eye = cx.meshes.add(Cuboid::new(0.05, 0.06, 0.03));
    cx.commands
        .spawn((
            ink(head, bone.clone(), cx.ink, Transform::from_xyz(at.x, at.y + 0.16, at.z), Vec3::splat(1.2)),
            LevelEntity,
        ))
        .with_child((Mesh3d(eye.clone()), MeshMaterial3d(dark.clone()), Transform::from_xyz(-0.06, 0.02, 0.13)))
        .with_child((Mesh3d(eye), MeshMaterial3d(dark), Transform::from_xyz(0.06, 0.02, 0.13)));
    let _ = grow_box;
}

pub fn flicker(time: Res<Time>, mut lights: Query<(&Flicker, &mut PointLight)>) {
    let t = time.elapsed_secs();
    for (f, mut l) in &mut lights {
        let k = 0.8 + 0.2 * (t * 13.0 + f.phase).sin() + 0.1 * (t * 29.0 + f.phase * 2.0).sin();
        l.intensity = f.base * k;
    }
}

// --------------------------------------------------------- Granmèt Nwit

fn taunt(n: u32) -> &'static str {
    match n {
        0 => "\"Plip, plip! Mwen se chandèl, van pa tenyen m. Ro... map tann ou.\"",
        1 => "\"Plip, plip! Ou pa ka kouri, ti gason.\"",
        _ => "\"Ro, mwen pral fè ou chèf zonbi mwen.\"",
    }
}

#[allow(clippy::too_many_arguments)]
pub fn nwit_ai(
    time: Res<Time>,
    clock: Res<GameClock>,
    boxes: Res<Boxes>,
    l2: Res<Level2>,
    fx: Res<Fx>,
    mut commands: Commands,
    mut rng: ResMut<Rng>,
    mut shake: ResMut<Shake>,
    mut poison: ResMut<Poison>,
    mut health: ResMut<Health>,
    mut msg: ResMut<Message>,
    player: Single<(&Transform, &Mover), (With<Player>, Without<Nwit>)>,
    nwit: Single<(&mut Nwit, &mut Guard, &mut Transform, &mut Visibility, &mut Mover), Without<Player>>,
) {
    let dt = time.delta_secs().min(0.1);
    let (pt, _pm) = player.into_inner();
    let (mut n, mut g, mut tf, mut vis, mut mv) = nwit.into_inner();
    let ppos = pt.translation;
    let night = clock.minute >= NIGHT_MINUTE;

    // Vanish into the walls and wait for the next ambush.
    let vanish = |n: &mut Nwit, tf: &mut Transform, vis: &mut Visibility, mv: &mut Mover, g: &mut Guard, rng: &mut Rng| {
        n.state = NwitState::Lurk;
        // His power surges at night: ambushes come twice as often.
        n.timer = if night { 8.0 + rng.next() * 4.0 } else { 16.0 + rng.next() * 8.0 };
        tf.translation = Vec3::new(0.0, -50.0, 0.0);
        *vis = Visibility::Hidden;
        mv.speed = 0.0;
        g.pain = 0.0;
    };

    if !n.enabled {
        if tf.translation.y > -40.0 {
            vanish(&mut n, &mut tf, &mut vis, &mut mv, &mut g, &mut rng);
        }
        return;
    }

    match n.state {
        NwitState::Lurk => {
            n.timer -= dt;
            if n.timer <= 0.0 {
                // Pick a secret passage with a clear line to Ro, 5 to 14 m away.
                let eye = ppos + Vec3::Y * 1.2;
                let mut best: Option<(f32, Vec3)> = None;
                for p in &l2.passages {
                    let d = Vec2::new(p.x - ppos.x, p.z - ppos.z).length();
                    if !(5.0..=14.0).contains(&d) {
                        continue;
                    }
                    let clear = !segment_blocked(*p + Vec3::Y * 1.2, eye, &boxes.0);
                    let score = rng.next() + if clear { 2.0 } else { 0.0 };
                    if best.is_none_or(|(b, _)| score > b) {
                        best = Some((score, *p));
                    }
                }
                let Some((_, at)) = best else {
                    n.timer = 2.0;
                    return;
                };
                n.at = at;
                n.state = NwitState::Telegraph;
                n.timer = 1.4;
                msg.0 = taunt(n.attempts).to_string();
                shake.0 = (shake.0 + 0.25).min(1.0);
                dust_puff(&mut commands, &fx, at + Vec3::Y * 1.0, Vec3::Y, 0.4, 1.2);
                fx.play(&mut commands, crate::audio::Sfx::Plip, 1.0, 1.0, 0.0);
            }
        }
        NwitState::Telegraph => {
            n.timer -= dt;
            shake.0 = (shake.0 + 0.4 * dt).min(0.5);
            if n.timer <= 0.0 {
                tf.translation = n.at;
                *vis = Visibility::Inherited;
                let to = ppos - tf.translation;
                tf.rotation = Quat::from_rotation_y((-to.x).atan2(-to.z));
                n.state = NwitState::Charge;
            }
        }
        NwitState::Charge => {
            let dist = Vec2::new(ppos.x - tf.translation.x, ppos.z - tf.translation.z).length();
            mv.armed = false;
            walk_to(&mut tf, &mut mv, ppos, 6.0, dt, &boxes.0);
            if g.pain > 0.0 {
                // Shot or cut: he cries out and retreats through the wall.
                msg.0 = "\"Woy!\" He shrieks, and the wall swallows him.".into();
                dust_puff(&mut commands, &fx, tf.translation + Vec3::Y, Vec3::Y, 0.5, 1.0);
                vanish(&mut n, &mut tf, &mut vis, &mut mv, &mut g, &mut rng);
            } else if dist < 1.2 {
                if n.attempts < 2 {
                    n.attempts += 1;
                    health.hp -= 25.0;
                    health.hurt = 1.0;
                    shake.0 = 0.8;
                    msg.0 = "He slashes you and laughs: \"Woy!\" Then he is gone.".into();
                } else {
                    // The stab in the back: the poison clock starts.
                    health.hp = (health.hp - 15.0).max(5.0);
                    health.hurt = 1.0;
                    shake.0 = 1.0;
                    poison.active = true;
                    poison.remaining = POISON_SECONDS;
                    poison.bottles = 0;
                    n.enabled = false;
                    poison.alert = POISON_ALERT_SECONDS;
                    msg.0 = "He stabs you in the back: \"Ro, mwen se chandèl. I will make you my zombie chief.\" The blade was poisoned. You have 2 minutes to find the potion room (the altar, in the north) and drink the cure to survive.".into();
                }
                vanish(&mut n, &mut tf, &mut vis, &mut mv, &mut g, &mut rng);
            }
        }
    }
}

// ----------------------------------------------------------- poison, vials

pub fn poison_tick(
    time: Res<Time>,
    mut poison: ResMut<Poison>,
    mut health: ResMut<Health>,
    mut msg: ResMut<Message>,
) {
    let dt = time.delta_secs().min(0.1);
    if poison.drugged > 0.0 {
        poison.drugged += dt;
    }
    poison.alert = (poison.alert - dt).max(0.0);
    if poison.active {
        poison.remaining -= dt;
        if poison.remaining <= 0.0 {
            poison.remaining = 0.0;
            health.hp = 0.0;
            msg.0 = "The poison wins.".into();
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn interact2(
    keys: Res<ButtonInput<KeyCode>>,
    mut prompt: ResMut<Prompt>,
    mut msg: ResMut<Message>,
    mut poison: ResMut<Poison>,
    mut health: ResMut<Health>,
    mut l2: ResMut<Level2>,
    mut boxes: ResMut<Boxes>,
    mut checkpoint: ResMut<Checkpoint>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut nwits: Query<&mut Nwit>,
    mut vials: Query<(&Vial, &mut Visibility)>,
    notes: Query<&Note>,
    player: Single<&Transform, With<Player>>,
) {
    let pp = player.translation;
    let near = |p: Vec3, r: f32| Vec2::new(p.x - pp.x, p.z - pp.z).length() < r;
    prompt.0.clear();
    let e = keys.just_pressed(KeyCode::KeyE);

    for n in &notes {
        if near(n.pos, 1.8) {
            prompt.0 = "E: read the note".into();
            if e {
                msg.0 = n.text.to_string();
            }
        }
    }

    for (v, mut vis) in &mut vials {
        if l2.drunk[v.slot] || !near(v.pos, 1.7) {
            continue;
        }
        let role = l2.roles[v.slot];
        prompt.0 = format!("Vial {}: it smells of {}.   E: drink", v.slot + 1, role.smell());
        if !e {
            continue;
        }
        l2.drunk[v.slot] = true;
        *vis = Visibility::Hidden;
        match role {
            VialRole::Cure => {
                poison.active = false;
                poison.alert = 0.0;
                poison.drugged = 0.0;
                poison.bottles = 5;
                health.hp = 100.0;
                checkpoint.0 = ALTAR_SPAWN; // the first save point
                l2.door_open = true;
                boxes.0.pop(); // the door was the last box pushed
                if let Some(mut m) = mats.get_mut(&l2.door_mat) {
                    m.base_color = Color::srgb(0.2, 1.0, 0.45);
                    m.emissive = LinearRgba::rgb(0.6, 4.0, 1.6);
                }
                for mut n in &mut nwits {
                    n.enabled = false;
                }
                msg.0 = "It tastes of leaves and lemon. The fire in your back fades. Saved. You keep the bottle: 5 uses (H). The north door opens.".into();
            }
            VialRole::Death => {
                health.hp = 0.0;
                msg.0 = "It burns like acid. Wrong vial.".into();
            }
            VialRole::Drug => {
                poison.drugged = 0.01;
                msg.0 = "Sweet, then dizzy. The room starts to swim. Find the cure.".into();
            }
        }
    }
}

/// Reaching the doorway after the cure ends the level.
pub fn goal2(
    l2: Res<Level2>,
    mut next: ResMut<NextState<AppState>>,
    player: Single<&Transform, With<Player>>,
) {
    if l2.door_open && player.translation.z < -19.3 {
        next.set(AppState::Complete2);
    }
}
