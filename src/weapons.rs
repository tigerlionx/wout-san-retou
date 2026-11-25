//! Ro's gear: knife, pistol, AK-47 and grenades, plus the physical effects
//! that make them read as real: ammo and reloads, recoil and spread, hitscan
//! ballistics with headshots, tracers, muzzle flash, impacts, shell casings,
//! bouncing grenades, blasts and camera shake. Violence here has a cost:
//! every kill adds Weight, nobody gets a slow-motion reward, and the dead
//! rise again at night.

use crate::character::{ray_box, ray_guard, segment_blocked, Box3, Boxes, Mover};
use crate::style::{glow, grow_box, ink, matte, Ink};
use crate::*;
use bevy::window::PrimaryWindow;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WeaponKind {
    Knife,
    Pistol,
    Ak47,
    Grenade,
    /// A heavy belt-fed machine gun: loud, slow to carry, hard to control.
    M50,
}

impl WeaponKind {
    pub fn label(self) -> &'static str {
        match self {
            WeaponKind::Knife => "Knife",
            WeaponKind::Pistol => "Pistol",
            WeaponKind::Ak47 => "AK-47",
            WeaponKind::M50 => "M50",
            WeaponKind::Grenade => "Grenade",
        }
    }
    /// Held with the arms raised, pointing forward.
    pub fn ranged(self) -> bool {
        matches!(self, WeaponKind::Pistol | WeaponKind::Ak47 | WeaponKind::M50)
    }
}

#[derive(Resource, Clone)]
pub struct Loadout {
    pub current: WeaponKind,
    pub pistol_mag: u32,
    pub pistol_reserve: u32,
    pub ak_mag: u32,
    pub ak_reserve: u32,
    /// Tight quarters: the M50 is unwieldy (level 6).
    pub cramped: bool,
    /// Disarmed: guns and grenades are gone until Ro gets his weapon back (level 7).
    pub disarmed: bool,
    pub has_m50: bool,
    pub m50_mag: u32,
    pub m50_reserve: u32,
    pub grenades: u32,
    pub cooldown: f32,
    pub knife_cd: f32,
    pub throw_cd: f32,
    /// Seconds of reload left (0 = not reloading).
    pub reload: f32,
    /// Extra aim error from sustained fire.
    pub spread: f32,
}

impl Default for Loadout {
    fn default() -> Self {
        Self {
            current: WeaponKind::Pistol,
            pistol_mag: 15,
            pistol_reserve: 45,
            ak_mag: 30,
            ak_reserve: 120, // five magazines in all
            cramped: false,
            disarmed: false,
            has_m50: false,
            m50_mag: 200,
            m50_reserve: 400,
            grenades: 5,
            cooldown: 0.0,
            knife_cd: 0.0,
            throw_cd: 0.0,
            reload: 0.0,
            spread: 0.0,
        }
    }
}

/// The ammunition Ro had at the last save point (or the start of the level); he comes back to it after a death.
#[derive(Resource, Default)]
pub struct LoadoutSave(pub Option<Loadout>);

/// Whenever the save point changes (a level starts, a new save point is reached), remember what Ro is carrying.
pub fn snapshot_loadout(checkpoint: Res<Checkpoint>, loadout: Res<Loadout>, mut save: ResMut<LoadoutSave>) {
    if checkpoint.is_changed() {
        save.0 = Some(loadout.clone());
    }
}

impl Loadout {
    /// Back to the ammunition of the save point: magazines, reserves and grenades. What was found since
    /// (the M50, a weapon handed back) stays found, and every timer starts fresh.
    pub fn restore_from(&mut self, saved: &Loadout) {
        self.pistol_mag = saved.pistol_mag;
        self.pistol_reserve = saved.pistol_reserve;
        self.ak_mag = saved.ak_mag;
        self.ak_reserve = saved.ak_reserve;
        self.grenades = saved.grenades;
        if saved.has_m50 {
            self.m50_mag = saved.m50_mag;
            self.m50_reserve = saved.m50_reserve;
        }
        self.cooldown = 0.0;
        self.knife_cd = 0.0;
        self.throw_cd = 0.0;
        self.reload = 0.0;
        self.spread = 0.0;
    }
}

#[derive(Resource)]
pub struct Health {
    pub hp: f32,
    /// 1.0 right after being hit, fades to 0 (drives the red flash).
    pub hurt: f32,
}

impl Default for Health {
    fn default() -> Self {
        Self { hp: 100.0, hurt: 0.0 }
    }
}

/// Loud events this frame: (position, how far they carry in metres).
#[derive(Resource, Default)]
pub struct Noises(pub Vec<(Vec3, f32)>);

#[derive(Resource, Default)]
pub struct Shake(pub f32);

/// Testing hook: aim here instead of at the mouse cursor.
#[derive(Resource, Default)]
pub struct AimOverride(pub Option<Vec3>);

#[derive(Resource)]
pub struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> f32 {
        // xorshift64*
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        let v = self.0.wrapping_mul(0x2545_F491_4F6C_DD1D);
        (v >> 40) as f32 / (1u64 << 24) as f32
    }
    pub fn signed(&mut self) -> f32 {
        self.next() * 2.0 - 1.0
    }
}

/// Blast centres this frame (level 4 listens for them to crack the glass).
#[derive(Resource, Default)]
pub struct Blasts(pub Vec<Vec3>);

#[derive(Component)]
pub struct HeldWeapon(pub WeaponKind);

/// Short-lived physical bits: sparks, smoke, flashes, tracers, casings, holes.
#[derive(Component)]
pub struct Particle {
    pub vel: Vec3,
    pub life: f32,
    pub max: f32,
    pub gravity: f32,
    /// Scale multiplier reached at the end of life.
    pub grow: f32,
    pub fade: bool,
    pub bounce: bool,
    pub base: Vec3,
}

#[derive(Component)]
pub struct Grenade {
    pub vel: Vec3,
    pub fuse: f32,
}

#[derive(Resource)]
pub struct Fx {
    cube: Handle<Mesh>,
    sphere: Handle<Mesh>,
    spark: Handle<StandardMaterial>,
    flash: Handle<StandardMaterial>,
    fire: Handle<StandardMaterial>,
    smoke: Handle<StandardMaterial>,
    dust: Handle<StandardMaterial>,
    tracer: Handle<StandardMaterial>,
    brass: Handle<StandardMaterial>,
    hole: Handle<StandardMaterial>,
    glass: Handle<StandardMaterial>,
    crack: Handle<StandardMaterial>,
    pub audio: crate::audio::AudioAssets,
    pub muted: bool,
    /// Music ducking level and effects volume (f32 bits; Fx is shared, so atomics).
    pub duck: std::sync::atomic::AtomicU32,
    pub sfx_gain: std::sync::atomic::AtomicU32,
}

