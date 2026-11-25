//! Articulated characters (head, torso, swinging arms and legs) and the
//! shared collision / line-of-sight math used by the player and the bots.

use crate::style::Ink;
use bevy::animation::RepeatAnimation;
use bevy::world_serialization::WorldInstanceReady;
use std::collections::HashMap;
use std::time::Duration;
use crate::*;

/// Movement state shared by the player and every bot.
#[derive(Component, Default)]
pub struct Mover {
    pub vel: Vec3,
    pub grounded: bool,
    /// 0 = standing, 1 = fully crouched.
    pub crouch: f32,
    pub phase: f32,
    pub sprinting: bool,
    pub moving: bool,
    /// Horizontal speed in m/s (drives the limb animation).
    pub speed: f32,
    /// How far, in metres, the character is currently audible.
    pub noise: f32,
    /// Holding a gun: arms raised and pointing forward.
    pub armed: bool,
    /// Recoil kick, 1.0 right after a shot and fading.
    pub kick: f32,
    /// Melee / throwing swing, 1.0 at the start and fading.
    pub slash: f32,
    /// Smoothed speed that drives the walk cycle (so legs ease in and out).
    pub gait: f32,
    /// What the current melee / throw swing is, so the right clip plays.
    pub swing: Swing,
    /// Shambles instead of walking (the risen).
    pub zombie: bool,
    /// Seconds in the air since the feet left the ground (a short hop plays the jump clip, a long drop the falling one).
    pub air: f32,
    /// Reloading: the reload clip plays.
    pub reloading: bool,
    /// A hit reaction, 1.0 right after being hurt and fading.
    pub flinch: f32,
    /// Lying down: 0 = not, 1 = flat on the belly (eases in and out).
    pub prone: f32,
    /// The player wants to be prone (toggled with Z).
    pub prone_on: bool,
    /// Looking down the sights (right mouse button).
    pub aiming: bool,
    /// The weapon in hand is a pistol (one-handed clips).
    pub pistol: bool,
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub enum Swing {
    #[default]
    None,
    Stab,
    Throw,
    Punch,
    Bite,
}

/// Axis-aligned box: center and half extents.
#[derive(Clone, Copy)]
pub struct Box3 {
    pub c: Vec3,
    pub h: Vec3,
}

impl Box3 {
    pub fn top(&self) -> f32 {
        self.c.y + self.h.y
    }
}

/// All static solid objects (walls, crates, vehicles...).
#[derive(Resource, Default)]
pub struct Boxes(pub Vec<Box3>);

/// Obstacles shorter than this (above the feet) can be stepped onto.
pub const STEP: f32 = 0.35;

/// Push a circle of radius `r` out of every box that is too tall to step on.
pub fn resolve(mut p: Vec3, r: f32, boxes: &[Box3]) -> Vec3 {
    for _ in 0..2 {
        for b in boxes {
            if b.top() <= p.y + STEP {
                continue; // low enough to step or vault over
            }
            if p.y + 1.7 < b.c.y - b.h.y {
                continue; // we are below it (e.g. a roof overhang)
            }
            let dx = p.x - b.c.x;
            let dz = p.z - b.c.z;
            let ox = b.h.x + r - dx.abs();
            let oz = b.h.z + r - dz.abs();
            if ox > 0.0 && oz > 0.0 {
                if ox < oz {
                    p.x += ox * dx.signum();
                } else {
                    p.z += oz * dz.signum();
                }
            }
        }
    }
    p
}

/// Height of the floor under `p` (the ground, or the top of something low
/// enough to stand on).
pub fn ground_at(p: Vec3, boxes: &[Box3]) -> f32 {
    let mut g = 0.0_f32;
    for b in boxes {
        let inside = (p.x - b.c.x).abs() < b.h.x + 0.15 && (p.z - b.c.z).abs() < b.h.z + 0.15;
        if inside && b.top() <= p.y + STEP {
            g = g.max(b.top());
        }
    }
    g
}

/// Does the straight line from `a` to `b` pass through any box?
pub fn segment_blocked(a: Vec3, b: Vec3, boxes: &[Box3]) -> bool {
    let d = b - a;
    'boxes: for bx in boxes {
        let lo = bx.c - bx.h;
        let hi = bx.c + bx.h;
        let (mut t0, mut t1) = (0.0_f32, 1.0_f32);
        for axis in 0..3 {
            let (o, dir) = (a[axis], d[axis]);
            if dir.abs() < 1e-6 {
                if o < lo[axis] || o > hi[axis] {
                    continue 'boxes;
                }
            } else {
                let (mut u, mut v) = ((lo[axis] - o) / dir, (hi[axis] - o) / dir);
                if u > v {
                    std::mem::swap(&mut u, &mut v);
                }
                t0 = t0.max(u);
                t1 = t1.min(v);
                if t0 > t1 {
                    continue 'boxes;
                }
            }
        }
        return true;
    }
    false
}

