pub mod zoom;

use bevy::prelude::*;
pub use zoom::ZoomLevel;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ZoomLevel::default())
           .add_systems(Startup, spawn_camera);
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: 60.0_f32.to_radians(),
            near: 1.0,
            far: 60_000.0,
            ..default()
        }),
        Transform::from_xyz(0.0, 120.0, 700.0).looking_at(Vec3::new(0.0, 0.0, 600.0), Vec3::Y),
    ));
}
