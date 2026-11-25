//! Level 6: Granmèt Nwit's quarters. Rich rooms, a vast bathroom with a
//! Jacuzzi, a ritual chamber, a gallery of the kids he took, and secret
//! passages. In his own house Nwit can be hurt. He plays hide-and-seek: a
//! taunt and a glow in the wall tell you where he will step out, he lunges,
//! and each wound drives him back into the walls. At half health he flees to
//! the roof. Tight quarters make the M50 unwieldy. At 6:00 PM, the dead and
//! the freed of earlier floors who were not laid to rest rise to join him.

use crate::ai::{walk_to, AiState, Guard, GuardKind};
use crate::character::{segment_blocked, spawn_humanoid, Boxes, Cast, Mover, Skin};
use crate::level::Ctx;
use crate::level2::{candle, skull, Note, Nwit, NwitState};
use crate::level3::{wall_line, Story, DOOR_W, WALL_H};
use crate::style::{glow, ink, matte, Ink};
use crate::weapons::{dust_puff, Fx, Health, Loadout, Rng, Shake};
use crate::*;

pub const ARRIVAL: Vec3 = Vec3::new(25.55, 0.0, -10.6); // just outside the stair core door (the core stacks on every floor)
const STAIR_DOOR: Vec3 = Vec3::new(25.55, 1.2, -12.25); // the NE stair core, stacked on every floor
const CELL_W: f32 = 12.0;
const CELL_H: f32 = 13.0;
const ROWS: i32 = 3;
const COLS: i32 = 5;
const NWIT_HP: f32 = 700.0;

#[derive(Resource)]
pub struct Level6 {
    pub fled: bool,
    pub stairs_open: bool,
    pub night_done: bool,
    pub passages: Vec<Vec3>,
    /// Nwit's health when he stepped out of the wall this time.
    pub charge_start_hp: f32,
    /// Health bottles still in Nwit's stash. While any are left he cannot be hurt past
    /// the edge of death: he drinks, and he is back. Take them all and he is mortal.
    pub reserve: u32,
}

/// One of Nwit's health bottles, standing in a room of his house.
#[derive(Component)]
pub struct Stash;

/// The most Brutes (the strongest risen) that can ever exist in the game.
pub const BRUTE_CAP: u32 = 10;
/// Where his bottles stand: (row, col) of the cell and an offset inside it.
const STASH_CELLS: [(i32, i32, f32, f32); 8] = [
    (0, 0, 3.0, -3.0),
    (0, 2, -3.0, 3.0),
    (0, 4, 3.0, 3.0),
    (1, 1, -3.0, -3.0),
    (1, 3, 3.0, 2.0),
    (2, 1, 2.5, -3.0),
    (2, 3, -3.0, 3.0),
    (2, 4, 3.0, -3.0),
];

/// Doorways between neighbouring cells (row, col).
const OPEN: &[((i32, i32), (i32, i32))] = &[
    ((2, 0), (2, 1)),
    ((2, 1), (2, 2)),
    ((2, 2), (2, 3)),
    ((2, 3), (2, 4)),
    ((1, 0), (1, 1)),
    ((1, 1), (1, 2)),
    ((1, 2), (1, 3)),
    ((1, 3), (1, 4)),
    ((0, 0), (0, 1)),
    ((0, 2), (0, 3)),
    ((0, 3), (0, 4)),
    ((2, 0), (1, 0)),
    ((2, 2), (1, 2)),
    ((2, 4), (1, 4)),
    ((1, 1), (0, 1)),
    ((1, 3), (0, 3)),
    ((1, 4), (0, 4)),
    ((1, 0), (0, 0)),
    ((0, 1), (0, 2)),
];

fn is_open(a: (i32, i32), b: (i32, i32)) -> bool {
    OPEN.iter().any(|&(x, y)| (x == a && y == b) || (x == b && y == a))
}