/// Where a ray first enters a box, and the face normal there.
pub fn ray_box(o: Vec3, d: Vec3, b: &Box3) -> Option<(f32, Vec3)> {
    let lo = b.c - b.h;
    let hi = b.c + b.h;
    let (mut t0, mut t1) = (0.0_f32, f32::MAX);
    let mut n = Vec3::ZERO;
    for axis in 0..3 {
        let (oa, da) = (o[axis], d[axis]);
        if da.abs() < 1e-6 {
            if oa < lo[axis] || oa > hi[axis] {
                return None;
            }
        } else {
            let (mut u, mut v) = ((lo[axis] - oa) / da, (hi[axis] - oa) / da);
            if u > v {
                std::mem::swap(&mut u, &mut v);
            }
            if u > t0 {
                t0 = u;
                n = Vec3::ZERO;
                n[axis] = if da > 0.0 { -1.0 } else { 1.0 };
            }
            t1 = t1.min(v);
            if t0 > t1 {
                return None;
            }
        }
    }
    if t0 > 0.0 { Some((t0, n)) } else { None }
}

fn ray_sphere(o: Vec3, d: Vec3, c: Vec3, r: f32) -> Option<f32> {
    let oc = o - c;
    let b = oc.dot(d);
    let disc = b * b - (oc.dot(oc) - r * r);
    if disc < 0.0 {
        return None;
    }
    let t = -b - disc.sqrt();
    if t > 0.0 { Some(t) } else { None }
}

fn ray_cylinder(o: Vec3, d: Vec3, c: Vec3, r: f32, y0: f32, y1: f32) -> Option<f32> {
    let (ox, oz) = (o.x - c.x, o.z - c.z);
    let a = d.x * d.x + d.z * d.z;
    if a < 1e-8 {
        return None;
    }
    let b = 2.0 * (ox * d.x + oz * d.z);
    let cc = ox * ox + oz * oz - r * r;
    let disc = b * b - 4.0 * a * cc;
    if disc < 0.0 {
        return None;
    }
    let t = (-b - disc.sqrt()) / (2.0 * a);
    let y = o.y + t * d.y;
    if t > 0.0 && y >= y0 && y <= y1 { Some(t) } else { None }
}

/// How tall a person is when crouched or lying down (1.0 standing).
pub fn person_scale(crouch: f32, prone: f32) -> f32 {
    (1.0 - 0.32 * crouch - 0.7 * prone).max(0.28)
}

/// A standing, crouched or prone person as a head sphere on a body cylinder.
/// Returns the hit distance and whether it was the head.
pub fn ray_person(o: Vec3, d: Vec3, feet: Vec3, crouch: f32, prone: f32) -> Option<(f32, bool)> {
    let sy = person_scale(crouch, prone);
    let head = ray_sphere(o, d, feet + Vec3::Y * (1.62 * sy), 0.17);
    let body = ray_cylinder(o, d, feet, 0.32, feet.y, feet.y + 1.75 * sy);
    // The head sphere sits inside the body cylinder, so any hit on it counts.
    match (head, body) {
        (Some(h), _) => Some((h, true)),
        (None, Some(b)) => Some((b, false)),
        _ => None,
    }
}

pub fn ray_guard(o: Vec3, d: Vec3, feet: Vec3, crouch: f32) -> Option<(f32, bool)> {
    ray_person(o, d, feet, crouch, 0.0)
}

/// Three points to look at or shoot at: head, chest, legs. Cover hides them one by one, from the
/// legs up when it is low, so how much of a person is protected depends on the height of the cover.
pub fn body_points(feet: Vec3, crouch: f32, prone: f32) -> [Vec3; 3] {
    let sy = person_scale(crouch, prone);
    [feet + Vec3::Y * (1.55 * sy), feet + Vec3::Y * (1.1 * sy), feet + Vec3::Y * (0.45 * sy)]
}

