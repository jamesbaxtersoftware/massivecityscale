/// All world positions are in km. The flat planet surface spans ±320 km from origin.

pub const CELL_KM: f32 = 10.0;
pub const WORLD_HALF_KM: f32 = 320.0;

pub const DIST_SCALE: f32 = 2.0;
pub const BLOCK_SCALE: f32 = 0.25;
pub const BLDG_POS_SCALE: f32 = 0.10;
pub const BLDG_W_SCALE: f32 = 0.20;
pub const BLDG_H_SCALE: f32 = 0.05;

pub const LOD_BUILDINGS: (f32, f32) = (0.0, 20.0); // always visible — appear as dots at globe, buildings up close
pub const SPHERE_CENTER_Y: f32 = -350.0; // sphere top = Y 0 (ground level), camera outside sphere
pub const SPHERE_RADIUS: f32 = 350.0;

pub const LOD_PLANET: (f32, f32) = (0.0, 20.0);            // sphere always visible — it is the world
pub const LOD_SPHERE_CONTINENTS: (f32, f32) = (0.0, 20.0); // continent patches always visible
pub const LOD_SOLAR: (f32, f32) = (3.0, 20.0);
