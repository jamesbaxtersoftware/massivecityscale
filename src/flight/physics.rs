use bevy::prelude::*;

/// Integrate one physics step: apply thrust acceleration, exponential drag,
/// then clamp to `max_speed`. `drag_half_life` is the time for an un-thrusted
/// velocity to halve. Large half-life / max_speed effectively disable each.
pub fn integrate_velocity(
    v: Vec3,
    thrust_accel: Vec3,
    drag_half_life: f32,
    max_speed: f32,
    dt: f32,
) -> Vec3 {
    let mut next = v + thrust_accel * dt;
    let decay = 0.5_f32.powf(dt / drag_half_life.max(1e-6));
    next *= decay;
    if next.length() > max_speed {
        next = next.normalize_or_zero() * max_speed;
    }
    next
}

/// If `pos` is inside `radius + margin` of a planet at `center`, project it
/// back onto that shell and remove the inward component of `vel` (slide, not
/// bounce). Returns the corrected (pos, vel). Outside the shell, returns the
/// inputs unchanged.
pub fn clamp_to_planet(
    pos: Vec3,
    vel: Vec3,
    center: Vec3,
    radius: f32,
    margin: f32,
) -> (Vec3, Vec3) {
    let shell = radius + margin;
    let offset = pos - center;
    let dist = offset.length();
    if dist >= shell || dist < 1e-4 {
        return (pos, vel);
    }
    let outward = offset / dist;
    let new_pos = center + outward * shell;
    let inward = vel.dot(outward).min(0.0); // negative when heading inward
    let new_vel = vel - outward * inward;    // subtract inward component
    (new_pos, new_vel)
}

/// True when two spheres touch or intersect.
pub fn spheres_overlap(a: Vec3, ra: f32, b: Vec3, rb: f32) -> bool {
    let r = ra + rb;
    a.distance_squared(b) <= r * r
}

/// Camera + creature placement for an in-world battle framed on a wild monster.
pub struct BattlePose {
    pub camera: Vec3,
    pub look_at: Vec3,
    pub player: Vec3,
}

/// Frame the camera on a wild monster at a fixed distance/elevation and place
/// the player's creature beside it. Mirrors the previous orbit-based framing
/// (distance = scale*12, elevation 0.2) but as a pure function.
pub fn battle_camera_pose(wild_pos: Vec3, creature_scale: f32) -> BattlePose {
    let distance = creature_scale * 12.0;
    let elevation = 0.2_f32;
    // Fixed azimuth (0) offset in the XZ plane, raised by elevation.
    let dir = Vec3::new(0.0, elevation.sin(), elevation.cos()).normalize();
    let camera = wild_pos + dir * distance;
    let fwd = (wild_pos - camera).normalize();
    let right = fwd.cross(Vec3::Y).normalize();
    let player = wild_pos - fwd * creature_scale * 4.0 + right * creature_scale * 2.0;
    BattlePose { camera, look_at: wild_pos, player }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thrust_increases_speed_along_direction() {
        // Starting at rest, one second of forward thrust, no drag clamp issues.
        let v0 = Vec3::ZERO;
        let thrust = Vec3::new(0.0, 0.0, -1.0) * 80.0; // accel km/s^2
        let v1 = integrate_velocity(v0, thrust, 1.0e9, 1.0e9, 0.5);
        assert!(v1.z < 0.0, "should gain velocity in -Z");
        assert!((v1.length() - 40.0).abs() < 1.0, "≈ accel * dt with negligible drag");
    }

    #[test]
    fn drag_halves_velocity_over_half_life() {
        let v0 = Vec3::new(0.0, 0.0, -100.0);
        let v1 = integrate_velocity(v0, Vec3::ZERO, 0.7, 1.0e9, 0.7);
        assert!((v1.length() - 50.0).abs() < 1.0, "one half-life halves speed");
    }

    #[test]
    fn speed_is_clamped_to_max() {
        let v0 = Vec3::new(0.0, 0.0, -1000.0);
        let v1 = integrate_velocity(v0, Vec3::ZERO, 1.0e9, 100.0, 0.016);
        assert!(v1.length() <= 100.0 + 1e-3, "clamped to max speed");
    }

    #[test]
    fn boost_raises_the_clamp() {
        let v0 = Vec3::new(0.0, 0.0, -1000.0);
        let base = integrate_velocity(v0, Vec3::ZERO, 1.0e9, 100.0, 0.016);
        let boost = integrate_velocity(v0, Vec3::ZERO, 1.0e9, 100.0 * 10.0, 0.016);
        assert!(boost.length() > base.length(), "boost allows higher speed");
    }

    #[test]
    fn clamp_pushes_ship_outside_planet_and_zeros_inward_velocity() {
        let center = Vec3::ZERO;
        let radius = 350.0;
        let margin = 12.0;
        // Ship 100 km deep inside the planet, moving further inward.
        let pos = Vec3::new(0.0, 0.0, 250.0);
        let vel = Vec3::new(0.0, 0.0, -100.0); // heading toward center
        let (new_pos, new_vel) = clamp_to_planet(pos, vel, center, radius, margin);
        let surface = radius + margin;
        assert!((new_pos.length() - surface).abs() < 1e-2, "pushed to surface+margin");
        // Inward (toward center) component removed; here outward is +Z.
        assert!(new_vel.z >= -1e-3, "inward velocity removed");
    }

    #[test]
    fn clamp_leaves_distant_ship_untouched() {
        let pos = Vec3::new(0.0, 0.0, 5000.0);
        let vel = Vec3::new(1.0, 2.0, 3.0);
        let (p, v) = clamp_to_planet(pos, vel, Vec3::ZERO, 350.0, 12.0);
        assert_eq!(p, pos);
        assert_eq!(v, vel);
    }

    #[test]
    fn spheres_overlap_detects_contact() {
        assert!(spheres_overlap(Vec3::ZERO, 10.0, Vec3::new(15.0, 0.0, 0.0), 10.0));
        assert!(!spheres_overlap(Vec3::ZERO, 10.0, Vec3::new(25.0, 0.0, 0.0), 10.0));
    }

    #[test]
    fn battle_camera_pose_faces_the_monster_at_distance() {
        let wild = Vec3::new(100.0, 0.0, 100.0);
        let scale = 40.0;
        let pose = battle_camera_pose(wild, scale);
        let d = (pose.camera - wild).length();
        assert!((d - scale * 12.0).abs() < 1.0, "camera at framing distance");
        assert_eq!(pose.look_at, wild, "camera looks at the monster");
        // Player creature is placed near the monster, not on top of it.
        let pd = (pose.player - wild).length();
        assert!(pd > scale && pd < scale * 8.0, "player framed beside the monster");
    }
}
