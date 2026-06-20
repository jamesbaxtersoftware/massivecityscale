pub mod physics;
pub mod systems;

use bevy::prelude::*;
use bevy::ecs::schedule::IntoSystemConfigs;
use crate::origin::FrameSet;

/// Every flight/feel tunable in one place (Global Constraint). Adjust here.
#[derive(Resource)]
pub struct FlightTuning {
    pub base_max_speed: f32,
    pub boost_mult: f32,
    pub thrust_accel: f32,
    pub strafe_accel: f32,
    pub drag_half_life: f32,
    pub turn_rate: f32,
    pub steer_deadzone: f32,
    pub cam_back: f32,
    pub cam_up: f32,
    pub cam_lookahead: f32,
    pub cam_smooth_half_life: f32,
}

impl Default for FlightTuning {
    fn default() -> Self {
        Self {
            // Metre scale: cruise ~4,000 km/s so a ~40,000 km hop takes ~10s; boost 10x.
            base_max_speed: 4.0e6,
            boost_mult: 10.0,
            thrust_accel: 2.0e6,
            strafe_accel: 1.0e6,
            drag_half_life: 0.7,
            turn_rate: 1.2,
            steer_deadzone: 0.08,
            // Camera offsets in metres, framed for a ~30 m ship.
            cam_back: 70.0,
            cam_up: 28.0,
            cam_lookahead: 140.0,
            cam_smooth_half_life: 0.15,
        }
    }
}

#[derive(Component)]
pub struct PlayerShip {
    pub radius: f32,
}

#[derive(Component, Default)]
pub struct ShipVelocity(pub Vec3);

#[derive(Component, Default)]
pub struct ShipControl {
    pub yaw: f32,
    pub pitch: f32,
}

pub struct ShipPlugin;

impl Plugin for ShipPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FlightTuning>()
            .add_systems(Startup, systems::spawn_ship)
            .add_systems(Update, systems::center_cursor_once)
            .add_systems(Update, systems::aim_at_nearest_planet)
            .add_systems(Update, systems::flight_input.in_set(FrameSet::Input))
            .add_systems(Update, systems::ship_move.in_set(FrameSet::Move))
            .add_systems(Update, systems::set_origin.in_set(FrameSet::Origin))
            .add_systems(Update, systems::chase_camera.in_set(FrameSet::Camera));
    }
}
