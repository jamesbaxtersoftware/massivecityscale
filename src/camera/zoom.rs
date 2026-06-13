use bevy::prelude::*;

pub const ZOOM_MIN: f32 = 0.0;
pub const ZOOM_MAX: f32 = 1.0;
pub const ZOOM_EXPONENT: f32 = 4.2;

/// Maps camera-to-nearest-planet-surface distance (km) to an effective ortho
/// scale before inverting `to_ortho_scale`. Tuned so close flight lands in the
/// buildings band and interplanetary distance crosses the solar band (3.0).
pub const ZOOM_DIST_K: f32 = 0.0015;

// ── ZoomLevel ────────────────────────────────────────────────────────────────
// Derived (not user input) — written every frame by the flight module's
// `derive_zoom_from_distance` system. The LOD system reads it to choose which
// scene tier (buildings / solar / stars) is visible.

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

/// Derive a [0,1] `ZoomLevel::value` from the camera's distance to the nearest
/// planet surface. Inverts `to_ortho_scale` so the existing LOD bands work
/// unchanged. Clamped to [0,1].
pub fn zoom_value_from_distance(dist_to_surface: f32) -> f32 {
    let scale = (dist_to_surface.max(0.0) * ZOOM_DIST_K).max(1e-6);
    ((scale / 0.001).log10() / ZOOM_EXPONENT).clamp(ZOOM_MIN, ZOOM_MAX)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_clamps_to_range() {
        let mut zoom = ZoomLevel { value: 0.0 };
        zoom.value = (zoom.value - 100.0 * 0.05).clamp(ZOOM_MIN, ZOOM_MAX);
        assert_eq!(zoom.value, ZOOM_MIN);
        zoom.value = (zoom.value + 100.0 * 0.05).clamp(ZOOM_MIN, ZOOM_MAX);
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
    fn zoom_value_round_trips_through_distance() {
        let v = 0.5_f32;
        let scale = ZoomLevel { value: v }.to_ortho_scale();
        let dist = scale / ZOOM_DIST_K;
        let recovered = zoom_value_from_distance(dist);
        assert!((recovered - v).abs() < 1e-3, "round trip: {recovered} vs {v}");
    }

    #[test]
    fn zoom_value_clamps_at_both_ends() {
        assert_eq!(zoom_value_from_distance(0.0), 0.0);
        assert_eq!(zoom_value_from_distance(1.0e9), 1.0);
    }

    #[test]
    fn close_distance_below_solar_band_far_above() {
        // Solar LOD turns on at ortho scale 3.0.
        let close = zoom_value_from_distance(50.0);
        let far = zoom_value_from_distance(8000.0);
        assert!(ZoomLevel { value: close }.to_ortho_scale() < 3.0, "close: street/buildings");
        assert!(ZoomLevel { value: far }.to_ortho_scale() >= 3.0, "far: solar visible");
    }
}
