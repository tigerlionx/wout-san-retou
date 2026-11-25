//! Full player mobility (walk, sprint, crouch, jump, step onto low things,
//! 360-degree look) and the camera (high-angle follow or first person).

use crate::character::{ground_at, resolve, Boxes};
use crate::*;
use bevy::input::mouse::AccumulatedMouseMotion;

const WALK: f32 = 4.2;
const SPRINT: f32 = 7.0;
const CROUCH: f32 = 2.2;
const PRONE: f32 = 1.0;
const JUMP_SPEED: f32 = 5.6;
const GRAVITY: f32 = 15.0;
const RADIUS: f32 = 0.38;

pub fn control(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    weight: Res<Weight>,
    poison: Res<Poison>,
    grab: Res<crate::level3::Grab>,
    water: Option<Res<crate::level3::Water>>,
    holes: Option<Res<crate::level4::Holes>>,
    loadout: Res<crate::weapons::Loadout>,
    mut health: ResMut<crate::weapons::Health>,
    rig: Res<CamRig>,
    boxes: Res<Boxes>,
    mut stats: ResMut<Stats>,
    mut last_hurt: Local<f32>,
    player: Single<(&mut Transform, &mut Mover), With<Player>>,
) {
    let dt = time.delta_secs().min(0.1);
    stats.elapsed += dt;
    let (mut tf, mut mv) = player.into_inner();

    // Camera-relative input.
    let fwd = Vec3::new(-rig.yaw.sin(), 0.0, -rig.yaw.cos());
    let right = Vec3::new(rig.yaw.cos(), 0.0, -rig.yaw.sin());
    let mut dir = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) {
        dir += fwd;
    }
    if keys.pressed(KeyCode::KeyS) {
        dir -= fwd;
    }
    if keys.pressed(KeyCode::KeyD) {
        dir += right;
    }
    if keys.pressed(KeyCode::KeyA) {
        dir -= right;
    }
    let mut dir = dir.normalize_or_zero();
    if poison.drugged > 0.0 {
        // Drugged: the controls drift and the world sways.
        dir = Quat::from_rotation_y((poison.drugged * 1.7).sin() * 0.9) * dir;
    }
    mv.moving = dir != Vec3::ZERO;

    // Crouch eases in and out; sprinting is impossible while crouched.
    // Z lies down and gets up; Space or C stands you back up.
    if keys.just_pressed(KeyCode::KeyZ) {
        mv.prone_on = !mv.prone_on;
    } else if mv.prone_on && (keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::KeyC)) {
        mv.prone_on = false;
    }
    let want_crouch = mv.prone_on || keys.pressed(KeyCode::KeyC) || keys.pressed(KeyCode::ControlLeft);
    let target = if want_crouch { 1.0 } else { 0.0 };
    mv.crouch += (target - mv.crouch) * (1.0 - (-10.0 * dt).exp());
    let prone_target = if mv.prone_on { 1.0 } else { 0.0 };
    mv.prone += (prone_target - mv.prone) * (1.0 - (-7.0 * dt).exp());
    mv.aiming = rig.aim > 0.5;
    mv.pistol = loadout.current == crate::weapons::WeaponKind::Pistol;
    mv.sprinting = keys.pressed(KeyCode::ShiftLeft) && mv.moving && mv.crouch < 0.4 && !mv.prone_on;

    let mut speed = if mv.prone_on {
        PRONE
    } else if mv.crouch > 0.5 {
        CROUCH
    } else if mv.sprinting {
        SPRINT
    } else {
        WALK
    };
    // The Weight you carry slows you down; so do clinging hands and wading.
    speed *= 1.0 - 0.4 * (weight.0 / 100.0);
    speed *= 1.0 - 0.4 * rig.aim; // aiming down the sights: slow and steady
    speed /= 1.0 + 0.45 * grab.0.min(4) as f32;
    if loadout.current == crate::weapons::WeaponKind::M50 {
        // A belt-fed machine gun is heavy; in tight quarters it is worse.
        speed *= if loadout.cramped { 0.5 } else { 0.7 };
    }
    let in_water = water.as_ref().is_some_and(|w| {
        tf.translation.x > w.min.x && tf.translation.x < w.max.x && tf.translation.z > w.min.y && tf.translation.z < w.max.y && tf.translation.y < w.y_max
    });
    if in_water && mv.grounded {
        speed *= 0.6;
    }

    // Smooth horizontal velocity; less control in the air.
    let k = 1.0 - (-(if mv.grounded { 14.0 } else { 5.0 }) * dt).exp();
    let want = dir * speed;
    mv.vel.x += (want.x - mv.vel.x) * k;
    mv.vel.z += (want.z - mv.vel.z) * k;

    // Jump.
    if keys.just_pressed(KeyCode::Space) && mv.grounded && mv.crouch < 0.5 {
        mv.vel.y = JUMP_SPEED;
        mv.grounded = false;
        mv.noise = mv.noise.max(6.0);
    }
    mv.vel.y -= GRAVITY * dt;

    // Integrate, collide, land.
    let mut p = tf.translation + mv.vel * dt;
    p = resolve(p, RADIUS, &boxes.0);
    // A collapsed floor: below it the pit has its own ground and walls.
    let hole = holes.as_ref().and_then(|h| h.at(p.x, p.z));
    let mut floor = ground_at(p, &boxes.0);
    if let (Some(h), Some(hs)) = (hole, holes.as_ref()) {
        floor = hs.pit_ground(h, p, &boxes.0);
    }
    if let Some(hs) = holes.as_ref() {
        if p.y < -0.5 {
            p = hs.keep_in_pit(p);
        }
    }
    if p.y <= floor && mv.vel.y <= 0.0 {
        if !mv.grounded && mv.vel.y < -3.0 {
            mv.noise = mv.noise.max(8.0);
        }
        if mv.vel.y < -9.0 {
            // A long fall hurts.
            health.hp -= 18.0 + (-mv.vel.y - 9.0) * 3.0;
            health.hurt = 1.0;
        }
        p.y = floor;
        mv.vel.y = 0.0;
        mv.grounded = true;
    } else {
        mv.grounded = false;
    }
    tf.translation = p;
    mv.speed = Vec2::new(mv.vel.x, mv.vel.z).length();
    // Animation hints: time in the air, reloading, a flinch when hurt.
    mv.air = if mv.grounded { 0.0 } else { mv.air + dt };
    mv.reloading = loadout.reload > 0.0;
    if health.hurt > 0.8 && *last_hurt <= 0.8 {
        mv.flinch = 1.0;
    }
    *last_hurt = health.hurt;

    // How far you can be heard.
    let base: f32 = if !mv.moving || !mv.grounded {
        0.0
    } else if mv.prone_on {
        0.8
    } else if mv.crouch > 0.5 {
        1.5
    } else if mv.sprinting {
        11.0
    } else {
        4.5
    } + if in_water && mv.moving { 4.0 } else { 0.0 };
    mv.noise = base.max(mv.noise - 20.0 * dt);

    // Facing: towards the camera in first person, else towards the motion.
    let facing = if rig.first {
        Some(Vec3::new(-rig.yaw.sin(), 0.0, -rig.yaw.cos()))
    } else if mv.moving {
        Some(dir)
    } else {
        None
    };
    if let Some(d) = facing {
        let yaw = (-d.x).atan2(-d.z);
        tf.rotation = tf
            .rotation
            .slerp(Quat::from_rotation_y(yaw), (12.0 * dt).min(1.0));
    }
}