pub fn setup_fx(
    server: Res<AssetServer>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(Fx {
        cube: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        sphere: meshes.add(Sphere::new(1.0)),
        spark: glow(&mut mats, Vec3::new(1.0, 0.75, 0.25), 10.0),
        flash: glow(&mut mats, Vec3::new(1.0, 0.85, 0.5), 14.0),
        fire: glow(&mut mats, Vec3::new(1.0, 0.45, 0.1), 6.0),
        smoke: matte(&mut mats, 0.30),
        dust: matte(&mut mats, 0.55),
        tracer: glow(&mut mats, Vec3::new(1.0, 0.9, 0.5), 12.0),
        brass: mats.add(StandardMaterial {
            base_color: Color::srgb(0.85, 0.65, 0.2),
            metallic: 0.8,
            perceptual_roughness: 0.4,
            ..default()
        }),
        hole: matte(&mut mats, 0.02),
        glass: glow(&mut mats, Vec3::new(0.5, 0.85, 1.0), 2.0),
        crack: glow(&mut mats, Vec3::new(0.9, 0.97, 1.0), 4.0),
        audio: crate::audio::AudioAssets::load(&server),
        // Scripted playtests run silently.
        muted: std::env::var_os("WSR_AUTOPLAY").is_some(),
        duck: std::sync::atomic::AtomicU32::new(0.0_f32.to_bits()),
        sfx_gain: std::sync::atomic::AtomicU32::new(1.0_f32.to_bits()),
    });
    commands.insert_resource(Rng(0x9E37_79B9_7F4A_7C15));
}

fn particle(
    commands: &mut Commands,
    mesh: &Handle<Mesh>,
    mat: &Handle<StandardMaterial>,
    pos: Vec3,
    rot: Quat,
    p: Particle,
) {
    commands.spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(mat.clone()),
        Transform::from_translation(pos)
            .with_rotation(rot)
            .with_scale(p.base),
        p,
        LevelEntity,
    ));
}

pub fn spark_burst(commands: &mut Commands, fx: &Fx, rng: &mut Rng, at: Vec3, normal: Vec3, n: usize) {
    for _ in 0..n {
        let v = (normal * 2.5
            + Vec3::new(rng.signed(), rng.signed().abs(), rng.signed()) * 3.0)
            * (0.5 + rng.next());
        particle(
            commands,
            &fx.cube,
            &fx.spark,
            at,
            Quat::IDENTITY,
            Particle {
                vel: v,
                life: 0.35,
                max: 0.35,
                gravity: 14.0,
                grow: 0.2,
                fade: true,
                bounce: false,
                base: Vec3::splat(0.06),
            },
        );
    }
}

pub fn dust_puff(commands: &mut Commands, fx: &Fx, at: Vec3, normal: Vec3, size: f32, life: f32) {
    particle(
        commands,
        &fx.sphere,
        &fx.dust,
        at + normal * 0.05,
        Quat::IDENTITY,
        Particle {
            vel: normal * 0.6 + Vec3::Y * 0.3,
            life,
            max: life,
            gravity: -0.2,
            grow: 4.0,
            fade: true,
            bounce: false,
            base: Vec3::splat(size),
        },
    );
}

/// A streak from `from` to `to` that vanishes almost at once.
pub fn tracer(commands: &mut Commands, fx: &Fx, from: Vec3, to: Vec3) {
    let len = from.distance(to).max(0.1);
    let mid = (from + to) * 0.5;
    let rot = Transform::from_translation(mid)
        .looking_at(to, Vec3::Y)
        .rotation;
    particle(
        commands,
        &fx.cube,
        &fx.tracer,
        mid,
        rot,
        Particle {
            vel: Vec3::ZERO,
            life: 0.07,
            max: 0.07,
            gravity: 0.0,
            grow: 1.0,
            fade: true,
            bounce: false,
            base: Vec3::new(0.04, 0.04, len),
        },
    );
}

/// Muzzle flash: a hot sphere plus a brief warm light.
pub fn muzzle_flash(commands: &mut Commands, fx: &Fx, at: Vec3, size: f32) {
    commands.spawn((
        Mesh3d(fx.sphere.clone()),
        MeshMaterial3d(fx.flash.clone()),
        Transform::from_translation(at).with_scale(Vec3::splat(size)),
        Particle {
            vel: Vec3::ZERO,
            life: 0.07,
            max: 0.07,
            gravity: 0.0,
            grow: 1.0,
            fade: true,
            bounce: false,
            base: Vec3::splat(size),
        },
        LevelEntity,
        children![(
            PointLight {
                color: Color::srgb(1.0, 0.8, 0.45),
                intensity: 700_000.0,
                range: 12.0,
                ..default()
            },
            Transform::default(),
        )],
    ));
}

/// Ejected brass: tumbles, bounces once, and lies on the ground.
pub fn casing(commands: &mut Commands, fx: &Fx, rng: &mut Rng, at: Vec3, right: Vec3) {
    particle(
        commands,
        &fx.cube,
        &fx.brass,
        at,
        Quat::from_euler(EulerRot::XYZ, rng.next() * 6.0, rng.next() * 6.0, 0.0),
        Particle {
            vel: right * (1.8 + rng.next()) + Vec3::Y * (2.0 + rng.next()),
            life: 3.0,
            max: 3.0,
            gravity: 12.0,
            grow: 1.0,
            fade: false,
            bounce: true,
            base: Vec3::new(0.035, 0.016, 0.016),
        },
    );
}

/// Shards of glass raining down over `min..max` (x, z).
pub fn glass_shatter(commands: &mut Commands, fx: &Fx, rng: &mut Rng, min: Vec2, max: Vec2) {
    for _ in 0..90 {
        let p = Vec3::new(
            min.x + rng.next() * (max.x - min.x),
            0.1,
            min.y + rng.next() * (max.y - min.y),
        );
        particle(
            commands,
            &fx.cube,
            &fx.glass,
            p,
            Quat::from_euler(EulerRot::XYZ, rng.next() * 6.0, rng.next() * 6.0, 0.0),
            Particle {
                vel: Vec3::new(rng.signed() * 1.5, -1.0 - rng.next() * 3.0, rng.signed() * 1.5),
                life: 2.0,
                max: 2.0,
                gravity: 10.0,
                grow: 1.0,
                fade: true,
                bounce: false,
                base: Vec3::new(0.18, 0.04, 0.18),
            },
        );
    }
}

/// A web of cracks radiating from `at` on the glass.
pub fn glass_cracks(commands: &mut Commands, fx: &Fx, rng: &mut Rng, at: Vec3, marker: impl Bundle + Clone) {
    for _ in 0..7 {
        let a = rng.next() * std::f32::consts::TAU;
        let len = 3.0 + rng.next() * 5.0;
        let dir = Vec3::new(a.cos(), 0.0, a.sin());
        commands.spawn((
            Mesh3d(fx.cube.clone()),
            MeshMaterial3d(fx.crack.clone()),
            Transform::from_translation(Vec3::new(at.x, 0.05, at.z) + dir * len * 0.5)
                .with_rotation(Quat::from_rotation_y(-a))
                .with_scale(Vec3::new(len, 0.02, 0.05)),
            marker.clone(),
            LevelEntity,
        ));
    }
}

