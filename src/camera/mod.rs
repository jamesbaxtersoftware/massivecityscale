pub mod zoom;

use bevy::prelude::*;
pub use zoom::ZoomLevel;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ZoomLevel::default())
           .insert_resource(zoom::DoubleClickState::default())
           .insert_resource(zoom::OrbitState::default())
           .add_systems(Startup, spawn_camera)
           .add_systems(Update, (
               zoom::sync_camera_transform,
               zoom::handle_scroll,
               zoom::handle_double_click_zoom,
               zoom::handle_keyboard_orbit,
               zoom::handle_orbit,
           ));
    }
}

fn spawn_camera(mut commands: Commands) {
    let orbit = zoom::OrbitState::default();
    let scale = ZoomLevel::default().to_ortho_scale();
    commands.spawn((
        Camera3d::default(),
        Projection::Orthographic(OrthographicProjection {
            scale,
            near: -20000.0,
            far:   20000.0,
            ..OrthographicProjection::default_3d()
        }),
        Transform::from_translation(orbit.camera_pos()).looking_at(orbit.pivot, Vec3::Y),
    ));
}
