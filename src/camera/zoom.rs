use bevy::prelude::*;
use bevy::input::mouse::{MouseWheel, MouseMotion};

pub const ZOOM_SPEED: f32 = 0.05;
pub const KEYBOARD_ZOOM_SPEED: f32 = 0.01; // per-frame step when Up/Down held
pub const ZOOM_MIN: f32 = 0.0;
pub const ZOOM_MAX: f32 = 1.0;
pub const ZOOM_EXPONENT: f32 = 4.2; // maps zoom [0,1] to ortho_scale [0.001, ~16]: 10^(0*4.2)=1, 10^(1*4.2)≈15849
pub const ORBIT_SPEED: f32 = 0.006;
const ORBIT_PIVOT_Y: f32 = -350.0; // sphere center — matches SPHERE_CENTER_Y in scale_consts

#[derive(Resource, Debug, Clone)]
pub struct ZoomLevel {
    pub value: f32,
}

impl ZoomLevel {
    pub fn to_ortho_scale(&self) -> f32 {
        // 0.001 at street (zoom 0) → ~16 at solar system (zoom 1)
        0.001 * (10.0f32).powf(self.value * ZOOM_EXPONENT)
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

pub fn handle_keyboard_zoom(
    keys: Res<ButtonInput<KeyCode>>,
    mut zoom: ResMut<ZoomLevel>,
    mut proj_query: Query<&mut Projection, With<Camera3d>>,
) {
    let delta = if keys.pressed(KeyCode::ArrowDown) {
        KEYBOARD_ZOOM_SPEED   // zoom out
    } else if keys.pressed(KeyCode::ArrowUp) {
        -KEYBOARD_ZOOM_SPEED  // zoom in
    } else {
        return;
    };
    zoom.value = (zoom.value + delta).clamp(ZOOM_MIN, ZOOM_MAX);
    if let Ok(mut proj) = proj_query.get_single_mut() {
        if let Projection::Orthographic(ref mut ortho) = *proj {
            ortho.scale = zoom.to_ortho_scale();
        }
    }
}

pub fn handle_orbit(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut motion: EventReader<MouseMotion>,
    mut cam_query: Query<&mut Transform, With<Camera3d>>,
) {
    if !mouse_buttons.pressed(MouseButton::Left) {
        motion.clear();
        return;
    }
    let pivot = Vec3::new(0.0, ORBIT_PIVOT_Y, 0.0);
    for ev in motion.read() {
        if let Ok(mut transform) = cam_query.get_single_mut() {
            let offset = transform.translation - pivot;
            let right = transform.right().as_vec3();
            // Horizontal drag → yaw around world Y; vertical → pitch around camera right
            let yaw = Quat::from_rotation_y(-ev.delta.x * ORBIT_SPEED);
            let pitch = Quat::from_axis_angle(right, -ev.delta.y * ORBIT_SPEED);
            let after_yaw = yaw * offset;
            let after_both = pitch * after_yaw;
            // Clamp elevation: skip pitch if it would flip past a pole
            let new_offset = if after_both.normalize().y.abs() < 0.98 {
                after_both
            } else {
                after_yaw
            };
            transform.translation = pivot + new_offset;
            transform.look_at(pivot, Vec3::Y);
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
