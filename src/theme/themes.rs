use bevy::prelude::*;
use super::{Theme, BuildingColors, TerrainColors, SolarColors};

pub fn dark_neon() -> Theme {
    Theme {
        name: "dark_neon",
        background: Color::srgb(0.03, 0.05, 0.06),
        buildings: BuildingColors {
            residential: Color::srgb(0.48, 0.33, 0.58),
            commercial: Color::srgb(0.13, 0.47, 0.24),
            park: Color::srgb(0.10, 0.35, 0.15),
        },
        terrain: TerrainColors {
            ground: Color::srgb(0.06, 0.10, 0.06),
            water: Color::srgb(0.04, 0.08, 0.18),
            land_continent: Color::srgb(0.10, 0.20, 0.10),
        },
        solar: SolarColors {
            sun: Color::srgb(0.96, 0.62, 0.04),
            planet: Color::srgb(0.10, 0.20, 0.30),
            orbital_ring: Color::srgb(0.16, 0.16, 0.28),
        },
    }
}

pub fn clean_pastel() -> Theme {
    Theme {
        name: "clean_pastel",
        background: Color::srgb(0.94, 0.96, 0.98),
        buildings: BuildingColors {
            residential: Color::srgb(0.75, 0.86, 0.99),
            commercial: Color::srgb(0.73, 0.97, 0.81),
            park: Color::srgb(0.86, 0.99, 0.73),
        },
        terrain: TerrainColors {
            ground: Color::srgb(0.88, 0.91, 0.85),
            water: Color::srgb(0.68, 0.85, 0.97),
            land_continent: Color::srgb(0.78, 0.88, 0.72),
        },
        solar: SolarColors {
            sun: Color::srgb(1.0, 0.87, 0.40),
            planet: Color::srgb(0.60, 0.78, 0.95),
            orbital_ring: Color::srgb(0.80, 0.80, 0.90),
        },
    }
}

pub fn earthy() -> Theme {
    Theme {
        name: "earthy",
        background: Color::srgb(0.11, 0.10, 0.08),
        buildings: BuildingColors {
            residential: Color::srgb(0.49, 0.40, 0.27),
            commercial: Color::srgb(0.29, 0.42, 0.23),
            park: Color::srgb(0.22, 0.34, 0.16),
        },
        terrain: TerrainColors {
            ground: Color::srgb(0.18, 0.15, 0.10),
            water: Color::srgb(0.08, 0.14, 0.22),
            land_continent: Color::srgb(0.23, 0.20, 0.14),
        },
        solar: SolarColors {
            sun: Color::srgb(0.95, 0.75, 0.30),
            planet: Color::srgb(0.25, 0.32, 0.20),
            orbital_ring: Color::srgb(0.22, 0.20, 0.16),
        },
    }
}
