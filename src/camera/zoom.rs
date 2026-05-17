use bevy::prelude::*;
use bevy::input::mouse::{MouseWheel, MouseMotion};
use bevy::window::PrimaryWindow;

pub const ZOOM_SPEED: f32 = 0.05;
pub const DOUBLE_CLICK_ZOOM_STEP: f32 = 0.15;
pub const DOUBLE_CLICK_THRESHOLD_SECS: f64 = 0.30;
pub const ZOOM_MIN: f32 = 0.0;
pub const ZOOM_MAX: f32 = 1.0;
pub const ZOOM_EXPONENT: f32 = 4.2;

const KEYBOARD_ORBIT_FACTOR: f32 = 5.0;
const ORBIT_PIVOT_Y: f32 = -350.0; // default sphere center Y — kept for tests
const SPHERE_RADIUS_KM: f32 = 350.0;
// Max elevation before look_at(pivot, Vec3::Y) becomes degenerate (~85°)
const MAX_ELEVATION: f32 = std::f32::consts::FRAC_PI_2 - 0.1;

// ── ZoomLevel ────────────────────────────────────────────────────────────────

#[derive(Resource, Debug, Clone)]
pub struct ZoomLevel {
    pub value: f32,
}

impl ZoomLevel {
    pub fn to_ortho_scale(&self) -> f32 {
        0.001 * (10.0f32).powf(self.value * ZOOM_EXPONENT)
    }
}

impl Default for ZoomLevel {
    fn default() -> Self {
        Self { value: 0.85 }
    }
}

// ── OrbitState ───────────────────────────────────────────────────────────────
// Stores camera position as explicit azimuth + elevation so orbit never gets
// stuck at poles from quaternion accumulation / gimbal lock.
// `pivot` tracks the home planet's sphere center in world space and is updated
// every frame by the solar orbit system.

#[derive(Resource, Debug, Clone)]
pub struct OrbitState {
    pub azimuth:   f32, // radians, wraps freely
    pub elevation: f32, // radians, clamped to ±MAX_ELEVATION
    pub distance:  f32, // km from pivot
    pub pivot:     Vec3, // world-space sphere center of the home planet
}

impl OrbitState {
    /// World position of the camera given current angles.
    pub fn camera_pos(&self) -> Vec3 {
        let xz = self.elevation.cos() * self.distance;
        self.pivot + Vec3::new(
            xz * self.azimuth.sin(),
            self.elevation.sin() * self.distance,
            xz * self.azimuth.cos(),
        )
    }
}

impl Default for OrbitState {
    fn default() -> Self {
        // Initial camera offset from sphere center: (350, 0, 350) - (0, -350, 0) = (350, 350, 350)
        let offset = Vec3::new(350.0, 350.0, 350.0);
        let dist = offset.length();
        Self {
            azimuth:   offset.x.atan2(offset.z),
            elevation: (offset.y / dist).asin(),
            distance:  dist,
            pivot:     Vec3::new(0.0, ORBIT_PIVOT_Y, 0.0),
        }
    }
}

// ── Scroll zoom ──────────────────────────────────────────────────────────────

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

// ── Double-click zoom ─────────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct DoubleClickState {
    last_left_secs:  f64,
    last_right_secs: f64,
}

fn ray_sphere_intersect(origin: Vec3, dir: Vec3, center: Vec3, radius: f32) -> Option<Vec3> {
    let l = origin - center;
    let b = l.dot(dir);
    let c = l.dot(l) - radius * radius;
    let disc = b * b - c;
    if disc < 0.0 { return None; }
    let sq = disc.sqrt();
    let t1 = -b - sq;
    let t2 = -b + sq;
    let t = if t1 > 0.0 { t1 } else if t2 > 0.0 { t2 } else { return None; };
    Some(origin + dir * t)
}

pub fn handle_double_click_zoom(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut state: ResMut<DoubleClickState>,
    mut zoom: ResMut<ZoomLevel>,
    mut orbit: ResMut<OrbitState>,
    mut cam_query: Query<(&mut Transform, &mut Projection), With<Camera3d>>,
) {
    let now = time.elapsed_secs_f64();

    // Right double-click → zoom out
    if mouse_buttons.just_pressed(MouseButton::Right) {
        if now - state.last_right_secs < DOUBLE_CLICK_THRESHOLD_SECS {
            state.last_right_secs = 0.0;
            let Ok((_, mut proj)) = cam_query.get_single_mut() else { return };
            let Projection::Orthographic(ref mut ortho) = *proj else { return };
            zoom.value = (zoom.value + DOUBLE_CLICK_ZOOM_STEP).clamp(ZOOM_MIN, ZOOM_MAX);
            ortho.scale = zoom.to_ortho_scale();
        } else {
            state.last_right_secs = now;
        }
        return;
    }

    if !mouse_buttons.just_pressed(MouseButton::Left) { return; }
    if now - state.last_left_secs >= DOUBLE_CLICK_THRESHOLD_SECS {
        state.last_left_secs = now;
        return;
    }
    state.last_left_secs = 0.0;

    let Ok((mut transform, mut proj)) = cam_query.get_single_mut() else { return };
    let Projection::Orthographic(ref mut ortho) = *proj else { return };

    // Orbit so the clicked sphere point becomes screen centre
    if let Ok(window) = windows.get_single() {
        if let Some(cursor_px) = window.cursor_position() {
            let win = Vec2::new(window.width(), window.height());
            let ndc = Vec2::new(
                cursor_px.x / win.x * 2.0 - 1.0,
                1.0 - cursor_px.y / win.y * 2.0,
            );
            let right   = transform.right().as_vec3();
            let up      = transform.up().as_vec3();
            let forward = transform.forward().as_vec3();
            let half_w  = ortho.scale * win.x / 2.0;
            let half_h  = ortho.scale * win.y / 2.0;
            let ray_origin = transform.translation + ndc.x * half_w * right + ndc.y * half_h * up;
            let pivot = orbit.pivot;

            if let Some(hit) = ray_sphere_intersect(ray_origin, forward, pivot, SPHERE_RADIUS_KM) {
                let outward = (hit - pivot).normalize();
                let new_elev = outward.y.asin();
                if new_elev.abs() < MAX_ELEVATION {
                    orbit.elevation = new_elev;
                    orbit.azimuth   = outward.x.atan2(outward.z);
                    transform.translation = orbit.camera_pos();
                    transform.look_at(pivot, Vec3::Y);
                }
            }
        }
    }

    zoom.value = (zoom.value - DOUBLE_CLICK_ZOOM_STEP).clamp(ZOOM_MIN, ZOOM_MAX);
    ortho.scale = zoom.to_ortho_scale();
}

