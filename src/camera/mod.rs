pub mod zoom;

use bevy::prelude::*;
pub use zoom::ZoomLevel;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ZoomLevel::default())
           .add_systems(Startup, spawn_camera)
           .add_systems(Update, (zoom::handle_scroll, zoom::handle_keyboard_zoom, zoom::handle_orbit));
    }
}

fn spawn_camera(mut commands: Commands) {
    let scale = ZoomLevel::default().to_ortho_scale();
    // (350, 0, 350) → sphere center (0,-350,0): same diagonal viewing direction as (50,50,50)→origin
    let pivot = Vec3::new(0.0, -350.0, 0.0);
    commands.spawn((
        Camera3d::default(),
        Projection::Orthographic(OrthographicProjection {
            scale,
            near: -20000.0, // solar system spans ±5000 km; give plenty of headroom
            far:   20000.0,
            ..OrthographicProjection::default_3d()
        }),
        Transform::from_xyz(350.0, 0.0, 350.0).looking_at(pivot, Vec3::Y),
    ));
}
