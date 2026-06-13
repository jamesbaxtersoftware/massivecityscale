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
}
