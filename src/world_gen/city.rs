#[derive(Debug, Clone)]
pub struct BuildingData {
    pub x: f32,
    pub z: f32,
    pub width: f32,
    pub depth: f32,
    pub height: f32,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct RoadData {
    pub x: f32,
    pub z: f32,
    pub length: f32,
    pub horizontal: bool,
}

#[derive(Debug, Clone)]
pub struct BlockData {
    pub x: f32,
    pub z: f32,
    pub buildings: Vec<BuildingData>,
    pub roads: Vec<RoadData>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct DistrictData {
    pub x: f32,
    pub z: f32,
    pub blocks: Vec<BlockData>,
}

#[derive(Debug, Clone)]
pub struct CityData {
    pub lat: f32,
    pub lon: f32,
    pub districts: Vec<DistrictData>,
}

use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use crate::world_gen::planet::PlanetData;

pub const CITY_COUNT_TARGET: usize = 8;
pub const CITY_MIN_DISTANCE: f32 = 6.0;
pub const DISTRICTS_PER_CITY: usize = 4;
pub const BLOCKS_PER_DISTRICT: usize = 9;

pub fn generate(planet: &PlanetData, seed: u64) -> Vec<CityData> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed.wrapping_add(1));
    let land_cells: Vec<(f32, f32)> = planet.continents.iter()
        .flat_map(|c| c.cells.iter().map(|(x, z)| (*x as f32, *z as f32)))
        .collect();

    if land_cells.is_empty() {
        return vec![];
    }

    let mut cities: Vec<CityData> = Vec::new();
    let mut attempts = 0;

    while cities.len() < CITY_COUNT_TARGET && attempts < 1000 {
        attempts += 1;
        let idx = rng.gen_range(0..land_cells.len());
        let (lat, lon) = land_cells[idx];

        let too_close = cities.iter().any(|c| {
            let dx = c.lat - lat;
            let dz = c.lon - lon;
            (dx * dx + dz * dz).sqrt() < CITY_MIN_DISTANCE
        });

        if !too_close {
            let districts = generate_districts(&mut rng, lat, lon);
            cities.push(CityData { lat, lon, districts });
        }
    }

    cities
}

fn generate_districts(rng: &mut ChaCha8Rng, city_lat: f32, city_lon: f32) -> Vec<DistrictData> {
    let mut districts = Vec::new();
    let side = (DISTRICTS_PER_CITY as f32).sqrt() as i32;
    for dx in 0..side {
        for dz in 0..side {
            let x = city_lat + dx as f32 * 3.0;
            let z = city_lon + dz as f32 * 3.0;
            let blocks = generate_blocks(rng, x, z);
            districts.push(DistrictData { x, z, blocks });
        }
    }
    districts
}

fn generate_blocks(_rng: &mut ChaCha8Rng, district_x: f32, district_z: f32) -> Vec<BlockData> {
    let mut blocks = Vec::new();
    let side = (BLOCKS_PER_DISTRICT as f32).sqrt() as i32;
    for bx in 0..side {
        for bz in 0..side {
            let x = district_x + bx as f32;
            let z = district_z + bz as f32;
            blocks.push(BlockData { x, z, buildings: vec![], roads: vec![] });
        }
    }
    blocks
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_gen::planet;

    fn test_planet() -> PlanetData {
        planet::generate(42)
    }

    #[test]
    fn cities_placed_on_land() {
        let planet = test_planet();
        let land_set: std::collections::HashSet<(i32, i32)> = planet.continents.iter()
            .flat_map(|c| c.cells.iter().cloned())
            .collect();
        let cities = generate(&planet, 42);
        for city in &cities {
            assert!(
                land_set.contains(&(city.lat as i32, city.lon as i32)),
                "city at ({}, {}) is not on land", city.lat, city.lon
            );
        }
    }

    #[test]
    fn cities_are_minimum_distance_apart() {
        let planet = test_planet();
        let cities = generate(&planet, 42);
        for i in 0..cities.len() {
            for j in (i + 1)..cities.len() {
                let dx = cities[i].lat - cities[j].lat;
                let dz = cities[i].lon - cities[j].lon;
                let dist = (dx * dx + dz * dz).sqrt();
                assert!(dist >= CITY_MIN_DISTANCE, "cities {} and {} are too close: {}", i, j, dist);
            }
        }
    }

    #[test]
    fn city_generation_is_deterministic() {
        let planet = test_planet();
        let a = generate(&planet, 42);
        let b = generate(&planet, 42);
        assert_eq!(a.len(), b.len());
    }
}
