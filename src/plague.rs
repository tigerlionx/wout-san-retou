//! The gang is a plague. Remy and James (the two gang-shooter bodies) are not a handful
//! of guards: they are everywhere. Each level starts with gang patrols scattered across
//! the map, and whenever one goes down (or is woken by name) another arrives from
//! outside your view, heading for where you were last heard. You do not clear them
//! out; you slip past them, wake them, or leave. Every kill still adds Weight.
//!
//! Armed enemies are adults (21 and up). Everyone here is fictional.

use crate::ai::{AiState, Guard, GuardKind};
use crate::character::{resolve, segment_blocked, Box3, Boxes, Cast, Skin};
use crate::style::{glow, Ink};
use crate::weapons::Rng;
use crate::*;

/// Marks a gang member sent by the plague (so the system can count and tidy them).
#[derive(Component)]
pub struct Gang {
    pub born: f32,
}

#[derive(Resource, Default)]
pub struct Plague {
    state: Option<AppState>,
    timer: f32,
    spawned: u32,
    clock: f32,
}

/// Fictional adult names (first names only, ages 21 to 36).
const NAMES: [&str; 28] = [
    "Wilson", "Jonas", "Berto", "Mackendy", "Rodrigue", "Fritz", "Ti Jak", "Nixon", "Pierrot", "Edner",
    "Gasner", "Clifford", "Ronald", "Jean-Marc", "Lucson", "Emmanuel", "Kervens", "Stevenson", "Fabrice",
    "Wooky", "Samuel", "Dorval", "Evel", "Joseph", "Marcelin", "Hervé", "Sony", "Yvenson",
];

/// Per level: (patrols placed at the start, most alive at once, seconds between arrivals).
fn config(state: AppState) -> Option<(u32, u32, f32)> {
    match state {
        AppState::Playing => Some((5, 9, 13.0)),
        AppState::Level2 => Some((5, 9, 13.0)),
        AppState::Level3 => Some((3, 6, 18.0)),
        AppState::Level4 => Some((3, 6, 18.0)),
        AppState::Level5 => Some((3, 6, 18.0)),
        AppState::Level6 => Some((0, 4, 24.0)),
        _ => None, // the rooftop finale stays a duel with Granmèt Nwit
    }
}

fn is_free(p: Vec3, boxes: &[Box3]) -> bool {
    let q = resolve(p, 0.55, boxes);
    (q.x - p.x).abs() < 0.02 && (q.z - p.z).abs() < 0.02
}

fn extent(boxes: &[Box3]) -> Option<(Vec2, Vec2)> {
    let mut lo = Vec2::splat(f32::MAX);
    let mut hi = Vec2::splat(f32::MIN);
    for b in boxes {
        // Floors and rugs do not bound the map; walls and props do.
        if b.top() <= crate::character::STEP || b.h.x > 80.0 || b.h.z > 80.0 {
            continue;
        }
        lo = lo.min(Vec2::new(b.c.x - b.h.x, b.c.z - b.h.z));
        hi = hi.max(Vec2::new(b.c.x + b.h.x, b.c.z + b.h.z));
    }
    if lo.x < hi.x && lo.y < hi.y {
        Some((lo + Vec2::splat(2.0), hi - Vec2::splat(2.0)))
    } else {
        None
    }
}

