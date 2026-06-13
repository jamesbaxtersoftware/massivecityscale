use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use super::physics::{integrate_velocity, clamp_to_planet, spheres_overlap};
use super::{
    PlayerShip, ShipVelocity, CollisionCooldown,
    BASE_MAX_SPEED, BOOST_MULT, THRUST_ACCEL, STRAFE_ACCEL, DRAG_HALF_LIFE,
    TURN_RATE, ROLL_RATE, PLANET_MARGIN,
    CAM_BACK, CAM_UP, CAM_LOOKAHEAD, CAM_SMOOTH_HALF_LIFE,
};
use crate::camera::zoom::{ZoomLevel, zoom_value_from_distance};
use crate::renderer::solar::CelestialBody;
use crate::creatures::{WildMonster, ClickSphere, GameState, BattleSession};

/// Mouse steers (cursor offset from screen centre), W/S thrust, A/D strafe,
/// hold E boosts. Updates ship facing and `ShipVelocity`.
pub fn flight_input(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut ship_q: Query<(&mut Transform, &mut ShipVelocity), With<PlayerShip>>,
) {
    let dt = time.delta_secs();
    let Ok((mut tf, mut vel)) = ship_q.get_single_mut() else { return };

    // Mouse steering: normalized cursor offset from centre in [-1, 1].
    if let Ok(window) = windows.get_single() {
        if let Some(cursor) = window.cursor_position() {
            let half = Vec2::new(window.width(), window.height()) * 0.5;
            let off = (cursor - half) / half; // y down in screen space
            let yaw = -off.x * TURN_RATE * dt;
            let pitch = -off.y * TURN_RATE * dt; // cursor down → pitch down
            tf.rotate_local_y(yaw);
            tf.rotate_local_x(pitch);
        }
    }

    let boosting = keys.pressed(KeyCode::KeyE);
    let max_speed = if boosting { BASE_MAX_SPEED * BOOST_MULT } else { BASE_MAX_SPEED };
    let thrust_mag = if boosting { THRUST_ACCEL * BOOST_MULT } else { THRUST_ACCEL };

    let fwd = tf.forward().as_vec3();
    let right = tf.right().as_vec3();
    let mut accel = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) { accel += fwd * thrust_mag; }
    if keys.pressed(KeyCode::KeyS) { accel -= fwd * thrust_mag; }
    if keys.pressed(KeyCode::KeyD) { accel += right * STRAFE_ACCEL; tf.rotate_local_z(-ROLL_RATE * dt); }
    if keys.pressed(KeyCode::KeyA) { accel -= right * STRAFE_ACCEL; tf.rotate_local_z(ROLL_RATE * dt); }

    vel.0 = integrate_velocity(vel.0, accel, DRAG_HALF_LIFE, max_speed, dt);
}

/// Integrate position from velocity, then keep the ship outside every planet.
pub fn ship_move(
    time: Res<Time>,
    planets: Query<(&GlobalTransform, &CelestialBody)>,
    mut ship_q: Query<(&mut Transform, &mut ShipVelocity, &PlayerShip)>,
) {
    let dt = time.delta_secs();
    let Ok((mut tf, mut vel, ship)) = ship_q.get_single_mut() else { return };
    tf.translation += vel.0 * dt;
    for (gt, body) in &planets {
        let (p, v) = clamp_to_planet(
            tf.translation, vel.0, gt.translation(),
            body.radius + ship.radius, PLANET_MARGIN,
        );
        tf.translation = p;
        vel.0 = v;
    }
}

/// Chase camera: smoothed follow behind and above the ship.
pub fn chase_camera(
    time: Res<Time>,
    ship_q: Query<&Transform, (With<PlayerShip>, Without<Camera3d>)>,
    mut cam_q: Query<&mut Transform, With<Camera3d>>,
) {
    let Ok(ship) = ship_q.get_single() else { return };
    let Ok(mut cam) = cam_q.get_single_mut() else { return };

    let fwd = ship.forward().as_vec3();
    let up = ship.up().as_vec3();
    let target = ship.translation - fwd * CAM_BACK + up * CAM_UP;
    let look = ship.translation + fwd * CAM_LOOKAHEAD;

    let t = 1.0 - 0.5_f32.powf(time.delta_secs() / CAM_SMOOTH_HALF_LIFE.max(1e-6));
    cam.translation = cam.translation.lerp(target, t);
    cam.look_at(look, Vec3::Y);
}

/// Derive `ZoomLevel` from the camera's distance to the nearest planet surface.
/// Runs in all states so the LOD system stays driven.
pub fn derive_zoom_from_distance(
    cam_q: Query<&Transform, With<Camera3d>>,
    planets: Query<(&GlobalTransform, &CelestialBody)>,
    mut zoom: ResMut<ZoomLevel>,
) {
    let Ok(cam) = cam_q.get_single() else { return };
    let mut nearest = f32::MAX;
    for (gt, body) in &planets {
        let d = (cam.translation - gt.translation()).length() - body.radius;
        nearest = nearest.min(d.max(0.0));
    }
    if nearest == f32::MAX { zoom.value = 1.0; return; } // no planets → solar view
    zoom.value = zoom_value_from_distance(nearest);
}

