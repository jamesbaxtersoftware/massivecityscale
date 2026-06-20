use bevy::prelude::*;
use crate::origin::{FloatingOrigin, FrameSet, WorldPos};
use crate::galaxy::PlanetBody;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum LodTier {
    Culled,
    Point,
    Sphere,
    Terrain,
}

/// Distance thresholds (km, measured to the planet *surface*) at which a body
/// changes representation. Slice defaults; tune for feel.
#[derive(Resource)]
pub struct LodThresholds {
    pub cull: f64,
    pub sphere: f64,
    pub terrain: f64,
}

impl Default for LodThresholds {
    fn default() -> Self {
        // Surface-distance thresholds in metres: show a sphere within ~50,000 km,
        // resolve terrain within ~2,000 km of the surface, cull only very far away.
        Self { cull: 5.0e9, sphere: 5.0e7, terrain: 2.0e6 }
    }
}

/// Choose a representation from the distance to the body's surface.
pub fn tier_for_distance(surface_dist: f64, t: &LodThresholds) -> LodTier {
    if surface_dist > t.cull {
        LodTier::Culled
    } else if surface_dist > t.sphere {
        LodTier::Point
    } else if surface_dist > t.terrain {
        LodTier::Sphere
    } else {
        LodTier::Terrain
    }
}

#[derive(Component)]
pub struct LodBody {
    pub tier: LodTier,
}

pub struct StreamingPlugin;

impl Plugin for StreamingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LodThresholds>()
            .add_systems(Update, update_lod.in_set(FrameSet::Stream));
    }
}

/// For each planet, compute distance from the floating origin (the ship) to its
/// surface, pick a tier, and toggle visibility. Planets are hidden only when
/// fully culled; tier transitions to Sphere/Terrain are consumed by `terrain`.
pub fn update_lod(
    origin: Res<FloatingOrigin>,
    thresholds: Res<LodThresholds>,
    mut q: Query<(&WorldPos, &PlanetBody, &mut LodBody, &mut Visibility)>,
) {
    for (wp, body, mut lod, mut vis) in &mut q {
        let center_dist = (wp.0 - origin.0).length();
        let surface_dist = center_dist - body.radius as f64;
        let tier = tier_for_distance(surface_dist, &thresholds);
        lod.tier = tier;
        *vis = match tier {
            LodTier::Culled => Visibility::Hidden,
            _ => Visibility::Visible,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiers_step_in_by_distance() {
        let t = LodThresholds::default();
        assert_eq!(tier_for_distance(1.0e10, &t), LodTier::Culled); // beyond cull
        assert_eq!(tier_for_distance(1.0e8, &t), LodTier::Point); // far in-system
        assert_eq!(tier_for_distance(1.0e7, &t), LodTier::Sphere); // approaching
        assert_eq!(tier_for_distance(1.0e5, &t), LodTier::Terrain); // near surface
    }

    #[test]
    fn boundaries_round_down_to_the_nearer_tier() {
        let t = LodThresholds::default();
        // Exactly at a threshold is NOT "greater than", so it falls to the nearer tier.
        assert_eq!(tier_for_distance(t.terrain, &t), LodTier::Terrain);
        assert_eq!(tier_for_distance(t.sphere, &t), LodTier::Sphere);
    }
}
