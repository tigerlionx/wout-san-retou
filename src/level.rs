//! Level 1: the compound's yard and the low, brown house at its far end.
//! Objectives: sabotage the truck, the fuel and the food, then reach the door.

use crate::ai::{AiState, Guard, GuardKind};
use crate::character::{spawn_humanoid, Box3, Boxes, Mover};
use crate::style::{glow, grow_box, ink, matte, Ink};
use crate::*;

pub struct Ctx<'a, 'w, 's> {
    pub commands: &'a mut Commands<'w, 's>,
    pub meshes: &'a mut Assets<Mesh>,
    pub ink: &'a Ink,
    pub boxes: Vec<Box3>,
}

impl Ctx<'_, '_, '_> {
    /// A solid (or decorative) box standing on `y0`, centred on (x, z).
    pub fn block(
        &mut self,
        mat: &Handle<StandardMaterial>,
        x: f32,
        y0: f32,
        z: f32,
        size: Vec3,
        solid: bool,
    ) -> Entity {
        let mesh = self.meshes.add(Cuboid::new(size.x, size.y, size.z));
        let c = Vec3::new(x, y0 + size.y / 2.0, z);
        if solid {
            self.boxes.push(Box3 { c, h: size / 2.0 });
        }
        self.commands
            .spawn((
                ink(
                    mesh,
                    mat.clone(),
                    self.ink,
                    Transform::from_translation(c),
                    grow_box(size, 0.04),
                ),
                LevelEntity,
            ))
            .id()
    }

    /// A bush: a squashed sphere that hides a crouching player.
    pub fn bush(&mut self, mat: &Handle<StandardMaterial>, x: f32, z: f32) {
        let mesh = self.meshes.add(Sphere::new(1.0));
        self.boxes.push(Box3 {
            c: Vec3::new(x, 0.6, z),
            h: Vec3::new(1.25, 0.75, 0.95),
        });
        self.commands.spawn((
            ink(
                mesh,
                mat.clone(),
                self.ink,
                Transform::from_xyz(x, 0.6, z).with_scale(Vec3::new(1.3, 0.75, 1.0)),
                Vec3::splat(1.06),
            ),
            LevelEntity,
        ));
    }

    pub fn drum(&mut self, mat: &Handle<StandardMaterial>, x: f32, z: f32) {
        let mesh = self.meshes.add(Cylinder::new(0.45, 1.1));
        self.boxes.push(Box3 {
            c: Vec3::new(x, 0.55, z),
            h: Vec3::new(0.45, 0.55, 0.45),
        });
        self.commands.spawn((
            ink(
                mesh,
                mat.clone(),
                self.ink,
                Transform::from_xyz(x, 0.55, z),
                Vec3::new(1.08, 1.04, 1.08),
            ),
            LevelEntity,
        ));
    }
}

