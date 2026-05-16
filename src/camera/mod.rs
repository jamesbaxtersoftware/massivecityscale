pub mod zoom;

use bevy::prelude::*;
pub use zoom::ZoomLevel;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ZoomLevel::default())
           .add_systems(Startup, spawn_camera)
           .add_systems(Update, (zoom::handle_scroll, zoom::handle_pan));
    }
}

fn spawn_camera(mut commands: Commands) {
    let scale = ZoomLevel::default().to_ortho_scale();
    commands.spawn((
        Camera3d::default(),
        Projection::Orthographic(OrthographicProjection {
            scale,
            ..OrthographicProjection::default_3d()
        }),
        Transform::from_xyz(50.0, 50.0, 50.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