fn cell(r: i32, c: i32) -> Vec3 {
    Vec3::new(-24.0 + CELL_W * c as f32, 0.0, -13.0 + CELL_H * r as f32)
}

// ------------------------------------------------------------------- setup

#[allow(clippy::too_many_arguments)]
pub fn setup_level6(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    ink_res: Res<Ink>,
    mut rng: ResMut<Rng>,
    mut checkpoint: ResMut<Checkpoint>,
    mut msg: ResMut<Message>,
    mut health: ResMut<Health>,
    mut rig: ResMut<CamRig>,
    mut objectives: ResMut<Objectives>,
    mut stats: ResMut<Stats>,
    mut loadout: ResMut<Loadout>,
) {
    let ink_res = ink_res.clone();
    checkpoint.0 = ARRIVAL;
    health.hp = health.hp.max(70.0);
    health.hurt = 0.0;
    loadout.cramped = true;
    *objectives = Objectives::default();
    *stats = Stats::default();
    rig.yaw = 0.0;
    rig.first = false;
    msg.0 = "His rooms. Silk, brass and candle smoke. Somewhere in the walls, a man is humming.".into();

    let floor = matte(&mut mats, 0.12);
    let rug = matte(&mut mats, 0.2);
    let wall = matte(&mut mats, 0.28);
    let wood = matte(&mut mats, 0.2);
    let silk = matte(&mut mats, 0.38);
    let trim = matte(&mut mats, 0.62);
    let bone = matte(&mut mats, 0.8);
    let tile = matte(&mut mats, 0.45);
    let water = mats.add(StandardMaterial {
        base_color: Color::srgb(0.1, 0.4, 0.5),
        emissive: LinearRgba::rgb(0.05, 0.7, 1.0),
        perceptual_roughness: 0.15,
        ..default()
    });
    let photo = glow(&mut mats, Vec3::new(0.9, 0.9, 0.8), 1.2);

    let mut cx = Ctx {
        commands: &mut commands,
        meshes: &mut meshes,
        ink: &ink_res,
        boxes: Vec::new(),
    };
    let plane = cx.meshes.add(Plane3d::default().mesh().size(64.0, 44.0));
    cx.commands.spawn((Mesh3d(plane), MeshMaterial3d(floor), LevelEntity));

    // --- Walls and doorways.
    for k in 0..=COLS {
        let x = -30.0 + CELL_W * k as f32;
        for r in 0..ROWS {
            let zc = -13.0 + CELL_H * r as f32;
            let doors: &[f32] = if (1..COLS).contains(&k) && is_open((r, k - 1), (r, k)) { &[zc] } else { &[] };
            wall_line(&mut cx, &wall, true, x, zc - CELL_H / 2.0, zc + CELL_H / 2.0, doors);
        }
    }
    for k in 0..=ROWS {
        let z = -19.5 + CELL_H * k as f32;
        for c in 0..COLS {
            let xc = -24.0 + CELL_W * c as f32;
            let doors: Vec<f32> = if (1..ROWS).contains(&k) && is_open((k - 1, c), (k, c)) {
                vec![xc]
            } else {
                vec![]
            };
            wall_line(&mut cx, &wall, false, z, xc - CELL_W / 2.0, xc + CELL_W / 2.0, &doors);
        }
    }

    // The NE stair core: the roof stairs, sealed until Nwit flees (the door is the removable block).
    wall_line(&mut cx, &wall, true, 24.0, -19.5, -12.25, &[]);
    wall_line(&mut cx, &wall, false, -12.25, 24.0, 30.0, &[25.55]);

    // --- Rooms. Every cell gets candles; the rest by theme.
    let mut n = 0;
    for r in 0..ROWS {
        for c in 0..COLS {
            let ctr = cell(r, c);
            let col = [Vec3::new(1.0, 0.75, 0.35), Vec3::new(1.0, 0.25, 0.2), Vec3::new(0.35, 0.5, 1.0)][n % 3];
            candle(&mut cx, &mut mats, ctr + Vec3::new(-4.8, 0.0, -5.0), col, &mut rng);
            candle(&mut cx, &mut mats, ctr + Vec3::new(4.8, 0.0, 5.0), col, &mut rng);
            n += 1;
        }
    }
    // Antechamber (arrival): a long rug.
    let r1 = cx.meshes.add(Cuboid::new(8.0, 0.02, 3.0));
    cx.commands.spawn((Mesh3d(r1), MeshMaterial3d(rug.clone()), Transform::from_xyz(-24.0, 0.012, 13.0), LevelEntity));
    // Sitting room (2,1): sofas and a low table.
    let c = cell(2, 1);
    cx.block(&silk, c.x - 3.0, 0.0, c.z + 3.5, Vec3::new(3.6, 0.8, 1.2), true);
    cx.block(&silk, c.x + 4.0, 0.0, c.z, Vec3::new(1.2, 0.8, 3.4), true);
    cx.block(&wood, c.x, 0.0, c.z, Vec3::new(2.0, 0.5, 1.0), true);
    // Wardrobe (2,2): tall cabinets.
    let c = cell(2, 2);
    for i in 0..5 {
        cx.block(&wood, c.x - 4.4 + i as f32 * 2.2, 0.0, c.z + 5.9, Vec3::new(1.8, 2.2, 0.7), true);
    }
    // Library (2,3): shelves and a desk.
    let c = cell(2, 3);
    cx.block(&wood, c.x, 0.0, c.z - 5.8, Vec3::new(9.0, 2.2, 0.6), false);
    cx.block(&wood, c.x + 2.0, 0.0, c.z + 1.0, Vec3::new(3.0, 0.9, 1.4), true);
    // Study (2,4): a desk and the first note.
    let c = cell(2, 4);
    cx.block(&wood, c.x + 2.0, 0.0, c.z, Vec3::new(3.0, 0.9, 1.4), true);
    // Bedroom (1,0): a vast bed and nightstands.
    let c = cell(1, 0);
    cx.block(&silk, c.x - 1.0, 0.0, c.z - 2.0, Vec3::new(4.4, 0.7, 3.4), true);
    cx.block(&wood, c.x + 3.0, 0.0, c.z - 4.0, Vec3::new(0.9, 0.6, 0.9), true);
    cx.block(&wood, c.x - 5.0, 0.0, c.z - 4.0, Vec3::new(0.9, 0.6, 0.9), true);
    // Gallery of the taken (1,1): framed photographs of the kids, and a note.
    let c = cell(1, 1);
    for i in 0..8 {
        let m = cx.meshes.add(Cuboid::new(0.7, 0.9, 0.06));
        cx.commands.spawn((
            Mesh3d(m),
            MeshMaterial3d(photo.clone()),
            Transform::from_xyz(c.x - 5.0 + i as f32 * 1.4, 1.5, c.z - 6.1),
            LevelEntity,
        ));
    }
    cx.block(&wood, c.x, 0.0, c.z + 1.0, Vec3::new(3.0, 0.8, 1.0), true);
    // The ritual chamber (1,2): a ring of skulls and candles round a black slab.
    let c = cell(1, 2);
    cx.block(&wood, c.x, 0.0, c.z, Vec3::new(3.2, 0.9, 1.6), true);
    for i in 0..12 {
        let a = i as f32 / 12.0 * std::f32::consts::TAU;
        let p = Vec3::new(c.x + a.cos() * 4.0, 0.0, c.z + a.sin() * 3.6);
        skull(&mut cx, &mut mats, &bone, Vec3::new(p.x, 0.0, p.z));
        if i % 2 == 0 {
            candle(&mut cx, &mut mats, Vec3::new(c.x + a.cos() * 2.6, 0.0, c.z + a.sin() * 2.4), Vec3::new(1.0, 0.2, 0.15), &mut rng);
        }
    }
    // The vast bathroom (1,3): a Jacuzzi under glowing water, tiles, pillars.
    let c = cell(1, 3);
    let tile_mesh = cx.meshes.add(Cuboid::new(11.0, 0.02, 12.0));
    cx.commands.spawn((Mesh3d(tile_mesh), MeshMaterial3d(tile.clone()), Transform::from_xyz(c.x, 0.012, c.z), LevelEntity));
    let ring = cx.meshes.add(Cylinder::new(3.6, 0.8));
    cx.boxes.push(crate::character::Box3 { c: Vec3::new(c.x, 0.4, c.z), h: Vec3::new(3.6, 0.4, 3.6) });
    cx.commands.spawn((
        ink(ring, trim.clone(), cx.ink, Transform::from_xyz(c.x, 0.4, c.z), Vec3::new(1.03, 1.05, 1.03)),
        LevelEntity,
    ));
    let pool = cx.meshes.add(Cylinder::new(3.2, 0.06));
    cx.commands.spawn((Mesh3d(pool), MeshMaterial3d(water), Transform::from_xyz(c.x, 0.81, c.z), LevelEntity));
    cx.commands.spawn((
        PointLight {
            color: Color::srgb(0.3, 0.85, 1.0),
            intensity: 1_800_000.0,
            range: 12.0,
            ..default()
        },
        Transform::from_xyz(c.x, 1.6, c.z),
        LevelEntity,
    ));
    for (dx, dz) in [(-5.0, -5.5), (5.0, -5.5), (-5.0, 5.5), (5.0, 5.5)] {
        cx.block(&trim, c.x + dx, 0.0, c.z + dz, Vec3::new(0.9, 2.6, 0.9), true);
    }
    // Bath annex (1,4) and the treasure and shrine rooms along the north.
    let c = cell(1, 4);
    cx.block(&tile, c.x + 3.0, 0.0, c.z - 2.0, Vec3::new(2.6, 0.7, 4.0), true);
    for (rr, cc) in [(0, 0), (0, 1), (0, 2), (0, 3)] {
        let c = cell(rr, cc);
        // A shrine shelf of skulls and candles.
        cx.block(&wood, c.x, 0.0, c.z - 5.8, Vec3::new(8.0, 1.2, 0.6), false);
        for i in 0..5 {
            skull(&mut cx, &mut mats, &bone, Vec3::new(c.x - 3.0 + i as f32 * 1.5, 1.2, c.z - 5.8));
        }
        // Straw effigies on poles (charms, not bodies).
        for dx in [-3.5_f32, 3.5] {
            let m = cx.meshes.add(Cuboid::new(0.5, 1.4, 0.5));
            cx.boxes.push(crate::character::Box3 { c: Vec3::new(c.x + dx, 0.7, c.z + 2.0), h: Vec3::new(0.25, 0.7, 0.25) });
            cx.commands.spawn((
                ink(m, silk.clone(), cx.ink, Transform::from_xyz(c.x + dx, 0.7, c.z + 2.0), Vec3::splat(1.1)),
                LevelEntity,
            ));
        }
    }
    // Voice horns in the walls: Nwit's "spirit voice" is a tube and a man.
    let horn_m = matte(&mut mats, 0.3);
    for (x, z) in [(-18.0_f32, -6.4_f32), (6.0, -6.4), (18.0, 6.4), (-6.0, 6.4)] {
        let m = cx.meshes.add(Cone::new(0.35, 0.7));
        cx.commands.spawn((
            Mesh3d(m),
            MeshMaterial3d(horn_m.clone()),
            Transform::from_xyz(x, 1.8, z).with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
            LevelEntity,
        ));
    }

    // --- Notes: what the man really is.
    for (pos, text) in [
        (
            cell(2, 4) + Vec3::new(-1.0, 0.0, 0.0),
            "A note, in a tired hand: \"The voice in the walls is mine. The spoon charm is wood. The poison is a root and a lie. They are afraid because I let them be.\"",
        ),
        (
            cell(1, 1) + Vec3::new(2.0, 0.0, 3.0),
            "A note: \"Fifty-one kids this year. I wrote their names so I would not forget what I owe. Nobody has ever asked me to stop.\"",
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

    // --- The roof stairs: a sealed door in the north wall of the north-east room.
    let door_m = glow(&mut mats, Vec3::new(1.0, 0.15, 0.1), 2.0);
    cx.block(&door_m, STAIR_DOOR.x, 0.0, STAIR_DOOR.z, Vec3::new(DOOR_W, WALL_H - 0.2, 0.4), true);

    cx.commands.spawn((
        Sun,
        DirectionalLight {
            illuminance: 420.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.2, 0.4, 0.0)),
        LevelEntity,
    ));

    // --- Secret passages: the spots Nwit steps out of the walls.
    let mut passages = Vec::new();
    for r in 0..ROWS {
        for c in 0..COLS {
            let ctr = cell(r, c);
            passages.push(ctr + Vec3::new(0.0, 0.0, -5.2));
            passages.push(ctr + Vec3::new(0.0, 0.0, 5.2));
            passages.push(ctr + Vec3::new(-4.8, 0.0, 0.0));
            passages.push(ctr + Vec3::new(4.8, 0.0, 0.0));
        }
    }
    let boxes = std::mem::take(&mut cx.boxes);
    commands.insert_resource(Boxes(boxes));
    commands.insert_resource(crate::level3::Grab(0));
    commands.insert_resource(Level6 {
        fled: false,
        stairs_open: false,
        night_done: false,
        passages,
        charge_start_hp: NWIT_HP,
        reserve: STASH_CELLS.len() as u32,
    });
    // --- Nwit's stash of health bottles.
    let bottle_m = glow(&mut mats, Vec3::new(0.3, 1.0, 0.45), 3.5);
    let bottle = meshes.add(Cylinder::new(0.09, 0.3));
    for (r, c, dx, dz) in STASH_CELLS {
        let at = cell(r, c) + Vec3::new(dx, 0.0, dz);
        commands.spawn((
            Stash,
            Mesh3d(bottle.clone()),
            MeshMaterial3d(bottle_m.clone()),
            Transform::from_translation(at + Vec3::Y * 0.25),
            LevelEntity,
        ));
        commands.spawn((
            PointLight {
                color: Color::srgb(0.3, 1.0, 0.5),
                intensity: 90_000.0,
                range: 4.0,
                ..default()
            },
            Transform::from_translation(at + Vec3::Y * 0.8),
            LevelEntity,
        ));
    }

    // --- Ro, and Nwit hiding in the walls (mortal now).
    let ro = spawn_humanoid(&mut commands, &mut meshes, &mut mats, &ink_res, ARRIVAL, std::f32::consts::PI, 0.62, 0.22, 0.34);
    commands.entity(ro).insert((Player, crate::character::Skin(crate::character::Cast::Hero)));
    crate::weapons::attach_player_weapons(&mut commands, &mut meshes, &mut mats, &ink_res, ro);

    let nwit = spawn_humanoid(&mut commands, &mut meshes, &mut mats, &ink_res, Vec3::new(0.0, -50.0, 0.0), 0.0, 0.08, 0.06, 0.28);
    commands.entity(nwit).insert(crate::character::Skin(crate::character::Cast::Nwit));
    let blade_mat = glow(&mut mats, Vec3::new(0.3, 1.0, 0.4), 4.0);
    let blade = meshes.add(Cuboid::new(0.03, 0.05, 0.4));
    commands.entity(nwit).with_child((
        Mesh3d(blade),
        MeshMaterial3d(blade_mat),
        Transform::from_xyz(0.2, 1.1, -0.45),
    ));
    commands.entity(nwit).insert((
        Guard {
            hp: NWIT_HP,
            max_hp: NWIT_HP,
            mortal: true,
            range: 10.0,
            ..Guard::base("Granmèt Nwit", 52, GuardKind::Phantom)
        },
        Nwit {
            state: NwitState::Lurk,
            timer: 6.0,
            attempts: 3,
            enabled: true,
            at: Vec3::ZERO,
        },
        Visibility::Hidden,
    ));
    let _ = (segment_blocked, ink);
}

// -------------------------------------------------------------- the hunt

fn compass(from: Vec3, to: Vec3) -> &'static str {
    let d = to - from;
    let a = d.x.atan2(-d.z).to_degrees(); // 0 = north (-z), 90 = east
    let a = (a + 360.0 + 22.5) % 360.0;
    ["north", "north-east", "east", "south-east", "south", "south-west", "west", "north-west"][(a / 45.0) as usize % 8]
}