// ------------------------------------------------------------ the body
//
// A Mixamo skeleton animated by Mixamo clips. Every Mixamo rig shares its bone
// names, so any clip plays on any character. The files live in assets/mixamo
// (kept out of git; see tools/make_characters.py and tools/mixamo_to_glb.py) and
// are loaded when a character first appears.

/// Every clip converted into assets/mixamo/<name>.glb.
pub const CLIPS: [&str; 48] = [
    "idle", "rifle_idle", "walk", "walk_back", "run", "run_back", "strafe_left", "strafe_right",
    "crouch_down", "crouch_walk", "jump", "fire_rifle", "pistol_move", "reload", "grenade", "stab",
    "slash", "punch", "hit", "hit_big", "death_back", "death_left", "dying", "get_up", "falling",
    "kneel", "zombie_walk", "zombie_idle", "zombie_bite",
    // second batch: lying down, crouch and aim poses, landings, pistol gaits, faster risen
    "prone_down", "prone_idle", "prone_crawl", "prone_up", "prone_fire", "crouch_idle", "crouch_aim",
    "crouch_up", "crouch_walk_rifle", "land_hard", "run_jump", "walk_rifle", "aim_idle", "pistol_walk",
    "pistol_run", "zombie_run", "zombie_attack", "zombie_hit", "hit_small",
];

/// The cast: one Mixamo model each (see tools/make_characters.py).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Cast {
    /// The Swat Man, the hero.
    Hero,
    /// Granmèt Nwit (Romero): dreadlocks folded into a bun.
    Nwit,
    /// Madanm Flanm (Elizabeth): heavy, in a swimsuit, on narcotics.
    Flanm,
    /// Tèt Wòch (Dunny): built like a wall, on heavy narcotics.
    Woch,
    /// Zonbi Manchet (Paladin): ceremonial suit and a cowboy hat.
    Manchet,
    /// Gang shooters.
    GangA,
    GangB,
    /// The three kinds of risen dead.
    Brute,
    Yaku,
    Prisoner,
}

impl Cast {
    fn file(self) -> &'static str {
        match self {
            Cast::Hero => "hero",
            Cast::Nwit => "nwit",
            Cast::Flanm => "flanm",
            Cast::Woch => "woch",
            Cast::Manchet => "manchet",
            Cast::GangA => "gang_a",
            Cast::GangB => "gang_b",
            Cast::Brute => "brute",
            Cast::Yaku => "yaku",
            Cast::Prisoner => "prisoner",
        }
    }

    /// (height of the model in its file, its hip height in cm, height in the game)
    fn fit(self) -> (f32, f32, f32) {
        match self {
            Cast::Hero => (1.78, 96.37, 1.80),
            Cast::Nwit => (1.773, 105.16, 1.78),
            Cast::Flanm => (1.773, 99.97, 1.72),
            Cast::Woch => (1.763, 95.4, 1.95),
            Cast::Manchet => (1.724, 95.58, 1.90),
            Cast::GangA => (3.785, 209.15, 1.80),
            Cast::GangB => (1.827, 96.63, 1.82),
            Cast::Brute => (2.182, 121.86, 2.10),
            Cast::Yaku => (2.061, 110.26, 1.85),
            Cast::Prisoner => (2.032, 108.79, 1.78),
        }
    }

    /// A gang shooter's look, chosen from where he stands.
    pub fn gang(at: Vec3) -> Cast {
        if ((at.x * 7.0 + at.z * 13.0).floor() as i32).rem_euclid(2) == 0 {
            Cast::GangA
        } else {
            Cast::GangB
        }
    }
}

/// Which body a character wears. Insert it again (before the next frame) to choose
/// another; `spawn_humanoid` starts everybody as a gang shooter.
#[derive(Component)]
pub struct Skin(pub Cast);

/// The body has been attached to this root.
#[derive(Component)]
pub struct Dressed;

/// Far from the player: the body is hidden and its animation paused (150 skinned bodies are too heavy otherwise).
#[derive(Component)]
pub struct Dormant;

/// Change a character's body later (the fallen rising at night as the dead): the old body is
/// removed and the new one attached next frame.
#[derive(Component)]
pub struct ReskinTo(pub Cast);

