//! Level 5: the second floor. A maze of rooms and timed spike traps, an
//! armory with the M50, and the vast altar hall. When Ro steps into the hall
//! the swarm closes in and the Sword Man comes out from behind the altar.
//! He is a man who was taken at twelve and has been the gang's blade for
//! twenty-eight years. He cannot be shot down in practice (about a thousand
//! bullets), grenades do nothing, and every wound makes him faster. His blade
//! bleeds soul: one bottle per cut. Bottles are on the altar, or there is a
//! hidden reserve room. The ledger gives his real name; say it and he stops.
//! Breaking the binding at the altar opens the stairs up to Granmèt Nwit.

use crate::ai::{Guard, GuardKind};
use crate::character::{spawn_humanoid, Box3, Boxes, Mover};
use crate::level::Ctx;
use crate::level2::{candle, skull};
use crate::level3::{spawn_lost_crowd, wall_line, Level3, DOOR_W, WALL_H};
use crate::style::{glow, ink, matte, Ink};
use crate::weapons::{Health, Loadout, Rng};
use crate::*;

pub const ARRIVAL: Vec3 = Vec3::new(25.55, 0.0, -10.6); // just outside the stair core door (the core stacks on every floor)
const ARMORY: Vec3 = Vec3::new(-25.0, 0.0, 16.5);
const LEVER: Vec3 = Vec3::new(-20.0, 0.0, -6.5);
const DRUMS: Vec3 = Vec3::new(-27.0, 0.0, -8.0);
const SOURCE_DRINK: Vec3 = Vec3::new(-27.0, 0.0, -17.5);
const SOURCE_SEVER: Vec3 = Vec3::new(-27.0, 0.0, -14.0);
const BINDING: Vec3 = Vec3::new(0.0, 0.0, -8.2);
const SWORD_START: Vec3 = Vec3::new(0.0, 0.0, -15.5);
const RESERVE_DOOR: Vec3 = Vec3::new(-22.0, 1.2, -16.0);
const STAIR_DOOR: Vec3 = Vec3::new(25.55, 1.2, -12.25); // the NE stair core, stacked on every floor
const ROWS: i32 = 3;
const COLS: i32 = 6;

// ------------------------------------------------------------------- state

/// Soul-bleed from the blade: seconds left. Ends with a bottle.
#[derive(Resource, Default)]
pub struct Bleed(pub f32);

