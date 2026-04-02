use std::f32::consts::PI;

use bevy::anti_alias::fxaa::Fxaa;
use bevy::anti_alias::taa::TemporalAntiAliasing;
use bevy::asset::AssetMetaCheck;
use bevy::camera::Hdr;
use bevy::camera_controller::free_camera::{FreeCamera, FreeCameraPlugin};
use bevy::light::{CascadeShadowConfigBuilder, NotShadowCaster};
use bevy::math::VectorSpace;
use bevy::pbr::{ContactShadows, ScreenSpaceAmbientOcclusion};
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            meta_check: AssetMetaCheck::Never,
            ..default()
        }))
        .add_plugins(FreeCameraPlugin)
        .insert_resource(GlobalAmbientLight::NONE)
        .add_systems(Startup, setup)
        .add_systems(Update, interact)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn(SceneRoot(
        asset_server.load(GltfAssetLabel::Scene(0).from_asset("rat1.glb")),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(500.0, 500.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.001, 0.05, 0.001))),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.5, 0.5, 0.5))),
        MeshMaterial3d(materials.add(Color::srgb_u8(128, 128, 128))),
        Transform::from_xyz(0.0, 0.5, 0.0),
        CursorObject,
    ));

    // obelisk or somesuch
    let obelisk_color = Color::srgb(10.0, 4.0, 1.0);
    let obelisk_pos = Transform::from_xyz(0.0, 1.25, 0.0);
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.0).mesh().uv(64, 48))),
        MeshMaterial3d(materials.add(obelisk_color)),
        obelisk_pos,
        NotShadowCaster,
    ));
    commands.spawn((
        PointLight {
            intensity: 800.0,
            radius: 0.125,
            shadow_maps_enabled: true,
            color: obelisk_color,
            ..default()
        },
        obelisk_pos,
    ));

    // camera
    let mut camera_emcds = commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(10.0, 10.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        FreeCamera {
            walk_speed: 10.0,
            run_speed: 20.0,
            ..default()
        },
        EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            intensity: 1000.0,
            ..default()
        },
        Hdr,
        Bloom {
            intensity: 0.1,
            ..default()
        },
    ));

    #[cfg(not(target_arch = "wasm32"))]
    camera_emcds.insert((
        Msaa::Off,
        Fxaa::default(),
        ContactShadows::default(),
        ScreenSpaceAmbientOcclusion::default(),
        TemporalAntiAliasing::default(),
    ));

    #[cfg(target_arch = "wasm32")]
    camera_emcds.insert(Fxaa::default());

    commands.spawn((
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, PI * -0.35, PI * -0.13, 0.0)),
        DirectionalLight {
            color: Color::srgb(1.0, 0.87, 0.78),
            illuminance: 5000.0,
            shadow_maps_enabled: true,
            contact_shadows_enabled: true,
            shadow_depth_bias: 0.03,
            shadow_normal_bias: 0.05,
            ..default()
        },
        CascadeShadowConfigBuilder {
            num_cascades: 2,
            minimum_distance: 0.05,
            maximum_distance: 30.0,
            first_cascade_far_bound: 5.0,
            overlap_proportion: 0.2,
        }
        .build(),
    ));
}

fn interact(
    mut commands: Commands,
    window: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform)>,
    mut cursor_trans: Single<&mut Transform, With<CursorObject>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    let (camera, camera_transform) = *camera;
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_pos) else {
        return;
    };
    if let Some(t) = ray.intersect_plane(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y)) {
        let hitp = ray.origin + ray.direction.as_vec3() * t;
        cursor_trans.translation = hitp;
        if buttons.just_pressed(MouseButton::Left) {
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.5, 2.0, 0.5))),
                MeshMaterial3d(materials.add(Color::srgb_u8(128, 128, 128))),
                Transform::from_translation(hitp),
                Turret,
            ));
        }
    }
}

#[derive(Component)]
struct CursorObject;

#[derive(Component)]
struct Turret;
