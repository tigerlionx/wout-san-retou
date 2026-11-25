//! Level 4: the first floor. Two penthouses face each other across a gallery
//! whose floor is bulletproof glass over the pool, six metres below. Tèt Wòch
//! (helmet, vest, nothing a knife can touch) holds the east penthouse. Bullets
//! cannot break the glass; blasts can. A first grenade cracks it, a second
//! collapses it: whoever stands on it falls into the pool and is stunned.
//! Ro can climb out by the pit stairs. Kids (the Lost) walk the rooms but
//! will not step onto glass, and are only woken or calmed, never shot.

use crate::ai::{Guard, GuardKind};
use crate::character::{spawn_humanoid, Box3, Boxes, Mover, STEP};
use crate::level::Ctx;
use crate::level3::{spawn_lost_crowd, wall_line, Level3, Story, Water, DOOR_W, WALL_H};
use crate::style::{glow, grow_box, ink, matte, Ink};
use crate::weapons::{glass_cracks, glass_shatter, Blasts, Fx, Health, Rng, Shake};
use crate::*;

pub const ARRIVAL: Vec3 = Vec3::new(25.55, 0.0, -10.6); // just outside the stair core door (the core stacks on every floor)
const GLASS_MIN: Vec2 = Vec2::new(-9.0, -12.0);
const GLASS_MAX: Vec2 = Vec2::new(9.0, 12.0);
const DEPTH: f32 = 6.0;
const BREAKER: Vec3 = Vec3::new(-14.0, 0.0, -17.4);

// ------------------------------------------------------------------- state

/// The glass over the pool (kept as a resource so the Lost can avoid it).
#[derive(Resource)]
pub struct GlassRect {
    pub min: Vec2,
    pub max: Vec2,
}

pub struct Hole {
    pub min: Vec2,
    pub max: Vec2,
    pub depth: f32,
}

/// Where the floor has collapsed (inserted when the glass gives way).
#[derive(Resource)]
pub struct Holes(pub Vec<Hole>);

impl Holes {
    pub fn at(&self, x: f32, z: f32) -> Option<&Hole> {
        self.0
            .iter()
            .find(|h| x > h.min.x && x < h.max.x && z > h.min.y && z < h.max.y)
    }

    /// The floor under `p` inside the pit: the pit bottom, or a step.
    pub fn pit_ground(&self, h: &Hole, p: Vec3, boxes: &[Box3]) -> f32 {
        let mut g = -h.depth;
        for b in boxes {
            let inside = (p.x - b.c.x).abs() < b.h.x + 0.15 && (p.z - b.c.z).abs() < b.h.z + 0.15;
            if inside && b.top() <= p.y + STEP {
                g = g.max(b.top());
            }
        }
        g
    }

    /// Below the first floor the pit has walls.
    pub fn keep_in_pit(&self, mut p: Vec3) -> Vec3 {
        if let Some(h) = self.0.first() {
            p.x = p.x.clamp(h.min.x + 0.4, h.max.x - 0.4);
            p.z = p.z.clamp(h.min.y + 0.4, h.max.y - 0.4);
        }
        p
    }
}

#[derive(Resource)]
pub struct Level4 {
    pub cracks: u32,
    pub collapsed: bool,
    pub stairs_open: bool,
    pub rage: bool,
}

#[derive(Component, Clone)]
pub struct Glass;

// ------------------------------------------------------------------- setup

fn floor_plane(cx: &mut Ctx, mat: &Handle<StandardMaterial>, x0: f32, x1: f32, z0: f32, z1: f32, y: f32) {
    let m = cx.meshes.add(Plane3d::default().mesh().size(x1 - x0, z1 - z0));
    cx.commands.spawn((
        Mesh3d(m),
        MeshMaterial3d(mat.clone()),
        Transform::from_xyz((x0 + x1) / 2.0, y, (z0 + z1) / 2.0),
        LevelEntity,
    ));
}

