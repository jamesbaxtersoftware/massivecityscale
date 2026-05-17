/// All world positions are in km. The flat planet surface spans ±320 km from origin.

pub const CELL_KM: f32 = 10.0;
pub const WORLD_HALF_KM: f32 = 320.0;

pub const DIST_SCALE: f32 = 2.0;
pub const BLOCK_SCALE: f32 = 0.25;
pub const BLDG_POS_SCALE: f32 = 0.10;
pub const BLDG_W_SCALE: f32 = 0.20;
pub const BLDG_H_SCALE: f32 = 0.05;

pub const LOD_BUILDINGS: (f32, f32) = (0.0, 0.030);
pub const LOD_BLOCKS: (f32, f32) = (0.005, 0.20);
pub const LOD_CITIES: (f32, f32) = (0.05, 1.2);
pub const LOD_OCEAN: (f32, f32) = (0.0, 4.0);
pub const LOD_CONTINENTS: (f32, f32) = (0.04, 4.0);
pub const LOD_PLANET: (f32, f32) = (0.8, 20.0);
pub const LOD_SOLAR: (f32, f32) = (3.0, 20.0);