pub fn update_particles(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut Transform, &mut Particle)>,
) {
    let dt = time.delta_secs();
    for (e, mut tf, mut p) in &mut q {
        p.life -= dt;
        if p.life <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        p.vel.y -= p.gravity * dt;
        tf.translation += p.vel * dt;
        if p.bounce && tf.translation.y < 0.02 {
            tf.translation.y = 0.02;
            p.vel.y *= -0.3;
            p.vel.x *= 0.5;
            p.vel.z *= 0.5;
        }
        let t = 1.0 - p.life / p.max;
        let size = 1.0 + (p.grow - 1.0) * t;
        let fade = if p.fade { 1.0 - t * t } else { 1.0 };
        tf.scale = p.base * size * fade;
    }
}

// --------------------------------------------------------------------- aiming

fn flat(v: Vec3) -> Vec3 {
    Vec3::new(v.x, 0.0, v.z)
}

/// Where the player is aiming: the first-person crosshair, or the mouse
/// cursor projected onto the world in third person.
fn aim_point(
    rig: &CamRig,
    cam: (&Camera, &GlobalTransform),
    window: &Window,
    over: &AimOverride,
    player: Vec3,
    facing: Vec3,
) -> Vec3 {
    if let Some(p) = over.0 {
        return p;
    }
    if rig.first {
        return cam.1.translation() + cam.1.forward().as_vec3() * 60.0;
    }
    if let Some(cursor) = window.cursor_position() {
        if let Ok(ray) = cam.0.viewport_to_world(cam.1, cursor) {
            let d = ray.direction.as_vec3();
            if d.y < -0.01 {
                let t = (1.2 - ray.origin.y) / d.y;
                return ray.origin + d * t;
            }
        }
    }
    player + Vec3::Y * 1.25 + facing * 20.0
}

/// Third person: aim at whatever the cursor is over, at any height: the first
/// person or wall the cursor ray hits, so you can target the sniper on his roof,
/// someone in the pit, or a far wall. `None` means the cursor sees only open ground.
fn cursor_pick(
    rig: &CamRig,
    cam: (&Camera, &GlobalTransform),
    window: &Window,
    boxes: &[Box3],
    people: &[(Vec3, f32)],
) -> Option<Vec3> {
    if rig.first {
        return None;
    }
    let ray = cam.0.viewport_to_world(cam.1, window.cursor_position()?).ok()?;
    let (o, d) = (ray.origin, ray.direction.as_vec3());
    let mut best: Option<f32> = None;
    for b in boxes {
        if let Some((t, _)) = ray_box(o, d, b) {
            if best.is_none_or(|v| t < v) {
                best = Some(t);
            }
        }
    }
    for &(feet, crouch) in people {
        if let Some((t, _)) = ray_guard(o, d, feet, crouch) {
            if best.is_none_or(|v| t < v) {
                best = Some(t);
            }
        }
    }
    best.map(|t| o + d * t)
}

fn yaw_to(d: Vec3) -> Quat {
    let f = flat(d);
    if f.length_squared() < 1e-6 {
        return Quat::IDENTITY;
    }
    Quat::from_rotation_y((-f.x).atan2(-f.z))
}

// ---------------------------------------------------------- selection, reload

pub fn select_and_reload(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    fx: Res<Fx>,
    mut commands: Commands,
    mut loadout: ResMut<Loadout>,
    mut msg: ResMut<Message>,
) {
    let dt = time.delta_secs();
    for (key, w) in [
        (KeyCode::Digit1, WeaponKind::Knife),
        (KeyCode::Digit2, WeaponKind::Pistol),
        (KeyCode::Digit3, WeaponKind::Ak47),
        (KeyCode::Digit4, WeaponKind::Grenade),
        (KeyCode::Digit5, WeaponKind::M50),
    ] {
        if w == WeaponKind::M50 && !loadout.has_m50 {
            continue;
        }
        if keys.just_pressed(key) && loadout.current != w {
            loadout.current = w;
            loadout.reload = 0.0; // switching cancels a reload
            loadout.cooldown = 0.25; // time to raise the weapon
            loadout.spread = 0.0;
        }
    }
    loadout.knife_cd = (loadout.knife_cd - dt).max(0.0);
    loadout.throw_cd = (loadout.throw_cd - dt).max(0.0);

    // Reload.
    let (mag, reserve, cap, secs) = match loadout.current {
        WeaponKind::Pistol => (loadout.pistol_mag, loadout.pistol_reserve, 15, 1.6),
        WeaponKind::Ak47 => (loadout.ak_mag, loadout.ak_reserve, 30, 2.4),
        WeaponKind::M50 => (loadout.m50_mag, loadout.m50_reserve, 200, 4.5),
        _ => return,
    };
    if loadout.reload > 0.0 {
        loadout.reload -= dt;
        if loadout.reload <= 0.0 {
            loadout.reload = 0.0;
            msg.0.clear();
            let take = (cap - mag).min(reserve);
            match loadout.current {
                WeaponKind::Pistol => {
                    loadout.pistol_mag += take;
                    loadout.pistol_reserve -= take;
                }
                WeaponKind::M50 => {
                    loadout.m50_mag += take;
                    loadout.m50_reserve -= take;
                }
                _ => {
                    loadout.ak_mag += take;
                    loadout.ak_reserve -= take;
                }
            }
        }
    } else if (keys.just_pressed(KeyCode::KeyR) || mag == 0) && mag < cap && reserve > 0 {
        loadout.reload = secs;
        msg.0 = "Reloading...".into();
        fx.play(&mut commands, crate::audio::Sfx::Reload, 1.0, 1.0, 0.0);
    }
}

// -------------------------------------------------------------------- hit feedback

/// Hit confirmation: the crosshair flashes when a bullet lands (X), brighter for a headshot, red for a kill.
#[derive(Resource, Default)]
pub struct HitMark {
    pub t: f32,
    pub head: bool,
    pub kill: bool,
}

pub fn hitmark_ui(
    time: Res<Time>,
    mut hit: ResMut<HitMark>,
    mut cross: Single<(&mut Text, &mut TextColor, &mut TextFont), With<Crosshair>>,
) {
    hit.t = (hit.t - time.delta_secs()).max(0.0);
    let (text, color, font) = &mut *cross;
    if hit.t > 0.0 {
        text.0 = "X".into();
        color.0 = if hit.kill {
            Color::srgb(1.0, 0.15, 0.1)
        } else if hit.head {
            Color::srgb(1.0, 0.85, 0.1)
        } else {
            Color::WHITE
        };
        font.font_size = FontSize::Px(if hit.head || hit.kill { 38.0 } else { 32.0 });
    } else {
        text.0 = "+".into();
        color.0 = Color::WHITE;
        font.font_size = FontSize::Px(28.0);
    }
}

// -------------------------------------------------------------------- shooting

