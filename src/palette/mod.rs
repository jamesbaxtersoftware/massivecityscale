use bevy::prelude::*;
use crate::galaxy::gen::PlanetType;

/// Emissive-ish off-white for distant stars.
pub const STAR_COLOR: Color = Color::srgb(0.95, 0.93, 0.80);

/// Flat body color per planet type (dark-neon palette carried from v1).
pub fn planet_color(kind: PlanetType) -> Color {
    match kind {
        PlanetType::Rock => Color::srgb(0.35, 0.33, 0.30),
        PlanetType::Fire => Color::srgb(0.70, 0.25, 0.12),
        PlanetType::Water => Color::srgb(0.12, 0.30, 0.55),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_type_maps_to_a_distinct_color() {
        let r = planet_color(PlanetType::Rock).to_srgba();
        let f = planet_color(PlanetType::Fire).to_srgba();
        let w = planet_color(PlanetType::Water).to_srgba();
        assert!(r != f && f != w && r != w, "planet types are visually distinct");
    }
}
