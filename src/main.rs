use std::f32::consts::{PI, TAU};

use bevy::anti_alias::fxaa::Fxaa;
#[cfg(not(target_arch = "wasm32"))]
use bevy::anti_alias::taa::TemporalAntiAliasing;
use bevy::asset::AssetMetaCheck;
use bevy::camera::Hdr;
use bevy::camera_controller::free_camera::{FreeCamera, FreeCameraPlugin};
use bevy::light::{CascadeShadowConfigBuilder, NotShadowCaster};
#[cfg(not(target_arch = "wasm32"))]
use bevy::pbr::{ContactShadows, ScreenSpaceAmbientOcclusion};
use bevy::post_process::bloom::Bloom;
#[cfg(not(target_arch = "wasm32"))]
use bevy::post_process::motion_blur::MotionBlur;
use bevy::prelude::*;

use crate::noise::hash_noise;

pub mod noise;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            meta_check: AssetMetaCheck::Never,
            ..default()
        }))
        .add_plugins(FreeCameraPlugin)
        .insert_resource(GlobalAmbientLight::NONE)
        .add_systems(Startup, setup)
        .add_systems(Update, (interact, spawn_rats, move_rats, rats_reach_center))
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
    let obelisk_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.01, 0.004, 0.001),
        emissive: obelisk_color.to_linear(),
        ..default()
    });
    let light_entity = commands
        .spawn((
            PointLight {
                intensity: 800.0,
                radius: 0.125,
                shadow_maps_enabled: true,
                color: obelisk_color,
                ..default()
            },
            obelisk_pos,
        ))
        .id();
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.0).mesh().uv(64, 48))),
        MeshMaterial3d(obelisk_material.clone()),
        obelisk_pos,
        NotShadowCaster,
        Obelisk {
            health: 100.0,
            material: obelisk_material,
            light_entity,
        },
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
        MotionBlur {
            shutter_angle: 1.0,
            ..Default::default()
        },
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
            maximum_distance: 40.0,
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

#[derive(Component)]
struct Obelisk {
    pub health: f32,
    pub material: Handle<StandardMaterial>,
    light_entity: Entity,
}

#[derive(Component)]
struct Rat;

fn spawn_rats(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    mut time_since_last_spawn: Local<f32>,
) {
    let spawn_every = 0.01;
    *time_since_last_spawn += time.delta_secs();
    if *time_since_last_spawn >= spawn_every {
        let n = hash_noise(uvec2(0, 0), (time.elapsed_secs() / spawn_every) as u32);
        let x = (n * TAU).cos() * 100.0;
        let z = (n * TAU).sin() * 100.0;
        commands.spawn((
            SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset("rat1.glb"))),
            Transform::from_translation(vec3(x, 0.0, z)),
            Rat,
        ));
        *time_since_last_spawn = 0.0;
    }
}

fn move_rats(time: Res<Time>, mut rats: Query<&mut Transform, With<Rat>>) {
    let rat_speed = 30.0;
    for mut rat_trans in &mut rats {
        let dest = Vec3::ZERO;
        let dir = (dest - rat_trans.translation).normalize_or_zero();
        *rat_trans = rat_trans.looking_at(dest, Vec3::Y);
        rat_trans.translation += dir * time.delta_secs() * rat_speed;
    }
}

fn rats_reach_center(
    mut commands: Commands,
    rats: Query<(Entity, &Transform), With<Rat>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut obelisk: Single<&mut Obelisk>,
    mut lights: Query<&mut PointLight, Without<Obelisk>>,
) {
    for (entity, rat_trans) in &rats {
        if rat_trans.translation.distance(Vec3::ZERO) < 1.0 {
            commands.entity(entity).despawn();
            obelisk.health -= 1.0;
            if let Some(mut mat) = materials.get_mut(obelisk.material.id()) {
                mat.emissive = LinearRgba::from_vec3(mat.emissive.to_vec3() * 0.9);
            }
            if let Ok(mut obelisk_light) = lights.get_mut(obelisk.light_entity) {
                obelisk_light.intensity *= 0.9;
            }
        }
    }
}