#[allow(clippy::too_many_arguments)]
pub fn shoot(
    time: Res<Time>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut loadout: ResMut<Loadout>,
    mut rig: ResMut<CamRig>,
    aimo: Res<AimOverride>,
    mut rng: ResMut<Rng>,
    mut noises: ResMut<Noises>,
    mut shake: ResMut<Shake>,
    boxes: Res<Boxes>,
    fx: Res<Fx>,
    mut commands: Commands,
    camera: Single<(&Camera, &GlobalTransform), With<GameCamera>>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut hitmark: ResMut<HitMark>,
    mut guards: Query<(Entity, &mut Guard, &Transform, &Mover), Without<Player>>,
    player: Single<(&mut Transform, &mut Mover), (With<Player>, Without<Guard>)>,
) {
    let dt = time.delta_secs();
    loadout.cooldown = (loadout.cooldown - dt).max(0.0);
    loadout.spread = (loadout.spread - 0.25 * dt).max(0.0);
    let (mut ptf, mut pm) = player.into_inner();
    pm.kick = (pm.kick - 8.0 * dt).max(0.0);

    let weapon = loadout.current;
    let wants = match weapon {
        WeaponKind::Ak47 | WeaponKind::M50 => mouse.pressed(MouseButton::Left),
        WeaponKind::Pistol => mouse.just_pressed(MouseButton::Left),
        _ => false,
    };
    if !wants || loadout.cooldown > 0.0 || loadout.reload > 0.0 || loadout.disarmed {
        return;
    }
    let mag = match weapon {
        WeaponKind::Ak47 => &mut loadout.ak_mag,
        WeaponKind::M50 => &mut loadout.m50_mag,
        _ => &mut loadout.pistol_mag,
    };
    if *mag == 0 {
        loadout.cooldown = 0.3; // dry click
        return;
    }
    *mag -= 1;

    // Turn to face the aim point, then fire from the muzzle.
    let facing = flat(ptf.forward().as_vec3()).normalize_or_zero();
    let people: Vec<(Vec3, f32)> = guards
        .iter()
        .filter(|(_, g, _, _)| g.status == LostStatus::Standing)
        .map(|(_, _, tf, mv)| (tf.translation, mv.crouch))
        .collect();
    let picked = if aimo.0.is_none() {
        cursor_pick(&rig, (camera.0, camera.1), &window, &boxes.0, &people)
    } else {
        None
    };
    let aim = picked
        .unwrap_or_else(|| aim_point(&rig, (camera.0, camera.1), &window, &aimo, ptf.translation, facing));
    let to_aim = aim - (ptf.translation + Vec3::Y * 1.25);
    if !rig.first && flat(to_aim).length() > 0.3 {
        ptf.rotation = yaw_to(to_aim);
    }
    let muzzle = ptf.translation + ptf.rotation * Vec3::new(0.16, 1.25, -0.75);
    let mut dir = (aim - muzzle).normalize_or_zero();
    if dir == Vec3::ZERO {
        dir = facing;
    }

    // Aim error: a base cone plus the spread built up by sustained fire,
    // worse while moving, better crouched.
    let (base_spread, dmg, cooldown, range_noise, kick, shake_add) = match weapon {
        WeaponKind::Ak47 => (0.012, 34.0, 0.1, 60.0, 0.012, 0.10),
        WeaponKind::M50 => (0.022, 30.0, 0.065, 85.0, 0.01, 0.16),
        _ => (0.008, 26.0, 0.2, 40.0, 0.02, 0.16),
    };
    let stance = if pm.prone_on { 0.4 } else if pm.crouch > 0.5 { 0.6 } else if pm.moving { 1.6 } else { 1.0 };
    let cramped = if loadout.cramped && weapon == WeaponKind::M50 { 2.4 } else { 1.0 };
    let cone = (base_spread + loadout.spread) * stance * cramped * (1.0 - 0.55 * rig.aim);
    let right = dir.cross(Vec3::Y).normalize_or_zero();
    dir = (Quat::from_rotation_y(rng.signed() * cone)
        * Quat::from_axis_angle(right, rng.signed() * cone * 0.7)
        * dir)
        .normalize();

    // Hitscan: nearest of the world and the guards.
    let max_range = 120.0;
    let mut best_t = max_range;
    let mut hit_world: Option<Vec3> = None;
    for b in &boxes.0 {
        if let Some((t, n)) = ray_box(muzzle, dir, b) {
            if t < best_t {
                best_t = t;
                hit_world = Some(n);
            }
        }
    }
    let mut hit_guard: Option<(Entity, bool, f32)> = None;
    for (e, g, tf, mv) in &guards {
        if g.status != LostStatus::Standing {
            continue;
        }
        if let Some((t, head)) = ray_guard(muzzle, dir, tf.translation, mv.crouch) {
            if t < best_t {
                best_t = t;
                hit_guard = Some((e, head, t));
                hit_world = None;
            }
        }
    }
    let end = muzzle + dir * best_t;
    tracer(&mut commands, &fx, muzzle, end);
    let sfx = match weapon {
        WeaponKind::Ak47 => crate::audio::Sfx::Ak,
        WeaponKind::M50 => crate::audio::Sfx::M50,
        _ => crate::audio::Sfx::Pistol,
    };
    let (sv, sr) = (rng.next(), rng.next());
    fx.play(&mut commands, sfx, 1.0, 0.97 + 0.06 * sr, sv);
    muzzle_flash(&mut commands, &fx, muzzle + dir * 0.2, if weapon == WeaponKind::Ak47 { 0.3 } else if weapon == WeaponKind::M50 { 0.4 } else { 0.22 });
    casing(&mut commands, &fx, &mut rng, ptf.translation + ptf.rotation * Vec3::new(0.2, 1.2, -0.4), right);

    if let Some((e, head, _)) = hit_guard {
        if let Ok((_, mut g, _, _)) = guards.get_mut(e) {
            let damage = g.gun_damage(head, dmg);
            hurt_guard(&mut g, damage, ptf.translation);
            *hitmark = HitMark { t: 0.14, head, kill: g.hp <= 0.0 };
            let (hv, hp) = (rng.next(), if head { 1.35 } else { 1.0 });
            fx.play(&mut commands, crate::audio::Sfx::Hit, if head { 0.9 } else { 0.55 }, hp, hv);
            // A small puff of cloth and dust at the impact, nothing more.
            dust_puff(&mut commands, &fx, end, -dir, 0.12, 0.35);
        }
    } else if let Some(n) = hit_world {
        spark_burst(&mut commands, &fx, &mut rng, end, n, 7);
        dust_puff(&mut commands, &fx, end, n, 0.10, 0.5);
        // A bullet hole.
        commands.spawn((
            Mesh3d(fx.cube.clone()),
            MeshMaterial3d(fx.hole.clone()),
            Transform::from_translation(end + n * 0.012)
                .with_rotation(Quat::from_rotation_arc(Vec3::Z, n))
                .with_scale(Vec3::new(0.07, 0.07, 0.01)),
            Particle {
                vel: Vec3::ZERO,
                life: 40.0,
                max: 40.0,
                gravity: 0.0,
                grow: 1.0,
                fade: false,
                bounce: false,
                base: Vec3::new(0.07, 0.07, 0.01),
            },
            LevelEntity,
        ));
    }

    // Recoil, noise, camera kick.
    loadout.cooldown = cooldown;
    loadout.spread = (loadout.spread + if weapon == WeaponKind::Ak47 { 0.012 } else if weapon == WeaponKind::M50 { 0.01 } else { 0.02 }).min(0.09);
    pm.kick = 1.0;
    pm.noise = pm.noise.max(range_noise);
    noises.0.push((ptf.translation, range_noise));
    shake.0 = (shake.0 + shake_add).min(1.0);
    if rig.first {
        // The muzzle climbs while you fire, then settles back (see player::camera).
        rig.recoil = (rig.recoil + kick + rng.signed().abs() * 0.003).min(0.3);
        rig.yaw += rng.signed() * 0.003;
    }
}