#[allow(clippy::too_many_arguments)]
pub fn setup_level(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    ink_res: Res<Ink>,
    mut clock: ResMut<GameClock>,
    mut weight: ResMut<Weight>,
    mut msg: ResMut<Message>,
    mut objectives: ResMut<Objectives>,
    mut stats: ResMut<Stats>,
    mut caught: ResMut<Caught>,
    mut rig: ResMut<CamRig>,
    mut loadout: ResMut<crate::weapons::Loadout>,
    mut health: ResMut<crate::weapons::Health>,
    checkpoint: Res<Checkpoint>,
    fx: Res<crate::weapons::Fx>,
) {
    // Reset the run.
    clock.minute = START_MINUTE;
    clock.night_fell = false;
    weight.0 = 0.0;
    msg.0 = "Get to the house. Stay out of sight.".into();
    *objectives = Objectives::default();
    *stats = Stats::default();
    caught.0 = None;
    rig.yaw = 0.0;
    rig.first = false;
    *loadout = Default::default();
    *health = Default::default();
    fx.ambience(&mut commands, 0.22); // a distant gunfight beyond the walls

    let ink_res = ink_res.clone();
    let ground = matte(&mut mats, 0.17);
    let lawn = matte(&mut mats, 0.30);
    let wall = matte(&mut mats, 0.50);
    let house = matte(&mut mats, 0.36);
    let roof = matte(&mut mats, 0.60);
    let post = matte(&mut mats, 0.72);
    let crate_m = matte(&mut mats, 0.46);
    let truck = matte(&mut mats, 0.52);
    let tire = matte(&mut mats, 0.08);
    let glass = matte(&mut mats, 0.10);
    let bush_m = matte(&mut mats, 0.25);
    let drum_m = matte(&mut mats, 0.58);
    let shed = matte(&mut mats, 0.42);
    let sack = matte(&mut mats, 0.74);

    let mut cx = Ctx {
        commands: &mut commands,
        meshes: &mut meshes,
        ink: &ink_res,
        boxes: Vec::new(),
    };

    // Ground and lawn.
    let plane = cx.meshes.add(Plane3d::default().mesh().size(64.0, 64.0));
    cx.commands.spawn((Mesh3d(plane), MeshMaterial3d(ground), LevelEntity));
    let lawn_mesh = cx.meshes.add(Plane3d::default().mesh().size(40.0, 22.0));
    cx.commands.spawn((
        Mesh3d(lawn_mesh),
        MeshMaterial3d(lawn),
        Transform::from_xyz(4.0, 0.02, 6.0),
        LevelEntity,
    ));

    // Perimeter wall.
    cx.block(&wall, 0.0, 0.0, -30.3, Vec3::new(61.0, 2.6, 0.6), true);
    cx.block(&wall, 0.0, 0.0, 30.3, Vec3::new(61.0, 2.6, 0.6), true);
    cx.block(&wall, -30.3, 0.0, 0.0, Vec3::new(0.6, 2.6, 60.0), true);
    cx.block(&wall, 30.3, 0.0, 0.0, Vec3::new(0.6, 2.6, 60.0), true);

    // The house: low, flat roof, pergola, big windows, neon OPEN sign.
    cx.block(&house, 0.0, 0.0, -23.0, Vec3::new(18.0, 3.6, 7.0), true);
    cx.block(&roof, 0.0, 3.6, -23.0, Vec3::new(20.0, 0.25, 9.4), true);
    cx.block(&roof, 0.0, 3.3, -18.9, Vec3::new(18.0, 0.25, 1.8), false);
    for x in [-8.0, -4.0, 0.0, 4.0, 8.0] {
        cx.block(&post, x, 0.0, -18.2, Vec3::new(0.3, 3.3, 0.3), false);
    }
    for x in [-6.5, 6.5] {
        cx.block(&glass, x, 0.9, -19.46, Vec3::new(3.2, 1.7, 0.1), false);
    }
    // Door (changes colour when all three objectives are done).
    let door_mat = glow(&mut mats, Vec3::new(1.0, 0.15, 0.1), 2.0);
    cx.block(&door_mat, 0.0, 0.0, -19.42, Vec3::new(1.6, 2.4, 0.16), false);
    cx.commands.insert_resource(DoorMat(door_mat.clone()));
    // Neon sign: red letters on a blue frame.
    let red = glow(&mut mats, Vec3::new(1.0, 0.1, 0.1), 7.0);
    let blue = glow(&mut mats, Vec3::new(0.2, 0.35, 1.0), 7.0);
    cx.block(&blue, -6.5, 1.75, -19.38, Vec3::new(1.6, 0.7, 0.06), false);
    cx.block(&red, -6.5, 1.85, -19.3, Vec3::new(1.2, 0.4, 0.06), false);
    for (color, intensity) in [(Color::srgb(1.0, 0.15, 0.1), 500_000.0), (Color::srgb(0.2, 0.3, 1.0), 400_000.0)] {
        cx.commands.spawn((
            PointLight {
                color,
                intensity,
                range: 9.0,
                ..default()
            },
            Transform::from_xyz(-6.5, 2.2, -17.4),
            LevelEntity,
        ));
    }
    // String lights along the pergola: tiny colour accents in the grey.
    let bulbs = [
        Vec3::new(1.0, 0.2, 0.2),
        Vec3::new(0.3, 1.0, 0.4),
        Vec3::new(0.3, 0.5, 1.0),
        Vec3::new(1.0, 0.8, 0.2),
    ];
    for (i, x) in [-8.0_f32, -6.0, -4.0, -2.0, 0.0, 2.0, 4.0, 6.0, 8.0].iter().enumerate() {
        let rgb = bulbs[i % bulbs.len()];
        let m = glow(&mut mats, rgb, 6.0);
        let mesh = cx.meshes.add(Sphere::new(0.09));
        cx.commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(m),
            Transform::from_xyz(*x, 3.15, -18.2),
            LevelEntity,
        ));
        cx.commands.spawn((
            PointLight {
                color: Color::srgb(rgb.x, rgb.y, rgb.z),
                intensity: 60_000.0,
                range: 4.5,
                ..default()
            },
            Transform::from_xyz(*x, 3.0, -18.0),
            LevelEntity,
        ));
    }

    // Sniper's outbuilding (west side) with the nest on its roof.
    cx.block(&shed, -25.0, 0.0, -12.0, Vec3::new(5.0, 3.0, 4.0), true);
    cx.block(&crate_m, -26.8, 3.0, -12.0, Vec3::new(0.8, 0.5, 2.0), false);

    // Supply truck (west).
    cx.block(&truck, -16.0, 0.5, -1.2, Vec3::new(2.4, 1.6, 5.2), true);
    cx.block(&truck, -16.0, 0.5, -4.7, Vec3::new(2.4, 1.3, 1.8), true);
    for (x, z) in [(-17.3, -4.4), (-14.7, -4.4), (-17.3, 0.6), (-14.7, 0.6)] {
        cx.block(&tire, x, 0.0, z, Vec3::new(0.35, 1.0, 1.0), false);
    }
    // Fuel store (east): drums.
    cx.drum(&drum_m, 18.0, -8.0);
    cx.drum(&drum_m, 19.0, -8.9);
    cx.drum(&drum_m, 17.8, -9.3);
    cx.drum(&drum_m, 19.2, -7.6);
    // Food store (south east): a shed plus sacks.
    cx.block(&shed, 14.0, 0.0, 17.5, Vec3::new(5.0, 2.6, 3.5), true);
    cx.block(&sack, 11.0, 0.0, 13.6, Vec3::new(1.0, 0.6, 0.7), true);
    cx.block(&sack, 12.1, 0.0, 13.6, Vec3::new(1.0, 0.6, 0.7), true);

    // Crates you can vault (1.0 m) and hide behind.
    for (x, z, sx, sz) in [
        (-4.0, 16.0, 1.6, 1.6),
        (-2.2, 17.2, 1.6, 1.6),
        (8.0, 6.0, 1.8, 1.2),
        (-9.0, 9.0, 1.4, 1.4),
        (5.0, -8.0, 1.4, 1.4),
        (-12.0, -10.0, 1.6, 1.2),
        (22.0, 2.0, 1.6, 1.6),
    ] {
        cx.block(&crate_m, x, 0.0, z, Vec3::new(sx, 1.0, sz), true);
    }
    // Bushes (they hide you only if you crouch: 1.35 m tall).
    for (x, z) in [(-3.0, 8.0), (10.0, -1.0), (-8.0, -12.0), (2.0, -10.0), (22.0, -18.0), (-22.0, 12.0), (4.0, 20.0)] {
        cx.bush(&bush_m, x, z);
    }

    // Sabotage targets: invisible trigger + a beacon that turns green when done.
    let beacon_mesh = cx.meshes.add(Sphere::new(0.14));
    let target_defs = [
        (
            0,
            Vec3::new(-16.0, 0.0, 1.9),
            "Hold E: slash the truck's tires",
            "The tires are flat. The truck is going nowhere.",
        ),
        (
            1,
            Vec3::new(17.0, 0.0, -6.6),
            "Hold E: open the fuel valves",
            "The fuel drains into the dirt. No more fuel.",
        ),
        (
            2,
            Vec3::new(14.0, 0.0, 14.9),
            "Hold E: spoil the food store",
            "The food is ruined. They will go hungry tonight.",
        ),
    ];
    for (idx, pos, prompt, done_msg) in target_defs {
        let beacon = glow(&mut mats, Vec3::new(1.0, 0.8, 0.2), 4.0);
        cx.commands.spawn((
            SabotageTarget {
                idx,
                prompt,
                done_msg,
                pos,
                secs: 2.5,
                beacon: beacon.clone(),
            },
            Mesh3d(beacon_mesh.clone()),
            MeshMaterial3d(beacon),
            Transform::from_translation(pos + Vec3::Y * 1.2),
            LevelEntity,
        ));
    }

    // Light.
    cx.commands.spawn((
        Sun,
        DirectionalLight {
            illuminance: 9_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.95, 0.45, 0.0)),
        LevelEntity,
    ));

    let boxes = std::mem::take(&mut cx.boxes);
    commands.insert_resource(Boxes(boxes));

    // Ro: the player.
    let ro = spawn_humanoid(
        &mut commands,
        &mut meshes,
        &mut mats,
        &ink_res,
        checkpoint.0,
        0.0,
        0.62,
        0.22,
        0.34,
    );
    commands.entity(ro).insert((Player, crate::character::Skin(crate::character::Cast::Hero)));
    crate::weapons::attach_player_weapons(&mut commands, &mut meshes, &mut mats, &ink_res, ro);

    // The guards: kids recruited by the Candle Brigade. Fictional, with names and ages.
    let walkers = [
        ("Dieuseul", 24, 0.30, 0.15, vec![v(-8.0, 12.0), v(8.0, 12.0), v(8.0, 2.0), v(-8.0, 2.0)]),
        ("Kendy", 22, 0.55, 0.20, vec![v(12.0, -12.0), v(12.0, -4.0), v(22.0, -4.0), v(22.0, -14.0)]),
        ("Roody", 27, 0.40, 0.12, vec![v(-10.0, -15.0), v(10.0, -15.0)]),
        ("Frantz", 31, 0.48, 0.25, vec![v(-20.0, 10.0), v(-20.0, -10.0), v(-10.0, -8.0), v(-10.0, 8.0)]),
    ];
    for (name, age, shirt, pants, route) in walkers {
        let first = route[0];
        let yaw = {
            let d = route[1] - first;
            (-d.x).atan2(-d.z)
        };
        spawn_guard(
            &mut commands,
            &mut meshes,
            &mut mats,
            &ink_res,
            Guard {
                name,
                age,
                kind: GuardKind::Walker,
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
                fall_yaw: yaw,
                awareness: 0.0,
                last_known: first,
                timer: 0.0,
                unseen: 0.0,
                sees: false,
                aim: 0.0,
                route,
                wp: 1,
                home_yaw: yaw,
                fov: 100.0_f32.to_radians(),
                range: 14.0,
                eye: 1.5,
                marker: Handle::default(),
            },
            first,
            yaw,
            (shirt, pants, 0.36),
            0.0,
        );
    }
    // The sniper on the outbuilding roof, looking east over the yard.
    spawn_guard(
        &mut commands,
        &mut meshes,
        &mut mats,
        &ink_res,
        Guard {
            name: "Mackenson",
            age: 26,
            kind: GuardKind::Sniper,
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
            home_y: 3.0,
            fall: 0.0,
            fall_yaw: -std::f32::consts::FRAC_PI_2,
            awareness: 0.0,
            last_known: Vec3::ZERO,
            timer: 0.0,
            unseen: 0.0,
            sees: false,
            aim: 0.0,
            route: Vec::new(),
            wp: 0,
            home_yaw: -std::f32::consts::FRAC_PI_2,
            fov: 60.0_f32.to_radians(),
            range: 45.0,
            eye: 1.2,
            marker: Handle::default(),
        },
        Vec3::new(-25.0, 3.0, -12.0),
        -std::f32::consts::FRAC_PI_2,
        (0.35, 0.12, 0.30),
        0.8,
    );

}

