//! Landing and on-foot mode. Landing switches to a local surface scene (its own
//! render layer) — ground, sky, a third-person avatar you walk with WASD — while
//! the space scene is left intact but unrendered. Take off to return to flight.

use bevy::prelude::*;
use bevy::ecs::schedule::{IntoSystemConfigs, IntoSystemSetConfigs};
use bevy::input::mouse::MouseMotion;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::view::RenderLayers;
use noise::{NoiseFn, Perlin};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use crate::galaxy::PlanetBody;
use crate::origin::{FrameSet, WorldPos};
use crate::ship::PlayerShip;

/// Surface entities render on this layer; the camera switches to it on foot.
pub const SURFACE_LAYER: usize = 1;
/// Land when the ship is within this surface distance (m) of a planet.
pub const LAND_RANGE: f64 = 5.0e6;
const WALK_SPEED: f32 = 14.0;
const GROUND_HALF: f32 = 230.0;
const LOOK_SENS: f32 = 0.005;

/// Rolling-terrain height at a surface (x, z), in metres. Gentle hills.
pub fn surface_height(x: f32, z: f32) -> f32 {
    let p = Perlin::new(7);
    let a = p.get([(x * 0.012) as f64, (z * 0.012) as f64]) as f32;
    let b = p.get([(x * 0.045) as f64, (z * 0.045) as f64]) as f32 * 0.35;
    (a + b) * 5.0
}

/// Build a displaced grid mesh of the local ground using `surface_height`.
fn build_terrain_mesh(half: f32, res: u32) -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let step = half * 2.0 / res as f32;
    for iz in 0..=res {
        for ix in 0..=res {
            let x = -half + ix as f32 * step;
            let z = -half + iz as f32 * step;
            positions.push([x, surface_height(x, z), z]);
        }
    }
    let row = res + 1;
    for iz in 0..res {
        for ix in 0..res {
            let a = iz * row + ix;
            let b = a + row;
            // Wound so normals point up.
            indices.extend_from_slice(&[a, b, a + 1, a + 1, b, b + 1]);
        }
    }
    // Smooth normals from the displaced geometry.
    let mut acc = vec![Vec3::ZERO; positions.len()];
    for tri in indices.chunks(3) {
        let (i0, i1, i2) = (tri[0] as usize, tri[1] as usize, tri[2] as usize);
        let p0 = Vec3::from(positions[i0]);
        let p1 = Vec3::from(positions[i1]);
        let p2 = Vec3::from(positions[i2]);
        let n = (p1 - p0).cross(p2 - p0);
        acc[i0] += n;
        acc[i1] += n;
        acc[i2] += n;
    }
    let normals: Vec<[f32; 3]> = acc
        .iter()
        .map(|n| {
            let u = n.normalize_or_zero();
            [u.x, u.y, u.z]
        })
        .collect();
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// Orbit angles for the third-person foot camera (mouse-look).
#[derive(Resource)]
struct FootCam {
    yaw: f32,
    pitch: f32,
}
impl Default for FootCam {
    fn default() -> Self {
        Self { yaw: 0.0, pitch: 0.35 }
    }
}

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Mode {
    #[default]
    Flight,
    OnFoot,
}

#[derive(Component)]
pub struct Avatar;
/// Tags everything spawned for the surface scene, for teardown on take-off.
#[derive(Component)]
struct SurfaceEntity;

pub struct OnFootPlugin;

impl Plugin for OnFootPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<Mode>()
            .init_resource::<FootCam>()
            // Flight systems run only in flight (so WASD doesn't also fly the ship).
            .configure_sets(
                Update,
                (
                    FrameSet::Input,
                    FrameSet::Move,
                    FrameSet::Origin,
                    FrameSet::Sync,
                    FrameSet::Camera,
                    FrameSet::Stream,
                )
                    .run_if(in_state(Mode::Flight)),
            )
            .add_systems(Update, land_input.run_if(in_state(Mode::Flight)))
            .add_systems(
                Update,
                foot_camera
                    .run_if(in_state(Mode::OnFoot))
                    .run_if(in_state(crate::battle::Phase::Roam)),
            )
            // Walking and take-off are disabled mid-battle.
            .add_systems(
                Update,
                (walk_avatar, takeoff_input)
                    .run_if(in_state(Mode::OnFoot))
                    .run_if(in_state(crate::battle::Phase::Roam)),
            )
            .add_systems(OnEnter(Mode::OnFoot), enter_onfoot)
            .add_systems(OnExit(Mode::OnFoot), exit_onfoot);
    }
}