/// A guard takes a hit: knows where it came from, staggers, and turns on you.
pub fn hurt_guard(g: &mut Guard, damage: f32, from: Vec3) {
    if g.kind == GuardKind::Phantom {
        g.pain = 1.0; // he cannot be killed, only driven off...
        if g.mortal {
            g.hp = (g.hp - damage * g.armor).max(g.hp_floor.min(g.hp)); // ...unless he is in his own house
        }
        return;
    }
    g.hp = (g.hp - damage * g.armor).max(g.hp_floor.min(g.hp));
    g.awareness = 1.0;
    g.last_known = from;
    if !g.melee {
        g.stagger = 0.3;
    } else {
        // He grows faster and angrier with every wound.
        g.speed_mul = 1.0 + 0.8 * (1.0 - (g.hp / g.max_hp).clamp(0.0, 1.0));
    }
    if g.hp > 0.0 && g.state != AiState::Chase {
        g.state = AiState::Chase;
        g.timer = 0.0;
    }
}

/// Anyone who reaches zero health goes down; the kill is paid for in Weight.
pub fn resolve_deaths(
    mut guards: Query<(&mut Guard, &Transform)>,
    mut weight: ResMut<Weight>,
    mut stats: ResMut<Stats>,
    mut msg: ResMut<Message>,
    mut loadout: ResMut<Loadout>,
    mut rng: ResMut<Rng>,
) {
    for (mut g, tf) in &mut guards {
        if g.status == LostStatus::Standing && g.hp <= 0.0 {
            g.status = LostStatus::Fallen;
            g.fall = 0.0;
            g.fall_yaw = tf.rotation.to_euler(EulerRot::YXZ).0;
            weight.0 = (weight.0 + 20.0).min(100.0);
            stats.kills += 1;
            msg.0 = format!(
                "{}, {}, goes down. Weight grows. He will rise at night.",
                g.name, g.age
            );
            // Armed men carry ammunition: most drop some (6 in 10), so a bad streak never strands you.
            if !g.melee && g.kind != GuardKind::Phantom && rng.next() < 0.6 && !loadout.disarmed {
                loadout.ak_reserve = (loadout.ak_reserve + 12).min(180);
                loadout.pistol_reserve = (loadout.pistol_reserve + 6).min(60);
                msg.0.push_str(" You take the ammunition from his belt.");
            }
        }
    }
}

// ---------------------------------------------------------------------- knife

#[allow(clippy::too_many_arguments)]
pub fn melee(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut loadout: ResMut<Loadout>,
    mut noises: ResMut<Noises>,
    mut rng: ResMut<Rng>,
    fx: Res<Fx>,
    mut commands: Commands,
    l7: Option<Res<crate::level7::Level7>>,
    mut guards: Query<(&mut Guard, &Transform), Without<Player>>,
    player: Single<(&mut Transform, &mut Mover), (With<Player>, Without<Guard>)>,
) {
    let slash = keys.just_pressed(KeyCode::KeyF)
        || (loadout.current == WeaponKind::Knife && mouse.just_pressed(MouseButton::Left));
    let (mut ptf, mut pm) = player.into_inner();
    if !slash || loadout.knife_cd > 0.0 {
        return;
    }
    loadout.knife_cd = 0.6;
    pm.slash = 1.0;
    // On the rooftop the last fight is with bare fists.
    pm.swing = if l7.is_some() { crate::character::Swing::Punch } else { crate::character::Swing::Stab };

    // The nearest standing guard within reach.
    let mut best: Option<(f32, usize)> = None;
    for (i, (g, tf)) in guards.iter().enumerate() {
        let d = flat(tf.translation - ptf.translation).length();
        if g.status == LostStatus::Standing && d < 1.9 && best.is_none_or(|(b, _)| d < b) {
            best = Some((d, i));
        }
    }
    let Some((_, i)) = best else {
        noises.0.push((ptf.translation, 3.0));
        fx.play(&mut commands, crate::audio::Sfx::KnifeSwish, 1.0, 1.0, 0.0);
        return;
    };
    fx.play(&mut commands, crate::audio::Sfx::KnifeHit, 1.0, 1.0, 0.0);
    let Some((mut g, gtf)) = guards.iter_mut().nth(i) else {
        return;
    };
    let to_guard = gtf.translation - ptf.translation;
    ptf.rotation = yaw_to(to_guard);

    // From behind and unnoticed: a silent takedown. Otherwise a plain cut.
    let guard_facing = flat(gtf.forward().as_vec3());
    let from_behind = guard_facing.dot(flat(ptf.translation - gtf.translation).normalize_or_zero()) < -0.3;
    if g.armor < 0.5 {
        // Helmet and vest: the blade skids off.
        g.awareness = 1.0;
        g.last_known = ptf.translation;
        spark_burst(&mut commands, &fx, &mut rng, gtf.translation + Vec3::Y * 1.1, Vec3::Y, 6);
        noises.0.push((ptf.translation, 14.0));
        return;
    }
    let silent = from_behind && g.awareness < 0.5;
    let damage = g.scaled_damage(if silent { 200.0 } else { 50.0 }, 34.0);
    hurt_guard(&mut g, damage, ptf.translation);
    let at = gtf.translation + Vec3::Y * 1.1;
    dust_puff(&mut commands, &fx, at, -flat(to_guard).normalize_or_zero(), 0.10, 0.3);
    spark_burst(&mut commands, &fx, &mut rng, at, Vec3::Y, 2);
    noises.0.push((ptf.translation, if silent { 2.0 } else { 12.0 }));
    pm.noise = pm.noise.max(if silent { 1.0 } else { 6.0 });
}

// ------------------------------------------------------------------ grenades