pub fn reskin(
    mut commands: Commands,
    marked: Query<(Entity, &ReskinTo)>,
    bodies: Query<(Entity, &Body)>,
) {
    for (e, to) in &marked {
        for (b, body) in &bodies {
            if body.root == e {
                commands.entity(b).despawn();
            }
        }
        commands.entity(e).remove::<(Dressed, ReskinTo)>().insert(Skin(to.0));
    }
}

/// Freeze a character in one frame of one clip (key art, posters): `animate` leaves it alone and
/// `poster_pose` plays the clip, seeks to `t` seconds and pauses.
#[derive(Component)]
pub struct PosterPose {
    pub clip: &'static str,
    pub t: f32,
}

#[derive(Resource, Clone)]
pub struct Rig {
    graph: Handle<AnimationGraph>,
    nodes: HashMap<&'static str, AnimationNodeIndex>,
}

impl Rig {
    pub fn node(&self, name: &str) -> Option<AnimationNodeIndex> {
        self.nodes.get(name).copied()
    }
}

impl Pilot {
    pub fn root(&self) -> Entity {
        self.root
    }
}

/// Hold posed characters on their frame (see `PosterPose`).
pub fn poster_pose(
    rig: Option<Res<Rig>>,
    mut commands: Commands,
    poses: Query<&PosterPose>,
    mut pilots: Query<(Entity, &Pilot, &mut AnimationPlayer, Option<&mut AnimationTransitions>)>,
    mut done: Local<std::collections::HashSet<Entity>>,
) {
    let Some(rig) = rig else { return };
    for (e, pilot, mut player, tr) in &mut pilots {
        let Ok(pose) = poses.get(pilot.root()) else { continue };
        if tr.is_some() {
            // from now on `animate` skips this body
            commands.entity(e).remove::<AnimationTransitions>();
            continue;
        }
        if done.contains(&e) {
            continue;
        }
        let Some(node) = rig.node(pose.clip) else { continue };
        player.stop_all();
        let a = player.play(node);
        a.seek_to(pose.t);
        a.pause();
        done.insert(e);
    }
}

pub fn setup_rig(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    let mut graph = AnimationGraph::new();
    let mut nodes = HashMap::new();
    for name in CLIPS {
        let clip: Handle<AnimationClip> =
            assets.load(GltfAssetLabel::Animation(0).from_asset(format!("mixamo/{name}.glb")));
        nodes.insert(name, graph.add_clip(clip, 1.0, graph.root));
    }
    commands.insert_resource(Rig {
        graph: graphs.add(graph),
        nodes,
    });
}

/// Where weapons are held: a child of the right hand bone.
#[derive(Component)]
pub struct Socket {
    pub root: Entity,
}

/// On the model's scene entity: who it belongs to and how it was scaled to fit.
#[derive(Component)]
pub struct Body {
    root: Entity,
    scale: f32,
}

/// On the entity that holds the `AnimationPlayer`: the owner and what is playing.
#[derive(Component)]
pub struct Pilot {
    root: Entity,
    clip: &'static str,
    fire: f32,
    last_kick: f32,
    /// 0 standing, 1 crouched, 2 prone: the last stance seen (a change plays a transition clip).
    stance: u8,
    /// A stance-change clip in progress: its name and the seconds left.
    trans_clip: &'static str,
    trans: f32,
    /// After getting up from prone, stand up from the crouch as well.
    chain: bool,
    /// The longest time in the air during this jump, and the seconds left of a hard-landing clip.
    air_seen: f32,
    land: f32,
}

/// Spawn a person (a gang shooter unless `Skin` is changed right after). The body
/// appears once its glTF has loaded; the character faces local -Z and stands on the
/// root's origin.
#[allow(clippy::too_many_arguments)]
pub fn spawn_humanoid(
    commands: &mut Commands,
    _meshes: &mut Assets<Mesh>,
    _mats: &mut Assets<StandardMaterial>,
    _ink_res: &Ink,
    at: Vec3,
    yaw: f32,
    _shirt: f32,
    _pants: f32,
    _skin: f32,
) -> Entity {
    commands
        .spawn((
            Transform::from_translation(at).with_rotation(Quat::from_rotation_y(yaw)),
            Visibility::default(),
            Mover {
                grounded: true,
                ..default()
            },
            Skin(Cast::gang(at)),
            LevelEntity,
        ))
        .id()
}