/// Start a battle when the ship overlaps a wild monster (unless cooling down).
pub fn collide_with_monsters(
    time: Res<Time>,
    mut cooldown: ResMut<CollisionCooldown>,
    mut ship_q: Query<(&Transform, &mut ShipVelocity, &PlayerShip)>,
    monsters: Query<(Entity, &GlobalTransform, &ClickSphere), With<WildMonster>>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
) {
    cooldown.0.tick(time.delta());
    if !cooldown.0.finished() { return; }

    let Ok((tf, mut vel, ship)) = ship_q.get_single_mut() else { return };
    for (entity, gt, cs) in &monsters {
        if spheres_overlap(tf.translation, ship.radius, gt.translation(), cs.radius) {
            vel.0 = Vec3::ZERO;
            commands.insert_resource(BattleSession::new(entity));
            next_state.set(GameState::Battle);
            return;
        }
    }
}

/// On leaving a battle, push the ship just outside the (surviving) monster's
/// collision sphere and restart the cooldown so it does not re-trigger.
pub fn nudge_ship_off_monster(
    session: Res<BattleSession>,
    mut cooldown: ResMut<CollisionCooldown>,
    monsters: Query<(&GlobalTransform, &ClickSphere), With<WildMonster>>,
    mut ship_q: Query<(&mut Transform, &PlayerShip)>,
) {
    cooldown.0.reset();
    let Ok((mut tf, ship)) = ship_q.get_single_mut() else { return };
    if let Ok((gt, cs)) = monsters.get(session.wild_entity) {
        let away = (tf.translation - gt.translation()).normalize_or_zero();
        let away = if away == Vec3::ZERO { Vec3::Z } else { away };
        tf.translation = gt.translation() + away * (cs.radius + ship.radius + 5.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use crate::creatures::{Creature, Species};

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
           .add_plugins(bevy::state::app::StatesPlugin)
           .init_state::<GameState>()
           .init_resource::<crate::flight::CollisionCooldown>();
        app
    }

    /// Tick the cooldown to finished so collisions are live.
    fn arm_collisions(app: &mut App) {
        let mut cd = app.world_mut().resource_mut::<crate::flight::CollisionCooldown>();
        cd.0.reset();
        cd.0.tick(std::time::Duration::from_secs(5));
    }

    #[test]
    fn overlap_starts_a_battle() {
        let mut app = test_app();
        arm_collisions(&mut app);

        app.world_mut().spawn((
            PlayerShip { radius: 10.0 },
            ShipVelocity(Vec3::new(0.0, 0.0, -50.0)),
            Transform::from_xyz(0.0, 0.0, 0.0),
            GlobalTransform::default(),
        ));
        app.world_mut().spawn((
            WildMonster,
            ClickSphere { radius: 20.0 },
            Transform::from_xyz(15.0, 0.0, 0.0),
            GlobalTransform::from_xyz(15.0, 0.0, 0.0),
            Creature::new(Species::Emberling, 3),
        ));

        app.world_mut().run_system_once(collide_with_monsters).unwrap();

        assert!(app.world().get_resource::<BattleSession>().is_some(),
            "battle session inserted on overlap");
        match app.world().resource::<NextState<GameState>>() {
            NextState::Pending(s) => assert_eq!(*s, GameState::Battle),
            _ => panic!("expected transition to Battle"),
        }
    }

    #[test]
    fn no_battle_when_separated() {
        let mut app = test_app();
        arm_collisions(&mut app);

        app.world_mut().spawn((
            PlayerShip { radius: 10.0 },
            ShipVelocity::default(),
            Transform::from_xyz(0.0, 0.0, 0.0),
            GlobalTransform::default(),
        ));
        app.world_mut().spawn((
            WildMonster,
            ClickSphere { radius: 10.0 },
            Transform::from_xyz(500.0, 0.0, 0.0),
            GlobalTransform::from_xyz(500.0, 0.0, 0.0),
            Creature::new(Species::Emberling, 3),
        ));

        app.world_mut().run_system_once(collide_with_monsters).unwrap();
        assert!(app.world().get_resource::<BattleSession>().is_none());
    }

    #[test]
    fn cooldown_suppresses_immediate_retrigger() {
        let mut app = test_app();
        // Fresh cooldown (default starts finished); reset to running, do NOT arm.
        app.world_mut().resource_mut::<crate::flight::CollisionCooldown>().0.reset();

        app.world_mut().spawn((
            PlayerShip { radius: 10.0 },
            ShipVelocity::default(),
            Transform::from_xyz(0.0, 0.0, 0.0),
            GlobalTransform::default(),
        ));
        app.world_mut().spawn((
            WildMonster,
            ClickSphere { radius: 20.0 },
            Transform::from_xyz(15.0, 0.0, 0.0),
            GlobalTransform::from_xyz(15.0, 0.0, 0.0),
            Creature::new(Species::Emberling, 3),
        ));

        app.world_mut().run_system_once(collide_with_monsters).unwrap();
        assert!(app.world().get_resource::<BattleSession>().is_none(),
            "cooldown still running suppresses the trigger");
    }
}