#[allow(clippy::too_many_arguments)]
pub fn setup_level4(
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
    story: Res<Story>,
) {
    let ink_res = ink_res.clone();
    checkpoint.0 = ARRIVAL; // the save point at the top of the stairs
    health.hp = health.hp.max(70.0);
    health.hurt = 0.0;
    *objectives = Objectives::default();
    *stats = Stats::default();
    rig.yaw = 0.0;
    rig.first = false;
    let rage = story.flanm_killed;
    msg.0 = if rage {
        "Saved. Up here the air is cold. Someone is screaming Madanm Flanm's name across the glass.".into()
    } else {
        "Saved. Up here the air is cold. Across the glass, a man in a helmet is waiting, very still.".into()
    };

    let floor = matte(&mut mats, 0.13);
    let pit_floor = matte(&mut mats, 0.09);
    let wall = matte(&mut mats, 0.36);
    let trim = matte(&mut mats, 0.6);
    let fabric = matte(&mut mats, 0.42);
    let wood = matte(&mut mats, 0.22);
    let metal = matte(&mut mats, 0.55);
    let glass_m = mats.add(StandardMaterial {
        base_color: Color::srgba(0.55, 0.85, 1.0, 0.16),
        emissive: LinearRgba::rgb(0.02, 0.08, 0.12),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.05,
        ..default()
    });
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

    // --- Floors: everything solid except the glass rectangle.
    floor_plane(&mut cx, &floor, -30.0, -12.0, -20.0, 20.0, 0.0);
    floor_plane(&mut cx, &floor, 12.0, 30.0, -20.0, 20.0, 0.0);
    floor_plane(&mut cx, &floor, -12.0, 12.0, -20.0, GLASS_MIN.y, 0.0);
    floor_plane(&mut cx, &floor, -12.0, 12.0, GLASS_MAX.y, 20.0, 0.0);
    floor_plane(&mut cx, &floor, -12.0, GLASS_MIN.x, GLASS_MIN.y, GLASS_MAX.y, 0.0);
    floor_plane(&mut cx, &floor, GLASS_MAX.x, 12.0, GLASS_MIN.y, GLASS_MAX.y, 0.0);

    // --- The pool below the glass, its walls, and the stairs out of the pit.
    floor_plane(&mut cx, &pit_floor, GLASS_MIN.x, GLASS_MAX.x, GLASS_MIN.y, GLASS_MAX.y, -DEPTH);
    floor_plane(&mut cx, &water_m, GLASS_MIN.x + 1.0, GLASS_MAX.x - 1.0, GLASS_MIN.y + 1.0, GLASS_MAX.y - 1.0, -DEPTH + 0.15);
    let w = Vec2::new(GLASS_MAX.x - GLASS_MIN.x, GLASS_MAX.y - GLASS_MIN.y);
    for (x, z, sx, sz) in [
        (0.0, GLASS_MIN.y, w.x, 0.3),
        (0.0, GLASS_MAX.y, w.x, 0.3),
        (GLASS_MIN.x, 0.0, 0.3, w.y),
        (GLASS_MAX.x, 0.0, 0.3, w.y),
    ] {
        cx.block(&wall, x, -DEPTH, z, Vec3::new(sx, DEPTH, sz), false);
    }
    for (x, z) in [(-6.0, 7.0), (5.0, 6.0), (-3.0, -4.0)] {
        cx.block(&fabric, x, -DEPTH, z, Vec3::new(0.8, 0.4, 2.0), false); // loungers on the pool deck
    }
    cx.commands.spawn((
        PointLight {
            color: Color::srgb(0.25, 0.75, 1.0),
            intensity: 4_000_000.0,
            range: 18.0,
            ..default()
        },
        Transform::from_xyz(0.0, -3.0, 0.0),
        LevelEntity,
    ));
    // Stairs out of the pit, rising to the west edge along the north wall.
    for k in 1..=19 {
        let top = -DEPTH + 0.32 * k as f32;
        let h = (top + DEPTH) / 2.0;
        let x = GLASS_MIN.x + 0.3 + 0.6 * (19 - k) as f32;
        cx.boxes.push(Box3 {
            c: Vec3::new(x, top - h, -10.8),
            h: Vec3::new(0.3, h, 0.8),
        });
        let m = cx.meshes.add(Cuboid::new(0.6, h * 2.0, 1.6));
        cx.commands.spawn((
            Mesh3d(m),
            MeshMaterial3d(trim.clone()),
            Transform::from_xyz(x, top - h, -10.8),
            LevelEntity,
        ));
    }

    // --- The glass: panes, a frame and a grid of bars.
    let pane = cx.meshes.add(Cuboid::new(w.x, 0.06, w.y));
    cx.commands.spawn((Glass, Mesh3d(pane), MeshMaterial3d(glass_m), Transform::from_xyz(0.0, -0.03, 0.0), LevelEntity));
    for i in 0..=6 {
        let x = GLASS_MIN.x + i as f32 * w.x / 6.0;
        let m = cx.meshes.add(Cuboid::new(0.12, 0.1, w.y));
        cx.commands.spawn((Glass, Mesh3d(m), MeshMaterial3d(trim.clone()), Transform::from_xyz(x, -0.02, 0.0), LevelEntity));
    }
    for j in 0..=6 {
        let z = GLASS_MIN.y + j as f32 * w.y / 6.0;
        let m = cx.meshes.add(Cuboid::new(w.x, 0.1, 0.12));
        cx.commands.spawn((Glass, Mesh3d(m), MeshMaterial3d(trim.clone()), Transform::from_xyz(0.0, -0.02, z), LevelEntity));
    }

    // --- Walls and doorways.
    wall_line(&mut cx, &wall, false, -20.0, -30.0, 30.0, &[]);
    wall_line(&mut cx, &wall, false, 20.0, -30.0, 30.0, &[]);
    wall_line(&mut cx, &wall, true, -30.0, -20.0, 20.0, &[]);
    wall_line(&mut cx, &wall, true, 30.0, -20.0, 20.0, &[]);
    wall_line(&mut cx, &wall, true, -12.0, -14.0, 14.0, &[-8.0, 0.0, 8.0]);
    wall_line(&mut cx, &wall, true, 12.0, -14.0, 14.0, &[-8.0, 0.0, 8.0]);
    wall_line(&mut cx, &wall, false, 0.0, -30.0, -12.0, &[-21.0]);
    wall_line(&mut cx, &wall, false, 0.0, 12.0, 30.0, &[21.0]);
    wall_line(&mut cx, &wall, true, 24.0, -20.0, -12.25, &[]);
    wall_line(&mut cx, &wall, false, -12.25, 24.0, 30.0, &[25.55]);

    // --- West penthouse: lounge and bedroom, a breaker panel in the north-west.
    cx.block(&fabric, -24.0, 0.0, -10.0, Vec3::new(3.2, 0.8, 1.2), true);
    cx.block(&fabric, -17.0, 0.0, -6.0, Vec3::new(1.2, 0.8, 3.2), true);
    cx.block(&wood, -27.0, 0.0, -17.5, Vec3::new(6.0, 1.1, 1.0), true); // bar
    cx.block(&fabric, -26.0, 0.0, 6.0, Vec3::new(2.4, 0.6, 3.4), true); // bed
    cx.block(&wood, -18.0, 0.0, 12.0, Vec3::new(2.0, 0.9, 2.0), true);
    let panel_m = glow(&mut mats, Vec3::new(1.0, 0.85, 0.2), 3.0);
    cx.block(&panel_m, BREAKER.x, 0.4, -19.6, Vec3::new(1.4, 1.6, 0.3), false);
    // --- East penthouse: Tèt Wòch's gym, bedroom and trophy table.
    cx.block(&metal, 14.0, 0.0, -17.0, Vec3::new(1.0, 1.3, 2.0), true);
    cx.block(&metal, 14.0, 0.0, -13.0, Vec3::new(1.0, 1.3, 2.0), true);
    cx.block(&fabric, 27.0, 0.0, 8.0, Vec3::new(3.0, 0.6, 2.6), true);
    cx.block(&wood, 20.0, 0.0, 10.0, Vec3::new(4.0, 0.9, 1.0), true);
    cx.block(&wood, 20.0, 0.0, -6.0, Vec3::new(3.0, 0.9, 1.2), true);
    cx.block(&fabric, 17.0, 0.0, 4.0, Vec3::new(3.0, 0.8, 1.2), true);
    // Neon accents.
    for (c, x, z) in [
        (Vec3::new(1.0, 0.2, 0.6), -26.0_f32, -19.0_f32),
        (Vec3::new(0.2, 0.9, 1.0), 20.0, -19.0),
        (Vec3::new(1.0, 0.3, 0.2), 22.0, 19.0),
    ] {
        let m = glow(&mut mats, c, 7.0);
        cx.block(&m, x, 1.8, z, Vec3::new(2.6, 0.18, 0.1), false);
        cx.commands.spawn((
            PointLight {
                color: Color::srgb(c.x, c.y, c.z),
                intensity: 800_000.0,
                range: 11.0,
                ..default()
            },
            Transform::from_xyz(x, 1.8, z * 0.9),
            LevelEntity,
        ));
    }
    cx.commands.spawn((
        Sun,
        DirectionalLight {
            illuminance: 600.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.2, 0.4, 0.0)),
        LevelEntity,
    ));

    // --- The sealed stair door (the last box pushed).
    let door_mat = glow(&mut mats, Vec3::new(1.0, 0.15, 0.1), 2.0);
    cx.block(&door_mat, 25.55, 0.0, -12.25, Vec3::new(DOOR_W, WALL_H - 0.2, 0.4), true);

    let boxes = std::mem::take(&mut cx.boxes);
    commands.insert_resource(Boxes(boxes.clone()));
    commands.insert_resource(GlassRect { min: GLASS_MIN, max: GLASS_MAX });
    commands.insert_resource(Water { min: GLASS_MIN, max: GLASS_MAX, y_max: -1.0 });
    commands.insert_resource(Blasts::default());
    commands.insert_resource(crate::level3::Grab(0));
    commands.insert_resource(Level4 { cracks: 0, collapsed: false, stairs_open: false, rage });

    // --- Ro.
    let ro = spawn_humanoid(&mut commands, &mut meshes, &mut mats, &ink_res, ARRIVAL, std::f32::consts::PI, 0.62, 0.22, 0.34);
    commands.entity(ro).insert((Player, crate::character::Skin(crate::character::Cast::Hero)));
    crate::weapons::attach_player_weapons(&mut commands, &mut meshes, &mut mats, &ink_res, ro);

    // --- The Lost: about a hundred and twenty, in the two penthouses and the corridors.
    let rooms: [(f32, f32, f32, f32, usize); 4] = [
        (-29.0, -13.0, -19.0, 19.0, 50),
        (13.0, 29.0, -19.0, 19.0, 56),
        (-11.0, 11.0, -19.0, -14.0, 7),
        (-11.0, 11.0, 14.0, 19.0, 7),
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
        &|p: Vec3| p.x > 22.0 && p.z < -6.0 && p.z > -14.0, // leave room around Tèt Wòch
    );
    commands.insert_resource(Level3 {
        music: true, // "trance on"
        cut_progress: 0.0,
        wake_progress: 0.0,
        stairs_open: false,
        total_lost: total,
        switch: BREAKER,
        switch_prompt: "Hold E: throw the breaker",
        switch_done: "The lights die. The Lost stand swaying in the dark, no longer coming for you.",
        disco: Vec::new(),
        woken_mat,
    });

    // --- Tèt Wòch (helmet, vest) and two enforcers.
    let walkers: [(&str, u8, f32, f32, Vec<Vec3>); 2] = [
        ("Gaspard", 29, 0.30, 0.15, vec![Vec3::new(-8.0, 0.0, -17.0), Vec3::new(8.0, 0.0, -17.0)]),
        ("Lucien", 33, 0.48, 0.20, vec![Vec3::new(-8.0, 0.0, 17.0), Vec3::new(8.0, 0.0, 17.0)]),
    ];
    for (name, age, shirt, pants, route) in walkers {
        let first = route[0];
        let d = route[1] - first;
        let yaw = (-d.x).atan2(-d.z);
        let id = spawn_humanoid(&mut commands, &mut meshes, &mut mats, &ink_res, first, yaw, shirt, pants, 0.36);
        crate::weapons::attach_guard_pistol(&mut commands, &mut meshes, &mut mats, &ink_res, id);
        let marker = glow(&mut mats, Vec3::splat(0.75), 0.8);
        let mm = meshes.add(Sphere::new(0.09));
        commands
            .entity(id)
            .insert(Guard {
                route,
                wp: 1,
                home_yaw: yaw,
                fall_yaw: yaw,
                last_known: first,
                marker: marker.clone(),
                ..Guard::base(name, age, GuardKind::Walker)
            })
            .with_child((Mesh3d(mm), MeshMaterial3d(marker), Transform::from_xyz(0.0, 1.85, 0.0)));
    }
    let boss_yaw = std::f32::consts::FRAC_PI_2;
    let boss_at = Vec3::new(19.0, 0.0, -8.0);
    let id = spawn_humanoid(&mut commands, &mut meshes, &mut mats, &ink_res, boss_at, boss_yaw, 0.2, 0.14, 0.34);
    commands.entity(id).insert(crate::character::Skin(crate::character::Cast::Woch));
    crate::weapons::attach_guard_pistol(&mut commands, &mut meshes, &mut mats, &ink_res, id);
    let marker = glow(&mut mats, Vec3::splat(0.75), 0.8);
    let mm = meshes.add(Sphere::new(0.1));
    commands
        .entity(id)
        .insert(Guard {
            fire_gap: if rage { 0.5 } else { 0.62 },
            route: vec![boss_at],
            wp: 0,
            home_yaw: boss_yaw,
            fall_yaw: boss_yaw,
            last_known: boss_at,
            marker: marker.clone(),
            ..Guard::tet_woch()
        })
        .with_child((Mesh3d(mm), MeshMaterial3d(marker), Transform::from_xyz(0.0, 2.3, 0.0)));
}

