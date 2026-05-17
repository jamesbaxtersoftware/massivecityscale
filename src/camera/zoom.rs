use bevy::prelude::*;
use bevy::input::mouse::{MouseWheel, MouseMotion};

pub const ZOOM_SPEED: f32 = 0.05;
pub const ZOOM_MIN: f32 = 0.0;
pub const ZOOM_MAX: f32 = 1.0;
pub const PAN_SPEED: f32 = 0.5;

#[derive(Resource, Debug, Clone)]
pub struct ZoomLevel {
    pub value: f32,
}

impl Default for ZoomLevel {
    fn default() -> Self {
        Self { value: 0.1 }
    }
}

impl ZoomLevel {
    pub fn to_ortho_scale(&self) -> f32 {
        // 0.05 at street (20px per world unit), 5.0 at solar (~1600 units visible)
        0.05 * (10.0f32).powf(self.value * 2.0)
    }
}

pub fn handle_scroll(
    mut scroll: EventReader<MouseWheel>,
    mut zoom: ResMut<ZoomLevel>,
    mut proj_query: Query<&mut Projection, With<Camera3d>>,
) {
    for ev in scroll.read() {
        zoom.value = (zoom.value - ev.y * ZOOM_SPEED).clamp(ZOOM_MIN, ZOOM_MAX);
    }
    if let Ok(mut proj) = proj_query.get_single_mut() {
        if let Projection::Orthographic(ref mut ortho) = *proj {
            ortho.scale = zoom.to_ortho_scale();
        }
    }
}

pub fn handle_pan(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut motion: EventReader<MouseMotion>,
    mut cam_query: Query<&mut Transform, With<Camera3d>>,
    zoom: Res<ZoomLevel>,
) {
    if !mouse_buttons.pressed(MouseButton::Left) {
        motion.clear();
        return;
    }
    let scale = zoom.to_ortho_scale();
    for ev in motion.read() {
        if let Ok(mut transform) = cam_query.get_single_mut() {
            let right = transform.right().as_vec3();
            let up = transform.up().as_vec3();
            transform.translation -= right * ev.delta.x * PAN_SPEED * scale * 0.001;
            transform.translation += up * ev.delta.y * PAN_SPEED * scale * 0.001;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_clamps_to_range() {
        let mut zoom = ZoomLevel { value: 0.0 };
        // simulate scrolling way out
        zoom.value = (zoom.value - 100.0 * ZOOM_SPEED).clamp(ZOOM_MIN, ZOOM_MAX);
        assert_eq!(zoom.value, ZOOM_MIN);
        // simulate scrolling way in
        zoom.value = (zoom.value + 100.0 * ZOOM_SPEED).clamp(ZOOM_MIN, ZOOM_MAX);
        assert_eq!(zoom.value, ZOOM_MAX);
    }

    #[test]
    fn ortho_scale_increases_with_zoom() {
        let street = ZoomLevel { value: 0.0 };
        let solar = ZoomLevel { value: 1.0 };
        assert!(street.to_ortho_scale() < solar.to_ortho_scale());
    }

    #[test]
    fn default_zoom_is_mid_range() {
        let z = ZoomLevel::default();
        assert!(z.value > ZOOM_MIN && z.value < ZOOM_MAX);
    }
}
