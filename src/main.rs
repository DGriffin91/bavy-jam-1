#[cfg(not(target_arch = "wasm32"))]
use argh::FromArgs;
use bevy::anti_alias::fxaa::Fxaa;
#[cfg(not(target_arch = "wasm32"))]
use bevy::anti_alias::taa::TemporalAntiAliasing;
use bevy::asset::AssetMetaCheck;
use bevy::camera::Hdr;
use bevy::camera_controller::free_camera::{FreeCamera, FreeCameraPlugin};
use bevy::light::{CascadeShadowConfigBuilder, NotShadowCaster};
#[cfg(not(target_arch = "wasm32"))]
use bevy::pbr::{ContactShadows, ScreenSpaceAmbientOcclusion};
use bevy::post_process::bloom::{Bloom, BloomCompositeMode, BloomPrefilter};
use bevy::post_process::dof::DepthOfField;
#[cfg(not(target_arch = "wasm32"))]
use bevy::post_process::motion_blur::MotionBlur;
use bevy::prelude::*;
use std::f32::consts::{PI, TAU};
use std::ops::DerefMut;

use crate::noise::hash_noise;

pub mod noise;

#[cfg(not(target_arch = "wasm32"))]
#[derive(FromArgs, Resource, Clone)]
/// Config
pub struct Args {
    /// enable ssr
    #[argh(switch)]
    ssr: bool,

    /// enable dof
    #[argh(switch)]
    dof: bool,
}

fn main() {
    let mut app = App::new();

    #[cfg(not(target_arch = "wasm32"))]
    {
        let args: Args = argh::from_env();
        app.insert_resource(args.clone());
        if args.ssr {
            app.insert_resource(bevy::pbr::DefaultOpaqueRendererMethod::deferred());
        }
    }

    app.insert_resource(ClearColor(Color::BLACK))
        .insert_resource(PlayerData {
            monies: STARTING_MONIES,
            kills: 0,
            started: false,
        })
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            meta_check: AssetMetaCheck::Never,
            ..default()
        }))
        .add_plugins(FreeCameraPlugin)
        .insert_resource(GlobalAmbientLight::NONE)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                interact,
                spawn_rats,
                move_rats,
                rats_reach_center,
                set_hud_ui,
                make_turrets_face_camera,
                lasers_shoot_at_rats,
            ),
        )
        .run();
}

const OBELISK_COLOR: Color = Color::srgb(10.0, 4.0, 1.0);
const MAX_HEALTH: f32 = 200.0;
const LASER_MAX_RANGE: f32 = 30.0;
const TURRET_DMG: f32 = 350.0;
const PAY_FOR_KILL: u32 = 2;
const TURRET_COST: u32 = 100;
const STARTING_MONIES: u32 = 200;
const INITIAL_RAT_SPEED: f32 = 9.0;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    #[cfg(not(target_arch = "wasm32"))] args: Res<Args>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(5000.0, 5000.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::BLACK,
            perceptual_roughness: 0.3,
            ..default()
        })),
    ));

    commands.spawn((
        SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset("turret.glb"))),
        Transform::from_scale(Vec3::splat(0.75)),
        CursorObject,
    ));

    // obelisk or somesuch
    let obelisk_pos = Transform::from_xyz(0.0, 1.25, 0.0);
    let obelisk_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.0, 0.0, 0.0),
        emissive: OBELISK_COLOR.to_linear() * 10.0,
        ..default()
    });
    let light_entity = commands
        .spawn((
            PointLight {
                intensity: 5000.0,
                radius: 0.125,
                shadow_maps_enabled: true,
                color: OBELISK_COLOR,
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
            health: MAX_HEALTH,
            material: obelisk_material,
            light_entity,
        },
    ));

    // camera
    let mut camera_emcds = commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(20.0, 20.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
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
            #[cfg(not(target_arch = "wasm32"))]
            intensity: 0.4,
            #[cfg(target_arch = "wasm32")]
            intensity: 0.2, // Stronger for some reason
            low_frequency_boost: 0.4,
            low_frequency_boost_curvature: 0.95,
            high_pass_frequency: 1.0,
            prefilter: BloomPrefilter {
                threshold: 0.0,
                threshold_softness: 0.0,
            },
            composite_mode: BloomCompositeMode::Additive,
            scale: Vec2::ONE,
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
            shutter_angle: 4.0,
            ..Default::default()
        },
    ));

    #[cfg(not(target_arch = "wasm32"))]
    if args.ssr {
        camera_emcds.insert(bevy::pbr::ScreenSpaceReflections {
            min_perceptual_roughness: 0.08..0.12,
            max_perceptual_roughness: 0.55..0.6,
            linear_steps: 10,
            bisection_steps: 5,
            use_secant: true,
            thickness: 0.25,
            linear_march_exponent: 1.0,
            edge_fadeout: 0.0..0.0,
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    if args.dof {
        camera_emcds.insert(bevy::post_process::dof::DepthOfField {
            mode: bevy::post_process::dof::DepthOfFieldMode::Bokeh,
            focal_distance: 10.0,
            aperture_f_stops: 0.2,
            sensor_height: 0.1866,
            max_circle_of_confusion_diameter: 64.0,
            max_depth: f32::INFINITY,
        });
    }

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
            maximum_distance: 200.0,
            first_cascade_far_bound: 100.0,
            overlap_proportion: 0.2,
        }
        .build(),
    ));

    commands
        .spawn((
            Node {
                left: px(1.5),
                top: px(1.5),
                ..default()
            },
            GlobalZIndex(-1),
        ))
        .with_children(|parent| {
            parent.spawn((Text::new(""), TextColor(Color::BLACK), EconText));
        });
    commands.spawn(Node::default()).with_children(|parent| {
        parent.spawn((Text::new(""), TextColor(Color::WHITE), EconText));
    });
}

