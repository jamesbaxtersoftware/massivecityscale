use bevy::prelude::*;
use bevy::math::DVec3;
use bevy::window::PrimaryWindow;
use crate::origin::{WorldPos, FloatingOrigin};
use super::physics::{steer, ship_rotation, integrate_velocity};
use super::{FlightTuning, PlayerShip, ShipVelocity, ShipControl};

/// Spawn the player ship (a cube) at the world origin, plus its chase camera.
pub fn spawn_ship(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        PlayerShip { radius: 10.0 },
        ShipVelocity::default(),
        ShipControl::default(),
        WorldPos(DVec3::ZERO),
        Mesh3d(meshes.add(Cuboid::new(8.0, 8.0, 16.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.8, 0.85, 0.95),
            emissive: LinearRgba::rgb(0.1, 0.2, 0.4),
            ..default()
        })),
        Transform::default(),
    ));
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 25.0, 60.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Mouse steers (cursor offset from screen centre), W/S thrust, A/D strafe,
/// hold Shift boosts. Facing tracked as yaw/pitch and rebuilt each frame so the
/// ship never rolls.
pub fn flight_input(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    tune: Res<FlightTuning>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut ship_q: Query<(&mut Transform, &mut ShipVelocity, &mut ShipControl), With<PlayerShip>>,
) {
    let dt = time.delta_secs();
    let Ok((mut tf, mut vel, mut ctl)) = ship_q.get_single_mut() else { return };

    if let Ok(window) = windows.get_single() {
        if let Some(cursor) = window.cursor_position() {
            let half = Vec2::new(window.width(), window.height()) * 0.5;
            let mut off = (cursor - half) / half;
            if off.length() < tune.steer_deadzone { off = Vec2::ZERO; }
            let (yaw, pitch) = steer(ctl.yaw, ctl.pitch, off, tune.turn_rate, dt);
            ctl.yaw = yaw;
            ctl.pitch = pitch;
        }
    }
    tf.rotation = ship_rotation(ctl.yaw, ctl.pitch);

    let boosting = keys.pressed(KeyCode::ShiftLeft);
    let max_speed = if boosting { tune.base_max_speed * tune.boost_mult } else { tune.base_max_speed };
    let thrust_mag = if boosting { tune.thrust_accel * tune.boost_mult } else { tune.thrust_accel };

    let fwd = tf.forward().as_vec3();
    let right = tf.right().as_vec3();
    let mut accel = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) { accel += fwd * thrust_mag; }
    if keys.pressed(KeyCode::KeyS) { accel -= fwd * thrust_mag; }
    if keys.pressed(KeyCode::KeyD) { accel += right * tune.strafe_accel; }
    if keys.pressed(KeyCode::KeyA) { accel -= right * tune.strafe_accel; }

    vel.0 = integrate_velocity(vel.0, accel, tune.drag_half_life, max_speed, dt);
}

/// Integrate the ship's f64 world position from its f32 velocity, then keep it
/// outside every planet shell (slide, not bounce). Collision math runs in f32
/// relative to each planet, where the offset is small enough to stay precise.
pub fn ship_move(
    time: Res<Time>,
    planets: Query<(&WorldPos, &crate::galaxy::PlanetBody)>,
    mut ship_q: Query<(&mut WorldPos, &mut ShipVelocity, &PlayerShip), Without<crate::galaxy::PlanetBody>>,
) {
    use super::physics::clamp_to_planet;
    const PLANET_MARGIN: f32 = 0.1; // km clearance so the ship can still reach the surface
    let dt = time.delta_secs();
    let Ok((mut wp, mut vel, ship)) = ship_q.get_single_mut() else { return };
    wp.0 += (vel.0 * dt).as_dvec3();

    for (planet_wp, body) in &planets {
        let rel = (wp.0 - planet_wp.0).as_vec3(); // small near the surface
        let (new_rel, new_vel) = clamp_to_planet(
            rel, vel.0, Vec3::ZERO, body.radius + ship.radius, PLANET_MARGIN,
        );
        wp.0 = planet_wp.0 + new_rel.as_dvec3();
        vel.0 = new_vel;
    }
}

/// Publish the ship's world position as the floating origin, so it renders at
/// (0,0,0) and everything else is positioned relative to it.
pub fn set_origin(
    ship_q: Query<&WorldPos, With<PlayerShip>>,
    mut origin: ResMut<FloatingOrigin>,
) {
    if let Ok(wp) = ship_q.get_single() {
        origin.0 = wp.0;
    }
}

/// Chase camera: smoothed follow behind and above the ship, in render space
/// (the ship sits at ~origin after sync_transforms).
pub fn chase_camera(
    time: Res<Time>,
    tune: Res<FlightTuning>,
    ship_q: Query<&Transform, (With<PlayerShip>, Without<Camera3d>)>,
    mut cam_q: Query<&mut Transform, With<Camera3d>>,
) {
    let Ok(ship) = ship_q.get_single() else { return };
    let Ok(mut cam) = cam_q.get_single_mut() else { return };

    let fwd = ship.forward().as_vec3();
    let target = ship.translation - fwd * tune.cam_back + Vec3::Y * tune.cam_up;
    let look = ship.translation + fwd * tune.cam_lookahead;

    let t = 1.0 - 0.5_f32.powf(time.delta_secs() / tune.cam_smooth_half_life.max(1e-6));
    cam.translation = cam.translation.lerp(target, t);
    cam.look_at(look, Vec3::Y);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    #[test]
    fn ship_move_advances_world_pos_along_velocity() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        let e = app.world_mut().spawn((
            PlayerShip { radius: 10.0 },
            ShipVelocity(Vec3::new(0.0, 0.0, -100.0)),
            WorldPos(DVec3::ZERO),
        )).id();
        // Advance time so delta_secs() > 0.
        app.update();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(16));
        app.world_mut().run_system_once(ship_move).unwrap();
        let wp = app.world().get::<WorldPos>(e).unwrap();
        assert!(wp.0.z < 0.0, "ship advanced in -Z world space");
    }

    #[test]
    fn ship_move_keeps_ship_outside_planet() {
        use crate::galaxy::PlanetBody;
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        // Planet of radius 300 km at +200 on Z; ship just inside heading inward.
        app.world_mut().spawn((
            PlanetBody { radius: 300.0 },
            WorldPos(DVec3::new(0.0, 0.0, 0.0)),
        ));
        let ship = app.world_mut().spawn((
            PlayerShip { radius: 10.0 },
            ShipVelocity(Vec3::new(0.0, 0.0, -50.0)),
            WorldPos(DVec3::new(0.0, 0.0, 250.0)),
        )).id();
        app.update();
        app.world_mut().run_system_once(ship_move).unwrap();
        let wp = app.world().get::<WorldPos>(ship).unwrap();
        let dist = wp.0.length();
        assert!(dist >= 300.0 + 10.0 - 1.0, "ship stays outside planet shell (got {dist})");
    }

    #[test]
    fn set_origin_follows_the_ship() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).init_resource::<FloatingOrigin>();
        app.world_mut().spawn((
            PlayerShip { radius: 10.0 },
            WorldPos(DVec3::new(42.0, 0.0, -7.0)),
        ));
        app.world_mut().run_system_once(set_origin).unwrap();
        let origin = app.world().resource::<FloatingOrigin>();
        assert_eq!(origin.0, DVec3::new(42.0, 0.0, -7.0));
    }
}