fn v(x: f32, z: f32) -> Vec3 {
    Vec3::new(x, 0.0, z)
}

#[allow(clippy::too_many_arguments)]
fn spawn_guard(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mats: &mut Assets<StandardMaterial>,
    ink_res: &Ink,
    mut guard: Guard,
    at: Vec3,
    yaw: f32,
    (shirt, pants, skin): (f32, f32, f32),
    crouch: f32,
) {
    let id = spawn_humanoid(commands, meshes, mats, ink_res, at, yaw, shirt, pants, skin);
    let marker = glow(mats, Vec3::splat(0.75), 0.8);
    guard.marker = marker.clone();
    let marker_mesh = meshes.add(Sphere::new(0.09));
    crate::weapons::attach_guard_pistol(commands, meshes, mats, ink_res, id);
    commands.entity(id).insert(guard).with_child((
        Mesh3d(marker_mesh),
        MeshMaterial3d(marker),
        Transform::from_xyz(0.0, 1.85, 0.0),
    ));
    if crouch > 0.0 {
        commands.queue(move |world: &mut World| {
            if let Some(mut mv) = world.get_mut::<Mover>(id) {
                mv.crouch = crouch;
            }
        });
    }
}

// --------------------------------------------------------------- interaction

#[allow(clippy::too_many_arguments)]
pub fn interact(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut weight: ResMut<Weight>,
    mut msg: ResMut<Message>,
    mut prompt: ResMut<Prompt>,
    mut objectives: ResMut<Objectives>,
    mut stats: ResMut<Stats>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    targets: Query<&SabotageTarget>,
    mut guards: Query<(&mut Guard, &Transform, &mut Visibility), Without<Player>>,
    player: Single<(&Transform, &mut Mover), With<Player>>,
) {
    let dt = time.delta_secs().min(0.1);
    let (pt, mut pm) = player.into_inner();
    let pp = pt.translation;
    prompt.0.clear();

    let flat_dist = |a: Vec3, b: Vec3| Vec2::new(a.x - b.x, a.z - b.z).length();

    // Is a standing guard close enough to talk to or strike?
    let guard_close = guards.iter().any(|(g, tf, _)| {
        g.status == LostStatus::Standing
            && g.kind == GuardKind::Walker
            && flat_dist(pp, tf.translation) < 2.4
    });

    // Nearest unfinished sabotage target.
    let near_target = targets
        .iter()
        .filter(|t| !objectives.done[t.idx] && flat_dist(pp, t.pos) < 2.4)
        .min_by(|a, b| flat_dist(pp, a.pos).total_cmp(&flat_dist(pp, b.pos)));

    let holding = keys.pressed(KeyCode::KeyE);

    // Sabotage progress: hold E near a target.
    for t in &targets {
        if objectives.done[t.idx] {
            continue;
        }
        let working = holding && near_target.is_some_and(|n| n.idx == t.idx);
        if working {
            objectives.progress[t.idx] += dt / t.secs;
            pm.noise = pm.noise.max(2.5);
            if objectives.progress[t.idx] >= 1.0 {
                objectives.done[t.idx] = true;
                objectives.progress[t.idx] = 1.0;
                msg.0 = t.done_msg.to_string();
                if let Some(mut m) = mats.get_mut(&t.beacon) {
                    m.base_color = Color::srgb(0.2, 1.0, 0.45);
                    m.emissive = LinearRgba::rgb(0.6, 4.0, 1.6);
                }
            }
        } else {
            objectives.progress[t.idx] = (objectives.progress[t.idx] - dt * 0.8).max(0.0);
        }
    }
    if let Some(t) = near_target {
        prompt.0 = t.prompt.to_string();
    }

    // Guards: E wakes them by name (from behind, unnoticed).
    let wake = keys.just_pressed(KeyCode::KeyE);
    if guard_close {
        if prompt.0.is_empty() {
            prompt.0 = "E: say his name (wake)   F: knife".to_string();
        }
        if wake {
            // Pick the closest one now that we can mutate.
            let mut best: Option<(f32, usize)> = None;
            for (i, (g, tf, _)) in guards.iter().enumerate() {
                let d = flat_dist(pp, tf.translation);
                if g.status == LostStatus::Standing
                    && g.kind == GuardKind::Walker
                    && d < 2.4
                    && best.is_none_or(|(b, _)| d < b)
                {
                    best = Some((d, i));
                }
            }
            if let Some((_, i)) = best {
                if let Some((mut g, _, _)) = guards.iter_mut().nth(i) {
                    if g.is_boss() {
                        g.state = AiState::Chase;
                        g.awareness = 1.0;
                        msg.0 = format!("{} laughs. \"My name? I gave you that name, ti gason.\" It will not work on her.", g.name);
                    } else if g.awareness < 0.5 && g.state != AiState::Chase {
                        g.status = LostStatus::Woken;
                        weight.0 = (weight.0 - 12.0).max(0.0);
                        stats.woken += 1;
                        msg.0 = format!(
                            "You say \"{}\" ({}). The trance breaks. He stops. Weight lifts.",
                            g.name, g.age
                        );
                    } else {
                        g.state = AiState::Chase;
                        g.awareness = 1.0;
                        msg.0 = format!("{} turns around. He sees you!", g.name);
                    }
                }
            }
        }
    }
}

