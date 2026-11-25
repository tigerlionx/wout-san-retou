//! Intelligent bots. Each guard has a vision cone with line of sight, hearing
//! (footsteps and gunfire), and a memory. They recognize the player, and they
//! recognize sabotage ("targets") when they find it, then react: patrol, grow
//! suspicious, investigate, chase, search, return to patrol. Hunting guards
//! stop and shoot back (accuracy drops with distance, cover, crouching and
//! movement), and they call out to nearby guards. A sniper does the same from
//! a fixed nest and takes time to aim, so you can break line of sight.

use crate::character::{body_points, ray_box, ray_person, resolve, segment_blocked, Box3, Boxes, Mover};
use crate::weapons::{
    dust_puff, muzzle_flash, spark_burst, tracer, Fx, Health, Noises, Rng, Shake,
};
use crate::*;
use std::f32::consts::FRAC_PI_2;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AiState {
    Patrol,
    Suspicious,
    Investigate,
    Chase,
    Search,
}

impl AiState {
    pub fn label(self) -> &'static str {
        match self {
            AiState::Patrol => "patrolling",
            AiState::Suspicious => "suspicious",
            AiState::Investigate => "investigating",
            AiState::Chase => "CHASING",
            AiState::Search => "searching",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GuardKind {
    Walker,
    Sniper,
    /// Granmèt Nwit: cannot be killed; being hit makes him vanish.
    Phantom,
}

#[derive(Component)]
pub struct Guard {
    pub name: &'static str,
    pub age: u8,
    pub kind: GuardKind,
    pub status: LostStatus,
    pub state: AiState,
    pub hp: f32,
    /// 0 = oblivious, 1 = has recognized the player.
    pub awareness: f32,
    pub last_known: Vec3,
    pub timer: f32,
    pub unseen: f32,
    pub sees: bool,
    pub aim: f32,
    /// Seconds until the next shot.
    pub shot_cd: f32,
    /// Damage per hit and the base gap between shots (bosses hit harder and faster).
    pub dmg: f32,
    pub fire_gap: f32,
    /// Damage multiplier from armor (1.0 = none). Below 0.5 the knife does nothing.
    pub armor: f32,
    /// A phantom who can be hurt (Granmèt Nwit in his own quarters).
    pub mortal: bool,
    /// Melee-only fighters (the Sword Man) run at you and cut.
    pub melee: bool,
    pub max_hp: f32,
    /// Health taken by one gun bullet (0 = the weapon's own damage). Lets a boss be
    /// "ten bullets to the chest" or "a thousand bullets" exactly.
    pub bullet_hp: f32,
    /// One headshot kills outright.
    pub head_kills: bool,
    /// A named boss (hits hard, cannot be woken by name).
    pub boss: bool,
    /// The risen dead: shamble, bite, and use the zombie clips.
    pub zombie: bool,
    /// Wounds that bleed soul (the Sword Man's blade).
    pub bleeds: bool,
    /// Health this character cannot be hurt below (Nwit while his reserve lasts).
    pub hp_floor: f32,
    /// Chase speed multiplier; grows as a melee boss is wounded.
    pub speed_mul: f32,
    /// Floor height this character stands on (the sniper's roof, a pit...).
    pub home_y: f32,
    /// Frozen briefly after a hit.
    pub stagger: f32,
    /// 1.0 when a phantom was just hit (makes him retreat).
    pub pain: f32,
    /// 0 upright .. 1 lying down.
    pub fall: f32,
    pub fall_yaw: f32,
    pub route: Vec<Vec3>,
    pub wp: usize,
    pub home_yaw: f32,
    /// Field of view in radians (full cone).
    pub fov: f32,
    pub range: f32,
    /// Eye height above the character's feet.
    pub eye: f32,
    pub marker: Handle<StandardMaterial>,
}

impl Guard {
    /// A standing guard with sensible defaults; override fields with `..` syntax.
    pub fn base(name: &'static str, age: u8, kind: GuardKind) -> Self {
        Guard {
            name,
            age,
            kind,
            status: LostStatus::Standing,
            state: AiState::Patrol,
            hp: 100.0,
            awareness: 0.0,
            last_known: Vec3::ZERO,
            timer: 0.0,
            unseen: 0.0,
            sees: false,
            aim: 0.0,
            shot_cd: 0.0,
            dmg: 15.0,
            fire_gap: 1.0,
            armor: 1.0,
            mortal: false,
            melee: false,
            max_hp: 100.0,
            bullet_hp: 0.0,
            head_kills: false,
            boss: false,
            zombie: false,
            bleeds: false,
            hp_floor: 0.0,
            speed_mul: 1.0,
            home_y: 0.0,
            stagger: 0.0,
            pain: 0.0,
            fall: 0.0,
            fall_yaw: 0.0,
            route: Vec::new(),
            wp: 0,
            home_yaw: 0.0,
            fov: 100.0_f32.to_radians(),
            range: 14.0,
            eye: 1.5,
            marker: Handle::default(),
        }
    }

    /// Madanm Flanm (Elizabeth): on narcotics. One headshot kills her; ten bullets to the
    /// chest put her down.
    pub fn elizabeth() -> Self {
        Guard {
            hp: 10.0,
            max_hp: 10.0,
            bullet_hp: 1.0,
            head_kills: true,
            boss: true,
            fire_gap: 0.7,
            range: 30.0,
            fov: 150.0_f32.to_radians(),
            ..Guard::base("Madanm Flanm", 41, GuardKind::Walker)
        }
    }

    /// Tèt Wòch (Dunny): on heavy narcotics, built like a wall. One headshot kills him;
    /// twenty bullets to the body put him down.
    pub fn tet_woch() -> Self {
        Guard {
            hp: 20.0,
            max_hp: 20.0,
            bullet_hp: 1.0,
            head_kills: true,
            boss: true,
            fire_gap: 0.62,
            range: 30.0,
            fov: 150.0_f32.to_radians(),
            ..Guard::base("Tèt Wòch", 38, GuardKind::Walker)
        }
    }

    /// Zonbi Manchet (Paladin), the Sword Man: a thousand bullets, headshot or not.
    pub fn manchet(name: &'static str, age: u8) -> Self {
        Guard {
            hp: 1000.0,
            max_hp: 1000.0,
            bullet_hp: 1.0,
            boss: true,
            melee: true,
            bleeds: true,
            dmg: 32.0,
            fire_gap: 0.4,
            range: 40.0,
            ..Guard::base(name, age, GuardKind::Walker)
        }
    }

    /// A risen enemy. `bullets` is how many rounds get rid of it; `hit` is the share of
    /// Ro's health each of its blows takes, in percent.
    pub fn risen(name: &'static str, age: u8, bullets: f32, hit: f32) -> Self {
        Guard {
            hp: bullets,
            max_hp: bullets,
            bullet_hp: 1.0,
            melee: true,
            zombie: true,
            dmg: hit,
            fire_gap: 1.0,
            range: 30.0,
            awareness: 1.0,
            state: AiState::Chase,
            ..Guard::base(name, age, GuardKind::Walker)
        }
    }

    /// Bosses hit hard and fast; waking by name does not work on them.
    pub fn is_boss(&self) -> bool {
        self.boss || self.fire_gap < 0.5
    }

    /// Health lost to one of Ro's gun bullets that lands on this person.
    pub fn gun_damage(&self, head: bool, weapon_dmg: f32) -> f32 {
        if self.bullet_hp > 0.0 {
            if head && self.head_kills {
                self.max_hp * 2.0
            } else {
                self.bullet_hp
            }
        } else if head {
            100.0
        } else {
            weapon_dmg
        }
    }

    /// A knife cut (a 34-point hit is one bullet) or a blast (10 points to a bullet).
    pub fn scaled_damage(&self, raw: f32, points_per_bullet: f32) -> f32 {
        if self.bullet_hp > 0.0 {
            raw / points_per_bullet * self.bullet_hp
        } else {
            raw
        }
    }

    pub fn kind_is_phantom(&self) -> bool {
        self.kind == GuardKind::Phantom
    }
}

/// Ro wears a helmet and a vest: six bullets put him down...
pub const BULLET_HIT: f32 = 100.0 / 6.0;
/// ...but a bullet to the helmet takes half his health, so two headshots kill.
pub const HEAD_HIT: f32 = 50.0;

/// A shot lands on Ro. Returns true when it hit the helmet.
pub fn hit_hero(health: &mut Health, rng: &mut Rng, head_chance: f32) -> bool {
    let head = rng.next() < head_chance;
    health.hp -= if head { HEAD_HIT } else { BULLET_HIT };
    health.hurt = 1.0;
    head
}

/// How much of the hero a guard at `eye` can see through the cover in between (0.4 head, 0.4 chest,
/// 0.2 legs), with the distance to the nearest visible part.
fn sees_body(eye: Vec3, facing: Vec3, fov: f32, range: f32, pts: &[Vec3; 3], boxes: &[Box3]) -> Option<(f32, f32)> {
    let mut dist = None;
    let mut vis = 0.0;
    for (p, w) in pts.iter().zip([0.4, 0.4, 0.2]) {
        if let Some(d) = sees(eye, facing, fov, range, *p, boxes) {
            vis += w;
            dist.get_or_insert(d);
        }
    }
    dist.map(|d| (d, vis))
}

/// Where an enemy bullet ends up, and what it hit.
struct Impact {
    end: Vec3,
    normal: Vec3,
    /// Some(true) when it hit Ro's head, Some(false) his body, None when it hit the world.
    hero: Option<bool>,
}

/// One enemy bullet: aimed at `aim` with an angular error of up to `spread` radians, it flies on until it
/// meets the first thing in its path: a wall, a crate or a car (so cover takes the bullet), the ground, or Ro.
#[allow(clippy::too_many_arguments)]
fn enemy_bullet(muzzle: Vec3, aim: Vec3, spread: f32, rng: &mut Rng, boxes: &[Box3], ppos: Vec3, crouch: f32, prone: f32) -> Impact {
    let mut dir = (aim - muzzle).normalize_or_zero();
    if dir == Vec3::ZERO {
        dir = Vec3::NEG_Z;
    }
    let right = dir.cross(Vec3::Y).normalize_or_zero();
    dir = (Quat::from_rotation_y(rng.signed() * spread) * Quat::from_axis_angle(right, rng.signed() * spread * 0.7) * dir).normalize();
    let mut best = 90.0_f32;
    let mut normal = Vec3::Y;
    if dir.y < -1e-3 && muzzle.y > 0.0 {
        let t = -muzzle.y / dir.y;
        if t < best {
            best = t;
        }
    }
    for b in boxes {
        if let Some((t, n)) = ray_box(muzzle, dir, b) {
            if t < best {
                best = t;
                normal = n;
            }
        }
    }
    let mut hero = None;
    if let Some((t, head)) = ray_person(muzzle, dir, ppos, crouch, prone) {
        if t < best {
            best = t;
            hero = Some(head);
        }
    }
    Impact { end: muzzle + dir * best, normal, hero }
}

/// Ro takes a bullet that really hit him; the helmet is a head hit.
fn hero_hit(health: &mut Health, head: bool) {
    health.hp -= if head { HEAD_HIT } else { BULLET_HIT };
    health.hurt = 1.0;
}

const PATROL_SPEED: f32 = 1.9;
const INVESTIGATE_SPEED: f32 = 3.4;
const CHASE_SPEED: f32 = 5.4;

fn flat(v: Vec3) -> Vec3 {
    Vec3::new(v.x, 0.0, v.z)
}

pub fn face(tf: &mut Transform, dir: Vec3, rate: f32, dt: f32) {
    let d = flat(dir);
    if d.length_squared() < 1e-6 {
        return;
    }
    let yaw = (-d.x).atan2(-d.z);
    tf.rotation = tf
        .rotation
        .slerp(Quat::from_rotation_y(yaw), (rate * dt).min(1.0));
}

/// Move toward `target` on the ground, colliding with the world. Returns the
/// distance that remained before the step.
pub fn walk_to(
    tf: &mut Transform,
    mv: &mut Mover,
    target: Vec3,
    speed: f32,
    dt: f32,
    boxes: &[Box3],
) -> f32 {
    let to = flat(target - tf.translation);
    let dist = to.length();
    if dist < 0.05 {
        mv.speed = 0.0;
        return dist;
    }
    let dir = to / dist;
    face(tf, dir, 10.0, dt);
    let step = (speed * dt).min(dist);
    let before = tf.translation;
    tf.translation = resolve(before + dir * step, 0.4, boxes);
    mv.speed = (tf.translation - before).length() / dt.max(1e-4);
    dist
}

/// Can a guard at `eye` facing `facing` see `target` right now? Returns the
/// distance if so.
fn sees(
    eye: Vec3,
    facing: Vec3,
    fov: f32,
    range: f32,
    target: Vec3,
    boxes: &[Box3],
) -> Option<f32> {
    let to = target - eye;
    let d = to.length();
    if d > range {
        return None;
    }
    let to_flat = flat(to).normalize_or_zero();
    // Close range is felt even outside the cone.
    if d > 1.8 && facing.dot(to_flat) < (fov * 0.5).cos() {
        return None;
    }
    if segment_blocked(eye, target, boxes) {
        return None;
    }
    Some(d)
}

#[allow(clippy::too_many_arguments)]
pub fn think(
    time: Res<Time>,
    only: Res<AiOnly>,
    boxes: Res<Boxes>,
    fx: Res<Fx>,
    mut commands: Commands,
    mut rng: ResMut<Rng>,
    mut noises: ResMut<Noises>,
    mut shake: ResMut<Shake>,
    mut health: ResMut<Health>,
    mut bleed: ResMut<crate::level5::Bleed>,
    mut objectives: ResMut<Objectives>,
    mut msg: ResMut<Message>,
    targets: Query<&SabotageTarget>,
    player: Single<(&Transform, &Mover), (With<Player>, Without<Guard>)>,
    mut guards: Query<(&mut Guard, &mut Transform, &mut Mover), Without<Player>>,
) {
    let dt = time.delta_secs().min(0.1);
    let (pt, pm) = player.into_inner();
    let ppos = pt.translation;
    let pts = body_points(ppos, pm.crouch, pm.prone);
    let head = pts[0];
    // Standing still and crouching are harder to notice; running is easy.
    let stance = if pm.prone_on {
        0.25
    } else if pm.crouch > 0.5 {
        0.45
    } else if pm.sprinting {
        1.6
    } else if pm.moving {
        1.0
    } else {
        0.7
    };

    let mut chase_started: Vec<Vec3> = Vec::new();
    let mut alarm: Option<(Vec3, &'static str, usize)> = None;
    let heard_noises: Vec<(Vec3, f32)> = std::mem::take(&mut noises.0);
    let mut fired: Vec<Vec3> = Vec::new();

    for (mut g, mut tf, mut mv) in &mut guards {
        if g.status != LostStatus::Standing {
            mv.speed = 0.0;
            mv.armed = false;
            continue;
        }
        if g.kind == GuardKind::Phantom {
            continue; // driven by level2::nwit_ai
        }
        if let Some(name) = only.0 {
            if name != g.name {
                mv.speed = 0.0;
                continue;
            }
        }
        g.shot_cd = (g.shot_cd - dt).max(0.0);
        if g.stagger > 0.0 {
            // Reeling from a hit.
            g.stagger -= dt;
            mv.speed = 0.0;
            continue;
        }
        let eye = tf.translation + Vec3::Y * g.eye;
        let facing = flat(tf.forward().as_vec3()).normalize_or_zero();

        // --- Recognize sabotage (the "targets" of the player's work).
        if g.kind == GuardKind::Walker && g.state != AiState::Chase {
            for t in &targets {
                if objectives.done[t.idx]
                    && !objectives.discovered[t.idx]
                    && sees(eye, facing, g.fov, 14.0, t.pos + Vec3::Y * 0.8, &boxes.0).is_some()
                {
                    objectives.discovered[t.idx] = true;
                    alarm = Some((t.pos, g.name, t.idx));
                }
            }
        }

        // --- See the player.
        let seen = sees_body(eye, facing, g.fov, g.range, &pts, &boxes.0);
        g.sees = seen.is_some();
        if let Some((d, vis)) = seen {
            let k = (1.0 - d / g.range).clamp(0.0, 1.0).powf(0.6).max(0.15);
            let close = if d < 2.5 { 3.0 } else { 1.0 };
            g.awareness = (g.awareness + 0.8 * k * stance * close * (0.4 + 0.6 * vis) * dt).min(1.0);
            g.last_known = ppos;
            g.unseen = 0.0;
        } else {
            g.unseen += dt;
            let floor = match g.state {
                AiState::Investigate | AiState::Search | AiState::Chase => 0.5,
                _ => 0.0,
            };
            let lowered = g.awareness - 0.12 * dt;
            g.awareness = if g.awareness > floor {
                lowered.max(floor)
            } else {
                lowered.max(0.0)
            };
        }

        // --- Hear footsteps and loud events (walkers; the sniper is too far).
        if g.kind == GuardKind::Walker {
            if flat(ppos - tf.translation).length() < pm.noise {
                g.awareness = (g.awareness.max(0.4) + 0.25 * dt).min(1.0);
                g.last_known = ppos;
            }
            for (at, radius) in &heard_noises {
                if flat(*at - tf.translation).length() < *radius
                    && matches!(g.state, AiState::Patrol | AiState::Suspicious | AiState::Search)
                {
                    g.awareness = g.awareness.max(0.85);
                    g.last_known = *at;
                    g.state = AiState::Investigate;
                    g.timer = 0.0;
                }
            }
        }

        // --- State machine.
        let was_chasing = g.state == AiState::Chase;
        g.timer += dt;
        mv.armed = matches!(g.state, AiState::Chase | AiState::Suspicious);
        match g.state {
            AiState::Patrol => {
                if g.awareness >= 0.35 {
                    g.state = AiState::Suspicious;
                    g.timer = 0.0;
                } else if g.kind == GuardKind::Walker && !g.route.is_empty() {
                    let wp = g.route[g.wp];
                    if g.timer < 0.0 {
                        mv.speed = 0.0; // pausing at a waypoint
                    } else if walk_to(&mut tf, &mut mv, wp, PATROL_SPEED, dt, &boxes.0) < 0.6 {
                        g.wp = (g.wp + 1) % g.route.len();
                        g.timer = -1.2;
                    }
                } else if g.kind == GuardKind::Walker {
                    // An observation post: stands his ground and slowly scans.
                    mv.speed = 0.0;
                    let yaw = g.home_yaw + (time.elapsed_secs() * 0.45 + g.age as f32).sin() * 0.9;
                    tf.rotation = Quat::from_rotation_y(yaw);
                } else if g.kind == GuardKind::Sniper {
                    let yaw = g.home_yaw + (time.elapsed_secs() * 0.5).sin() * 1.0;
                    tf.rotation = Quat::from_rotation_y(yaw);
                }
            }
            AiState::Suspicious => {
                mv.speed = 0.0;
                let to = g.last_known - tf.translation;
                face(&mut tf, to, 6.0, dt);
                if g.awareness >= 1.0 {
                    g.state = AiState::Chase;
                    g.timer = 0.0;
                } else if g.awareness >= 0.7 && g.kind == GuardKind::Walker {
                    g.state = AiState::Investigate;
                    g.timer = 0.0;
                } else if g.awareness < 0.15 {
                    g.state = AiState::Patrol;
                    g.timer = 0.0;
                }
            }
            AiState::Investigate => {
                let lk = g.last_known;
                let left = walk_to(&mut tf, &mut mv, lk, INVESTIGATE_SPEED, dt, &boxes.0);
                if g.awareness >= 1.0 {
                    g.state = AiState::Chase;
                    g.timer = 0.0;
                } else if left < 1.2 {
                    g.state = AiState::Search;
                    g.timer = 0.0;
                }
            }
            AiState::Chase => {
                let dist = flat(ppos - tf.translation).length();
                if g.melee {
                    // Melee fighters (the Sword Man, the risen): no stopping, no cover, no
                    // shooting. They run or shamble at you and strike.
                    let pace = if g.zombie { 0.55 } else { 0.9 };
                    walk_to(&mut tf, &mut mv, ppos, CHASE_SPEED * pace * g.speed_mul, dt, &boxes.0);
                    if dist < 1.7 && (ppos.y - tf.translation.y).abs() < 1.5 && g.shot_cd <= 0.0 {
                        g.shot_cd = if g.zombie { 1.0 } else { 1.2 };
                        health.hp -= g.dmg;
                        health.hurt = 1.0;
                        shake.0 = (shake.0 + 0.7).min(1.0);
                        if g.bleeds {
                            bleed.0 = 10.0;
                            msg.0 = "The blade cuts something that is not flesh. You are bleeding soul. Drink from a bottle (H).".into();
                            spark_burst(&mut commands, &fx, &mut rng, ppos + Vec3::Y, Vec3::Y, 8);
                        } else {
                            mv.slash = 1.0;
                            mv.swing = crate::character::Swing::Bite;
                            dust_puff(&mut commands, &fx, ppos + Vec3::Y, Vec3::Y, 0.15, 0.4);
                        }
                    }
                    g.unseen = 0.0;
                } else if g.kind == GuardKind::Walker {
                    if g.sees && dist < 24.0 {
                        // Stand and shoot; close the gap if far, hold if near.
                        let to = ppos - tf.translation;
                        face(&mut tf, to, 12.0, dt);
                        if dist > 11.0 {
                            walk_to(&mut tf, &mut mv, ppos, CHASE_SPEED * 0.6, dt, &boxes.0);
                        } else {
                            mv.speed = 0.0;
                        }
                        // A reaction delay, then a shot every second or so.
                        if g.timer > 0.6 && g.shot_cd <= 0.0 {
                            g.shot_cd = g.fire_gap * (0.7 + rng.next() * 0.6);
                            let muzzle = tf.translation + tf.rotation * Vec3::new(0.16, 1.22, -0.7);
                            // He aims for the chest (sometimes the head) of whatever he can see; the bullet
                            // flies for real and stops at the first cover in its way.
                            let aim = if rng.next() < if pm.crouch > 0.5 { 0.05 } else { 0.11 } { pts[0] } else { pts[1] };
                            let spread = (0.02 + 0.0022 * dist) * if pm.speed > 3.0 { 1.3 } else { 1.0 };
                            let shot = enemy_bullet(muzzle, aim, spread, &mut rng, &boxes.0, ppos, pm.crouch, pm.prone);
                            let end = shot.end;
                            tracer(&mut commands, &fx, muzzle, end);
                            let (sv, sr) = (rng.next(), rng.next());
                            fx.play(
                                &mut commands,
                                if g.fire_gap < 0.5 { crate::audio::Sfx::Ak } else { crate::audio::Sfx::Pistol },
                                (1.0 - dist / 50.0).clamp(0.12, 1.0),
                                0.94 + 0.1 * sr,
                                sv,
                            );
                            muzzle_flash(&mut commands, &fx, muzzle, 0.2);
                            if let Some(head) = shot.hero {
                                hero_hit(&mut health, head);
                                shake.0 = (shake.0 + if head { 0.6 } else { 0.3 }).min(1.0);
                                dust_puff(&mut commands, &fx, end, Vec3::Y, 0.1, 0.3);
                            } else {
                                spark_burst(&mut commands, &fx, &mut rng, end, shot.normal, 4);
                                dust_puff(&mut commands, &fx, end, shot.normal, 0.15, 0.5);
                            }
                            fired.push(tf.translation);
                        }
                    } else {
                        // No line of sight: hunt for the last known position.
                        let goal = g.last_known;
                        walk_to(&mut tf, &mut mv, goal, CHASE_SPEED, dt, &boxes.0);
                    }
                } else if g.kind == GuardKind::Sniper {
                    // Sniper: track and aim; breaking line of sight resets it.
                    let to = ppos - tf.translation;
                    face(&mut tf, to, 6.0, dt);
                    if g.sees {
                        g.aim += dt;
                        if g.aim >= 1.4 {
                            g.aim = 0.4;
                            let muzzle = tf.translation + Vec3::Y * g.eye;
                            let chance = if pm.crouch > 0.5 {
                                0.4
                            } else if pm.moving {
                                0.55
                            } else {
                                0.9
                            };
                            let aim = if rng.next() < 0.25 { pts[0] } else { pts[1] };
                            let spread = 0.006 * if pm.crouch > 0.5 { 1.2 } else if pm.moving { 3.0 } else { 1.0 };
                            let shot = enemy_bullet(muzzle, aim, spread, &mut rng, &boxes.0, ppos, pm.crouch, pm.prone);
                            let end = shot.end;
                            tracer(&mut commands, &fx, muzzle, end);
                            fx.play(&mut commands, crate::audio::Sfx::Sniper, (1.2 - dist / 70.0).clamp(0.3, 1.0), 1.0, 0.0);
                            muzzle_flash(&mut commands, &fx, muzzle, 0.35);
                            if let Some(head) = shot.hero {
                                hero_hit(&mut health, head);
                                shake.0 = 1.0;
                            } else {
                                spark_burst(&mut commands, &fx, &mut rng, end, shot.normal, 6);
                            }
                            fired.push(tf.translation);
                        }
                    } else {
                        g.aim = (g.aim - dt).max(0.0);
                    }
                }
                if g.unseen > 3.0 {
                    g.state = AiState::Search;
                    g.timer = 0.0;
                    g.aim = 0.0;
                }
            }
            AiState::Search => {
                mv.speed = 0.0;
                if g.kind == GuardKind::Walker {
                    // Look around slowly.
                    tf.rotate_y(1.1 * dt);
                } else {
                    let yaw = g.home_yaw + (time.elapsed_secs() * 0.5).sin() * 1.0;
                    tf.rotation = Quat::from_rotation_y(yaw);
                }
                if g.awareness >= 1.0 {
                    g.state = AiState::Chase;
                    g.timer = 0.0;
                } else if g.timer > 4.0 {
                    g.awareness = 0.0;
                    g.state = AiState::Patrol;
                    g.timer = 0.0;
                }
            }
        }
        if g.state == AiState::Chase && !was_chasing {
            chase_started.push(ppos);
        }
    }

    // --- Raise the alarm for found sabotage.
    if let Some((pos, finder, idx)) = alarm {
        msg.0 = format!("{finder} found the sabotage ({}). The yard is alert!", OBJECTIVES[idx]);
        for (mut g, _, _) in &mut guards {
            if g.status == LostStatus::Standing && g.state != AiState::Chase {
                g.last_known = pos;
                g.awareness = g.awareness.max(0.75);
                g.state = AiState::Investigate;
                g.timer = 0.0;
            }
        }
    }

    // --- A guard who starts chasing shouts, and gunfire carries: nearby
    //     guards come to the spot.
    let calls: Vec<(Vec3, Vec3)> = chase_started
        .into_iter()
        .map(|p| (p, p))
        .chain(fired.into_iter().map(|p| (p, ppos)))
        .collect();
    for (origin, goal) in calls {
        for (mut g, tf, _) in &mut guards {
            if g.status == LostStatus::Standing
                && g.kind == GuardKind::Walker
                && matches!(g.state, AiState::Patrol | AiState::Suspicious | AiState::Search)
                && flat(tf.translation - origin).length() < 22.0
            {
                g.last_known = goal;
                g.awareness = g.awareness.max(0.75);
                g.state = AiState::Investigate;
                g.timer = 0.0;
            }
        }
    }
}

/// The fallen collapse backwards and stay down (until night); the risen stand.
pub fn fall_animation(time: Res<Time>, mut guards: Query<(&mut Guard, &mut Transform)>) {
    let dt = time.delta_secs();
    for (mut g, mut tf) in &mut guards {
        let base_y = g.home_y;
        if g.status == LostStatus::Fallen {
            g.fall = (g.fall + 2.2 * dt).min(1.0);
            let ease = g.fall * g.fall * (3.0 - 2.0 * g.fall);
            tf.rotation =
                Quat::from_rotation_y(g.fall_yaw) * Quat::from_rotation_x(-FRAC_PI_2 * ease);
            tf.translation.y = base_y + 0.14 * ease;
        } else if g.fall > 0.0 {
            // Back on their feet.
            g.fall = 0.0;
            tf.rotation = Quat::from_rotation_y(g.fall_yaw);
            tf.translation.y = base_y;
        }
    }
}

/// Marker colours (the accent colour in a grey world), vision cones and the
/// sniper's laser.
pub fn visuals(
    guards: Query<(&Guard, &Transform)>,
    player: Single<(&Transform, &Mover), (With<Player>, Without<Guard>)>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut gizmos: Gizmos,
) {
    let (pt, pm) = player.into_inner();
    let head = pt.translation + Vec3::Y * (1.5 - 0.55 * pm.crouch - 0.6 * pm.prone);
    for (g, tf) in &guards {
        let (rgb, power) = match (g.status, g.state) {
            (LostStatus::Woken, _) => (Vec3::new(0.2, 1.0, 0.45), 3.0),
            (LostStatus::Fallen, _) => (Vec3::splat(0.2), 0.0),
            (_, AiState::Patrol) => (Vec3::splat(0.75), 0.8),
            (_, AiState::Suspicious) => (Vec3::new(1.0, 0.9, 0.2), 4.0),
            (_, AiState::Investigate) => (Vec3::new(1.0, 0.55, 0.1), 4.0),
            (_, AiState::Chase) => (Vec3::new(1.0, 0.1, 0.1), 7.0),
            (_, AiState::Search) => (Vec3::new(1.0, 0.7, 0.3), 2.5),
        };
        if let Some(mut m) = mats.get_mut(&g.marker) {
            m.base_color = Color::srgb(rgb.x, rgb.y, rgb.z);
            m.emissive = LinearRgba::rgb(rgb.x * power, rgb.y * power, rgb.z * power);
        }
        if g.status != LostStatus::Standing {
            continue;
        }
        let facing = flat(tf.forward().as_vec3()).normalize_or_zero();
        match g.kind {
            GuardKind::Walker => {
                // Show the vision cone while the guard is not yet chasing.
                if matches!(g.state, AiState::Patrol | AiState::Suspicious | AiState::Search) {
                    let from = tf.translation + Vec3::Y * 0.06;
                    for s in [-1.0_f32, 1.0] {
                        let edge = Quat::from_rotation_y(s * g.fov * 0.5) * facing;
                        gizmos.line(from, from + edge * 9.0, Color::srgb(0.95, 0.85, 0.5));
                    }
                }
            }
            GuardKind::Phantom => {}
            GuardKind::Sniper => {
                if g.sees && matches!(g.state, AiState::Suspicious | AiState::Chase) {
                    let from = tf.translation + Vec3::Y * g.eye;
                    gizmos.line(from, head, Color::srgb(1.0, 0.1, 0.1));
                }
            }
        }
    }
}
