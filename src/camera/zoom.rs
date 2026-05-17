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

impl ZoomLevel {
    pub fn to_ortho_scale(&self) -> f32 {
        // 0.001 at street (zoom 0) → ~16 at solar system (zoom 1)
        0.001 * (10.0f32).powf(self.value * 4.2)
    }
}

impl Default for ZoomLevel {
    fn default() -> Self {
        // zoom 0.45 → scale ≈ 0.078 → see ~100 km → city blobs visible on startup
        Self { value: 0.45 }
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
            // Project camera-up onto XZ plane so pan stays horizontal (no vertical drift)
            let cam_up = transform.up().as_vec3();
            let screen_up_xz = Vec3::new(cam_up.x, 0.0, cam_up.z).normalize_or_zero();
            transform.translation -= right * ev.delta.x * scale * PAN_SPEED;
            transform.translation += screen_up_xz * ev.delta.y * scale * PAN_SPEED;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_clamps_to_range() {
        let mut zoom = ZoomLevel { value: 0.0 };
        zoom.value = (zoom.value - 100.0 * ZOOM_SPEED).clamp(ZOOM_MIN, ZOOM_MAX);
        assert_eq!(zoom.value, ZOOM_MIN);
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
    fn street_scale_shows_buildings() {
        // At zoom 0, scale = 0.001, total view = 1.28 km.
        // Buildings at 25–50 m (0.025–0.050 km) → 25–50 px wide. Visible.
        let street = ZoomLevel { value: 0.0 };
        assert!(street.to_ortho_scale() < 0.01, "street ortho_scale should be < 0.01 to show buildings");
    }

    #[test]
    fn solar_scale_sees_solar_system() {
        // At zoom 1, scale ≈ 16, total view ≈ 20 000 km. Solar system (±5000 km) fits.
        let solar = ZoomLevel { value: 1.0 };
        assert!(solar.to_ortho_scale() > 5.0, "solar ortho_scale should be > 5.0 to see solar system");
    }

    #[test]
    fn default_zoom_is_mid_range() {
        let z = ZoomLevel::default();
        assert!(z.value > ZOOM_MIN && z.value < ZOOM_MAX);
    }
}
