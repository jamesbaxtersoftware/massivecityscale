use bevy::prelude::*;
use bevy::math::DVec3;
use bevy::window::PrimaryWindow;
use crate::origin::{WorldPos, FloatingOrigin};
use super::physics::{steer, ship_rotation, integrate_velocity};
use super::{FlightTuning, PlayerShip, ShipEngine, ShipVelocity, ShipControl};

/// Spawn the player ship (a cube) at the world origin, plus its chase camera.
pub fn spawn_ship(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // A low-poly craft assembled from primitives (forward = -Z, rear = +Z): hull,
    // pointed nose, cockpit, swept wings, engine housing, and a glowing plume.
    let hull_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.78, 0.83, 0.95),
        emissive: LinearRgba::rgb(0.04, 0.08, 0.18),
        ..default()
    });
    let trim_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.45, 0.50, 0.62),
        ..default()
    });
    let glass_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.35, 0.5),
        emissive: LinearRgba::rgb(0.1, 0.5, 0.8),
        ..default()
    });
    let glow_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.3, 0.6, 1.0),
        emissive: LinearRgba::rgb(0.6, 1.8, 4.0),
        unlit: true,
        ..default()
    });
    commands
        .spawn((
            PlayerShip { radius: 20.0 }, // ~30 m ship, 20 m collision radius
            ShipVelocity::default(),
            ShipControl::default(),
            WorldPos(DVec3::ZERO),
            Transform::default(),
            Visibility::default(),
        ))
        .with_children(|ship| {
            // Hull.
            ship.spawn((
                Mesh3d(meshes.add(Cuboid::new(9.0, 4.0, 22.0))),
                MeshMaterial3d(hull_mat.clone()),
                Transform::from_xyz(0.0, 0.0, 0.0),
            ));
            // Nose: a cuboid rotated 45° about its long axis reads as a pointed prow.
            ship.spawn((
                Mesh3d(meshes.add(Cuboid::new(4.0, 4.0, 10.0))),
                MeshMaterial3d(trim_mat.clone()),
                Transform::from_xyz(0.0, 0.0, -15.0)
                    .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_4)),
            ));
            // Cockpit canopy.
            ship.spawn((
                Mesh3d(meshes.add(Cuboid::new(4.0, 2.2, 6.0))),
                MeshMaterial3d(glass_mat.clone()),
                Transform::from_xyz(0.0, 2.4, -3.0),
            ));
            // Swept wings (slight dihedral).
            for side in [-1.0_f32, 1.0] {
                ship.spawn((
                    Mesh3d(meshes.add(Cuboid::new(11.0, 0.8, 8.0))),
                    MeshMaterial3d(trim_mat.clone()),
                    Transform::from_xyz(side * 8.5, -0.5, 4.0)
                        .with_rotation(Quat::from_rotation_z(side * -0.18)),
                ));
            }
            // Engine housing.
            ship.spawn((
                Mesh3d(meshes.add(Cuboid::new(7.0, 3.2, 5.0))),
                MeshMaterial3d(trim_mat.clone()),
                Transform::from_xyz(0.0, 0.0, 12.0),
            ));
            // Glowing exhaust plume — stretched by throttle in `engine_glow`.
            ship.spawn((
                ShipEngine,
                Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
                MeshMaterial3d(glow_mat.clone()),
                // Sits a touch below the hull line so the plume is visible from the
                // chase camera; length is driven by throttle in `engine_glow`.
                Transform::from_xyz(0.0, -1.2, 15.0).with_scale(Vec3::new(4.0, 2.0, 2.0)),
            ));
        });
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 25.0, 60.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Stretch the engine plume by throttle: a small nub at rest, a long flame at
/// cruise/boost. Front stays at the engine; it grows backward (+Z).
pub fn engine_glow(
    tune: Res<FlightTuning>,
    ship: Query<&ShipVelocity, With<PlayerShip>>,
    mut plume: Query<&mut Transform, With<ShipEngine>>,
) {
    let Ok(vel) = ship.get_single() else { return };
    let frac = (vel.0.length() / tune.base_max_speed).clamp(0.0, 4.0);
    let len = 2.0 + frac * 9.0;
    let width = 1.0 + frac * 0.3;
    for mut tf in &mut plume {
        tf.scale = Vec3::new(4.0 * width, 2.0 * width, len);
        tf.translation.z = 14.0 + len * 0.5;
    }
}

/// One-shot: warp the cursor to screen centre so steering starts neutral (in the
/// deadzone) instead of snapping toward wherever the pointer happened to be on a
/// cold start. After this the player moves the mouse to steer as normal.
pub fn center_cursor_once(
    mut done: Local<bool>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if *done {
        return;
    }
    if let Ok(mut w) = windows.get_single_mut() {
        let centre = Vec2::new(w.width(), w.height()) * 0.5;
        w.set_cursor_position(Some(centre));
        *done = true;
    }
}

/// One-shot (first frame planets exist): orient the ship toward the nearest
/// planet so the player spawns facing something to fly to, regardless of seed.
pub fn aim_at_nearest_planet(
    mut done: Local<bool>,
    planets: Query<&WorldPos, With<crate::galaxy::PlanetBody>>,
    mut ship: Query<(&WorldPos, &mut ShipControl), With<PlayerShip>>,
) {
    if *done {
        return;
    }
    let Ok((ship_wp, mut ctl)) = ship.get_single_mut() else { return };
    let mut nearest: Option<(f64, DVec3)> = None;
    for p in &planets {
        let d2 = (p.0 - ship_wp.0).length_squared();
        if nearest.map_or(true, |(best, _)| d2 < best) {
            nearest = Some((d2, p.0));
        }
    }
    if let Some((_, ppos)) = nearest {
        let (yaw, pitch) = super::physics::look_yaw_pitch((ppos - ship_wp.0).as_vec3());
        ctl.yaw = yaw;
        ctl.pitch = pitch;
        *done = true;
    }
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
        let shell = (body.radius + ship.radius) as f64 + PLANET_MARGIN as f64;
        // Distance check in f64 first: only the planet we're actually touching gets
        // the f32 clamp. Skipping distant bodies avoids an f32 round-trip that, for
        // light-year-scale planets, would lose billions of metres of precision and
        // teleport the ship.
        if (wp.0 - planet_wp.0).length() >= shell {
            continue;
        }
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
    // Lift the look target by half the camera's height: a gentle downward view
    // that keeps what's ahead near centre-frame while the ship sits lower-third.
    let look = ship.translation + fwd * tune.cam_lookahead + Vec3::Y * tune.cam_up * 0.5;

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