fn interact(
    mut commands: Commands,
    window: Single<&Window>,
    mut camera: Single<
        (&Camera, &GlobalTransform, Option<&mut DepthOfField>),
        Without<CursorObject>,
    >,
    mut cursor_trans: Single<&mut Transform, With<CursorObject>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut player: ResMut<PlayerData>,
    asset_server: Res<AssetServer>,
) {
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    let (camera, camera_transform, dof) = camera.deref_mut();
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_pos) else {
        return;
    };
    if let Some(t) = ray.intersect_plane(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y)) {
        let hitp = ray.origin + ray.direction.as_vec3() * t;
        if let Some(dof) = dof {
            dof.focal_distance = hitp.distance(camera_transform.translation());
        }
        if player.monies >= TURRET_COST {
            cursor_trans.translation = hitp;
            if buttons.just_pressed(MouseButton::Left) {
                player.started = true;
                player.monies -= TURRET_COST;
                commands.spawn((
                    SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset("turret.glb"))),
                    Transform::from_translation(hitp).with_scale(Vec3::splat(0.75)),
                    Turret,
                ));
                commands.spawn((
                    SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset("laser.glb"))),
                    Transform::from_translation(vec3(hitp.x, 2.95, hitp.z)),
                    Laser::default(),
                ));
            }
        } else {
            cursor_trans.translation = vec3(0.0, -100.0, 0.0);
        }
    }
}

fn set_hud_ui(
    mut text: Query<&mut Text, With<EconText>>,
    player: Res<PlayerData>,
    obelisk: Single<&Obelisk>,
) {
    for mut t in &mut text {
        t.0 = if player.started {
            format!(
                "$$$$$$ {}\nKILLLS {}\nHEALTH {}",
                player.monies, player.kills, obelisk.health as u32
            )
        } else {
            String::from("PLACE TURRET TO START")
        };
    }
}

#[derive(Component)]
struct CursorObject;

#[derive(Component)]
struct Turret;

#[derive(Component, Default)]
struct Laser {
    target: Option<Entity>,
}

#[derive(Component)]
struct Obelisk {
    pub health: f32,
    pub material: Handle<StandardMaterial>,
    light_entity: Entity,
}

#[derive(Component)]
struct Rat {
    health: f32,
}

impl Default for Rat {
    fn default() -> Self {
        Self { health: 100.0 }
    }
}

#[derive(Component)]
struct EconText;

#[derive(Resource)]
struct PlayerData {
    monies: u32,
    kills: u32,
    started: bool,
}

fn spawn_rats(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    mut time_since_last_spawn: Local<f32>,
    player: Res<PlayerData>,
    mut player_started_offset: Local<f32>,
) {
    if !player.started {
        *player_started_offset = time.elapsed_secs();
        return;
    }
    let elapse = time.elapsed_secs() - *player_started_offset;
    let spawn_every = 200.0 / (elapse.powf(2.5));
    *time_since_last_spawn += time.delta_secs();
    if *time_since_last_spawn >= spawn_every {
        let spawn_count = (*time_since_last_spawn / spawn_every) as u32;
        for i in 0..spawn_count {
            let n = hash_noise(uvec2(i, 0), (elapse / spawn_every) as u32);
            let x = (n * TAU).cos() * 100.0;
            let z = (n * TAU).sin() * 100.0;
            commands.spawn((
                SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset("rat1.glb"))),
                Transform::from_translation(vec3(x, 0.0, z))
                    .with_scale(1.0 + Vec3::splat(elapse.powf(0.3))),
                Rat::default(),
            ));
        }
        *time_since_last_spawn = 0.0;
    }
}

