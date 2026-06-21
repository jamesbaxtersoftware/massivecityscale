//! Interstellar warp: jump the ship between distant star systems (light-year
//! scale). Press J to warp to the next destination. The floating origin keeps the
//! arrival jitter-free, and the backdrop recenters so the sky follows.

use bevy::prelude::*;
use bevy::math::DVec3;
use bevy::ecs::schedule::IntoSystemConfigs;
use crate::galaxy::WarpTargets;
use crate::origin::{FrameSet, WorldPos};
use crate::ship::{PlayerShip, ShipControl, ShipVelocity};

/// Where the ship drops out of warp relative to a destination: a fixed distance
/// out along +Z so it arrives facing the planet (-Z).
pub fn warp_arrival(target: DVec3) -> DVec3 {
    target + DVec3::new(0.0, 0.0, 1.2e7)
}

pub struct WarpPlugin;

impl Plugin for WarpPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, warp_input.in_set(FrameSet::Input));
    }
}

/// J warps to the next destination: reposition the ship, kill velocity, face the
/// planet. Cycles through the target list.
pub fn warp_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut targets: ResMut<WarpTargets>,
    mut ship: Query<(&mut WorldPos, &mut ShipVelocity, &mut ShipControl, &mut Transform), With<PlayerShip>>,
) {
    if !keys.just_pressed(KeyCode::KeyJ) || targets.positions.is_empty() {
        return;
    }
    let idx = targets.current % targets.positions.len();
    let dest = warp_arrival(targets.positions[idx]);
    targets.current = (idx + 1) % targets.positions.len();

    if let Ok((mut wp, mut vel, mut ctl, mut tf)) = ship.get_single_mut() {
        wp.0 = dest;
        vel.0 = Vec3::ZERO;
        ctl.yaw = 0.0;
        ctl.pitch = 0.0;
        tf.rotation = crate::ship::physics::ship_rotation(0.0, 0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    fn tap_j(app: &mut App) {
        let mut input = ButtonInput::<KeyCode>::default();
        input.press(KeyCode::KeyJ);
        app.insert_resource(input);
    }

    #[test]
    fn warp_jumps_ship_to_next_target_and_stops_it() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        let a = DVec3::new(1.0e16, 0.0, -2.0e16);
        let b = DVec3::new(-3.0e16, 5.0e15, 1.0e16);
        app.insert_resource(WarpTargets { positions: vec![a, b], current: 0 });
        tap_j(&mut app);
        let ship = app
            .world_mut()
            .spawn((
                PlayerShip { radius: 20.0 },
                ShipVelocity(Vec3::new(0.0, 0.0, -500.0)),
                ShipControl::default(),
                WorldPos(DVec3::ZERO),
                Transform::default(),
            ))
            .id();

        app.world_mut().run_system_once(warp_input).unwrap();

        let wp = app.world().get::<WorldPos>(ship).unwrap();
        assert_eq!(wp.0, warp_arrival(a), "warped to first target");
        let vel = app.world().get::<ShipVelocity>(ship).unwrap();
        assert_eq!(vel.0, Vec3::ZERO, "velocity zeroed on arrival");
        assert_eq!(app.world().resource::<WarpTargets>().current, 1, "advances target");
    }
}