/// Put the chosen model on every character that has none yet.
pub fn attach_bodies(
    mut commands: Commands,
    assets: Res<AssetServer>,
    roots: Query<(Entity, &Skin), Without<Dressed>>,
) {
    for (root, skin) in &roots {
        let (file_h, hips, want_h) = skin.0.fit();
        let s = want_h / file_h;
        // The shared clips were made for a skeleton whose hips sit at 104 cm: lift or
        // lower the body so longer or shorter legs still reach the floor.
        let lift = 0.95 * s * (hips / 104.27 - 1.0);
        let scene = assets.load(GltfAssetLabel::Scene(0).from_asset(format!("mixamo/{}.glb", skin.0.file())));
        let body = commands
            .spawn((
                WorldAssetRoot(scene),
                // Mixamo characters face +Z; ours face -Z.
                Transform {
                    translation: Vec3::Y * lift,
                    rotation: Quat::from_rotation_y(std::f32::consts::PI),
                    scale: Vec3::splat(s),
                },
                Visibility::default(),
                Body { root, scale: s },
            ))
            .observe(body_ready)
            .id();
        commands.entity(root).insert(Dressed).add_child(body);
    }
}

#[allow(clippy::too_many_arguments)]
fn body_ready(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    rig: Res<Rig>,
    bodies: Query<&Body>,
    children: Query<&Children>,
    players: Query<(), With<AnimationPlayer>>,
    names: Query<&Name>,
    skins: Query<&Skin>,
    meshes: Query<(), With<Mesh3d>>,
) {
    let Ok(b) = bodies.get(ready.entity) else { return };
    // The risen come in crowds: their shadows are not worth four extra draws each.
    let crowd = skins.get(b.root).is_ok_and(|s| matches!(s.0, Cast::Brute | Cast::Yaku | Cast::Prisoner));
    for d in children.iter_descendants(ready.entity) {
        if crowd && meshes.contains(d) {
            commands.entity(d).insert(bevy::light::NotShadowCaster);
        }
        if players.contains(d) {
            commands.entity(d).insert((
                AnimationGraphHandle(rig.graph.clone()),
                AnimationTransitions::new(),
                Pilot { root: b.root, clip: "", fire: 0.0, last_kick: 0.0, stance: 0, trans_clip: "", trans: 0.0, chain: false, air_seen: 0.0, land: 0.0 },
            ));
        }
        let Ok(n) = names.get(d) else { continue };
        if n.as_str() == "mixamorig:RightHand" {
            // bones live in centimetres (the armature is scaled by 0.01), and the whole
            // body was scaled to fit, so undo both for the weapon
            commands.spawn((
                Socket { root: b.root },
                Transform::from_scale(Vec3::splat(100.0 / b.scale)),
                Visibility::default(),
                ChildOf(d),
            ));
        }
    }
}

