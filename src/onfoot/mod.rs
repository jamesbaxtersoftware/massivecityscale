//! Landing and on-foot mode. Landing switches to a local surface scene (its own
//! render layer) — ground, sky, a third-person avatar you walk with WASD — while
//! the space scene is left intact but unrendered. Take off to return to flight.

use bevy::prelude::*;
use bevy::ecs::schedule::{IntoSystemConfigs, IntoSystemSetConfigs};
use bevy::render::view::RenderLayers;
use crate::galaxy::PlanetBody;
use crate::origin::{FrameSet, WorldPos};
use crate::ship::PlayerShip;

/// Surface entities render on this layer; the camera switches to it on foot.
pub const SURFACE_LAYER: usize = 1;
/// Land when the ship is within this surface distance (m) of a planet.
pub const LAND_RANGE: f64 = 5.0e6;
const WALK_SPEED: f32 = 22.0;
const GROUND_HALF: f32 = 1800.0;

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Mode {
    #[default]
    Flight,
    OnFoot,
}

#[derive(Component)]
struct Avatar;
/// Tags everything spawned for the surface scene, for teardown on take-off.
#[derive(Component)]
struct SurfaceEntity;

pub struct OnFootPlugin;

impl Plugin for OnFootPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<Mode>()
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
                (walk_avatar, foot_camera, takeoff_input).run_if(in_state(Mode::OnFoot)),
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

    // Ground.
    commands.spawn((
        SurfaceEntity,
        layer.clone(),
        Mesh3d(meshes.add(Cuboid::new(GROUND_HALF * 2.0, 2.0, GROUND_HALF * 2.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.55, 0.25),
            ..default()
        })),
        Transform::from_xyz(0.0, -1.0, 0.0),
    ));

    // Avatar (capsule), facing -Z.
    commands.spawn((
        SurfaceEntity,
        Avatar,
        layer.clone(),
        Mesh3d(meshes.add(Capsule3d::new(0.6, 1.4))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.85, 0.3, 0.2),
            ..default()
        })),
        Transform::from_xyz(0.0, 1.3, 0.0),
    ));

    // Surface sun.
    commands.spawn((
        SurfaceEntity,
        layer,
        DirectionalLight {
            illuminance: 12_000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(1.0, 2.0, 0.5).looking_at(Vec3::ZERO, Vec3::Y),
    ));
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

/// WASD walks the avatar across the ground (relative to camera-forward = -Z).
fn walk_avatar(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut avatar: Query<&mut Transform, With<Avatar>>,
) {
    let Ok(mut tf) = avatar.get_single_mut() else { return };
    let mut dir = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) {
        dir.z -= 1.0;
    }
    if keys.pressed(KeyCode::KeyS) {
        dir.z += 1.0;
    }
    if keys.pressed(KeyCode::KeyA) {
        dir.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) {
        dir.x += 1.0;
    }
    if dir != Vec3::ZERO {
        let step = dir.normalize() * WALK_SPEED * time.delta_secs();
        tf.translation += step;
        tf.translation.x = tf.translation.x.clamp(-GROUND_HALF, GROUND_HALF);
        tf.translation.z = tf.translation.z.clamp(-GROUND_HALF, GROUND_HALF);
        // Face travel direction.
        let flat = Vec3::new(step.x, 0.0, step.z);
        if flat.length_squared() > 1e-6 {
            tf.rotation = Quat::from_rotation_arc(Vec3::NEG_Z, flat.normalize());
        }
    }
}

/// Third-person camera trailing the avatar.
fn foot_camera(
    avatar: Query<&Transform, (With<Avatar>, Without<Camera3d>)>,
    mut cam: Query<&mut Transform, With<Camera3d>>,
) {
    let Ok(a) = avatar.get_single() else { return };
    let Ok(mut c) = cam.get_single_mut() else { return };
    c.translation = a.translation + Vec3::new(0.0, 6.0, 14.0);
    c.look_at(a.translation + Vec3::new(0.0, 1.0, 0.0), Vec3::Y);
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
