pub mod physics;
pub mod ship;
pub mod systems;

use bevy::prelude::*;
use bevy::ecs::schedule::IntoSystemConfigs;
use bevy::time::{Timer, TimerMode};
use crate::creatures::GameState;

// ── Tuning constants (playtest to adjust) ──────────────────────────────────
pub const BASE_MAX_SPEED: f32   = 100.0; // km/s
pub const BOOST_MULT: f32       = 10.0;
pub const THRUST_ACCEL: f32     = 80.0;  // km/s^2
pub const STRAFE_ACCEL: f32     = 50.0;  // km/s^2
pub const DRAG_HALF_LIFE: f32   = 0.7;   // s
pub const TURN_RATE: f32        = 1.2;   // rad/s at full cursor offset
pub const STEER_DEADZONE: f32   = 0.08;  // normalized cursor radius treated as centre
pub const SHIP_RADIUS: f32      = 10.0;  // km
pub const PLANET_MARGIN: f32    = 0.1;   // km clearance kept outside a planet (lets the ship land)
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

/// Ship facing as explicit yaw/pitch so steering never accumulates roll and
/// "up stays up". Rebuilt into the transform each frame via `ship_rotation`.
#[derive(Component, Default)]
pub struct ShipControl {
    pub yaw: f32,
    pub pitch: f32,
}

// ── Resources ──────────────────────────────────────────────────────────────

/// Suppresses collision re-triggers for a short time after a battle ends.
#[derive(Resource)]
pub struct CollisionCooldown(pub Timer);

impl Default for CollisionCooldown {
    fn default() -> Self {
        let mut t = Timer::from_seconds(COLLISION_COOLDOWN, TimerMode::Once);
        t.tick(std::time::Duration::from_secs_f32(COLLISION_COOLDOWN)); // start finished
        Self(t)
    }
}

// ── Plugin ─────────────────────────────────────────────────────────────────

pub struct FlightPlugin;

impl Plugin for FlightPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CollisionCooldown>()
           .add_systems(OnEnter(GameState::Exploring), ship::spawn_ship)
           .add_systems(OnExit(GameState::Battle), systems::nudge_ship_off_monster)
           .add_systems(Update, systems::derive_zoom_from_distance)
           .add_systems(Update, (
               systems::flight_input,
               systems::ship_move,
               systems::chase_camera,
               systems::collide_with_monsters,
           ).chain().run_if(in_state(GameState::Exploring)));
    }
}