#[derive(Resource)]
pub struct Level5 {
    pub knows_name: bool,
    pub swarm: bool,
    pub reserve_open: bool,
    pub tether_cut: bool,
    pub binding: f32,
    pub broken: bool,
    pub wake: f32,
    pub stairs_open: bool,
    pub bottles_found: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PickKind {
    Armory,
    Bottle,
    Ledger,
    Lever,
    SourceDrink,
    SourceSever,
}

#[derive(Component)]
pub struct Pick {
    pub kind: PickKind,
    pub pos: Vec3,
    pub taken: bool,
}

#[derive(Component)]
pub struct Trap {
    pub min: Vec2,
    pub max: Vec2,
    pub phase: f32,
    /// The cycle in which this trap last cut someone (one cut per cycle).
    pub hit_cycle: i64,
    pub spikes: Entity,
    pub plate: Handle<StandardMaterial>,
}

#[derive(Component)]
pub struct Spikes;

#[derive(Component)]
pub struct SwordMan;

// ------------------------------------------------------------------- setup

fn cell_center(r: i32, c: i32) -> Vec3 {
    Vec3::new(-25.0 + 10.0 * c as f32, 0.0, 8.0 * r as f32)
}

/// A random maze (depth-first) plus a few extra doorways so it has loops.
fn gen_maze(rng: &mut Rng) -> Vec<((i32, i32), (i32, i32))> {
    let mut visited = vec![vec![false; COLS as usize]; ROWS as usize];
    let mut edges = Vec::new();
    let mut stack = vec![(ROWS - 1, COLS - 1)];
    visited[(ROWS - 1) as usize][(COLS - 1) as usize] = true;
    while let Some(&(r, c)) = stack.last() {
        let mut next: Vec<(i32, i32)> = [(r - 1, c), (r + 1, c), (r, c - 1), (r, c + 1)]
            .into_iter()
            .filter(|&(a, b)| a >= 0 && a < ROWS && b >= 0 && b < COLS && !visited[a as usize][b as usize])
            .collect();
        if next.is_empty() {
            stack.pop();
            continue;
        }
        let pick = (rng.next() * next.len() as f32) as usize % next.len();
        let n = next.swap_remove(pick);
        visited[n.0 as usize][n.1 as usize] = true;
        edges.push(((r, c), n));
        stack.push(n);
    }
    for _ in 0..5 {
        let r = (rng.next() * ROWS as f32) as i32 % ROWS;
        let c = (rng.next() * (COLS - 1) as f32) as i32 % (COLS - 1);
        edges.push(((r, c), (r, c + 1)));
    }
    edges
}

fn is_open(edges: &[((i32, i32), (i32, i32))], a: (i32, i32), b: (i32, i32)) -> bool {
    edges.iter().any(|&(x, y)| (x == a && y == b) || (x == b && y == a))
}

#[allow(clippy::too_many_arguments)]
pub fn setup_level5(
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
    mut bleed: ResMut<Bleed>,
) {
    let ink_res = ink_res.clone();
    checkpoint.0 = ARRIVAL;
    health.hp = health.hp.max(70.0);
    health.hurt = 0.0;
    bleed.0 = 0.0;
    *objectives = Objectives::default();
    *stats = Stats::default();
    rig.yaw = 0.0;
    rig.first = false;
    msg.0 = "The second floor. Rooms that turn back on themselves, and a drum that will not stop.".into();

    let floor = matte(&mut mats, 0.10);
    let wall = matte(&mut mats, 0.30);
    let stone = matte(&mut mats, 0.22);
    let bone = matte(&mut mats, 0.8);
    let metal = matte(&mut mats, 0.5);
    let wood = matte(&mut mats, 0.2);

    let mut cx = Ctx {
        commands: &mut commands,
        meshes: &mut meshes,
        ink: &ink_res,
        boxes: Vec::new(),
    };
    let plane = cx.meshes.add(Plane3d::default().mesh().size(64.0, 44.0));
    cx.commands.spawn((Mesh3d(plane), MeshMaterial3d(floor), LevelEntity));

    // --- Walls: the outer ring, the maze (z from -4 to 20), the hall wall.
    wall_line(&mut cx, &wall, false, -20.0, -30.0, 30.0, &[]); // north wall
    wall_line(&mut cx, &wall, false, 20.0, -30.0, 30.0, &[]);
    wall_line(&mut cx, &wall, true, -30.0, -20.0, 20.0, &[]);
    wall_line(&mut cx, &wall, true, 30.0, -20.0, 20.0, &[]);
    wall_line(&mut cx, &wall, false, -4.0, -30.0, 30.0, &[-15.0, 15.0]); // maze | hall
    let edges = gen_maze(&mut rng);
    for k in 1..COLS {
        let x = -30.0 + 10.0 * k as f32;
        for r in 0..ROWS {
            let zc = 8.0 * r as f32;
            let doors: &[f32] = if is_open(&edges, (r, k - 1), (r, k)) { &[zc] } else { &[] };
            wall_line(&mut cx, &wall, true, x, zc - 4.0, zc + 4.0, doors);
        }
    }
    for k in 1..ROWS {
        let z = -4.0 + 8.0 * k as f32;
        for c in 0..COLS {
            let xc = -25.0 + 10.0 * c as f32;
            let doors: &[f32] = if is_open(&edges, (k - 1, c), (k, c)) { &[xc] } else { &[] };
            wall_line(&mut cx, &wall, false, z, xc - 5.0, xc + 5.0, doors);
        }
    }
    // The reserve room behind the hall's west end: locked until the lever.
    wall_line(&mut cx, &wall, true, -22.0, -20.0, -12.0, &[-16.0]);
    wall_line(&mut cx, &wall, false, -12.0, -30.0, -22.0, &[]);
    // The NE stair core, stacked over the stair cores of the floors below (its door is the removable block).
    wall_line(&mut cx, &wall, true, 24.0, -20.0, -12.25, &[]);
    wall_line(&mut cx, &wall, false, -12.25, 24.0, 30.0, &[25.55]);

    // --- Maze rooms: candles, bones, a table or two.
    let mut n = 0;
    for r in 0..ROWS {
        for c in 0..COLS {
            let ctr = cell_center(r, c);
            let col = [Vec3::new(1.0, 0.75, 0.35), Vec3::new(1.0, 0.2, 0.15), Vec3::new(0.35, 0.5, 1.0)][n % 3];
            candle(&mut cx, &mut mats, ctr + Vec3::new(-3.8, 0.0, -2.8), col, &mut rng);
            candle(&mut cx, &mut mats, ctr + Vec3::new(3.8, 0.0, 2.8), col, &mut rng);
            if (r + c) % 2 == 1 {
                cx.block(&wood, ctr.x + 1.0, 0.0, ctr.z - 1.0, Vec3::new(2.0, 0.9, 0.9), true);
            }
            for _ in 0..4 {
                let a = rng.next() * 6.28;
                let m = cx.meshes.add(Cuboid::new(0.55, 0.06, 0.07));
                cx.commands.spawn((
                    Mesh3d(m),
                    MeshMaterial3d(bone.clone()),
                    Transform::from_xyz(ctr.x + rng.signed() * 4.0, 0.04, ctr.z + rng.signed() * 3.0)
                        .with_rotation(Quat::from_rotation_y(a)),
                    LevelEntity,
                ));
            }
            n += 1;
        }
    }
    // --- Spike traps in several cells (never the arrival or armory).
    let trap_cells = [(1, 4), (1, 3), (1, 2), (0, 5), (2, 3), (1, 0)];
    let spike_mat = matte(&mut mats, 0.75);
    for (i, &(r, c)) in trap_cells.iter().enumerate() {
        let ctr = cell_center(r, c);
        let plate = mats.add(StandardMaterial {
            base_color: Color::BLACK,
            emissive: LinearRgba::rgb(0.25, 0.02, 0.02),
            ..default()
        });
        let pm = cx.meshes.add(Cuboid::new(4.0, 0.04, 3.0));
        cx.commands.spawn((
            Mesh3d(pm),
            MeshMaterial3d(plate.clone()),
            Transform::from_xyz(ctr.x, 0.03, ctr.z),
            LevelEntity,
        ));
        let sm = cx.meshes.add(Cuboid::new(3.6, 1.0, 2.6));
        let spikes = cx
            .commands
            .spawn((
                Spikes,
                ink(sm, spike_mat.clone(), cx.ink, Transform::from_xyz(ctr.x, 0.0, ctr.z).with_scale(Vec3::new(1.0, 0.02, 1.0)), Vec3::splat(1.05)),
                LevelEntity,
            ))
            .id();
        cx.commands.spawn((
            Trap {
                min: Vec2::new(ctr.x - 2.0, ctr.z - 1.5),
                max: Vec2::new(ctr.x + 2.0, ctr.z + 1.5),
                phase: i as f32 * 1.3,
                hit_cycle: -1,
                spikes,
                plate: plate.clone(),
            },
            LevelEntity,
        ));
    }
    // --- The armory crate with the M50.
    let crate_m = glow(&mut mats, Vec3::new(1.0, 0.8, 0.3), 2.5);
    let am = cx.meshes.add(Cuboid::new(1.4, 0.7, 0.8));
    cx.boxes.push(Box3 { c: Vec3::new(ARMORY.x, 0.35, ARMORY.z - 1.0), h: Vec3::new(0.7, 0.35, 0.4) });
    cx.commands.spawn((
        Pick { kind: PickKind::Armory, pos: ARMORY, taken: false },
        ink(am, crate_m, cx.ink, Transform::from_xyz(ARMORY.x, 0.35, ARMORY.z - 1.0), Vec3::splat(1.08)),
        LevelEntity,
    ));

    // --- The vast altar hall.
    cx.block(&stone, 0.0, 0.0, -12.0, Vec3::new(8.0, 0.8, 3.0), true); // the altar
    cx.block(&stone, 0.0, 0.0, -14.5, Vec3::new(12.0, 1.4, 1.0), true); // its back
    for x in [-12.0_f32, -6.0, 6.0, 12.0] {
        cx.block(&metal, x, 0.0, -9.0, Vec3::new(0.8, 2.6, 0.8), true); // pillars
        cx.block(&metal, x, 0.0, -18.0, Vec3::new(0.8, 2.6, 0.8), true);
    }
    for i in 0..14 {
        let a = i as f32 / 14.0 * std::f32::consts::TAU;
        candle(&mut cx, &mut mats, Vec3::new(a.cos() * 5.0, 0.0, -12.0 + a.sin() * 3.6), Vec3::new(1.0, 0.7, 0.3), &mut rng);
    }
    for x in [-3.0_f32, -1.0, 1.0, 3.0] {
        skull(&mut cx, &mut mats, &bone, Vec3::new(x, 1.45, -14.5));
    }
    // The drums that keep the Lost coming (silence them at the west end).
    for (x, z) in [(-27.0_f32, -8.0_f32), (27.0, -8.0), (21.0, -6.5)] {
        let m = cx.meshes.add(Cylinder::new(0.7, 1.2));
        cx.boxes.push(Box3 { c: Vec3::new(x, 0.6, z), h: Vec3::new(0.7, 0.6, 0.7) });
        cx.commands.spawn((
            ink(m, wood.clone(), cx.ink, Transform::from_xyz(x, 0.6, z), Vec3::new(1.08, 1.04, 1.08)),
            LevelEntity,
        ));
    }
    // Bottles on the altar, the ledger, the lever and the sources.
    let bottle_m = glow(&mut mats, Vec3::new(0.3, 1.0, 0.5), 3.0);
    for i in 0..6 {
        let x = -2.5 + i as f32;
        let m = cx.meshes.add(Cylinder::new(0.09, 0.3));
        cx.commands.spawn((
            Pick { kind: PickKind::Bottle, pos: Vec3::new(x, 0.0, -10.0), taken: false },
            ink(m, bottle_m.clone(), cx.ink, Transform::from_xyz(x, 1.0, -11.0), Vec3::splat(1.2)),
            LevelEntity,
        ));
    }
    let ledger_m = glow(&mut mats, Vec3::new(0.95, 0.9, 0.6), 1.5);
    let lm = cx.meshes.add(Cuboid::new(0.5, 0.06, 0.4));
    cx.commands.spawn((
        Pick { kind: PickKind::Ledger, pos: Vec3::new(4.6, 0.0, -10.0), taken: false },
        Mesh3d(lm),
        MeshMaterial3d(ledger_m),
        Transform::from_xyz(3.8, 0.85, -12.0),
        LevelEntity,
    ));
    let lever_m = glow(&mut mats, Vec3::new(1.0, 0.8, 0.2), 3.0);
    let lvm = cx.meshes.add(Cuboid::new(0.2, 1.0, 0.2));
    cx.commands.spawn((
        Pick { kind: PickKind::Lever, pos: LEVER, taken: false },
        Mesh3d(lvm),
        MeshMaterial3d(lever_m),
        Transform::from_xyz(LEVER.x, 0.5, LEVER.z - 0.6),
        LevelEntity,
    ));
    for (pos, kind, col) in [
        (SOURCE_DRINK, PickKind::SourceDrink, Vec3::new(0.3, 1.0, 0.6)),
        (SOURCE_SEVER, PickKind::SourceSever, Vec3::new(1.0, 0.2, 0.4)),
    ] {
        let m = glow(&mut mats, col, 4.0);
        let sm = cx.meshes.add(Sphere::new(0.3));
        cx.commands.spawn((
            Pick { kind, pos, taken: false },
            ink(sm, m, cx.ink, Transform::from_xyz(pos.x, 1.0, pos.z - 0.8), Vec3::splat(1.15)),
            LevelEntity,
        ));
    }
    // The two removable doors: the reserve room and the stairs (matched by position).
    let door_m = glow(&mut mats, Vec3::new(1.0, 0.15, 0.1), 2.0);
    cx.block(&door_m, RESERVE_DOOR.x, 0.0, RESERVE_DOOR.z, Vec3::new(0.4, WALL_H - 0.2, DOOR_W), true);
    cx.block(&door_m, STAIR_DOOR.x, 0.0, STAIR_DOOR.z, Vec3::new(DOOR_W, WALL_H - 0.2, 0.4), true);

    cx.commands.spawn((
        Sun,
        DirectionalLight {
            illuminance: 450.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.2, 0.4, 0.0)),
        LevelEntity,
    ));
    let boxes = std::mem::take(&mut cx.boxes);
    commands.insert_resource(Boxes(boxes.clone()));
    commands.insert_resource(crate::level3::Grab(0));
    commands.insert_resource(Level5 {
        knows_name: false,
        swarm: false,
        reserve_open: false,
        tether_cut: false,
        binding: 0.0,
        broken: false,
        wake: 0.0,
        stairs_open: false,
        bottles_found: 0,
    });

