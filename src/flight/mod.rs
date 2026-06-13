pub mod physics;

use bevy::prelude::*;

// ── Tuning constants (playtest to adjust) ──────────────────────────────────
pub const BASE_MAX_SPEED: f32   = 100.0; // km/s
pub const BOOST_MULT: f32       = 10.0;
pub const THRUST_ACCEL: f32     = 80.0;  // km/s^2
pub const STRAFE_ACCEL: f32     = 50.0;  // km/s^2
pub const DRAG_HALF_LIFE: f32   = 0.7;   // s
pub const TURN_RATE: f32        = 1.2;   // rad/s at full cursor offset
pub const ROLL_RATE: f32        = 1.5;   // rad/s while strafing
pub const SHIP_RADIUS: f32      = 10.0;  // km
pub const PLANET_MARGIN: f32    = 12.0;  // km clearance kept outside a planet
pub const CAM_BACK: f32         = 60.0;  // km behind ship
pub const CAM_UP: f32           = 25.0;  // km above ship
pub const CAM_LOOKAHEAD: f32    = 80.0;  // km ahead of ship for look target
pub const CAM_SMOOTH_HALF_LIFE: f32 = 0.15; // s
pub const COLLISION_COOLDOWN: f32 = 2.0; // s

// ── Components ─────────────────────────────────────────────────────────────
#[derive(Component)]
pub struct PlayerShip {
    pub radius: f32,
}

#[derive(Component, Default)]
pub struct ShipVelocity(pub Vec3);