/// Camera: V toggles first / third person; right-drag or the arrow keys turn
/// 360 degrees; in third person the angle is adjustable (mouse up/down).
pub fn camera(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    mut rig: ResMut<CamRig>,
    shake: Res<crate::weapons::Shake>,
    mut smooth: Local<Option<Vec3>>,
    player: Single<(&Transform, &Mover, &mut Visibility), (With<Player>, Without<GameCamera>)>,
    mut cam: Single<(&mut Transform, &mut Projection), With<GameCamera>>,
) {
    let (mut cam, mut proj) = cam.into_inner();
    let dt = time.delta_secs().min(0.1);
    if keys.just_pressed(KeyCode::KeyV) {
        rig.first = !rig.first;
    }
    if keys.pressed(KeyCode::ArrowLeft) {
        rig.yaw += 1.8 * dt;
    }
    if keys.pressed(KeyCode::ArrowRight) {
        rig.yaw -= 1.8 * dt;
    }
    if rig.first {
        // First person: the mouse always looks (the cursor is locked, see cursor_mode).
        rig.yaw -= motion.delta.x * 0.0025;
        rig.pitch_first = (rig.pitch_first - motion.delta.y * 0.0025).clamp(-1.4, 1.4);
    } else if mouse.pressed(MouseButton::Right) {
        // Third person: hold the right button to turn; the cursor is your aim.
        rig.yaw -= motion.delta.x * 0.004;
        rig.pitch_third = (rig.pitch_third - motion.delta.y * 0.004).clamp(-1.35, -0.3);
    }
    // Recoil settles back quickly.
    rig.recoil *= (-7.0 * dt).exp();
    // Right mouse button in first person: aim down the sights (eased in and out).
    let want_aim = if rig.first && mouse.pressed(MouseButton::Right) { 1.0 } else { 0.0 };
    rig.aim += (want_aim - rig.aim) * (1.0 - (-12.0 * dt).exp());
    // Field of view: wide in first person, narrower when aiming, the old 45 degrees from above.
    let fov = if rig.first { 1.25 - 0.35 * rig.aim } else { std::f32::consts::FRAC_PI_4 };
    if let Projection::Perspective(p) = &mut *proj {
        p.fov += (fov - p.fov) * (1.0 - (-10.0 * dt).exp());
    }

    let (ptf, mv, mut vis) = player.into_inner();
    if rig.first {
        *vis = Visibility::Hidden;
        // A gentle walking bob, larger when running, none in the air.
        let t = time.elapsed_secs();
        let bob = if mv.grounded { (mv.speed / 4.2).min(1.7) } else { 0.0 };
        let eye_raw = ptf.translation + Vec3::Y * (1.55 - 0.55 * mv.crouch - 0.55 * mv.prone + (t * 9.0).sin().abs() * 0.035 * bob * (1.0 - 0.8 * rig.aim));
        // Smoothed, so steps, ramps and landings do not jerk the view.
        let prev = smooth.unwrap_or(eye_raw);
        let eye = Vec3::new(eye_raw.x, prev.y + (eye_raw.y - prev.y) * (1.0 - (-18.0 * dt).exp()), eye_raw.z);
        *smooth = Some(eye);
        cam.translation = eye;
        cam.rotation = Quat::from_euler(EulerRot::YXZ, rig.yaw, (rig.pitch_first + rig.recoil).clamp(-1.45, 1.45), 0.0);
    } else {
        *vis = Visibility::Inherited;
        // WSR_CAMDIST / WSR_CAMPITCH: close-up views for inspecting characters.
        let dist = std::env::var("WSR_CAMDIST").ok().and_then(|v| v.parse().ok()).unwrap_or(12.0);
        let pitch = std::env::var("WSR_CAMPITCH").ok().and_then(|v| v.parse().ok()).unwrap_or(rig.pitch_third);
        let yaw_off: f32 = std::env::var("WSR_CAMYAW").ok().and_then(|v| v.parse().ok()).unwrap_or(0.0);
        let rot = Quat::from_euler(EulerRot::YXZ, rig.yaw + yaw_off, pitch, 0.0);
        // Follow smoothly: a short ease on the vertical, so jumps and steps do not jerk the view.
        let raw = ptf.translation + Vec3::Y * 1.0;
        let prev = smooth.unwrap_or(raw);
        let target = Vec3::new(raw.x, prev.y + (raw.y - prev.y) * (1.0 - (-12.0 * dt).exp()), raw.z);
        *smooth = Some(target);
        cam.rotation = rot;
        cam.translation = target - rot * Vec3::NEG_Z * dist;
    }
    // Trauma-based shake from gunfire and blasts.
    if shake.0 > 0.0 {
        let t = time.elapsed_secs();
        let k = shake.0 * shake.0;
        cam.translation += Vec3::new((t * 53.0).sin(), (t * 67.0).sin(), (t * 41.0).sin()) * 0.35 * k;
        cam.rotation *= Quat::from_rotation_z((t * 59.0).sin() * 0.03 * k);
    }
}

