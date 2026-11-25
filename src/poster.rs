//! Key art: `WSR_POSTER=1 WSR_AUTOPLAY=1 WSR_SIZE=2400x3600 ./wout-san-retou` renders the cast on a flat key colour,
//! posed from the game's own animation clips, lit like a horror poster. tools/make_poster.py cuts them out of the key
//! colour and paints the vèvè sky, the title and the words around them.
use crate::character::{Cast, Mover, PosterPose, Skin};
use crate::style::Ink;
use crate::weapons::WeaponKind;
use crate::*;

/// Inserted by the autoplay script when it is time to build the scene.
#[derive(Resource)]
pub struct PosterRequest;

#[derive(Component)]
pub struct PosterEntity;

pub fn on() -> bool {
    std::env::var_os("WSR_POSTER").is_some()
}

/// The camera: low and close, looking slightly up at the hero with the horde rising behind him.
const CAM_POS: Vec3 = Vec3::new(0.0, 3.0, 11.8);
const CAM_LOOK: Vec3 = Vec3::new(0.0, 0.75, -2.6);

fn env_f(name: &str, default: f32) -> f32 {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

/// (who, x, z, how far he turns from facing the camera, clip, seconds into the clip, weapon)
type Slot = (Cast, f32, f32, f32, &'static str, f32, Option<WeaponKind>);

const CAST: [Slot; 16] = [
    // the front: Ro, and the man he came for
    (Cast::Hero, -0.8, 4.3, 0.5, "aim_idle", 0.0, Some(WeaponKind::Ak47)),
    (Cast::Nwit, 1.05, 3.5, -0.15, "stab", 0.35, Some(WeaponKind::Knife)),
    // the bosses
    (Cast::Flanm, -1.9, 1.0, 0.3, "idle", 0.0, None),
    (Cast::Woch, 2.1, 0.8, -0.3, "idle", 0.0, None),
    (Cast::Manchet, 0.0, -0.8, 0.0, "idle", 0.0, None),
    // the risen, rank on rank
    (Cast::Brute, -1.4, -2.4, 0.25, "zombie_attack", 0.3, None),
    (Cast::Yaku, 1.5, -2.6, -0.25, "zombie_walk", 0.2, None),
    (Cast::Prisoner, -2.5, -3.6, 0.5, "zombie_walk", 0.6, None),
    (Cast::Prisoner, 2.5, -3.8, -0.5, "zombie_attack", 0.5, None),
    (Cast::Brute, -0.5, -4.8, 0.1, "zombie_walk", 0.4, None),
    (Cast::Yaku, 1.0, -5.6, -0.1, "zombie_idle", 0.0, None),
    (Cast::Prisoner, -1.9, -6.2, 0.3, "zombie_walk", 0.1, None),
    (Cast::Prisoner, 2.0, -7.0, -0.3, "zombie_walk", 0.8, None),
    (Cast::Brute, -3.2, -7.6, 0.3, "zombie_attack", 0.6, None),
    (Cast::Yaku, 0.2, -8.4, 0.0, "zombie_walk", 0.3, None),
    (Cast::Prisoner, 2.9, -8.2, -0.3, "zombie_walk", 0.5, None),
];

#[allow(clippy::too_many_arguments)]
pub fn spawn_poster(
    req: Option<Res<PosterRequest>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    ink: Res<Ink>,
    level: Query<Entity, (With<LevelEntity>, Without<GameCamera>, Without<Player>)>,
    lights: Query<(), Or<(With<DirectionalLight>, With<PointLight>, With<SpotLight>)>>,
    player: Single<(&mut Transform, &mut Mover), With<Player>>,
) {
    if req.is_none() {
        return;
    }
    commands.remove_resource::<PosterRequest>();
    // the level goes: ground, walls, guards, their vision-cone gizmos
    for e in &level {
        if lights.get(e).is_err() {
            commands.entity(e).despawn();
        }
    }
    let (mut ptf, mut pmv) = player.into_inner();
    ptf.translation = Vec3::new(400.0, 0.0, 400.0);
    pmv.vel = Vec3::ZERO;

    let turn = env_f("WSR_POSTER_TURN", 1.0);
    for (cast, x, z, off, clip, t, weapon) in CAST {
        let zombie = matches!(cast, Cast::Brute | Cast::Yaku | Cast::Prisoner);
        let armed = weapon == Some(WeaponKind::Ak47);
        let e = commands
            .spawn((
                PosterEntity,
                Transform::from_translation(Vec3::new(x, 0.0, z)).with_rotation(Quat::from_rotation_y(std::f32::consts::PI + off * turn)),
                Visibility::default(),
                Mover { grounded: true, zombie, armed, aiming: armed, ..default() },
                Skin(cast),
                PosterPose { clip, t },
            ))
            .id();
        if let Some(kind) = weapon {
            crate::weapons::poster_weapon(&mut commands, &mut meshes, &mut mats, &ink, e, kind);
        }
    }

    // the light: cold moon from behind, amber on their faces, a red glow under the horde, violet at the back
    commands.spawn((
        PosterEntity,
        PointLight { color: Color::srgb(1.0, 0.8, 0.52), intensity: 7_500_000.0, range: 30.0, shadow_maps_enabled: true, ..default() },
        Transform::from_xyz(4.5, 3.5, 8.0),
    ));
    commands.spawn((
        PosterEntity,
        PointLight { color: Color::srgb(0.95, 0.12, 0.1), intensity: 250_000.0, range: 14.0, ..default() },
        Transform::from_xyz(0.0, 0.5, -2.5),
    ));
    commands.spawn((
        PosterEntity,
        PointLight { color: Color::srgb(0.6, 0.35, 1.0), intensity: 11_000_000.0, range: 30.0, ..default() },
        Transform::from_xyz(-5.0, 4.5, -5.0),
    ));
    commands.spawn((
        PosterEntity,
        PointLight { color: Color::srgb(0.15, 0.7, 0.95), intensity: 12_000_000.0, range: 30.0, ..default() },
        Transform::from_xyz(6.0, 3.0, -4.0),
    ));
}

/// Put the camera where the poster wants it, after the game's own camera has run.
pub fn poster_camera(mut cam: Single<(&mut Transform, &mut Projection), With<GameCamera>>) {
    let (tf, proj) = &mut *cam;
    let pos = Vec3::new(env_f("WSR_POSTER_CX", CAM_POS.x), env_f("WSR_POSTER_CY", CAM_POS.y), env_f("WSR_POSTER_CZ", CAM_POS.z));
    let look = Vec3::new(CAM_LOOK.x, env_f("WSR_POSTER_LY", CAM_LOOK.y), CAM_LOOK.z);
    **tf = Transform::from_translation(pos).looking_at(look, Vec3::Y);
    if let Projection::Perspective(p) = &mut **proj {
        p.fov = env_f("WSR_POSTER_FOV", 34.0).to_radians();
    }
}

/// After the level's lighting: dim violet ambient, a cold sun, no fog, and a flat key colour behind everything.
pub fn poster_lighting(
    mut sun: Single<&mut DirectionalLight, With<Sun>>,
    mut ambient: ResMut<GlobalAmbientLight>,
    mut clear: ResMut<ClearColor>,
    mut fog: Single<&mut DistanceFog, With<GameCamera>>,
    mut ui: Query<&mut Camera, With<crate::hud::UiCamera>>,
) {
    sun.color = Color::srgb(0.55, 0.62, 1.0);
    sun.illuminance = 1800.0;
    ambient.color = Color::srgb(0.5, 0.4, 0.8);
    ambient.brightness = 200.0;
    clear.0 = Color::srgb(1.0, 0.0, 1.0);
    fog.falloff = FogFalloff::Linear { start: 900.0, end: 1000.0 };
    for mut c in &mut ui {
        c.is_active = false;
    }
}

/// Granmet Nwit's bun of folded locks reads as an umbrella from this camera: leave it off the poster.
pub fn hide_bun(mut q: Query<(&Name, &mut Visibility)>) {
    for (name, mut vis) in &mut q {
        if name.as_str().contains("dread") || name.as_str().contains("bun_cord") {
            *vis = Visibility::Hidden;
        }
    }
}