/// F lands when close enough to a planet surface.
fn land_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut next: ResMut<NextState<Mode>>,
    ship: Query<&WorldPos, With<PlayerShip>>,
    planets: Query<(&WorldPos, &PlanetBody)>,
) {
    if !keys.just_pressed(KeyCode::KeyF) {
        return;
    }
    let Ok(s) = ship.get_single() else { return };
    let near = planets
        .iter()
        .any(|(p, b)| (p.0 - s.0).length() - b.radius as f64 <= LAND_RANGE);
    if near {
        next.set(Mode::OnFoot);
    }
}

/// F takes off back to flight.
fn takeoff_input(keys: Res<ButtonInput<KeyCode>>, mut next: ResMut<NextState<Mode>>) {
    if keys.just_pressed(KeyCode::KeyF) {
        next.set(Mode::Flight);
    }
}

fn enter_onfoot(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut clear: ResMut<ClearColor>,
    cam: Query<Entity, With<Camera3d>>,
) {
    clear.0 = Color::srgb(0.45, 0.65, 0.9); // daytime sky
    let layer = RenderLayers::layer(SURFACE_LAYER);

    // Point the (shared) 3D camera at the surface layer.
    if let Ok(cam) = cam.get_single() {
        commands.entity(cam).insert(layer.clone());
    }

    // Rolling terrain ground.
    commands.spawn((
        SurfaceEntity,
        layer.clone(),
        Mesh3d(meshes.add(build_terrain_mesh(GROUND_HALF + 40.0, 120))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.34, 0.56, 0.28),
            perceptual_roughness: 0.95,
            cull_mode: None,
            ..default()
        })),
        Transform::default(),
    ));

    // Avatar: a little blocky figure (body + head) so it reads as a character.
    commands
        .spawn((
            SurfaceEntity,
            Avatar,
            layer.clone(),
            Transform::from_xyz(0.0, surface_height(0.0, 0.0), 0.0),
            Visibility::default(),
        ))
        .with_children(|a| {
            a.spawn((
                layer.clone(),
                Mesh3d(meshes.add(Cuboid::new(0.9, 1.4, 0.6))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.85, 0.3, 0.2),
                    ..default()
                })),
                Transform::from_xyz(0.0, 1.0, 0.0),
            ));
            a.spawn((
                layer.clone(),
                Mesh3d(meshes.add(Cuboid::new(0.6, 0.6, 0.6))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.95, 0.8, 0.65),
                    ..default()
                })),
                Transform::from_xyz(0.0, 2.0, 0.0),
            ));
        });

    // Surface sun with shadows so things sit on the ground.
    commands.spawn((
        SurfaceEntity,
        layer.clone(),
        DirectionalLight {
            illuminance: 11_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.6, 1.5, 0.4).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // A big moon hanging in the sky (you're on an alien world).
    commands.spawn((
        SurfaceEntity,
        layer.clone(),
        Mesh3d(meshes.add(Sphere::new(140.0).mesh().ico(4).unwrap())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.7, 0.72, 0.85),
            emissive: LinearRgba::rgb(0.25, 0.27, 0.4),
            unlit: true,
            ..default()
        })),
        Transform::from_xyz(-700.0, 460.0, -1400.0),
    ));

    // Scatter rocks, trees and bushes near the landing site for terrain feel.
    let rock_mesh = meshes.add(Sphere::new(1.0).mesh().ico(1).unwrap());
    let bush_mesh = meshes.add(Sphere::new(1.0).mesh().ico(2).unwrap());
    let trunk_mesh = meshes.add(Cylinder::new(0.35, 3.0));
    let rock_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.48, 0.45),
        ..default()
    });
    let bush_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.22, 0.45, 0.2),
        ..default()
    });
    let leaf_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.18, 0.4, 0.18),
        ..default()
    });
    let trunk_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.25, 0.16),
        ..default()
    });
    let mut rng = ChaCha8Rng::seed_from_u64(0xF005);
    for _ in 0..70 {
        let x = rng.gen_range(-150.0..150.0_f32);
        let z = rng.gen_range(-150.0..150.0_f32);
        if x * x + z * z < 64.0 {
            continue; // keep the spawn clearing open
        }
        match rng.gen_range(0..3) {
            0 => {
                // Rock.
                let s = rng.gen_range(0.8..2.4);
                commands.spawn((
                    SurfaceEntity,
                    layer.clone(),
                    Mesh3d(rock_mesh.clone()),
                    MeshMaterial3d(rock_mat.clone()),
                    Transform::from_xyz(x, surface_height(x, z) + s * 0.5, z)
                        .with_scale(Vec3::splat(s)),
                ));
            }
            1 => {
                // Tree: trunk + leaf ball.
                commands
                    .spawn((
                        SurfaceEntity,
                        layer.clone(),
                        Mesh3d(trunk_mesh.clone()),
                        MeshMaterial3d(trunk_mat.clone()),
                        Transform::from_xyz(x, surface_height(x, z) + 1.5, z),
                    ))
                    .with_children(|t| {
                        t.spawn((
                            layer.clone(),
                            Mesh3d(bush_mesh.clone()),
                            MeshMaterial3d(leaf_mat.clone()),
                            Transform::from_xyz(0.0, 2.3, 0.0).with_scale(Vec3::splat(2.2)),
                        ));
                    });
            }
            _ => {
                // Bush.
                let s = rng.gen_range(0.7..1.6);
                commands.spawn((
                    SurfaceEntity,
                    layer.clone(),
                    Mesh3d(bush_mesh.clone()),
                    MeshMaterial3d(bush_mat.clone()),
                    Transform::from_xyz(x, surface_height(x, z) + s * 0.5, z)
                        .with_scale(Vec3::splat(s)),
                ));
            }
        }
    }
}