    // --- Ro.
    let ro = spawn_humanoid(&mut commands, &mut meshes, &mut mats, &ink_res, ARRIVAL, std::f32::consts::PI, 0.62, 0.22, 0.34);
    commands.entity(ro).insert((Player, crate::character::Skin(crate::character::Cast::Hero)));
    crate::weapons::attach_player_weapons(&mut commands, &mut meshes, &mut mats, &ink_res, ro);

    // --- The Lost: about a hundred and fifty, most of them in the hall.
    let rooms: [(f32, f32, f32, f32, usize); 4] = [
        (-29.0, 29.0, -19.0, -5.0, 90),
        (-29.0, 0.0, -3.0, 19.0, 28),
        (0.0, 29.0, -3.0, 19.0, 32),
        (-29.0, -23.0, -19.0, -13.0, 0),
    ];
    let (total, woken_mat) = spawn_lost_crowd(
        &mut commands,
        &mut meshes,
        &mut mats,
        &ink_res,
        &mut rng,
        &boxes,
        &rooms,
        ARRIVAL,
        &|p: Vec3| (p.x < -22.0 && p.z < -12.0) || (p.x.abs() < 6.0 && p.z < -8.0 && p.z > -16.0),
    );
    commands.insert_resource(Level3 {
        music: true, // the drums are playing
        cut_progress: 0.0,
        wake_progress: 0.0,
        stairs_open: false,
        total_lost: total,
        switch: DRUMS,
        switch_prompt: "Hold E: still the drum",
        switch_done: "The drum stops. The Lost stand swaying, no longer coming for you.",
        disco: Vec::new(),
        woken_mat,
    });