// ------------------------------------------------------------- the glass

/// Blasts crack the glass; the second one brings it down.
#[allow(clippy::too_many_arguments)]
pub fn glass4(
    mut commands: Commands,
    fx: Res<Fx>,
    mut rng: ResMut<Rng>,
    mut shake: ResMut<Shake>,
    mut l4: ResMut<Level4>,
    mut blasts: ResMut<Blasts>,
    mut msg: ResMut<Message>,
    glass: Query<Entity, With<Glass>>,
) {
    let blasts_now: Vec<Vec3> = std::mem::take(&mut blasts.0);
    for c in blasts_now {
        if l4.collapsed {
            continue;
        }
        // A blast on or near the glass counts.
        let near = c.x > GLASS_MIN.x - 4.0 && c.x < GLASS_MAX.x + 4.0 && c.z > GLASS_MIN.y - 4.0 && c.z < GLASS_MAX.y + 4.0;
        if !near {
            continue;
        }
        l4.cracks += 1;
        if l4.cracks < 2 {
            glass_cracks(&mut commands, &fx, &mut rng, c, Glass);
            shake.0 = (shake.0 + 0.4).min(1.0);
            msg.0 = "The glass cracks, a white web across the floor. It groans. One more blast and it will give.".into();
        } else {
            l4.collapsed = true;
            for e in &glass {
                commands.entity(e).despawn();
            }
            glass_shatter(&mut commands, &fx, &mut rng, GLASS_MIN, GLASS_MAX);
            commands.insert_resource(Holes(vec![Hole { min: GLASS_MIN, max: GLASS_MAX, depth: DEPTH }]));
            shake.0 = 1.0;
            msg.0 = "The glass shatters. Whatever was standing on it is falling into the pool.".into();
        }
    }
}

