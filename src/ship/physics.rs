use bevy::prelude::*;
use bevy::math::EulerRot;

/// Maximum pitch from level, in radians (~69°). Keeps the ship from flipping
/// over the top and keeps the chase camera's `look_at` well away from vertical.
pub const MAX_PITCH: f32 = 1.2;

/// Update `(yaw, pitch)` from a normalized cursor offset where centre = (0,0),
/// right = +x, down = +y. No roll is ever introduced, so world up stays up.
pub fn steer(yaw: f32, pitch: f32, cursor_off: Vec2, rate: f32, dt: f32) -> (f32, f32) {
    let new_yaw = yaw - cursor_off.x * rate * dt;
    let new_pitch = (pitch - cursor_off.y * rate * dt).clamp(-MAX_PITCH, MAX_PITCH);
    (new_yaw, new_pitch)
}

/// Build the ship's rotation from yaw/pitch with world up preserved (no roll).
/// Forward is `-Z` at neutral, matching Bevy's default facing.
pub fn ship_rotation(yaw: f32, pitch: f32) -> Quat {
    Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0)
}

/// Integrate one physics step: apply thrust acceleration, exponential drag,
/// then clamp to `max_speed`. `drag_half_life` is the time for an un-thrusted
/// velocity to halve.
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

/// If `pos` is inside `radius + margin` of a planet at `center`, project it back
/// onto that shell and remove the inward component of `vel` (slide, not bounce).
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
    let inward = vel.dot(outward).min(0.0);
    let new_vel = vel - outward * inward;
    (new_pos, new_vel)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ship_rotation_neutral_faces_forward_and_upright() {
        let r = ship_rotation(0.0, 0.0);
        assert!((r * Vec3::NEG_Z - Vec3::NEG_Z).length() < 1e-5);
        assert!((r * Vec3::Y - Vec3::Y).length() < 1e-5);
    }

    #[test]
    fn cursor_up_pitches_nose_up_down_pitches_down() {
        let (y, p) = steer(0.0, 0.0, Vec2::new(0.0, -1.0), 1.0, 0.1);
        assert!((ship_rotation(y, p) * Vec3::NEG_Z).y > 0.0);
        let (y, p) = steer(0.0, 0.0, Vec2::new(0.0, 1.0), 1.0, 0.1);
        assert!((ship_rotation(y, p) * Vec3::NEG_Z).y < 0.0);
    }

    #[test]
    fn cursor_right_turns_right_left_turns_left() {
        let (y, p) = steer(0.0, 0.0, Vec2::new(1.0, 0.0), 1.0, 0.1);
        assert!((ship_rotation(y, p) * Vec3::NEG_Z).x > 0.0);
        let (y, p) = steer(0.0, 0.0, Vec2::new(-1.0, 0.0), 1.0, 0.1);
        assert!((ship_rotation(y, p) * Vec3::NEG_Z).x < 0.0);
    }

    #[test]
    fn pitch_is_clamped() {
        let (_, p) = steer(0.0, 0.0, Vec2::new(0.0, -100.0), 5.0, 1.0);
        assert!(p <= MAX_PITCH + 1e-6 && p >= -MAX_PITCH - 1e-6);
    }

    #[test]
    fn ship_never_rolls_up_stays_above_horizon() {
        for &yaw in &[-3.1_f32, -1.0, 0.0, 2.0, 3.1] {
            for &pitch in &[-MAX_PITCH, -0.5_f32, 0.0, 0.5, MAX_PITCH] {
                let up = ship_rotation(yaw, pitch) * Vec3::Y;
                assert!(up.y > 0.0, "up stays above horizon (yaw={yaw}, pitch={pitch})");
            }
        }
    }

    #[test]
    fn thrust_increases_speed_along_direction() {
        let thrust = Vec3::new(0.0, 0.0, -1.0) * 80.0;
        let v1 = integrate_velocity(Vec3::ZERO, thrust, 1.0e9, 1.0e9, 0.5);
        assert!(v1.z < 0.0);
        assert!((v1.length() - 40.0).abs() < 1.0);
    }

    #[test]
    fn drag_halves_velocity_over_half_life() {
        let v1 = integrate_velocity(Vec3::new(0.0, 0.0, -100.0), Vec3::ZERO, 0.7, 1.0e9, 0.7);
        assert!((v1.length() - 50.0).abs() < 1.0);
    }

    #[test]
    fn speed_is_clamped_to_max() {
        let v1 = integrate_velocity(Vec3::new(0.0, 0.0, -1000.0), Vec3::ZERO, 1.0e9, 100.0, 0.016);
        assert!(v1.length() <= 100.0 + 1e-3);
    }

    #[test]
    fn boost_raises_the_clamp() {
        let base = integrate_velocity(Vec3::new(0.0, 0.0, -1000.0), Vec3::ZERO, 1.0e9, 100.0, 0.016);
        let boost = integrate_velocity(Vec3::new(0.0, 0.0, -1000.0), Vec3::ZERO, 1.0e9, 1000.0, 0.016);
        assert!(boost.length() > base.length());
    }

    #[test]
    fn clamp_pushes_ship_outside_planet_and_zeros_inward_velocity() {
        let (new_pos, new_vel) = clamp_to_planet(
            Vec3::new(0.0, 0.0, 250.0), Vec3::new(0.0, 0.0, -100.0),
            Vec3::ZERO, 350.0, 12.0);
        assert!((new_pos.length() - 362.0).abs() < 1e-2);
        assert!(new_vel.z >= -1e-3);
    }

    #[test]
    fn clamp_leaves_distant_ship_untouched() {
        let (p, v) = clamp_to_planet(
            Vec3::new(0.0, 0.0, 5000.0), Vec3::new(1.0, 2.0, 3.0),
            Vec3::ZERO, 350.0, 12.0);
        assert_eq!(p, Vec3::new(0.0, 0.0, 5000.0));
        assert_eq!(v, Vec3::new(1.0, 2.0, 3.0));
    }
}