#[allow(clippy::too_many_arguments)]
pub fn throw_grenade(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut loadout: ResMut<Loadout>,
    rig: Res<CamRig>,
    aimo: Res<AimOverride>,
    camera: Single<(&Camera, &GlobalTransform), With<GameCamera>>,
    window: Single<&Window, With<PrimaryWindow>>,
    ink_res: Res<Ink>,
    fx: Res<Fx>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
    player: Single<(&mut Transform, &mut Mover), (With<Player>, Without<Guard>)>,
) {
    let throw = keys.just_pressed(KeyCode::KeyG)
        || (loadout.current == WeaponKind::Grenade && mouse.just_pressed(MouseButton::Left));
    if !throw || loadout.grenades == 0 || loadout.throw_cd > 0.0 || loadout.disarmed {
        return;
    }
    loadout.grenades -= 1;
    loadout.throw_cd = 0.8;
    let (mut ptf, mut pm) = player.into_inner();
    pm.slash = 0.6;
    pm.swing = crate::character::Swing::Throw;
    let facing = flat(ptf.forward().as_vec3()).normalize_or_zero();
    let aim = aim_point(&rig, (camera.0, camera.1), &window, &aimo, ptf.translation, facing);
    let to = flat(aim - ptf.translation);
    if to.length() > 0.3 {
        ptf.rotation = yaw_to(to);
    }
    let dist = to.length().clamp(3.0, 22.0);
    let origin = ptf.translation + Vec3::Y * 1.4 + flat(to).normalize_or_zero() * 0.5;
    // A lob: horizontal speed from the distance, a fixed upward kick.
    let vel = flat(to).normalize_or_zero() * (dist * 0.78).clamp(4.0, 16.0) + Vec3::Y * 5.5;
    let body = matte(&mut mats, 0.18);
    commands.spawn((
        ink(
            fx.sphere.clone(),
            body,
            &ink_res,
            Transform::from_translation(origin).with_scale(Vec3::splat(0.09)),
            Vec3::splat(1.3),
        ),
        Grenade { vel, fuse: 2.5 },
        LevelEntity,
    ));
}

#[allow(clippy::too_many_arguments)]
pub fn update_grenades(
    time: Res<Time>,
    mut commands: Commands,
    boxes: Res<Boxes>,
    fx: Res<Fx>,
    mut rng: ResMut<Rng>,
    mut shake: ResMut<Shake>,
    mut noises: ResMut<Noises>,
    mut health: ResMut<Health>,
    mut objectives: ResMut<Objectives>,
    mut msg: ResMut<Message>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut blasts: Option<ResMut<Blasts>>,
    targets: Query<&SabotageTarget>,
    mut grenades: Query<(Entity, &mut Transform, &mut Grenade), Without<Guard>>,
    mut guards: Query<(&mut Guard, &Transform), Without<Grenade>>,
    player: Single<&Transform, (With<Player>, Without<Grenade>, Without<Guard>)>,
) {
    let dt = time.delta_secs().min(0.05);
    for (e, mut tf, mut gr) in &mut grenades {
        gr.vel.y -= 14.0 * dt;
        tf.translation += gr.vel * dt;
        // Bounce off the ground and off solid things.
        if tf.translation.y < 0.09 {
            tf.translation.y = 0.09;
            gr.vel.y *= -0.35;
            gr.vel.x *= 0.7;
            gr.vel.z *= 0.7;
        }
        for b in &boxes.0 {
            let d = tf.translation - b.c;
            let pen = b.h + Vec3::splat(0.09) - d.abs();
            if pen.x > 0.0 && pen.y > 0.0 && pen.z > 0.0 {
                if pen.x < pen.y && pen.x < pen.z {
                    tf.translation.x += pen.x * d.x.signum();
                    gr.vel.x *= -0.4;
                } else if pen.y < pen.z {
                    tf.translation.y += pen.y * d.y.signum();
                    gr.vel.y *= -0.35;
                } else {
                    tf.translation.z += pen.z * d.z.signum();
                    gr.vel.z *= -0.4;
                }
            }
        }
        gr.fuse -= dt;
        if gr.fuse > 0.0 {
            continue;
        }

        // --- Detonation.
        let c = tf.translation;
        commands.entity(e).despawn();
        explosion_fx(&mut commands, &fx, &mut rng, c);
        let dp = Vec2::new(player.translation.x - c.x, player.translation.z - c.z).length();
        fx.play(&mut commands, crate::audio::Sfx::Explosion, (1.1 - dp / 70.0).clamp(0.25, 1.0), 1.0, 0.0);
        shake.0 = (shake.0 + 0.9).min(1.0);
        noises.0.push((c, 70.0));
        if let Some(b) = blasts.as_mut() {
            b.0.push(c);
        }
        let radius = 7.5;
        let falloff = |chest: Vec3| -> f32 {
            let d = c.distance(chest);
            if d >= radius {
                return 0.0;
            }
            let cover = if segment_blocked(c + Vec3::Y * 0.2, chest, &boxes.0) { 0.25 } else { 1.0 };
            170.0 * (1.0 - d / radius) * cover
        };
        for (mut g, gtf) in &mut guards {
            if g.status == LostStatus::Standing {
                let dmg = falloff(gtf.translation + Vec3::Y * 1.0);
                if dmg > 0.0 && g.armor >= 0.05 {
                    // Blast pressure gets past most armor.
                    let eff = if g.armor < 1.0 { dmg * 0.9 / g.armor.max(0.05) } else { dmg };
                    let eff = g.scaled_damage(eff, 10.0);
                    crate::weapons::hurt_guard(&mut g, eff, c);
                }
            }
        }
        let self_dmg = falloff(player.translation + Vec3::Y * 1.0);
        if self_dmg > 0.0 {
            health.hp -= self_dmg;
            health.hurt = 1.0;
        }
        // Blasts also wreck the gang's supplies.
        for t in &targets {
            let reach = if t.idx == 1 { 9.0 } else { 6.5 };
            if !objectives.done[t.idx] && c.distance(t.pos) < reach {
                objectives.done[t.idx] = true;
                objectives.progress[t.idx] = 1.0;
                msg.0 = format!("The blast ruins it. {}", t.done_msg);
                if let Some(mut m) = mats.get_mut(&t.beacon) {
                    m.base_color = Color::srgb(0.2, 1.0, 0.45);
                    m.emissive = LinearRgba::rgb(0.6, 4.0, 1.6);
                }
            }
        }
    }
}

