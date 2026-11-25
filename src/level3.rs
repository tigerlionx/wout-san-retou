//! Level 3: the party wing, ground floor. Pool hall, gym, locker room, bar,
//! veranda and a ballroom with a DJ booth. The party is the bait. Madanm
//! Flanm, the Candle Brigade's recruiter, guards the stairs with a rifle.
//! About fifty "Lost" (kids who were recruited and lost themselves) drift
//! toward Ro in a trance. They are never shot: say a name to wake one, or cut
//! the music at the DJ booth so they stop coming. Gifts on the bar are free,
//! except that nothing is free.

use crate::ai::{Guard, GuardKind};
use crate::character::{spawn_humanoid, resolve, Box3, Boxes, Mover};
use crate::level::Ctx;
use crate::style::{glow, grow_box, ink, matte, Ink};
use crate::weapons::{Health, Loadout, Rng};
use crate::*;

pub const WALL_H: f32 = 2.8;
pub const DOOR_W: f32 = 2.6;

pub const ARRIVAL: Vec3 = Vec3::new(-27.0, 0.0, 16.0);
const BOOTH: Vec3 = Vec3::new(17.0, 0.0, -16.6);

// ------------------------------------------------------------------- state

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LostState {
    /// Walking toward Ro in a trance.
    Tranced,
    /// Woken by name: stands still, free.
    Woken,
}

#[derive(Component)]
pub struct Lost {
    pub name: &'static str,
    pub age: u8,
    pub state: LostState,
    pub phase: f32,
}

#[derive(Component)]
pub struct Gift {
    pub pos: Vec3,
    pub taken: bool,
}

#[derive(Resource)]
pub struct Level3 {
    pub music: bool,
    pub cut_progress: f32,
    pub wake_progress: f32,
    pub stairs_open: bool,
    pub total_lost: usize,
    /// Where to hold E to calm the Lost (the DJ booth here, a breaker in level 4).
    pub switch: Vec3,
    pub switch_prompt: &'static str,
    pub switch_done: &'static str,
    pub disco: Vec<Handle<StandardMaterial>>,
    pub woken_mat: Handle<StandardMaterial>,
}

/// A shallow pool: slows and splashes.
#[derive(Resource)]
pub struct Water {
    pub min: Vec2,
    pub max: Vec2,
    /// Only wade below this height (a pit in level 4; always in level 3).
    pub y_max: f32,
}

/// What the player remembers from earlier levels.
#[derive(Resource, Default)]
pub struct Story {
    pub flanm_killed: bool,
    pub tet_woch_killed: bool,
    pub evens_freed: bool,
}

/// How many Lost are clinging to Ro this frame.
#[derive(Resource, Default)]
pub struct Grab(pub u32);

pub const NAMES: [&str; 50] = [
    "Wilfrid", "Natacha", "Kervens", "Mirlande", "Jonel", "Stephanie", "Dieunel", "Rachelle",
    "Samuel", "Fedline", "Benito", "Marjorie", "Ludner", "Cassandre", "Evens", "Nadège",
    "Johnny", "Widlyne", "Roosevelt", "Esther", "Mackendy", "Sherley", "Peterson", "Guerline",
    "Daniel", "Flore", "Yvener", "Carline", "Moïse", "Tamara", "Jean-Marc", "Vanessa",
    "Rodrigue", "Eunice", "Fritz", "Ketia", "Alix", "Betina", "Junior", "Chrislene",
    "Gaby", "Mislène", "Frantzy", "Lovely", "Wedson", "Nathalie", "Pierrot", "Sandra",
    "Emmanuel", "Ruth",
];

// ------------------------------------------------------------------- setup