#[allow(clippy::too_many_arguments)]
pub fn nwit6(
    time: Res<Time>,
    clock: Res<GameClock>,
    mut boxes: ResMut<Boxes>,
    mut l6: ResMut<Level6>,
    fx: Res<Fx>,
    mut commands: Commands,
    mut rng: ResMut<Rng>,
    mut shake: ResMut<Shake>,
    mut health: ResMut<Health>,
    mut msg: ResMut<Message>,
    player: Single<(&Transform, &Mover), (With<Player>, Without<Nwit>)>,
    nwit: Single<(&mut Nwit, &mut Guard, &mut Transform, &mut Visibility, &mut Mover), Without<Player>>,
) {
    let dt = time.delta_secs().min(0.1);
    let (pt, _) = player.into_inner();
    let (mut n, mut g, mut tf, mut vis, mut mv) = nwit.into_inner();
    let ppos = pt.translation;
    let night = clock.minute >= NIGHT_MINUTE;
    if l6.fled {
        return;
    }

    let vanish = |n: &mut Nwit, tf: &mut Transform, vis: &mut Visibility, mv: &mut Mover, g: &mut Guard, rng: &mut Rng, hp_frac: f32| {
        n.state = NwitState::Lurk;
        // Angrier as he is hurt, and at night.
        let base = if night { 4.0 } else { 7.0 };
        n.timer = (base + rng.next() * 4.0) * (0.5 + 0.5 * hp_frac);
        tf.translation = Vec3::new(0.0, -50.0, 0.0);
        *vis = Visibility::Hidden;
        mv.speed = 0.0;
        g.pain = 0.0;
    };
    let frac = (g.hp / g.max_hp).clamp(0.0, 1.0);
    // While his stash lasts he cannot be hurt below 55 percent: he drinks.
    g.hp_floor = if l6.reserve > 0 { g.max_hp * 0.55 } else { 0.0 };

    // At half health he runs for the roof and the stairs unlock.
    if g.hp <= g.max_hp * 0.5 {
        l6.fled = true;
        l6.stairs_open = true;
        n.enabled = false;
        boxes.0.retain(|b| Vec2::new(b.c.x - STAIR_DOOR.x, b.c.z - STAIR_DOOR.z).length() > 0.3);
        dust_puff(&mut commands, &fx, tf.translation + Vec3::Y, Vec3::Y, 0.6, 1.2);
        vanish(&mut n, &mut tf, &mut vis, &mut mv, &mut g, &mut rng, 0.0);
        msg.0 = "\"Woy! Woy!\" He bolts, bleeding, up some hidden stair. A door in the north-east rattles open. The roof is above.".into();
        return;
    }

    match n.state {
        NwitState::Lurk => {
            n.timer -= dt;
            if n.timer <= 0.0 {
                let eye = ppos + Vec3::Y * 1.2;
                let mut best: Option<(f32, Vec3)> = None;
                for p in &l6.passages {
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
                // A sound cue and a direction.
                msg.0 = format!(
                    "\"Plip, plip!\" A panel clicks in the wall to the {}.",
                    compass(ppos, at)
                );
                shake.0 = (shake.0 + 0.2).min(1.0);
                dust_puff(&mut commands, &fx, at + Vec3::Y, Vec3::Y, 0.4, 1.4);
                fx.play(&mut commands, crate::audio::Sfx::Plip, 1.0, 1.0, 0.0);
                commands.spawn((
                    PointLight {
                        color: Color::srgb(0.6, 0.3, 1.0),
                        intensity: 1_200_000.0,
                        range: 6.0,
                        ..default()
                    },
                    Transform::from_translation(at + Vec3::Y * 1.2),
                    crate::weapons::Particle {
                        vel: Vec3::ZERO,
                        life: 1.4,
                        max: 1.4,
                        gravity: 0.0,
                        grow: 1.0,
                        fade: true,
                        bounce: false,
                        base: Vec3::ONE,
                    },
                    LevelEntity,
                ));
            }
        }
        NwitState::Telegraph => {
            n.timer -= dt;
            if n.timer <= 0.0 {
                tf.translation = n.at;
                *vis = Visibility::Inherited;
                let to = ppos - tf.translation;
                tf.rotation = Quat::from_rotation_y((-to.x).atan2(-to.z));
                n.state = NwitState::Charge;
                l6.charge_start_hp = g.hp;
                g.pain = 0.0;
            }
        }
        NwitState::Charge => {
            let dist = Vec2::new(ppos.x - tf.translation.x, ppos.z - tf.translation.z).length();
            mv.armed = false;
            walk_to(&mut tf, &mut mv, ppos, 6.5, dt, &boxes.0);
            if g.pain > 0.0 {
                // Any bullet that finds him: he cries out and flees through the wall.
                msg.0 = "\"Woy!\" He tears away from you into the wall. Every opening you give him costs him.".into();
                dust_puff(&mut commands, &fx, tf.translation + Vec3::Y, Vec3::Y, 0.5, 1.0);
                vanish(&mut n, &mut tf, &mut vis, &mut mv, &mut g, &mut rng, frac);
            } else if dist < 1.2 {
                health.hp -= 28.0;
                health.hurt = 1.0;
                shake.0 = 0.9;
                msg.0 = "He slashes you and laughs: \"Woy!\" Then he is gone into the wall.".into();
                vanish(&mut n, &mut tf, &mut vis, &mut mv, &mut g, &mut rng, frac);
            } else {
                g.pain = 0.0; // he shrugs off a graze and keeps coming
            }
        }
    }
}

/// Reading notes, and taking Nwit's health bottles from his stash.
#[allow(clippy::too_many_arguments)]
pub fn interact6(
    keys: Res<ButtonInput<KeyCode>>,
    mut prompt: ResMut<Prompt>,
    mut msg: ResMut<Message>,
    mut l6: ResMut<Level6>,
    mut poison: ResMut<crate::level2::Poison>,
    mut commands: Commands,
    notes: Query<&Note>,
    stash: Query<(Entity, &Transform), With<Stash>>,
    player: Single<&Transform, With<Player>>,
) {
    prompt.0.clear();
    let pp = player.translation;
    for n in &notes {
        if Vec2::new(n.pos.x - pp.x, n.pos.z - pp.z).length() < 1.8 {
            prompt.0 = "E: read the note".into();
            if keys.just_pressed(KeyCode::KeyE) {
                msg.0 = n.text.to_string();
            }
        }
    }
    for (e, tf) in &stash {
        if Vec2::new(tf.translation.x - pp.x, tf.translation.z - pp.z).length() < 1.6 {
            prompt.0 = format!("E: take Nwit's health bottle ({} left in his reserve)", l6.reserve);
            if keys.just_pressed(KeyCode::KeyE) {
                commands.entity(e).despawn();
                l6.reserve = l6.reserve.saturating_sub(1);
                if poison.bottles < 900 {
                    poison.bottles += 1;
                }
                msg.0 = if l6.reserve == 0 {
                    "That was the last of his reserve. Far off in the walls, Nwit howls. He can bleed now: wound him and he will run for the roof.".into()
                } else {
                    format!("You take his bottle: you have {}. {} left in his reserve. Without them he cannot heal.", poison.bottles, l6.reserve)
                };
            }
        }
    }
}

/// At 6:00 PM, those who were not laid to rest stand again and come for Ro.
#[allow(clippy::too_many_arguments)]
pub fn night6(
    clock: Res<GameClock>,
    story: Res<Story>,
    mut l6: ResMut<Level6>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    ink_res: Res<Ink>,
    mut rng: ResMut<Rng>,
    mut msg: ResMut<Message>,
) {
    if l6.night_done || clock.minute < NIGHT_MINUTE {
        return;
    }
    l6.night_done = true;
    // The named dead who were not laid to rest...
    let mut risen: Vec<(&'static str, u8, Cast, Guard)> = Vec::new();
    if story.flanm_killed {
        risen.push(("Madanm Flanm", 41, Cast::Flanm, Guard::risen("Madanm Flanm", 41, 50.0, 8.0)));
    }
    if story.tet_woch_killed {
        risen.push(("Tèt Wòch", 38, Cast::Woch, Guard::risen("Tèt Wòch", 38, 50.0, 8.0)));
    }
    if !story.evens_freed {
        risen.push(("Evens", 40, Cast::Manchet, Guard::manchet("Evens", 40)));
    }
    // ...and the horde: ten Brutes at the very most, three in ten of the rest are Yakus, and
    // all the others are prisoners.
    let horde = 24u32;
    let brutes = (horde / 5).min(BRUTE_CAP);
    let yakus = ((horde as f32) * 0.3).round() as u32;
    for i in 0..horde {
        if i < brutes {
            risen.push(("a Brute", 35, Cast::Brute, Guard::risen("a Brute", 35, 12.0, 10.0)));
        } else if i < brutes + yakus {
            risen.push(("a Yaku", 30, Cast::Yaku, Guard::risen("a Yaku", 30, 4.0, 5.0)));
        } else {
            risen.push(("a prisoner", 30, Cast::Prisoner, Guard::risen("a prisoner", 30, 2.0, 1.0)));
        }
    }
    let mut names = Vec::new();
    for (name, _age, cast, mut guard) in risen {
        let at = l6.passages[(rng.next() * l6.passages.len() as f32) as usize % l6.passages.len()];
        let id = spawn_humanoid(&mut commands, &mut meshes, &mut mats, &ink_res, at, 0.0, 0.2, 0.12, 0.3);
        let marker = glow(&mut mats, Vec3::new(1.0, 0.1, 0.1), 7.0);
        let mm = meshes.add(Sphere::new(0.1));
        guard.marker = marker.clone();
        let shambles = guard.zombie;
        commands
            .entity(id)
            .insert((guard, Skin(cast), Mover { grounded: true, zombie: shambles, ..default() }))
            .with_child((Mesh3d(mm), MeshMaterial3d(marker), Transform::from_xyz(0.0, 2.1, 0.0)));
        if !name.starts_with("a ") {
            names.push(name);
        }
    }
    msg.0 = if names.is_empty() {
        "6:00 PM. The house shudders. Doors open somewhere below. Many feet.".to_string()
    } else {
        format!("6:00 PM. The house hums. They rise again: {}. And behind them, a crowd.", names.join(", "))
    };
}

/// Nwit gone to the roof, the stairs open: reaching them ends the level.
pub fn goal6(
    l6: Res<Level6>,
    mut next: ResMut<NextState<AppState>>,
    player: Single<&Transform, With<Player>>,
) {
    let p = player.translation;
    if l6.stairs_open && p.z < -12.6 && p.x > 24.2 {
        next.set(AppState::Complete6);
    }
}