pub fn check_goal(
    mut objectives: ResMut<Objectives>,
    door: Res<DoorMat>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut msg: ResMut<Message>,
    mut next: ResMut<NextState<AppState>>,
    player: Single<&Transform, With<Player>>,
) {
    if !objectives.door_open && objectives.done.iter().all(|d| *d) {
        objectives.door_open = true;
        msg.0 = "All three done. The house door is unlocked.".into();
        if let Some(mut m) = mats.get_mut(&door.0) {
            m.base_color = Color::srgb(0.2, 1.0, 0.45);
            m.emissive = LinearRgba::rgb(0.6, 4.0, 1.6);
        }
    }
    let at_door = Vec2::new(player.translation.x, player.translation.z + 18.4).length() < 1.8;
    if objectives.door_open && at_door {
        next.set(AppState::Complete);
    }
}

/// When Ro's health runs out: he wakes back at the gate, the yard calms down.
pub fn handle_caught(
    mut caught: ResMut<Caught>,
    mut msg: ResMut<Message>,
    mut stats: ResMut<Stats>,
    mut health: ResMut<crate::weapons::Health>,
    mut poison: ResMut<crate::level2::Poison>,
    mut nwits: Query<&mut crate::level2::Nwit>,
    checkpoint: Res<Checkpoint>,
    mut loadout: ResMut<crate::weapons::Loadout>,
    saved: Res<crate::weapons::LoadoutSave>,
    mut guards: Query<&mut Guard>,
    player: Single<(&mut Transform, &mut Mover), With<Player>>,
) {
    if health.hp <= 0.0 && caught.0.is_none() {
        caught.0 = Some("You go down.".into());
    }
    let Some(why) = caught.0.take() else { return };
    let (mut tf, mut mv) = player.into_inner();
    tf.translation = checkpoint.0;
    mv.vel = Vec3::ZERO;
    health.hp = 100.0;
    health.hurt = 0.0;
    // back with the ammunition he had at the save point
    if let Some(saved) = &saved.0 {
        loadout.restore_from(saved);
    }
    stats.caught += 1;
    // Dying poisoned sends you back to be stabbed again.
    if poison.active || poison.drugged > 0.0 {
        poison.active = false;
        poison.alert = 0.0;
        poison.drugged = 0.0;
        for mut n in &mut nwits {
            n.enabled = true;
            n.attempts = 2;
            n.timer = 6.0;
        }
    }
    for mut g in &mut guards {
        if g.melee {
            continue; // the Sword Man never loses the scent
        }
        if g.status == LostStatus::Standing {
            g.state = AiState::Patrol;
            g.awareness = 0.0;
            g.aim = 0.0;
            g.timer = 0.0;
            g.stagger = 0.0;
        }
    }
    msg.0 = format!("{why} You come to at your last safe spot, with the ammunition you had there. Try again, quieter.");
}