fn exit_onfoot(
    mut commands: Commands,
    mut clear: ResMut<ClearColor>,
    cam: Query<Entity, With<Camera3d>>,
    surface: Query<Entity, With<SurfaceEntity>>,
) {
    clear.0 = Color::srgb(0.01, 0.01, 0.03); // space
    if let Ok(cam) = cam.get_single() {
        commands.entity(cam).insert(RenderLayers::layer(0));
    }
    for e in &surface {
        commands.entity(e).despawn();
    }
}

/// Camera-relative WASD walking: W moves where the camera faces, A/D strafe.
fn walk_avatar(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    foot: Res<FootCam>,
    mut avatar: Query<&mut Transform, With<Avatar>>,
) {
    let Ok(mut tf) = avatar.get_single_mut() else { return };
    // Horizontal forward/right from the camera yaw.
    let fwd = Vec3::new(-foot.yaw.sin(), 0.0, -foot.yaw.cos());
    let right = Vec3::new(foot.yaw.cos(), 0.0, -foot.yaw.sin());
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
    if dir.length_squared() > 1e-6 {
        let dir = dir.normalize();
        tf.translation += dir * WALK_SPEED * time.delta_secs();
        tf.translation.x = tf.translation.x.clamp(-GROUND_HALF, GROUND_HALF);
        tf.translation.z = tf.translation.z.clamp(-GROUND_HALF, GROUND_HALF);
        tf.translation.y = surface_height(tf.translation.x, tf.translation.z);
        tf.rotation = Quat::from_rotation_arc(Vec3::NEG_Z, dir);
    }
}

/// Third-person orbit camera with mouse-look around the avatar.
fn foot_camera(
    mut mouse: EventReader<MouseMotion>,
    mut foot: ResMut<FootCam>,
    avatar: Query<&Transform, (With<Avatar>, Without<Camera3d>)>,
    mut cam: Query<&mut Transform, With<Camera3d>>,
) {
    let mut delta = Vec2::ZERO;
    for ev in mouse.read() {
        delta += ev.delta;
    }
    foot.yaw -= delta.x * LOOK_SENS;
    foot.pitch = (foot.pitch + delta.y * LOOK_SENS).clamp(-0.2, 1.2);

    let Ok(a) = avatar.get_single() else { return };
    let Ok(mut c) = cam.get_single_mut() else { return };
    let target = a.translation + Vec3::Y * 1.4;
    let dist = 13.0;
    // Spherical orbit: behind (+Z at yaw 0) and raised by pitch, looking down.
    let h = dist * foot.pitch.cos();
    let offset = Vec3::new(h * foot.yaw.sin(), dist * foot.pitch.sin(), h * foot.yaw.cos());
    c.translation = target + offset;
    c.look_at(target, Vec3::Y);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use std::time::Duration;

    #[test]
    fn w_walks_avatar_forward_and_clamps_to_ground() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        let mut input = ButtonInput::<KeyCode>::default();
        input.press(KeyCode::KeyW);
        app.insert_resource(input);
        app.insert_resource(FootCam::default()); // yaw 0 -> forward is -Z
        let e = app
            .world_mut()
            .spawn((Avatar, Transform::from_xyz(0.0, 1.3, 0.0)))
            .id();
        app.update();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(16));
        app.world_mut().run_system_once(walk_avatar).unwrap();

        let tf = app.world().get::<Transform>(e).unwrap();
        assert!(tf.translation.z < 0.0, "W moves the avatar forward (-Z)");
        assert!(tf.translation.x.abs() <= GROUND_HALF, "stays on the ground");
    }
}