fn explosion_fx(commands: &mut Commands, fx: &Fx, rng: &mut Rng, c: Vec3) {
    // Flash of light.
    commands.spawn((
        PointLight {
            color: Color::srgb(1.0, 0.7, 0.35),
            intensity: 6_000_000.0,
            range: 30.0,
            ..default()
        },
        Transform::from_translation(c + Vec3::Y * 0.8),
        Particle {
            vel: Vec3::ZERO,
            life: 0.18,
            max: 0.18,
            gravity: 0.0,
            grow: 1.0,
            fade: true,
            bounce: false,
            base: Vec3::ONE,
        },
        LevelEntity,
    ));
    // Fireball, smoke column, sparks, debris.
    for i in 0..10 {
        let v = Vec3::new(rng.signed(), rng.next() * 1.2, rng.signed()) * 3.5;
        particle(
            commands,
            &fx.sphere,
            &fx.fire,
            c + Vec3::Y * 0.5,
            Quat::IDENTITY,
            Particle {
                vel: v,
                life: 0.5,
                max: 0.5,
                gravity: -1.0,
                grow: 2.2,
                fade: true,
                bounce: false,
                base: Vec3::splat(0.5 + 0.05 * i as f32),
            },
        );
    }
    for _ in 0..8 {
        let v = Vec3::new(rng.signed() * 1.6, 1.5 + rng.next() * 2.0, rng.signed() * 1.6);
        particle(
            commands,
            &fx.sphere,
            &fx.smoke,
            c + Vec3::Y * 0.6,
            Quat::IDENTITY,
            Particle {
                vel: v,
                life: 1.8,
                max: 1.8,
                gravity: -0.4,
                grow: 3.5,
                fade: true,
                bounce: false,
                base: Vec3::splat(0.6),
            },
        );
    }
    spark_burst(commands, fx, rng, c + Vec3::Y * 0.3, Vec3::Y, 24);
    for _ in 0..10 {
        let v = Vec3::new(rng.signed() * 5.0, 4.0 + rng.next() * 4.0, rng.signed() * 5.0);
        particle(
            commands,
            &fx.cube,
            &fx.dust,
            c + Vec3::Y * 0.3,
            Quat::from_euler(EulerRot::XYZ, rng.next() * 6.0, rng.next() * 6.0, 0.0),
            Particle {
                vel: v,
                life: 1.6,
                max: 1.6,
                gravity: 14.0,
                grow: 1.0,
                fade: false,
                bounce: true,
                base: Vec3::splat(0.08 + rng.next() * 0.08),
            },
        );
    }
    // Ground shockwave: a quickly widening flat ring of dust.
    particle(
        commands,
        &fx.sphere,
        &fx.dust,
        c + Vec3::Y * 0.1,
        Quat::IDENTITY,
        Particle {
            vel: Vec3::ZERO,
            life: 0.45,
            max: 0.45,
            gravity: 0.0,
            grow: 14.0,
            fade: true,
            bounce: false,
            base: Vec3::new(0.5, 0.05, 0.5),
        },
    );
}

// ------------------------------------------------- the weapons Ro carries

/// Spawn a weapon model into the hand of `owner` (a deferred command: the hand
/// is a child of the skeleton). `parts` are (position, size, material) relative
/// to the grip; guns point down the forearm (`along_arm`), a knife out of the fist.
fn give_weapon(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    ink_res: &Ink,
    owner: Entity,
    kind: WeaponKind,
    parts: Vec<(Vec3, Vec3, Handle<StandardMaterial>)>,
    along_arm: bool,
    hidden: bool,
) {
    let specs: Vec<(Vec3, Vec3, Handle<Mesh>, Handle<StandardMaterial>)> = parts
        .into_iter()
        .map(|(pos, size, mat)| (pos, size, meshes.add(Cuboid::new(size.x, size.y, size.z)), mat))
        .collect();
    let ink_res = ink_res.clone();
    commands.queue(move |world: &mut World| {
        let _ = along_arm; // the hand bone's frame decides the pose now
        // Spawned on the owner; `grip_weapons` moves it into the hand once the
        // skeleton has loaded.
        let gid = world
            .spawn((
                HeldWeapon(kind),
                PendingGrip(owner),
                Transform::IDENTITY,
                if hidden { Visibility::Hidden } else { Visibility::Inherited },
            ))
            .id();
        for (pos, size, mesh, mat) in specs {
            let part = world
                .spawn(ink(mesh, mat, &ink_res, Transform::from_translation(pos), grow_box(size, 0.012)))
                .id();
            world.entity_mut(gid).add_child(part);
        }
        world.entity_mut(owner).add_child(gid);
    });
}

/// Hand-bone frame -> weapon frame: barrels and blades point along the fingers.
const GRIP_ROT: Quat = Quat::from_xyzw(std::f32::consts::FRAC_1_SQRT_2, 0.0, 0.0, std::f32::consts::FRAC_1_SQRT_2);
const GRIP_POS: Vec3 = Vec3::ZERO;

/// A weapon waiting for its owner's hand bone to exist.
#[derive(Component)]
pub struct PendingGrip(pub Entity);

pub fn grip_weapons(
    mut commands: Commands,
    pending: Query<(Entity, &PendingGrip, &HeldWeapon)>,
    sockets: Query<(Entity, &crate::character::Socket)>,
) {
    for (w, p, _held) in &pending {
        if let Some((sock, _)) = sockets.iter().find(|(_, s)| s.root == p.0) {
            // WSR_GRIP="rx,ry,rz,x,y,z,scale": hand-tuning override (degrees, metres)
            let mut tf = Transform::from_rotation(GRIP_ROT).with_translation(GRIP_POS);
            if let Ok(v) = std::env::var("WSR_GRIP") {
                let n: Vec<f32> = v.split(',').filter_map(|x| x.trim().parse().ok()).collect();
                if n.len() >= 6 {
                    tf.rotation = Quat::from_euler(EulerRot::XYZ, n[0].to_radians(), n[1].to_radians(), n[2].to_radians());
                    tf.translation = Vec3::new(n[3], n[4], n[5]);
                }
                if let Some(sc) = n.get(6) {
                    tf.scale = Vec3::splat(*sc);
                }
            }
            commands.entity(w).remove::<PendingGrip>().insert((ChildOf(sock), tf));
        }
    }
}

/// Spawn the weapon models in Ro's hand; only the selected one shows.
pub fn attach_player_weapons(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mats: &mut Assets<StandardMaterial>,
    ink_res: &Ink,
    player: Entity,
) {
    let metal = matte(mats, 0.16);
    let wood = matte(mats, 0.30);
    let steel = matte(mats, 0.78);
    let v = Vec3::new;
    let mut give = |kind, parts: Vec<(Vec3, Vec3, Handle<StandardMaterial>)>, along| {
        give_weapon(commands, meshes, ink_res, player, kind, parts, along, true);
    };
    // AK-47: receiver, long barrel, wooden stock, curved magazine.
    give(
        WeaponKind::Ak47,
        vec![
            (v(0.0, 0.0, -0.15), v(0.07, 0.10, 0.5), metal.clone()),
            (v(0.0, 0.02, -0.55), v(0.03, 0.03, 0.38), metal.clone()),
            (v(0.0, -0.03, 0.15), v(0.06, 0.12, 0.26), wood.clone()),
            (v(0.0, -0.13, -0.2), v(0.05, 0.2, 0.09), metal.clone()),
        ],
        true,
    );
    give(
        WeaponKind::M50,
        vec![
            (v(0.0, 0.02, -0.2), v(0.12, 0.16, 0.7), metal.clone()),
            (v(0.0, 0.04, -0.7), v(0.05, 0.05, 0.5), metal.clone()),
            (v(0.0, -0.08, -0.05), v(0.16, 0.22, 0.24), wood.clone()),
            (v(0.0, -0.04, 0.2), v(0.07, 0.14, 0.3), wood.clone()),
        ],
        true,
    );
    give(
        WeaponKind::Pistol,
        vec![
            (v(0.0, 0.04, -0.06), v(0.05, 0.07, 0.24), metal.clone()),
            (v(0.0, -0.04, 0.02), v(0.045, 0.12, 0.07), wood.clone()),
        ],
        true,
    );
    give(
        WeaponKind::Knife,
        vec![
            (v(0.0, 0.0, -0.2), v(0.02, 0.04, 0.3), steel),
            (v(0.0, 0.0, 0.0), v(0.035, 0.05, 0.1), wood),
        ],
        false,
    );
    give(WeaponKind::Grenade, vec![(v(0.0, 0.0, 0.0), v(0.08, 0.1, 0.08), metal)], false);
}