    // --- The Sword Man, dormant under the floor until Ro steps into the hall.
    let id = spawn_humanoid(&mut commands, &mut meshes, &mut mats, &ink_res, Vec3::new(0.0, -50.0, 0.0), 0.0, 0.14, 0.1, 0.30);
    commands.entity(id).insert(crate::character::Skin(crate::character::Cast::Manchet));
    let marker = glow(&mut mats, Vec3::splat(0.75), 0.8);
    let mm = meshes.add(Sphere::new(0.1));
    commands
        .entity(id)
        .insert((
            SwordMan,
            Guard {
                stagger: 9999.0, // dormant
                awareness: 1.0,
                state: crate::ai::AiState::Chase,
                marker: marker.clone(),
                ..Guard::manchet("Evens", 40)
            },
            Visibility::Hidden,
        ))
        .with_child((Mesh3d(mm), MeshMaterial3d(marker), Transform::from_xyz(0.0, 2.0, 0.0)));
    let _ = SWORD_START;
}

// ------------------------------------------------------------- the systems

/// Spike traps cycle: dim, warn (red pulse), rise, retract.
pub fn traps5(
    time: Res<Time>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut health: ResMut<Health>,
    mut traps: Query<&mut Trap>,
    mut spikes: Query<&mut Transform, With<Spikes>>,
    player: Single<&Transform, (With<Player>, Without<Spikes>)>,
) {
    let t = time.elapsed_secs();
    let p = player.translation;
    for mut trap in &mut traps {
        let cycle = ((t + trap.phase) / 5.0) as i64;
        let c = (t + trap.phase) % 5.0;
        let (glow_level, up) = if c < 2.5 {
            (0.25, 0.0)
        } else if c < 3.5 {
            (1.0 + 3.0 * ((c - 2.5) * 10.0).sin().abs(), 0.0) // warning
        } else if c < 4.3 {
            (4.0, 1.0)
        } else {
            (0.4, 1.0 - (c - 4.3) / 0.7)
        };
        if let Some(mut m) = mats.get_mut(&trap.plate) {
            m.emissive = LinearRgba::rgb(glow_level, 0.05 * glow_level, 0.05 * glow_level);
        }
        if let Ok(mut tf) = spikes.get_mut(trap.spikes) {
            tf.scale.y = (up * 0.9).max(0.02);
            tf.translation.y = tf.scale.y * 0.5;
        }
        let inside = p.x > trap.min.x && p.x < trap.max.x && p.z > trap.min.y && p.z < trap.max.y;
        if inside && up > 0.7 && p.y < 0.7 && trap.hit_cycle != cycle {
            // One cut per cycle; jump to clear the spikes.
            trap.hit_cycle = cycle;
            health.hp -= 30.0;
            health.hurt = 1.0;
        }
    }
}

