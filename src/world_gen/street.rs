use noise::{NoiseFn, Simplex};
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use crate::world_gen::city::BlockData;

pub use crate::world_gen::city::{BuildingData, RoadData};

pub const MIN_BUILDING_HEIGHT: f32 = 0.2;
pub const MAX_BUILDING_HEIGHT: f32 = 3.0;
pub const BUILDINGS_PER_BLOCK: usize = 4;

pub fn populate_block(block: &mut BlockData, seed: u64) {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let height_noise = Simplex::new(seed as u32);

    for i in 0..BUILDINGS_PER_BLOCK {
        let bx = block.x + rng.gen::<f32>() * 0.8;
        let bz = block.z + rng.gen::<f32>() * 0.8;
        let width = 0.1 + rng.gen::<f32>() * 0.15;
        let depth = 0.1 + rng.gen::<f32>() * 0.15;
        let noise_val = height_noise.get([bx as f64, bz as f64]);
        let t = (noise_val + 1.0) / 2.0;
        let height = MIN_BUILDING_HEIGHT + t as f32 * (MAX_BUILDING_HEIGHT - MIN_BUILDING_HEIGHT);

        block.buildings.push(BuildingData { x: bx, z: bz, width, depth, height });

        // Road along x axis for each row
        if i == 0 {
            block.roads.push(RoadData {
                x: block.x,
                z: block.z - 0.05,
                length: 1.0,
                horizontal: true,
            });
            block.roads.push(RoadData {
                x: block.x - 0.05,
                z: block.z,
                length: 1.0,
                horizontal: false,
            });
        }
    }
}

pub fn populate_all_blocks(cities: &mut Vec<crate::world_gen::city::CityData>, seed: u64) {
    for city in cities.iter_mut() {
        for district in city.districts.iter_mut() {
            for block in district.blocks.iter_mut() {
                let block_seed = seed
                    .wrapping_add((block.x * 1000.0) as u64)
                    .wrapping_add((block.z * 997.0) as u64);
                populate_block(block, block_seed);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_gen::city::{BlockData};

    fn empty_block() -> BlockData {
        BlockData { x: 1.0, z: 1.0, buildings: vec![], roads: vec![] }
    }

    #[test]
    fn block_gets_correct_building_count() {
        let mut block = empty_block();
        populate_block(&mut block, 42);
        assert_eq!(block.buildings.len(), BUILDINGS_PER_BLOCK);
    }

    #[test]
    fn building_heights_in_range() {
        let mut block = empty_block();
        populate_block(&mut block, 42);
        for b in &block.buildings {
            assert!(b.height >= MIN_BUILDING_HEIGHT, "height {} below min", b.height);
            assert!(b.height <= MAX_BUILDING_HEIGHT, "height {} above max", b.height);
        }
    }

    #[test]
    fn block_gets_roads() {
        let mut block = empty_block();
        populate_block(&mut block, 42);
        assert!(!block.roads.is_empty());
    }

    #[test]
    fn block_population_is_deterministic() {
        let mut a = empty_block();
        let mut b = empty_block();
        populate_block(&mut a, 42);
        populate_block(&mut b, 42);
        assert_eq!(a.buildings.len(), b.buildings.len());
        assert!((a.buildings[0].height - b.buildings[0].height).abs() < f32::EPSILON);
    }
}