/// Anything standing over the hole falls; a hard landing hurts and stuns the armored.
pub fn fall4(
    time: Res<Time>,
    holes: Option<Res<Holes>>,
    mut msg: ResMut<Message>,
    mut guards: Query<(&mut Guard, &mut Transform, &mut Mover), Without<Player>>,
) {
    let Some(h) = holes else { return };
    let dt = time.delta_secs().min(0.1);
    for (mut g, mut tf, mut mv) in &mut guards {
        let p = tf.translation;
        if let Some(hole) = h.at(p.x, p.z) {
            if g.home_y > -hole.depth + 0.1 && p.y > -hole.depth + 0.02 {
                mv.vel.y -= 15.0 * dt;
                tf.translation.y += mv.vel.y * dt;
                if tf.translation.y <= -hole.depth {
                    tf.translation.y = -hole.depth;
                    g.home_y = -hole.depth;
                    mv.vel.y = 0.0;
                    g.hp -= g.scaled_damage(150.0, 10.0); // the fall: fifteen bullets' worth
                    g.awareness = 1.0;
                    if g.armor < 0.5 || g.boss {
                        g.stagger = 8.0;
                        msg.0 = format!("{} hits the water, hard, and does not get up. Finish him.", g.name);
                    }
                }
            }
        }
        if tf.translation.y < -0.5 {
            tf.translation = h.keep_in_pit(tf.translation);
        }
    }
}

/// Tèt Wòch down: the stair door opens; reaching the stairs ends the level.
pub fn stairs4(
    mut l4: ResMut<Level4>,
    mut story: ResMut<Story>,
    mut boxes: ResMut<Boxes>,
    mut msg: ResMut<Message>,
    mut next: ResMut<NextState<AppState>>,
    guards: Query<&Guard>,
    player: Single<&Transform, With<Player>>,
) {
    if !l4.stairs_open {
        let down = guards
            .iter()
            .any(|g| g.name == "Tèt Wòch" && g.status == LostStatus::Fallen);
        if down {
            story.tet_woch_killed = true;
            l4.stairs_open = true;
            boxes.0.pop(); // the stair door was the last box pushed
            msg.0 = "Tèt Wòch is down. The stair door unlocks. The way up is open.".into();
        }
    }
    let p = player.translation;
    if l4.stairs_open && p.x > 24.2 && p.z < -12.6 && p.y > -0.5 {
        next.set(AppState::Complete4);
    }
}