/// Pick the clip that matches what the person is doing and blend into it.
pub fn animate(
    time: Res<Time>,
    rig: Option<Res<Rig>>,
    mut movers: Query<&mut Mover>,
    dormant: Query<(), With<Dormant>>,
    mut pilots: Query<(&mut Pilot, &mut AnimationPlayer, &mut AnimationTransitions)>,
) {
    let dt = time.delta_secs();
    for mut mv in &mut movers {
        mv.slash = (mv.slash - 4.0 * dt).max(0.0);
        mv.flinch = (mv.flinch - 2.2 * dt).max(0.0);
    }
    let Some(rig) = rig else { return };
    for (mut pilot, mut player, mut tr) in &mut pilots {
        if dormant.contains(pilot.root) {
            if !player.all_paused() {
                player.pause_all();
            }
            continue;
        } else if player.all_paused() {
            player.resume_all();
        }
        let Ok(mv) = movers.get(pilot.root) else { continue };
        let sp = mv.speed;
        // a fresh shot restarts the short recoil clip
        if mv.kick > pilot.last_kick + 0.3 {
            pilot.fire = 0.3;
        }
        pilot.last_kick = mv.kick;
        pilot.fire = (pilot.fire - dt).max(0.0);

        // Stance changes (stand / crouch / prone) play a short transition clip first.
        let stance_now: u8 = if mv.prone_on { 2 } else if mv.crouch > 0.5 { 1 } else { 0 };
        if stance_now != pilot.stance && !mv.zombie {
            let (c, secs, chain) = match (pilot.stance, stance_now) {
                (_, 2) => ("prone_down", 0.95, false),
                (2, 1) => ("prone_up", 0.95, false),
                (2, _) => ("prone_up", 0.95, true),
                (1, 0) => ("crouch_up", 0.6, false),
                (0, 1) => ("crouch_down", 0.6, false),
                _ => ("", 0.0, false),
            };
            pilot.trans_clip = c;
            pilot.trans = secs;
            pilot.chain = chain;
        }
        pilot.stance = stance_now;
        if pilot.trans > 0.0 {
            pilot.trans -= dt;
            if pilot.trans <= 0.0 && pilot.chain {
                pilot.chain = false;
                pilot.trans_clip = "crouch_up";
                pilot.trans = 0.6;
            }
        }
        // A long drop ends with a hard landing.
        if !mv.grounded {
            pilot.air_seen = pilot.air_seen.max(mv.air);
        } else {
            if pilot.air_seen > 0.9 {
                pilot.land = 1.0;
            }
            pilot.air_seen = 0.0;
        }
        pilot.land = (pilot.land - dt).max(0.0);
        let even = pilot.root.index().index() % 2 == 0;

        // (clip, playback rate, loops, blend seconds)
        let (clip, rate, looped, blend): (&'static str, f32, bool, f32) = if mv.zombie {
            if mv.flinch > 0.0 {
                ("zombie_hit", 1.5, false, 0.05)
            } else if mv.slash > 0.0 {
                if even { ("zombie_bite", 1.6, false, 0.08) } else { ("zombie_attack", 2.2, false, 0.08) }
            } else if sp > 2.6 {
                ("zombie_run", (sp / 4.5).clamp(0.8, 1.8), true, 0.15)
            } else if sp > 0.3 {
                ("zombie_walk", (sp / 1.5).clamp(0.8, 2.4), true, 0.2)
            } else {
                ("zombie_idle", 1.0, true, 0.25)
            }
        } else if !mv.grounded {
            if mv.air > 0.9 {
                ("falling", 1.0, true, 0.2) // a long drop
            } else if mv.sprinting && sp > 4.5 {
                ("run_jump", 1.25, false, 0.08) // a running leap
            } else {
                ("jump", 1.55, false, 0.1) // the whole hop, landing included, fitted to the air time
            }
        } else if pilot.land > 0.0 {
            ("land_hard", 2.0, false, 0.06)
        } else if mv.flinch > 0.0 {
            if even { ("hit", 1.4, false, 0.04) } else { ("hit_small", 1.4, false, 0.04) }
        } else if pilot.trans > 0.0 && !pilot.trans_clip.is_empty() {
            (pilot.trans_clip, if pilot.trans_clip == "prone_down" || pilot.trans_clip == "prone_up" { 2.0 } else { 1.6 }, false, 0.1)
        } else if mv.reloading && mv.armed {
            ("reload", 1.5, false, 0.12)
        } else if mv.slash > 0.0 && mv.swing == Swing::Stab {
            ("stab", 2.4, false, 0.08)
        } else if mv.slash > 0.0 && mv.swing == Swing::Punch {
            ("punch", 2.2, false, 0.06)
        } else if mv.slash > 0.0 && mv.swing == Swing::Throw {
            ("grenade", 1.8, false, 0.1)
        } else if mv.prone_on {
            if pilot.fire > 0.0 && mv.armed {
                ("prone_fire", 1.0, false, 0.05)
            } else if sp > 0.2 {
                ("prone_crawl", (sp / 0.8).clamp(0.6, 1.6), true, 0.2)
            } else {
                ("prone_idle", 1.0, true, 0.3)
            }
        } else if pilot.fire > 0.0 && mv.armed {
            ("fire_rifle", 1.0, false, 0.05)
        } else if mv.crouch > 0.5 {
            if sp > 0.4 {
                if mv.armed {
                    ("crouch_walk_rifle", (sp / 1.2).clamp(0.6, 1.8), true, 0.2)
                } else {
                    ("crouch_walk", (sp / 1.4).clamp(0.6, 1.8), true, 0.2)
                }
            } else if mv.armed {
                ("crouch_aim", 1.0, true, 0.25)
            } else {
                ("crouch_idle", 1.0, true, 0.25)
            }
        } else if sp > 0.3 {
            if sp < 2.4 {
                if mv.pistol && mv.armed {
                    ("pistol_walk", (sp / 1.6).clamp(0.8, 2.0), true, 0.2)
                } else if mv.aiming && mv.armed {
                    ("walk_rifle", (sp / 1.4).clamp(0.7, 1.6), true, 0.2)
                } else {
                    ("walk", sp.clamp(0.7, 2.0), true, 0.2)
                }
            } else if mv.pistol && mv.armed {
                ("pistol_run", (sp / 3.1).clamp(0.8, 2.2), true, 0.2)
            } else {
                ("run", (sp / 3.0).clamp(0.8, 2.4), true, 0.2)
            }
        } else if mv.armed && mv.aiming {
            ("aim_idle", 1.0, true, 0.2)
        } else if mv.armed {
            ("rifle_idle", 1.0, true, 0.25)
        } else {
            ("idle", 1.0, true, 0.25)
        };
        let Some(&node) = rig.nodes.get(clip) else { continue };
        if pilot.clip != clip {
            pilot.clip = clip;
            let a = tr.play(&mut player, node, Duration::from_secs_f32(blend));
            if looped {
                a.repeat();
            } else {
                a.set_repeat(RepeatAnimation::Never);
            }
        }
        if let Some(a) = player.animation_mut(node) {
            a.set_speed(rate);
        }
    }
}