// ── Keyboard orbit ────────────────────────────────────────────────────────────

pub fn handle_keyboard_orbit(
    keys: Res<ButtonInput<KeyCode>>,
    mut cam_query: Query<&mut Transform, With<Camera3d>>,
    mut orbit: ResMut<OrbitState>,
    zoom: Res<ZoomLevel>,
) {
    let speed = zoom.to_ortho_scale() * KEYBOARD_ORBIT_FACTOR / SPHERE_RADIUS_KM;
    let d_az = if keys.pressed(KeyCode::ArrowLeft) {
        speed
    } else if keys.pressed(KeyCode::ArrowRight) {
        -speed
    } else {
        0.0
    };
    let d_el = if keys.pressed(KeyCode::ArrowUp) {
        speed
    } else if keys.pressed(KeyCode::ArrowDown) {
        -speed
    } else {
        0.0
    };
    if d_az == 0.0 && d_el == 0.0 { return; }

    orbit.azimuth   += d_az;
    orbit.elevation  = (orbit.elevation + d_el).clamp(-MAX_ELEVATION, MAX_ELEVATION);

    let pivot = orbit.pivot;
    if let Ok(mut transform) = cam_query.get_single_mut() {
        transform.translation = orbit.camera_pos();
        transform.look_at(pivot, Vec3::Y);
    }
}

// ── Mouse orbit ───────────────────────────────────────────────────────────────

pub fn handle_orbit(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut motion: EventReader<MouseMotion>,
    mut cam_query: Query<&mut Transform, With<Camera3d>>,
    mut orbit: ResMut<OrbitState>,
    zoom: Res<ZoomLevel>,
) {
    if !mouse_buttons.pressed(MouseButton::Left) {
        motion.clear();
        return;
    }
    let speed = zoom.to_ortho_scale() / SPHERE_RADIUS_KM;
    let pivot = orbit.pivot;
    for ev in motion.read() {
        orbit.azimuth   -= ev.delta.x * speed;
        orbit.elevation  = (orbit.elevation - ev.delta.y * speed).clamp(-MAX_ELEVATION, MAX_ELEVATION);
        if let Ok(mut transform) = cam_query.get_single_mut() {
            transform.translation = orbit.camera_pos();
            transform.look_at(pivot, Vec3::Y);
        }
    }
}

// ── Camera sync ───────────────────────────────────────────────────────────────
// Runs every frame to keep the camera positioned correctly as the home planet
// moves along its orbit. User-input systems run after this and may further
// adjust the transform on the same frame — no conflict since both compute from
// OrbitState and orbit.pivot is already updated by orbit_bodies.

pub fn sync_camera_transform(
    orbit: Res<OrbitState>,
    mut cam: Query<&mut Transform, With<Camera3d>>,
) {
    if let Ok(mut t) = cam.get_single_mut() {
        t.translation = orbit.camera_pos();
        t.look_at(orbit.pivot, Vec3::Y);
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

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
        let solar  = ZoomLevel { value: 1.0 };
        assert!(street.to_ortho_scale() < solar.to_ortho_scale());
    }

    #[test]
    fn street_scale_shows_buildings() {
        let street = ZoomLevel { value: 0.0 };
        assert!(street.to_ortho_scale() < 0.01, "street ortho_scale should be < 0.01 to show buildings");
    }

    #[test]
    fn solar_scale_sees_solar_system() {
        let solar = ZoomLevel { value: 1.0 };
        assert!(solar.to_ortho_scale() > 5.0, "solar ortho_scale should be > 5.0 to see solar system");
    }

    #[test]
    fn default_zoom_is_mid_range() {
        let z = ZoomLevel::default();
        assert!(z.value > ZOOM_MIN && z.value < ZOOM_MAX);
    }

    #[test]
    fn orbit_state_camera_pos_round_trips() {
        let orbit = OrbitState::default();
        let pos = orbit.camera_pos();
        let pivot = Vec3::new(0.0, ORBIT_PIVOT_Y, 0.0);
        let offset = pos - pivot;
        let dist = offset.length();
        let recovered_el = (offset.y / dist).asin();
        let recovered_az = offset.x.atan2(offset.z);
        assert!((dist - orbit.distance).abs() < 0.01);
        assert!((recovered_el - orbit.elevation).abs() < 0.001);
        assert!((recovered_az - orbit.azimuth).abs() < 0.001);
    }

    #[test]
    fn elevation_clamped_at_max() {
        let mut orbit = OrbitState::default();
        orbit.elevation = (orbit.elevation + 100.0).clamp(-MAX_ELEVATION, MAX_ELEVATION);
        assert!(orbit.elevation <= MAX_ELEVATION);
    }
}
