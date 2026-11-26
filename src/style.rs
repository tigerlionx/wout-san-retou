//! The look: a graphic-novel ink style. Desaturated cool greys, heavy black
//! outlines (inverted hull), foggy overcast light, and a few selective colour
//! accents that glow (neon sign, string lights, alert markers).

use crate::*;
use bevy::camera::Hdr;
use bevy::render::view::Msaa;
use bevy::light::NotShadowCaster;
use bevy::render::render_resource::Face;

#[derive(Resource, Clone)]
pub struct Ink {
    pub outline: Handle<StandardMaterial>,
}

/// A cool, slightly green-tinted grey (value 0..1).
pub fn tint(v: f32) -> Color {
    Color::srgb(v * 0.9, v, v * 0.96)
}

pub fn matte(m: &mut Assets<StandardMaterial>, v: f32) -> Handle<StandardMaterial> {
    m.add(StandardMaterial {
        base_color: tint(v),
        perceptual_roughness: 1.0,
        reflectance: 0.05,
        ..default()
    })
}

/// A glowing accent colour (the only saturated colour in the world).
pub fn glow(m: &mut Assets<StandardMaterial>, rgb: Vec3, power: f32) -> Handle<StandardMaterial> {
    m.add(StandardMaterial {
        base_color: Color::srgb(rgb.x, rgb.y, rgb.z),
        emissive: LinearRgba::rgb(rgb.x * power, rgb.y * power, rgb.z * power),
        ..default()
    })
}

/// Scale that thickens a box of `size` by `t` on every side.
pub fn grow_box(size: Vec3, t: f32) -> Vec3 {
    Vec3::ONE + Vec3::splat(2.0 * t) / size.max(Vec3::splat(0.05))
}

/// A mesh with a black inverted-hull outline child.
pub fn ink(
    mesh: Handle<Mesh>,
    mat: Handle<StandardMaterial>,
    outline: &Ink,
    tf: Transform,
    grow: Vec3,
) -> impl Bundle {
    (
        Mesh3d(mesh.clone()),
        MeshMaterial3d(mat),
        tf,
        children![(
            Mesh3d(mesh),
            MeshMaterial3d(outline.outline.clone()),
            Transform::from_scale(grow),
            NotShadowCaster,
        )],
    )
}

pub fn setup_camera(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut fonts: ResMut<Assets<Font>>,
) {
    // Replace the default font: Bevy's built-in subset has no accented letters.
    let _ = fonts.insert(
        Handle::<Font>::default().id(),
        Font::from_bytes(include_bytes!("assets/FiraMono-Medium.ttf").to_vec()),
    );
    commands.insert_resource(Ink {
        outline: materials.add(StandardMaterial {
            base_color: Color::BLACK,
            unlit: true,
            cull_mode: Some(Face::Front),
            ..default()
        }),
    });
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.85, 0.9, 0.9),
        brightness: 320.0,
        ..default()
    });
    let mut camera = commands.spawn((
        GameCamera,
        Camera3d::default(),
        Tonemapping::TonyMcMapface,
        // 4x MSAA at Retina resolution is too heavy for laptop GPUs; FXAA softens the ink edges for far less.
        // WSR_AA=msaa brings the old look back for comparison.
        if std::env::var("WSR_AA").is_ok_and(|v| v == "msaa") { Msaa::Sample4 } else { Msaa::Off },
        DistanceFog {
            color: Color::srgb(0.62, 0.66, 0.66),
            falloff: FogFalloff::Linear {
                start: 22.0,
                end: 70.0,
            },
            ..default()
        },
        Transform::from_xyz(0.0, 14.0, 40.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    // HDR and bloom are skipped in the browser: they are too heavy for WebGL on a laptop GPU.
    #[cfg(not(target_arch = "wasm32"))]
    camera.insert((Hdr, Bloom::NATURAL));
    #[cfg(target_arch = "wasm32")]
    let _ = &mut camera;
}

/// Browser build: no shadow maps (the sun's cascades and the point-light cubes are the heaviest part of a frame).
#[cfg(target_arch = "wasm32")]
pub fn web_lite(
    mut sun: Query<&mut DirectionalLight, Added<DirectionalLight>>,
    mut points: Query<&mut PointLight, Added<PointLight>>,
    mut spots: Query<&mut SpotLight, Added<SpotLight>>,
) {
    for mut l in &mut sun {
        l.shadow_maps_enabled = false;
    }
    for mut l in &mut points {
        l.shadow_maps_enabled = false;
    }
    for mut l in &mut spots {
        l.shadow_maps_enabled = false;
    }
}