pub fn wall_line(cx: &mut Ctx, mat: &Handle<StandardMaterial>, vertical: bool, fixed: f32, a: f32, b: f32, doors: &[f32]) {
    let mut cuts = doors.to_vec();
    cuts.sort_by(|x, y| x.total_cmp(y));
    let mut start = a;
    let mut segs: Vec<(f32, f32)> = Vec::new();
    for d in cuts {
        let end = d - DOOR_W / 2.0;
        if end > start + 0.05 {
            segs.push((start, end));
        }
        start = d + DOOR_W / 2.0;
    }
    if b > start + 0.05 {
        segs.push((start, b));
    }
    for (s, e) in segs {
        let c = (s + e) / 2.0;
        let len = e - s;
        if vertical {
            cx.block(mat, fixed, 0.0, c, Vec3::new(0.5, WALL_H, len), true);
        } else {
            cx.block(mat, c, 0.0, fixed, Vec3::new(len, WALL_H, 0.5), true);
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn setup_level3(
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
    mut loadout: ResMut<Loadout>,
) {
    let ink_res = ink_res.clone();
    checkpoint.0 = ARRIVAL;
    msg.0 = "Music, laughter, and the smell of chlorine. Everyone down here is smiling the wrong way.".into();
    health.hp = health.hp.max(60.0);
    health.hurt = 0.0;
    poison.active = false;
    poison.drugged = 0.0;
    *objectives = Objectives::default();
    *stats = Stats::default();
    rig.yaw = 0.0;
    rig.first = false;
    let _ = &mut loadout;

    let floor = matte(&mut mats, 0.12);
    let deck = matte(&mut mats, 0.20);
    let wall = matte(&mut mats, 0.34);
    let wood = matte(&mut mats, 0.22);
    let fabric = matte(&mut mats, 0.42);
    let metal = matte(&mut mats, 0.55);
    let locker = matte(&mut mats, 0.40);
    let water_m = mats.add(StandardMaterial {
        base_color: Color::srgb(0.1, 0.35, 0.5),
        emissive: LinearRgba::rgb(0.05, 0.6, 1.0),
        perceptual_roughness: 0.2,
        ..default()
    });

    let mut cx = Ctx {
        commands: &mut commands,
        meshes: &mut meshes,
        ink: &ink_res,
        boxes: Vec::new(),
    };

    // Floor.
    let plane = cx.meshes.add(Plane3d::default().mesh().size(64.0, 44.0));
    cx.commands.spawn((Mesh3d(plane), MeshMaterial3d(floor), LevelEntity));
    let deck_plane = cx.meshes.add(Plane3d::default().mesh().size(24.0, 40.0));
    cx.commands.spawn((
        Mesh3d(deck_plane),
        MeshMaterial3d(deck),
        Transform::from_xyz(0.0, 0.01, 0.0),
        LevelEntity,
    ));

    // --- Walls and doorways.
    wall_line(&mut cx, &wall, false, -20.0, -30.0, 30.0, &[]);
    wall_line(&mut cx, &wall, false, 20.0, -30.0, 30.0, &[]);
    wall_line(&mut cx, &wall, true, -30.0, -20.0, 20.0, &[]);
    wall_line(&mut cx, &wall, true, 30.0, -20.0, 20.0, &[]);
    // The pool hall is the full-depth central hall (x -12..12): the same walls carry the first-floor gallery.
    wall_line(&mut cx, &wall, true, -12.0, -20.0, 20.0, &[10.0, -14.0]); // west rooms | pool hall
    wall_line(&mut cx, &wall, false, 0.0, -30.0, -12.0, &[-24.0]); // locker | gym
    wall_line(&mut cx, &wall, true, 12.0, -20.0, 20.0, &[-14.0, -2.0, 14.0]); // pool hall | ballroom, veranda
    wall_line(&mut cx, &wall, false, 8.0, 12.0, 30.0, &[15.0, 24.0]); // ballroom | veranda
    // The stair core in the north-east corner (stacks on every floor), sealed while Madanm Flanm stands.
    wall_line(&mut cx, &wall, true, 24.0, -20.0, -12.25, &[]);
    wall_line(&mut cx, &wall, false, -12.25, 24.0, 30.0, &[25.55]);

    // --- Locker room.
    for i in 0..6 {
        cx.block(&locker, -28.0 + i as f32 * 1.7, 0.0, -19.4, Vec3::new(1.4, 2.0, 0.7), true);
    }
    cx.block(&wood, -24.0, 0.0, -10.0, Vec3::new(4.0, 0.5, 0.8), true);
    cx.block(&wood, -24.0, 0.0, -6.0, Vec3::new(4.0, 0.5, 0.8), true);
    // --- Gym: machines and a bench. The stairs come down in the south-west corner.
    for z in [4.0_f32, 8.0, 12.0] {
        cx.block(&metal, -27.5, 0.0, z, Vec3::new(1.0, 1.3, 2.0), true);
    }
    cx.block(&metal, -22.0, 0.0, 6.0, Vec3::new(2.2, 0.5, 0.9), true);
    cx.block(&fabric, -28.0, 0.0, 19.0, Vec3::new(3.0, 0.3, 2.0), false); // the foot of the stairs
    // --- Pool: shallow water you can wade through (slow and noisy).
    // 18 x 24 m, exactly under the first floor's glass floor (level 4).
    let pool_min = Vec2::new(-9.0, -12.0);
    let pool_max = Vec2::new(9.0, 12.0);
    let pool = cx.meshes.add(Cuboid::new(18.0, 0.04, 24.0));
    cx.commands.spawn((
        Mesh3d(pool),
        MeshMaterial3d(water_m),
        Transform::from_xyz(0.0, 0.03, 0.0),
        LevelEntity,
    ));
    let pool_light = Color::srgb(0.2, 0.7, 1.0);
    for (x, z) in [(-4.0, -5.0), (4.0, 5.0)] {
        cx.commands.spawn((
            PointLight {
                color: pool_light,
                intensity: 900_000.0,
                range: 14.0,
                ..default()
            },
            Transform::from_xyz(x, 0.6, z),
            LevelEntity,
        ));
    }
    for (x, z) in [(-9.5, 17.2), (-6.0, 17.2), (-2.0, 17.2), (2.0, 17.2), (6.0, 17.2), (9.5, 17.2)] {
        cx.block(&fabric, x, 0.0, z, Vec3::new(0.8, 0.4, 2.0), true); // loungers
    }
    // --- Bar: counter, stools, bottles, gifts.
    // The bar now stands at the north end of the pool hall.
    cx.block(&wood, 0.0, 0.0, -18.0, Vec3::new(14.0, 1.1, 1.0), true);
    cx.block(&wood, 0.0, 0.0, -19.55, Vec3::new(14.0, 2.2, 0.6), false); // back shelf
    for i in 0..6 {
        let m = cx.meshes.add(Cylinder::new(0.22, 0.6));
        cx.boxes.push(Box3 { c: Vec3::new(-3.5 + i as f32 * 1.4, 0.3, -16.6), h: Vec3::new(0.22, 0.3, 0.22) });
        cx.commands.spawn((
            ink(m, fabric.clone(), cx.ink, Transform::from_xyz(-3.5 + i as f32 * 1.4, 0.3, -16.6), Vec3::splat(1.15)),
            LevelEntity,
        ));
    }
    let pink = glow(&mut mats, Vec3::new(1.0, 0.2, 0.6), 7.0);
    let cyan = glow(&mut mats, Vec3::new(0.2, 0.9, 1.0), 7.0);
    cx.block(&pink, -8.0, 1.6, -19.0, Vec3::new(2.4, 0.18, 0.1), false);
    cx.block(&cyan, -2.0, 1.6, -19.0, Vec3::new(2.4, 0.18, 0.1), false);
    for (c, x) in [(Vec3::new(1.0, 0.2, 0.6), -8.0_f32), (Vec3::new(0.2, 0.9, 1.0), -2.0)] {
        cx.commands.spawn((
            PointLight {
                color: Color::srgb(c.x, c.y, c.z),
                intensity: 700_000.0,
                range: 10.0,
                ..default()
            },
            Transform::from_xyz(x, 1.8, -17.5),
            LevelEntity,
        ));
    }
    let gift_m = glow(&mut mats, Vec3::new(1.0, 0.8, 0.3), 3.0);
    for x in [-6.5_f32, -3.5] {
        let m = cx.meshes.add(Cuboid::new(0.5, 0.4, 0.4));
        cx.commands.spawn((
            Gift { pos: Vec3::new(x, 0.0, -16.0), taken: false },
            ink(m, gift_m.clone(), cx.ink, Transform::from_xyz(x, 1.3, -18.0), Vec3::splat(1.12)),
            LevelEntity,
        ));
    }
    // --- Ballroom: a glowing dance floor, the DJ booth, speakers, sofas.
    let mut disco: Vec<Handle<StandardMaterial>> = Vec::new();
    for _ in 0..6 {
        disco.push(mats.add(StandardMaterial {
            base_color: Color::BLACK,
            emissive: LinearRgba::rgb(1.0, 0.2, 0.5),
            ..default()
        }));
    }
    let tile = cx.meshes.add(Cuboid::new(2.0, 0.04, 2.0));
    for i in 0..6 {
        for j in 0..6 {
            cx.commands.spawn((
                Mesh3d(tile.clone()),
                MeshMaterial3d(disco[(i + j * 2) % 6].clone()),
                Transform::from_xyz(12.5 + i as f32 * 2.1, 0.03, -8.5 + j as f32 * 2.1 - 3.0),
                LevelEntity,
            ));
        }
    }
    cx.block(&metal, 17.0, 0.0, -19.0, Vec3::new(6.0, 1.2, 1.6), true); // DJ booth
    cx.block(&wood, 11.5, 0.0, -18.8, Vec3::new(1.2, 2.2, 1.2), true); // speakers
    cx.block(&wood, 22.5, 0.0, -18.8, Vec3::new(1.2, 2.2, 1.2), true);
    cx.block(&fabric, 28.0, 0.0, -6.0, Vec3::new(1.2, 0.8, 3.0), true); // sofas
    cx.block(&fabric, 28.0, 0.0, -1.0, Vec3::new(1.2, 0.8, 3.0), true);
    cx.block(&fabric, 11.0, 0.0, 2.0, Vec3::new(3.0, 0.8, 1.2), true);
    let ball = cx.meshes.add(Sphere::new(0.5));
    let ball_m = glow(&mut mats, Vec3::new(0.9, 0.9, 1.0), 3.0);
    cx.commands.spawn((
        Mesh3d(ball),
        MeshMaterial3d(ball_m),
        Transform::from_xyz(19.0, 3.4, -6.0),
        LevelEntity,
    ));
    for (c, p) in [
        (Vec3::new(1.0, 0.2, 0.5), Vec3::new(14.0, 3.0, -10.0)),
        (Vec3::new(0.2, 1.0, 0.6), Vec3::new(24.0, 3.0, -10.0)),
        (Vec3::new(0.3, 0.5, 1.0), Vec3::new(19.0, 3.0, -2.0)),
    ] {
        cx.commands.spawn((
            PointLight {
                color: Color::srgb(c.x, c.y, c.z),
                intensity: 2_500_000.0,
                range: 18.0,
                ..default()
            },
            Transform::from_translation(p),
            LevelEntity,
        ));
    }
    // --- Veranda: planters and string lights.
    for x in [12.0_f32, 18.0, 26.0] {
        cx.block(&wood, x, 0.0, 18.5, Vec3::new(2.0, 0.8, 0.8), true);
    }
    for (i, x) in [10.0_f32, 14.0, 18.0, 22.0, 26.0].iter().enumerate() {
        let col = [Vec3::new(1.0, 0.8, 0.2), Vec3::new(0.3, 1.0, 0.5), Vec3::new(1.0, 0.3, 0.3)][i % 3];
        let m = glow(&mut mats, col, 6.0);
        let s = cx.meshes.add(Sphere::new(0.1));
        cx.commands.spawn((Mesh3d(s), MeshMaterial3d(m), Transform::from_xyz(*x, 2.7, 7.0), LevelEntity));
    }

    // --- The sealed stair door (the last box pushed).
    let stair_mat = glow(&mut mats, Vec3::new(1.0, 0.15, 0.1), 2.0);
    cx.block(&stair_mat, 25.55, 0.0, -12.25, Vec3::new(DOOR_W, WALL_H - 0.2, 0.4), true);

    // --- Light: dim, the party lights do the work.
    cx.commands.spawn((
        Sun,
        DirectionalLight {
            illuminance: 650.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.2, 0.4, 0.0)),
        LevelEntity,
    ));

    let boxes = std::mem::take(&mut cx.boxes);
    commands.insert_resource(Boxes(boxes.clone()));
    commands.insert_resource(Water { min: pool_min, max: pool_max, y_max: 1.0 });
    commands.insert_resource(Grab(0));

    // --- Ro.
    let ro = spawn_humanoid(&mut commands, &mut meshes, &mut mats, &ink_res, ARRIVAL, std::f32::consts::FRAC_PI_2, 0.62, 0.22, 0.34);
    commands.entity(ro).insert((Player, crate::character::Skin(crate::character::Cast::Hero)));
    crate::weapons::attach_player_weapons(&mut commands, &mut meshes, &mut mats, &ink_res, ro);

    // --- The Lost: fifty kids, each with a name and an age.
    let rooms: [(f32, f32, f32, f32, usize); 5] = [
        (-29.0, -13.0, -19.0, -1.0, 6),   // locker room
        (-29.0, -13.0, 1.0, 19.0, 8),     // gym
        (-11.0, 11.0, -19.0, 19.0, 20),   // pool hall and bar
        (13.0, 29.0, -11.0, 7.0, 12),     // ballroom
        (13.0, 29.0, 9.0, 19.0, 4),       // veranda
    ];
    let (idx, woken_mat) = spawn_lost_crowd(&mut commands, &mut meshes, &mut mats, &ink_res, &mut rng, &boxes, &rooms, ARRIVAL, &|p: Vec3| p.x > 4.0 && p.z < -16.0 && p.x < 24.0);

    // --- The adults: three enforcers and Madanm Flanm at the DJ booth.
    let walkers: [(&str, u8, f32, f32, Vec<Vec3>); 3] = [
        ("Wilner", 28, 0.30, 0.15, vec![Vec3::new(-10.0, 0.0, 15.0), Vec3::new(10.0, 0.0, 15.0)]),
        ("Fabrice", 25, 0.50, 0.20, vec![Vec3::new(-10.0, 0.0, -14.0), Vec3::new(10.0, 0.0, -14.0), Vec3::new(18.0, 0.0, -8.0)]),
        ("Stanley", 30, 0.40, 0.12, vec![Vec3::new(12.0, 0.0, 12.0), Vec3::new(27.0, 0.0, 12.0)]),
    ];
    for (name, age, shirt, pants, route) in walkers {
        let first = route[0];
        let d = route[1] - first;
        let yaw = (-d.x).atan2(-d.z);
        let id = spawn_humanoid(&mut commands, &mut meshes, &mut mats, &ink_res, first, yaw, shirt, pants, 0.36);
        crate::weapons::attach_guard_pistol(&mut commands, &mut meshes, &mut mats, &ink_res, id);
        let marker = glow(&mut mats, Vec3::splat(0.75), 0.8);
        let mm = meshes.add(Sphere::new(0.09));
        commands.entity(id).insert(Guard {
            route,
            wp: 1,
            home_yaw: yaw,
            fall_yaw: yaw,
            last_known: first,
            marker: marker.clone(),
            ..Guard::base(name, age, GuardKind::Walker)
        }).with_child((Mesh3d(mm), MeshMaterial3d(marker), Transform::from_xyz(0.0, 1.85, 0.0)));
    }
    let fl_yaw = std::f32::consts::PI;
    let id = spawn_humanoid(&mut commands, &mut meshes, &mut mats, &ink_res, Vec3::new(19.0, 0.0, -14.5), fl_yaw, 0.82, 0.18, 0.40);
    commands.entity(id).insert(crate::character::Skin(crate::character::Cast::Flanm));
    crate::weapons::attach_guard_pistol(&mut commands, &mut meshes, &mut mats, &ink_res, id);
    let marker = glow(&mut mats, Vec3::splat(0.75), 0.8);
    let mm = meshes.add(Sphere::new(0.1));
    commands.entity(id).insert(Guard {
        route: vec![Vec3::new(19.0, 0.0, -14.5)],
        wp: 0,
        home_yaw: fl_yaw,
        fall_yaw: fl_yaw,
        last_known: Vec3::new(19.0, 0.0, -14.5),
        marker: marker.clone(),
        ..Guard::elizabeth()
    }).with_child((Mesh3d(mm), MeshMaterial3d(marker), Transform::from_xyz(0.0, 1.9, 0.0)));

    commands.insert_resource(Level3 {
        music: true,
        cut_progress: 0.0,
        wake_progress: 0.0,
        stairs_open: false,
        total_lost: idx,
        switch: BOOTH,
        switch_prompt: "Hold E: cut the music",
        switch_done: "The music dies. The dancing stops. The Lost stand swaying, no longer coming for you.",
        disco,
        woken_mat,
    });
    let _ = grow_box;
}

/// Spawn a crowd of tranced Lost in the given rooms (x0, x1, z0, z1, count),
/// away from `spawn` and from furniture. Returns how many and the "woken" material.
#[allow(clippy::too_many_arguments)]
pub fn spawn_lost_crowd(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mats: &mut Assets<StandardMaterial>,
    ink_res: &Ink,
    rng: &mut Rng,
    boxes: &[Box3],
    rooms: &[(f32, f32, f32, f32, usize)],
    spawn: Vec3,
    keep_out: &dyn Fn(Vec3) -> bool,
) -> (usize, Handle<StandardMaterial>) {
    let woken_mat = matte(mats, 0.30);
    let _ = (&meshes, ink_res);
    let mut idx = 0;
    for &(x0, x1, z0, z1, n) in rooms {
        let mut placed = 0;
        let mut tries = 0;
        while placed < n && tries < 600 {
            tries += 1;
            let p = Vec3::new(x0 + rng.next() * (x1 - x0), 0.0, z0 + rng.next() * (z1 - z0));
            // Not on top of furniture, not near the arrival stairs.
            if resolve(p, 0.4, boxes).distance(p) > 0.01 || p.distance(spawn) < 9.0 || keep_out(p) {
                continue;
            }
            let name = NAMES[idx % NAMES.len()];
            let age = 13 + ((idx * 7) % 8) as u8; // 13 to 20
            // The tranced are the dead-eyed: prisoner and Yaku bodies (7 in 10 and 3 in 10), a little smaller (they are
            // teenagers), shambling in the trance (the zombie clips) until woken by name.
            let cast = if idx % 10 < 3 { crate::character::Cast::Yaku } else { crate::character::Cast::Prisoner };
            commands.spawn((
                Lost {
                    name,
                    age,
                    state: LostState::Tranced,
                    phase: rng.next() * 6.28,
                },
                Transform::from_translation(p).with_scale(Vec3::splat(0.90 + 0.01 * (age - 13) as f32)),
                Visibility::default(),
                Mover { grounded: true, zombie: true, ..default() },
                crate::character::Skin(cast),
                LevelEntity,
            ));
            idx += 1;
            placed += 1;
        }
    }
    (idx, woken_mat)
}

// ----------------------------------------------------------------- the Lost

/// The Lost shamble toward Ro while the music plays, and cling when they arrive.
pub fn lost_ai(
    time: Res<Time>,
    l3: Res<Level3>,
    boxes: Res<Boxes>,
    mut grab: ResMut<Grab>,
    mut health: ResMut<Health>,
    glass: Option<Res<crate::level4::GlassRect>>,
    mut lost: Query<(Entity, &mut Lost, &mut Transform, &mut Mover, &mut Visibility), Without<Player>>,
    mut commands: Commands,
    player: Single<&Transform, (With<Player>, Without<Lost>)>,
) {
    let dt = time.delta_secs().min(0.1);
    let pp = player.translation;
    let mut clinging = 0u32;
    for (e, l, mut tf, mut mv, mut vis) in &mut lost {
        // Far from Ro: hidden and frozen (cheap); they still count and still wake.
        let far = Vec2::new(pp.x - tf.translation.x, pp.z - tf.translation.z).length() > 9.0;
        if far {
            *vis = Visibility::Hidden;
            commands.entity(e).insert(crate::character::Dormant);
            continue;
        }
        *vis = Visibility::Inherited;
        commands.entity(e).remove::<crate::character::Dormant>();
        mv.speed = 0.0;
        if l.state != LostState::Tranced {
            mv.zombie = false; // woken: they stand like anyone else
            continue;
        }
        let to = Vec3::new(pp.x - tf.translation.x, 0.0, pp.z - tf.translation.z);
        let d = to.length();
        if d < 0.95 {
            clinging += 1;
        }
        if l3.music && d < 15.0 && d > 0.8 {
            let dir = to / d;
            let before = tf.translation;
            let mut next = before + dir * 1.35 * dt;
            next = resolve(Vec3::new(next.x, 0.0, next.z), 0.35, &boxes.0);
            // The Lost will not step onto glass.
            if glass.as_ref().is_some_and(|g| next.x > g.min.x - 0.4 && next.x < g.max.x + 0.4 && next.z > g.min.y - 0.4 && next.z < g.max.y + 0.4) {
                next = before;
            }
            tf.translation = Vec3::new(next.x, before.y, next.z);
            mv.speed = 1.35;
            let yaw = (-dir.x).atan2(-dir.z);
            tf.rotation = Quat::from_rotation_y(yaw);
        } else if d < 15.0 && d > 0.8 {
            // The music is off: they only turn to watch.
            let dir = to / d;
            tf.rotation = Quat::from_rotation_y((-dir.x).atan2(-dir.z));
        }
    }
    grab.0 = clinging;
    if clinging > 0 {
        // Clinging hands: a slow drain, never a kill on its own.
        let drain = 1.6 * clinging.min(4) as f32 * dt;
        if health.hp > 12.0 {
            health.hp = (health.hp - drain).max(12.0);
        }
        health.hurt = health.hurt.max(0.15);
    }
}

#[allow(clippy::too_many_arguments)]
pub fn interact3(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut prompt: ResMut<Prompt>,
    mut msg: ResMut<Message>,
    mut l3: ResMut<Level3>,
    mut weight: ResMut<Weight>,
    mut stats: ResMut<Stats>,
    mut loadout: ResMut<Loadout>,
    mut gifts: Query<(&mut Gift, &mut Visibility)>,
    mut lost: Query<(&mut Lost, &Transform), Without<Player>>,
    player: Single<(&Transform, &mut Mover), With<Player>>,
) {
    let dt = time.delta_secs().min(0.1);
    let (pt, mut pm) = player.into_inner();
    let pp = pt.translation;
    prompt.0.clear();
    let holding = keys.pressed(KeyCode::KeyE);
    let flat = |p: Vec3| Vec2::new(p.x - pp.x, p.z - pp.z).length();

    // --- Gifts: free things, with a price.
    for (mut g, mut vis) in &mut gifts {
        if g.taken || flat(g.pos) > 1.8 {
            continue;
        }
        prompt.0 = "E: take the gift (a gift from Madanm Flanm)".into();
        if keys.just_pressed(KeyCode::KeyE) {
            g.taken = true;
            *vis = Visibility::Hidden;
            loadout.ak_reserve += 30;
            loadout.pistol_reserve += 15;
            weight.0 = (weight.0 + 10.0).min(100.0);
            msg.0 = "Ammo, free. You feel the Weight settle on you anyway: nothing here is free.".into();
        }
    }

    // --- The DJ booth: hold E to cut the music.
    if l3.music && flat(l3.switch) < 2.4 {
        prompt.0 = format!("{} ({:.0}%)", l3.switch_prompt, l3.cut_progress * 100.0);
        if holding {
            l3.cut_progress += dt / 3.0;
            pm.noise = pm.noise.max(2.5);
            if l3.cut_progress >= 1.0 {
                l3.music = false;
                msg.0 = l3.switch_done.to_string();
            }
        } else {
            l3.cut_progress = (l3.cut_progress - dt * 0.8).max(0.0);
        }
        return;
    }

    // --- Wake the nearest Lost by saying their name (hold E).
    let mut best: Option<(f32, &'static str, u8)> = None;
    for (l, tf) in &lost {
        let d = flat(tf.translation);
        if l.state == LostState::Tranced && d < 2.1 && best.is_none_or(|(b, _, _)| d < b) {
            best = Some((d, l.name, l.age));
        }
    }
    let Some((_, name, age)) = best else {
        l3.wake_progress = 0.0;
        return;
    };
    prompt.0 = format!("Hold E: say \"{name}\" ({age}) and wake them ({:.0}%)", l3.wake_progress * 100.0);
    if holding {
        l3.wake_progress += dt / 1.0;
        if l3.wake_progress >= 1.0 {
            l3.wake_progress = 0.0;
            for (mut l, tf) in &mut lost {
                if l.name == name && l.state == LostState::Tranced && flat(tf.translation) < 2.1 {
                    l.state = LostState::Woken;
                    weight.0 = (weight.0 - 6.0).max(0.0);
                    stats.woken += 1;
                    msg.0 = format!("You say \"{name}\" ({age}). The trance breaks. They blink, and they cry. Weight lifts.");
                    break;
                }
            }
        }
    } else {
        l3.wake_progress = 0.0;
    }
}

/// Madanm Flanm down: the stairs unlock. Reaching them ends the level.
pub fn stairs3(
    mut l3: ResMut<Level3>,
    mut story: ResMut<Story>,
    mut boxes: ResMut<Boxes>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut msg: ResMut<Message>,
    mut next: ResMut<NextState<AppState>>,
    guards: Query<&Guard>,
    player: Single<&Transform, With<Player>>,
) {
    if !l3.stairs_open {
        let down = guards
            .iter()
            .any(|g| g.name == "Madanm Flanm" && g.status == LostStatus::Fallen);
        if down {
            story.flanm_killed = true;
            l3.stairs_open = true;
            boxes.0.pop(); // the stair door was the last box pushed
            msg.0 = "Madanm Flanm is down. The seal on the stairs breaks. The way up is open.".into();
        }
    }
    let p = player.translation;
    if l3.stairs_open && p.x > 24.2 && p.z < -12.6 {
        next.set(AppState::Complete3);
    }
    let _ = &mut mats;
}

/// The dance floor pulses in colour while the music plays.
pub fn disco(
    time: Res<Time>,
    l3: Res<Level3>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    let t = time.elapsed_secs();
    for (k, h) in l3.disco.iter().enumerate() {
        if let Some(mut m) = mats.get_mut(h) {
            if l3.music {
                let hue = ((t * 0.35 + k as f32 / 6.0) % 1.0) * 360.0;
                let c = Color::hsl(hue, 1.0, 0.55).to_linear();
                m.emissive = LinearRgba::rgb(c.red * 3.0, c.green * 3.0, c.blue * 3.0);
            } else {
                m.emissive = LinearRgba::rgb(0.02, 0.02, 0.03);
            }
        }
    }
}