/// Key art: put a visible weapon in any character's hand (the rifle, or a long glowing red knife).
pub fn poster_weapon(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mats: &mut Assets<StandardMaterial>,
    ink_res: &Ink,
    owner: Entity,
    kind: WeaponKind,
) {
    let v = Vec3::new;
    match kind {
        WeaponKind::Ak47 => {
            let metal = matte(mats, 0.14);
            let steel = matte(mats, 0.3);
            let wood = mats.add(StandardMaterial {
                base_color: Color::srgb(0.36, 0.19, 0.09),
                perceptual_roughness: 0.55,
                reflectance: 0.25,
                ..default()
            });
            give_weapon(
                commands,
                meshes,
                ink_res,
                owner,
                kind,
                vec![
                    (v(0.0, 0.0, -0.12), v(0.06, 0.09, 0.42), metal.clone()),       // receiver
                    (v(0.0, 0.055, -0.12), v(0.045, 0.02, 0.40), steel.clone()),    // dust cover
                    (v(0.0, 0.0, -0.40), v(0.07, 0.075, 0.26), wood.clone()),       // handguard
                    (v(0.0, 0.052, -0.42), v(0.028, 0.026, 0.30), steel.clone()),   // gas tube
                    (v(0.0, 0.012, -0.66), v(0.022, 0.022, 0.30), metal.clone()),   // barrel
                    (v(0.0, 0.012, -0.83), v(0.034, 0.034, 0.06), steel),           // muzzle brake
                    (v(0.0, 0.045, -0.78), v(0.012, 0.05, 0.016), metal.clone()),   // front sight
                    (v(0.0, -0.02, 0.22), v(0.05, 0.12, 0.30), wood.clone()),       // stock
                    (v(0.0, -0.09, 0.06), v(0.04, 0.12, 0.05), wood),               // pistol grip
                    (v(0.0, -0.135, -0.19), v(0.045, 0.16, 0.07), metal.clone()),   // magazine, upper
                    (v(0.0, -0.235, -0.235), v(0.045, 0.10, 0.07), metal),          // magazine, curving forward
                ],
                true,
                false,
            );
        }
        _ => {
            // the knife: a short dark blade with a red glowing edge, a guard and a grip
            let blade = matte(mats, 0.55);
            let edge = crate::style::glow(mats, Vec3::new(1.0, 0.1, 0.06), 3.0);
            let grip = matte(mats, 0.08);
            give_weapon(
                commands,
                meshes,
                ink_res,
                owner,
                kind,
                vec![
                    (v(0.0, 0.0, -0.25), v(0.014, 0.06, 0.38), blade),
                    (v(0.0, -0.03, -0.25), v(0.02, 0.012, 0.38), edge),
                    (v(0.0, 0.0, -0.05), v(0.03, 0.10, 0.025), grip.clone()),
                    (v(0.0, 0.0, 0.04), v(0.05, 0.06, 0.16), grip),
                ],
                false,
                false,
            );
        }
    }
}

/// A pistol in a guard's hand.
pub fn attach_guard_pistol(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mats: &mut Assets<StandardMaterial>,
    ink_res: &Ink,
    guard: Entity,
) {
    let metal = matte(mats, 0.12);
    give_weapon(
        commands,
        meshes,
        ink_res,
        guard,
        WeaponKind::Pistol,
        vec![
            (Vec3::new(0.0, 0.04, -0.06), Vec3::new(0.05, 0.07, 0.24), metal.clone()),
            (Vec3::new(0.0, -0.04, 0.02), Vec3::new(0.045, 0.12, 0.07), metal),
        ],
        true,
        false,
    );
}

pub fn held_models(
    loadout: Res<Loadout>,
    mut models: Query<(&HeldWeapon, &mut Visibility, &ChildOf)>,
    sockets: Query<&crate::character::Socket>,
    player_q: Query<Entity, With<Player>>,
    mut player: Single<&mut Mover, With<Player>>,
) {
    let Ok(ro) = player_q.single() else { return };
    for (w, mut vis, parent) in &mut models {
        // guards' pistols stay visible; only Ro's models follow the loadout
        let Ok(socket) = sockets.get(parent.parent()) else { continue };
        if socket.root != ro {
            continue;
        }
        *vis = if w.0 == loadout.current {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    player.armed = loadout.current.ranged();
}

/// H: drink from the reusable bottle: heal, and stop the soul-bleed.
pub fn use_bottle(
    keys: Res<ButtonInput<KeyCode>>,
    mut poison: ResMut<Poison>,
    mut health: ResMut<Health>,
    mut bleed: ResMut<crate::level5::Bleed>,
    mut msg: ResMut<Message>,
) {
    if !keys.just_pressed(KeyCode::KeyH) || poison.bottles == 0 {
        return;
    }
    if health.hp >= 100.0 && bleed.0 <= 0.0 {
        return;
    }
    if poison.bottles < 900 {
        poison.bottles -= 1; // 900 and up is the unlimited source
    }
    health.hp = 100.0; // one bottle brings him back to full
    bleed.0 = 0.0;
    msg.0 = if poison.bottles >= 900 {
        "You drink from the bottle. It never empties.".into()
    } else {
        format!("You drink from the bottle. {} uses left.", poison.bottles)
    };
}

/// Camera trauma decays; health slowly returns when nobody is hunting you.
pub fn recover(
    time: Res<Time>,
    mut shake: ResMut<Shake>,
    mut health: ResMut<Health>,
    guards: Query<&Guard>,
) {
    let dt = time.delta_secs();
    shake.0 = (shake.0 - 1.8 * dt).max(0.0);
    health.hurt = (health.hurt - 1.5 * dt).max(0.0);
    let hunted = guards
        .iter()
        .any(|g| g.status == LostStatus::Standing && g.state == AiState::Chase);
    if !hunted && health.hp < 100.0 && health.hp > 0.0 {
        health.hp = (health.hp + 0.5 * dt).min(100.0);
    }
}