#[cfg(test)]
mod cover_tests {
    use super::*;

    fn wall(h: f32) -> Boxes {
        // a wall 4 m wide, 0.3 m thick, `h` tall, at z = 0
        Boxes(vec![Box3 { c: Vec3::new(0.0, h / 2.0, 0.0), h: Vec3::new(2.0, h / 2.0, 0.15) }])
    }

    fn visible(boxes: &Boxes, shooter: Vec3, feet: Vec3, crouch: f32, prone: f32) -> usize {
        body_points(feet, crouch, prone)
            .iter()
            .filter(|p| !segment_blocked(shooter, **p, &boxes.0))
            .count()
    }

    #[test]
    fn height_decides_who_is_covered() {
        let gun = Vec3::new(0.0, 1.5, 8.0); // a gunman 8 m from the wall, eye height
        let feet = Vec3::new(0.0, 0.0, -0.8); // Ro right behind the wall
        // waist-high (0.7 m): standing and crouched are partly seen, lying down is hidden
        let low = wall(0.7);
        assert_eq!(visible(&low, gun, feet, 0.0, 0.0), 2, "waist-high cover hides only the legs of a standing man");
        assert!(visible(&low, gun, feet, 1.0, 0.0) >= 2);
        assert_eq!(visible(&low, gun, feet, 0.0, 1.0), 0);
        // chest-high (1.2 m): standing shows the head, crouching hides everything
        let mid = wall(1.2);
        let standing = visible(&mid, gun, feet, 0.0, 0.0);
        assert!(standing >= 1 && standing < 3, "standing behind chest-high cover shows only the head, saw {standing}");
        assert_eq!(visible(&mid, gun, feet, 1.0, 0.0), 0);
        // a tall wall hides everyone
        let tall = wall(2.5);
        assert_eq!(visible(&tall, gun, feet, 0.0, 0.0), 0);
    }

    #[test]
    fn a_narrow_post_covers_less_than_a_wall() {
        let gun = Vec3::new(0.0, 1.5, 8.0);
        let post = Boxes(vec![Box3 { c: Vec3::new(0.0, 1.0, 0.0), h: Vec3::new(0.15, 1.0, 0.15) }]);
        let wide = wall(2.0);
        let feet_side = Vec3::new(0.8, 0.0, -1.5); // Ro a step to the side
        assert!(visible(&post, gun, feet_side, 0.0, 0.0) > visible(&wide, gun, feet_side, 0.0, 0.0));
    }

    #[test]
    fn bullets_stop_on_cover_and_prone_is_a_smaller_target() {
        let muzzle = Vec3::new(0.0, 1.5, 8.0);
        let feet = Vec3::new(0.0, 0.0, -1.5);
        let aim = Vec3::new(0.0, 0.5, -1.5);
        let dir = (aim - muzzle).normalize();
        assert!(ray_person(muzzle, dir, feet, 0.0, 1.0).is_some());
        let high = (Vec3::new(0.0, 1.5, -1.5) - muzzle).normalize();
        assert!(ray_person(muzzle, high, feet, 0.0, 1.0).is_none(), "a bullet at head height flies over a man lying down");
    }
}