pub fn bleed_tick(time: Res<Time>, mut bleed: ResMut<Bleed>, mut health: ResMut<Health>) {
    if bleed.0 > 0.0 {
        let dt = time.delta_secs().min(0.1);
        bleed.0 = (bleed.0 - dt).max(0.0);
        health.hp -= 7.0 * dt;
        health.hurt = health.hurt.max(0.2);
    }
}

#[allow(clippy::too_many_arguments)]
pub fn sword5(
    mut l5: ResMut<Level5>,
    mut msg: ResMut<Message>,
    mut sword: Single<(&mut Guard, &mut Transform, &mut Visibility), With<SwordMan>>,
    player: Single<&Transform, (With<Player>, Without<SwordMan>)>,
) {
    let (g, tf, vis) = &mut *sword;
    if !l5.swarm && player.translation.z < -5.0 {
        // The hall: the swarm converges, and the Sword Man comes out from behind the altar.
        l5.swarm = true;
        g.stagger = 0.0;
        g.last_known = player.translation;
        tf.translation = SWORD_START;
        **vis = Visibility::Inherited;
        msg.0 = "The hall. Faces from every side, all at once, and from behind the altar, something with a blade and no hurry.".into();
    }
    // Faster as he is hurt; slower if the tether to Granmèt Nwit is cut.
    if g.status == LostStatus::Standing {
        let growth = 1.0 + 0.8 * (1.0 - (g.hp / g.max_hp).clamp(0.0, 1.0));
        g.speed_mul = growth * if l5.tether_cut { 0.65 } else { 1.0 } * if l5.broken { 1.2 } else { 1.0 };
    }
}