fn move_rats(
    time: Res<Time>,
    mut rats: Query<&mut Transform, With<Rat>>,
    turrets_trans: Query<&Transform, (With<Turret>, Without<Rat>)>,
) {
    let rat_speed = INITIAL_RAT_SPEED + time.elapsed_secs() * 0.2;
    let spread = 3.0;
    for (i, mut rat_trans) in &mut rats.iter_mut().enumerate() {
        let v = hash_noise(uvec2(i as u32, 0), time.elapsed_secs() as u32);
        let scale = hash_noise(uvec2(i as u32, 1), time.elapsed_secs() as u32);
        let mut dest = vec3(
            (v * TAU).sin() * spread * scale,
            0.0,
            (v * TAU).cos() * spread * scale,
        );
        for turrets_trans in &turrets_trans {
            if turrets_trans.translation.distance(rat_trans.translation) < 2.0 {
                dest = turrets_trans.translation
                    + rat_trans.translation.cross(turrets_trans.translation) * 5.0;
            }
        }
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
    player: Res<PlayerData>,
) {
    let relative_health = obelisk.health / MAX_HEALTH;
    for (entity, rat_trans) in &rats {
        if rat_trans.translation.distance(Vec3::ZERO) < 1.0 {
            commands.entity(entity).try_despawn();
            obelisk.health -= 1.0;
            if obelisk.health <= 0.0 {
                println!(
                    "$$$$$$ {}\nKILLLS {}\nHEALTH {}",
                    player.monies, player.kills, obelisk.health as u32
                );
                unreachable!("How could you");
            }
            if let Some(mut mat) = materials.get_mut(obelisk.material.id()) {
                mat.emissive = LinearRgba::from_vec3(
                    OBELISK_COLOR.to_linear().to_vec3() * relative_health * 10.0,
                );
            }
            if let Ok(mut obelisk_light) = lights.get_mut(obelisk.light_entity) {
                obelisk_light.intensity = relative_health * 5000.0;
            }
        }
    }
}

fn make_turrets_face_camera(
    mut turrets_trans: Query<&mut Transform, Or<(With<Turret>, With<CursorObject>)>>,
    camera_trans: Single<&GlobalTransform, With<Camera>>,
) {
    let mut target = camera_trans.translation();
    target.y = 0.0;

    for mut turret_trans in &mut turrets_trans {
        turret_trans.look_at(target, Vec3::Y);
    }
}

fn lasers_shoot_at_rats(
    mut commands: Commands,
    time: Res<Time>,
    mut rats: Query<(Entity, &mut Transform, &mut Rat)>,
    mut lasers: Query<(&mut Transform, &mut Laser, &mut Visibility), Without<Rat>>,
    mut player: ResMut<PlayerData>,
) {
    let dt = time.delta_secs();
    for (mut laser_trans, mut laser, mut laser_vis) in &mut lasers {
        *laser_vis = Visibility::Hidden;
        let mut need_new_target = true;
        if let Some(target) = laser.target
            && let Ok((rat_entity, rat_trans, mut rat)) = rats.get_mut(target)
            && rat_trans.translation.distance(laser_trans.translation) < LASER_MAX_RANGE
        {
            need_new_target = false;
            rat.health -= dt * TURRET_DMG;
            if rat.health <= 0.0 {
                commands.entity(rat_entity).try_despawn();
                player.monies += PAY_FOR_KILL;
                player.kills += 1;
            }
            laser_trans.look_at(rat_trans.translation, Vec3::Y);
            *laser_vis = Visibility::Visible;
        }
        if need_new_target {
            laser.target = None;

            let closest_dist = f32::MAX;
            for (rat_entity, rat_trans, _rat) in &rats {
                let new_dist = rat_trans.translation.distance(laser_trans.translation);
                if new_dist < closest_dist && new_dist < LASER_MAX_RANGE {
                    laser.target = Some(rat_entity);
                }
            }
        }
    }
}
