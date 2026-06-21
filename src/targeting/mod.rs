//! Target selection + warp-to-target. Cycle through the closest planets (T),
//! engage warp (G) to fast-fly the ship to the selected one and stop near it.
//! Manual thrust cancels. The HUD reads the selected target for a live distance.

use bevy::prelude::*;
use bevy::math::DVec3;
use bevy::ecs::schedule::IntoSystemConfigs;
use crate::galaxy::PlanetBody;
use crate::origin::{FrameSet, WorldPos};
use crate::ship::physics::{look_yaw_pitch, ship_rotation};
use crate::ship::{FlightTuning, PlayerShip, ShipControl, ShipVelocity};

/// How many of the nearest planets are targetable.
pub const TARGET_COUNT: usize = 10;

#[derive(Resource, Default)]
pub struct Targets {
    /// Closest planets, nearest first.
    pub list: Vec<Entity>,
    pub selected: Option<Entity>,
}

#[derive(Resource, Default)]
pub struct Autopilot {
    pub on: bool,
}

pub struct TargetingPlugin;

impl Plugin for TargetingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Targets>()
            .init_resource::<Autopilot>()
            .add_systems(Update, (update_targets, cycle_target, engage_warp).in_set(FrameSet::Input))
            .add_systems(
                Update,
                autopilot_fly
                    .in_set(FrameSet::Move)
                    .before(crate::ship::systems::ship_move),
            );
    }
}

/// Indices of the `n` closest planets (by surface distance), nearest first.
pub fn closest_indices(ship: DVec3, planets: &[(DVec3, f32)], n: usize) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..planets.len()).collect();
    idx.sort_by(|&a, &b| {
        let da = (planets[a].0 - ship).length() - planets[a].1 as f64;
        let db = (planets[b].0 - ship).length() - planets[b].1 as f64;
        da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
    });
    idx.truncate(n);
    idx
}

fn update_targets(
    ship: Query<&WorldPos, With<PlayerShip>>,
    planets: Query<(Entity, &WorldPos, &PlanetBody)>,
    mut targets: ResMut<Targets>,
) {
    let Ok(s) = ship.get_single() else { return };
    let items: Vec<(Entity, DVec3, f32)> =
        planets.iter().map(|(e, p, b)| (e, p.0, b.radius)).collect();
    let pos: Vec<(DVec3, f32)> = items.iter().map(|(_, p, r)| (*p, *r)).collect();
    targets.list = closest_indices(s.0, &pos, TARGET_COUNT)
        .into_iter()
        .map(|i| items[i].0)
        .collect();
    // Default selection / keep a valid one.
    if targets.selected.map_or(true, |e| !targets.list.contains(&e)) {
        targets.selected = targets.list.first().copied();
    }
}

fn cycle_target(keys: Res<ButtonInput<KeyCode>>, mut targets: ResMut<Targets>) {
    if !keys.just_pressed(KeyCode::KeyT) || targets.list.is_empty() {
        return;
    }
    let next = match targets.selected.and_then(|e| targets.list.iter().position(|&x| x == e)) {
        Some(i) => (i + 1) % targets.list.len(),
        None => 0,
    };
    targets.selected = Some(targets.list[next]);
}

fn engage_warp(keys: Res<ButtonInput<KeyCode>>, mut ap: ResMut<Autopilot>, targets: Res<Targets>) {
    if keys.just_pressed(KeyCode::KeyG) && targets.selected.is_some() {
        ap.on = true;
    }
}

/// While engaged, steer + thrust at warp speed toward the selected target; stop
/// near it. Any manual thrust cancels.
fn autopilot_fly(
    keys: Res<ButtonInput<KeyCode>>,
    tune: Res<FlightTuning>,
    mut ap: ResMut<Autopilot>,
    targets: Res<Targets>,
    planets: Query<(&WorldPos, &PlanetBody)>,
    mut ship: Query<(&WorldPos, &mut ShipVelocity, &mut ShipControl, &mut Transform), With<PlayerShip>>,
) {
    if !ap.on {
        return;
    }
    let manual = [KeyCode::KeyW, KeyCode::KeyS, KeyCode::KeyA, KeyCode::KeyD]
        .iter()
        .any(|k| keys.pressed(*k));
    if manual {
        ap.on = false;
        return;
    }
    let Some(target) = targets.selected else {
        ap.on = false;
        return;
    };
    let Ok((twp, body)) = planets.get(target) else {
        ap.on = false;
        return;
    };
    let Ok((swp, mut vel, mut ctl, mut tf)) = ship.get_single_mut() else { return };

    let to = twp.0 - swp.0;
    let surf = to.length() - body.radius as f64;
    // Arrived: stop a comfortable distance out.
    if surf <= body.radius as f64 + 5.0e5 {
        ap.on = false;
        vel.0 = Vec3::ZERO;
        return;
    }
    let dir = to.as_vec3().normalize_or_zero();
    let (yaw, pitch) = look_yaw_pitch(dir);
    ctl.yaw = yaw;
    ctl.pitch = pitch;
    tf.rotation = ship_rotation(yaw, pitch);
    vel.0 = dir * (tune.base_max_speed * tune.boost_mult);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closest_indices_orders_by_surface_distance() {
        let ship = DVec3::ZERO;
        let planets = [
            (DVec3::new(1.0e8, 0.0, 0.0), 1.0e6), // surface ~9.9e7
            (DVec3::new(2.0e7, 0.0, 0.0), 1.0e6), // surface ~1.9e7 (closest)
            (DVec3::new(5.0e7, 0.0, 0.0), 1.0e6), // surface ~4.9e7
        ];
        assert_eq!(closest_indices(ship, &planets, 10), vec![1, 2, 0]);
    }

    #[test]
    fn closest_indices_truncates_to_n() {
        let ship = DVec3::ZERO;
        let planets: Vec<(DVec3, f32)> =
            (1..=20).map(|i| (DVec3::new(i as f64 * 1.0e7, 0.0, 0.0), 1.0e6)).collect();
        assert_eq!(closest_indices(ship, &planets, 10).len(), 10);
    }
}