#[allow(clippy::too_many_arguments)]
pub fn interact5(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut prompt: ResMut<Prompt>,
    mut msg: ResMut<Message>,
    mut poison: ResMut<Poison>,
    mut health: ResMut<Health>,
    mut weight: ResMut<Weight>,
    mut loadout: ResMut<Loadout>,
    mut stats: ResMut<Stats>,
    mut l5: ResMut<Level5>,
    mut story: ResMut<crate::level3::Story>,
    mut boxes: ResMut<Boxes>,
    mut picks: Query<(&mut Pick, &mut Visibility)>,
    mut sword: Single<(&mut Guard, &Transform), (With<SwordMan>, Without<Player>)>,
    player: Single<(&Transform, &mut Mover), With<Player>>,
) {
    let dt = time.delta_secs().min(0.1);
    let (pt, mut pm) = player.into_inner();
    let pp = pt.translation;
    let flat = |p: Vec3| Vec2::new(p.x - pp.x, p.z - pp.z).length();
    let e = keys.just_pressed(KeyCode::KeyE);
    let holding = keys.pressed(KeyCode::KeyE);

    // --- Hold E at the altar: break the binding.
    if !l5.broken && flat(BINDING) < 1.6 {
        prompt.0 = format!("Hold E: break the binding ({:.0}%)", l5.binding * 100.0);
        if holding {
            l5.binding += dt / 3.0;
            pm.noise = pm.noise.max(3.0);
            if l5.binding >= 1.0 {
                l5.broken = true;
                l5.stairs_open = true;
                boxes.0.retain(|b| Vec2::new(b.c.x - STAIR_DOOR.x, b.c.z - STAIR_DOOR.z).length() > 0.3);
                msg.0 = "The candles gutter. Something above, in the dark, turns to listen. The stairs are open.".into();
            }
        } else {
            l5.binding = (l5.binding - dt * 0.8).max(0.0);
        }
        return;
    }

    // --- The Sword Man: with his name, hold E beside him.
    let (g, gtf) = &mut *sword;
    if l5.knows_name && g.status == LostStatus::Standing && flat(gtf.translation) < 2.2 && l5.swarm {
        prompt.0 = format!("Hold E: say \"Evens\" ({:.0}%)", l5.wake * 100.0);
        if holding {
            l5.wake += dt / 1.5;
            if l5.wake >= 1.0 {
                g.status = LostStatus::Woken;
                story.evens_freed = true;
                weight.0 = (weight.0 - 25.0).max(0.0);
                stats.woken += 1;
                msg.0 = "You say \"Evens.\" The blade drops. He looks at his hands, and says in a boy's voice: \"Mwen te gen douz an.\" (I was twelve.)".into();
            }
        } else {
            l5.wake = (l5.wake - dt * 0.6).max(0.0);
        }
        return;
    }

    // --- Pickups: the nearest one within reach.
    let mut best: Option<(f32, Mut<Pick>, Mut<Visibility>)> = None;
    for (p, v) in &mut picks {
        if p.taken {
            continue;
        }
        let d = flat(p.pos);
        let reach = if p.kind == PickKind::Armory { 2.4 } else { 1.7 };
        if d < reach && best.as_ref().is_none_or(|(b, _, _)| d < *b) {
            best = Some((d, p, v));
        }
    }
    let Some((_, mut pick, mut vis)) = best else { return };
    match pick.kind {
        PickKind::Armory => prompt.0 = "E: take the M50 (heavy: it slows you, and it is loud)".into(),
        PickKind::Bottle => prompt.0 = "E: take a bottle".into(),
        PickKind::Ledger => prompt.0 = "E: read the ledger".into(),
        PickKind::Lever if !l5.reserve_open => prompt.0 = "E: pull the lever".into(),
        PickKind::SourceDrink => prompt.0 = "E: drink from the source (unlimited health)".into(),
        PickKind::SourceSever => prompt.0 = "E: sever the tether to Granmèt Nwit (the Sword Man slows)".into(),
        _ => {}
    }
    if !e {
        return;
    }
    match pick.kind {
        PickKind::Armory => {
            loadout.has_m50 = true;
            pick.taken = true;
            *vis = Visibility::Hidden;
            msg.0 = "The M50, belt-fed, five hundred rounds. It will slow you down and wake the whole floor. (Press 5.)".into();
        }
        PickKind::Bottle => {
            pick.taken = true;
            *vis = Visibility::Hidden;
            poison.bottles += 1;
            l5.bottles_found += 1;
            msg.0 = format!("A bottle: {} in all. One for every cut of the blade. (H)", poison.bottles);
        }
        PickKind::Ledger => {
            pick.taken = true;
            l5.knows_name = true;
            msg.0 = "A ledger, in a careful hand: \"Evens Joseph. Taken at twelve, from a village by the river. The first of the Brigad's blades. Twenty-eight years.\" Maybe a name still works on him.".into();
        }
        PickKind::Lever => {
            pick.taken = true;
            l5.reserve_open = true;
            boxes.0.retain(|b| (b.c - Vec3::new(RESERVE_DOOR.x, 0.0, RESERVE_DOOR.z)).length() > 1.5);
            msg.0 = "A grinding from the west end of the hall. A hidden door has opened onto the reserve room.".into();
        }
        PickKind::SourceDrink => {
            pick.taken = true;
            *vis = Visibility::Hidden;
            poison.bottles = 999;
            health.hp = 100.0;
            weight.0 = (weight.0 + 25.0).min(100.0);
            msg.0 = "You drink. The wounds close, and your bottle will never empty. The Weight settles on you: this was not made for you.".into();
        }
        PickKind::SourceSever => {
            pick.taken = true;
            *vis = Visibility::Hidden;
            l5.tether_cut = true;
            weight.0 = (weight.0 + 10.0).min(100.0);
            msg.0 = "You cut something that hums. Far above, Granmèt Nwit screams. The Sword Man will move like a man in water.".into();
        }
    }
}

pub fn goal5(
    l5: Res<Level5>,
    mut next: ResMut<NextState<AppState>>,
    player: Single<&Transform, With<Player>>,
) {
    let p = player.translation;
    if l5.stairs_open && p.z < -12.6 && p.x > 24.2 {
        next.set(AppState::Complete5);
    }
}