#[allow(clippy::too_many_arguments)]
pub fn plague(
    time: Res<Time>,
    state: Res<State<AppState>>,
    only: Res<AiOnly>,
    boxes: Res<Boxes>,
    glass: Option<Res<crate::level4::GlassRect>>,
    mut rng: ResMut<Rng>,
    mut plague: ResMut<Plague>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    ink_res: Res<Ink>,
    player: Single<&Transform, (With<Player>, Without<Guard>)>,
    guards: Query<(Entity, &Guard, &Transform, Option<&Gang>)>,
) {
    let st = *state.get();
    if plague.state != Some(st) {
        *plague = Plague { state: Some(st), ..default() };
    }
    let Some((seed, cap, interval)) = config(st) else { return };
    if only.0.is_some() || std::env::var_os("WSR_NOPLAGUE").is_some() {
        return;
    }
    let dt = time.delta_secs().min(0.1);
    // Level 4's glass floor over the pool: nobody is placed on it.
    let on_glass = |p: Vec3| {
        glass.as_ref().is_some_and(|g| p.x > g.min.x - 1.5 && p.x < g.max.x + 1.5 && p.z > g.min.y - 1.5 && p.z < g.max.y + 1.5)
    };
    plague.clock += dt;
    let Some((lo, hi)) = extent(&boxes.0) else {
        return;
    };
    let pp = player.translation;
    let forward = {
        let f = player.forward().as_vec3();
        Vec3::new(f.x, 0.0, f.z).normalize_or_zero()
    };

    // Count the living plague, tidy the fallen (the oldest bodies are carried off).
    let mut alive: Vec<Vec3> = Vec::new();
    let mut fallen: Vec<(f32, Entity)> = Vec::new();
    for (e, g, tf, gang) in &guards {
        if let Some(gang) = gang {
            if g.status == LostStatus::Standing {
                alive.push(tf.translation);
            } else {
                fallen.push((gang.born, e));
            }
        } else if g.status == LostStatus::Standing && g.kind == GuardKind::Walker {
            alive.push(tf.translation); // spread out from the level's own guards too
        }
    }
    if fallen.len() > 8 {
        fallen.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (_, e) in fallen.iter().take(fallen.len() - 8) {
            commands.entity(*e).despawn();
        }
    }
    let live_gang = guards
        .iter()
        .filter(|(_, g, _, gang)| gang.is_some() && g.status == LostStatus::Standing)
        .count() as u32;

    // First seconds of a level: scatter the patrols (retrying if a spot was bad).
    let seeding = plague.spawned < seed && plague.clock < 3.0;
    let want = if seeding {
        seed - plague.spawned
    } else {
        plague.timer -= dt;
        if live_gang < cap && plague.timer <= 0.0 {
            1
        } else {
            0
        }
    };
    if want == 0 {
        return;
    }

    for _ in 0..want {
        let mut placed = false;
        for try_no in 0..70 {
            // After many misses, let an arrival walk in from far away, even in front of you.
            let relaxed = try_no >= 40;
            let p = if seeding {
                // Anywhere on the map, out of sight, away from the start.
                Vec3::new(lo.x + rng.next() * (hi.x - lo.x), 0.0, lo.y + rng.next() * (hi.y - lo.y))
            } else {
                // Arrivals come from behind or beside the player.
                let a = rng.next() * std::f32::consts::TAU;
                let r = 20.0 + rng.next() * 14.0;
                Vec3::new(pp.x + a.cos() * r, 0.0, pp.z + a.sin() * r)
            };
            if p.x < lo.x || p.x > hi.x || p.z < lo.y || p.z > hi.y || !is_free(p, &boxes.0) || on_glass(p) {
                continue;
            }
            let to = Vec3::new(p.x - pp.x, 0.0, p.z - pp.z);
            let d = to.length();
            if d < if seeding { 12.0 } else if relaxed { 24.0 } else { 18.0 } {
                continue;
            }
            let in_front = to.normalize_or_zero().dot(forward) > 0.15;
            let eye_line = segment_blocked(pp + Vec3::Y * 1.4, p + Vec3::Y * 1.4, &boxes.0);
            if in_front && !eye_line && !relaxed {
                continue; // never pop into view
            }
            if alive.iter().any(|a| Vec3::new(a.x - p.x, 0.0, a.z - p.z).length() < 6.0) {
                continue; // spread them out
            }
            let waist = Vec3::Y;
            if !seeding && segment_blocked(p + waist, pp + waist, &boxes.0) {
                continue; // an arrival needs an open way to you: there is no pathfinding
            }
            // A patrol loop around the spot (seeds), or a march on the player (arrivals).
            // Two in five stand an observation post instead of walking.
            let sentry = seeding && (plague.spawned % 5 == 1 || plague.spawned % 5 == 3);
            let mut route = vec![p];
            if seeding && !sentry {
                for _ in 0..12 {
                    if route.len() >= 3 {
                        break;
                    }
                    let a = rng.next() * std::f32::consts::TAU;
                    let r = 4.0 + rng.next() * 6.0;
                    let q = Vec3::new(p.x + a.cos() * r, 0.0, p.z + a.sin() * r);
                    let last = *route.last().unwrap();
                    if q.x > lo.x
                        && q.x < hi.x
                        && q.z > lo.y
                        && q.z < hi.y
                        && is_free(q, &boxes.0)
                        && !on_glass(q)
                        && !segment_blocked(last + waist, q + waist, &boxes.0)
                    {
                        route.push(q);
                    }
                }
                if route.len() < 2 {
                    continue;
                }
            }
            let n = plague.spawned;
            plague.spawned += 1;
            let yaw = if sentry {
                // looks toward the middle of the map, where the action is
                (p.x).atan2(p.z)
            } else if seeding {
                let d = route[1] - p;
                (-d.x).atan2(-d.z)
            } else {
                to.x.atan2(to.z) // face the player
            };
            let ak = n % 4 == 3;
            let mut g = Guard::base(NAMES[(n as usize * 11 + 3) % NAMES.len()], 21 + (n * 5 % 16) as u8, GuardKind::Walker);
            g.fall_yaw = yaw;
            g.home_yaw = yaw;
            g.last_known = p;
            g.wp = 1;
            if ak {
                g.fire_gap = 0.45;
                g.dmg = 10.0;
            }
            if sentry {
                g.fov = 75.0_f32.to_radians();
                g.range = 22.0; // a watcher sees farther, through a narrower cone
            } else if seeding {
                g.route = route;
            } else {
                g.state = AiState::Investigate;
                g.awareness = 0.75;
                g.last_known = pp;
            }
            let marker = glow(&mut mats, Vec3::splat(0.75), 0.8);
            g.marker = marker.clone();
            let marker_mesh = meshes.add(Sphere::new(0.09));
            let id = crate::character::spawn_humanoid(
                &mut commands, &mut meshes, &mut mats, &ink_res, p, yaw, 0.3, 0.2, 0.3,
            );
            crate::weapons::attach_guard_pistol(&mut commands, &mut meshes, &mut mats, &ink_res, id);
            commands.entity(id).insert((
                g,
                Gang { born: plague.clock },
                // Remy and James, taking turns
                Skin(if n % 2 == 0 { Cast::GangA } else { Cast::GangB }),
            ));
            commands.entity(id).with_child((
                Mesh3d(marker_mesh),
                MeshMaterial3d(marker),
                Transform::from_xyz(0.0, 1.85, 0.0),
            ));
            alive.push(p);
            placed = true;
            break;
        }
        let _ = placed;
    }
    if !seeding {
        plague.timer = interval * (0.7 + rng.next() * 0.6);
    } else if plague.spawned >= seed {
        plague.timer = interval * 1.5;
    }
}