/// First person: lock and hide the cursor so the mouse can look around.
/// Third person and the menus: a normal cursor (it is your aim in third person).
pub fn cursor_mode(
    mut cursor: Single<&mut bevy::window::CursorOptions>,
    rig: Res<CamRig>,
    state: Res<State<AppState>>,
    mut automated: Local<Option<bool>>,
) {
    // Scripted playtests must not grab your mouse.
    let auto = *automated.get_or_insert_with(|| std::env::var_os("WSR_AUTOPLAY").is_some());
    let playing = matches!(
        state.get(),
        AppState::Playing
            | AppState::Level2
            | AppState::Level3
            | AppState::Level4
            | AppState::Level5
            | AppState::Level6
            | AppState::Level7
    );
    let lock = !auto && rig.first && playing;
    let (visible, mode) = if lock {
        (false, bevy::window::CursorGrabMode::Locked)
    } else {
        (true, bevy::window::CursorGrabMode::None)
    };
    if cursor.visible != visible || cursor.grab_mode != mode {
        cursor.visible = visible;
        cursor.grab_mode = mode;
    }
}

/// Ro's phone light: a soft cool glow that follows him through the dark floors (the house, the hall,
/// Nwit's quarters), so he is never lost in the black. Players hate not seeing.
#[derive(Component)]
pub struct PhoneLight;

pub fn phone_light(
    mut commands: Commands,
    existing: Query<(), With<PhoneLight>>,
    player: Single<Entity, With<Player>>,
) {
    if !existing.is_empty() {
        return;
    }
    commands.spawn((
        PhoneLight,
        PointLight {
            color: Color::srgb(0.82, 0.9, 1.0),
            intensity: 650_000.0,
            range: 11.0,
            ..default()
        },
        Transform::from_xyz(0.0, 2.3, 0.4),
        ChildOf(*player),
    ));
}
