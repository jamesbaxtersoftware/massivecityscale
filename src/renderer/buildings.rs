use bevy::prelude::*;
use crate::world_gen::{WorldData, city::{CityData, DistrictData, BlockData}};
use crate::world_gen::city::BuildingData;
use crate::theme::Theme;
use crate::lod::LodRange;
use crate::renderer::scale_consts::*;

fn city_km(city: &CityData) -> (f32, f32) {
    (
        city.lat * CELL_KM - WORLD_HALF_KM,
        city.lon * CELL_KM - WORLD_HALF_KM,
    )
}

fn building_world_pos(
    city: &CityData,
    district: &DistrictData,
    block: &BlockData,
    building: &BuildingData,
) -> Vec3 {
    let (cx, cz) = city_km(city);
    let dx = (district.x - city.lat) / 3.0 * DIST_SCALE;
    let dz = (district.z - city.lon) / 3.0 * DIST_SCALE;
    let bx = (block.x - district.x) * BLOCK_SCALE;
    let bz = (block.z - district.z) * BLOCK_SCALE;
    let ox = (building.x - block.x) * BLDG_POS_SCALE;
    let oz = (building.z - block.z) * BLDG_POS_SCALE;
    let h = building.height * BLDG_H_SCALE;
    Vec3::new(cx + dx + bx + ox, h / 2.0, cz + dz + bz + oz)
}

fn block_world_pos(city: &CityData, district: &DistrictData, block: &BlockData) -> Vec3 {
    let (cx, cz) = city_km(city);
    let dx = (district.x - city.lat) / 3.0 * DIST_SCALE;
    let dz = (district.z - city.lon) / 3.0 * DIST_SCALE;
    let bx = (block.x - district.x) * BLOCK_SCALE;
    let bz = (block.z - district.z) * BLOCK_SCALE;
    Vec3::new(cx + dx + bx, 0.0, cz + dz + bz)
}

pub fn spawn_street_buildings(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    world: Res<WorldData>,
    theme: Res<Theme>,
) {
    for city in &world.cities {
        for district in &city.districts {
            for block in &district.blocks {
                for building in &block.buildings {
                    let color = if building.height > 1.5 {
                        theme.buildings.commercial
                    } else {
                        theme.buildings.residential
                    };
                    let w = building.width * BLDG_W_SCALE;
                    let h = building.height * BLDG_H_SCALE;
                    let d = building.depth * BLDG_W_SCALE;
                    let mesh = meshes.add(Cuboid::new(w, h, d));
                    let material = materials.add(StandardMaterial { base_color: color, ..default() });
                    let pos = building_world_pos(city, district, block, building);
                    commands.spawn((
                        Mesh3d(mesh),
                        MeshMaterial3d(material),
                        Transform::from_translation(pos),
                        LodRange { min_scale: LOD_BUILDINGS.0, max_scale: LOD_BUILDINGS.1 },
                        Visibility::Hidden,
                    ));
                }
            }
        }
    }
}

pub fn spawn_block_level(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    world: Res<WorldData>,
    theme: Res<Theme>,
) {
    for city in &world.cities {
        for district in &city.districts {
            for block in &district.blocks {
                let avg_height: f32 = if block.buildings.is_empty() {
                    0.5
                } else {
                    block.buildings.iter().map(|b| b.height).sum::<f32>()
                        / block.buildings.len() as f32
                };
                let box_height = avg_height * BLDG_H_SCALE;
                let mut pos = block_world_pos(city, district, block);
                pos.y = box_height / 2.0;
                let mesh = meshes.add(Cuboid::new(BLOCK_SCALE * 0.85, box_height, BLOCK_SCALE * 0.85));
                let material = materials.add(StandardMaterial {
                    // Block level uses uniform color; individual height variation averages out
                    base_color: theme.buildings.residential,
                    ..default()
                });
                commands.spawn((
                    Mesh3d(mesh),
                    MeshMaterial3d(material),
                    Transform::from_translation(pos),
                    LodRange { min_scale: LOD_BLOCKS.0, max_scale: LOD_BLOCKS.1 },
                    Visibility::Hidden,
                ));
            }
        }
    }
}

pub fn spawn_city_level(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    world: Res<WorldData>,
    theme: Res<Theme>,
) {
    // City extent: 2 districts × DIST_SCALE + some block extent ≈ 4.5 km
    let city_size = 2.0 * DIST_SCALE + BLOCK_SCALE * 2.0;

    for city in &world.cities {
        let (cx, cz) = city_km(city);
        let mesh = meshes.add(Cuboid::new(city_size, 0.05, city_size));
        let material = materials.add(StandardMaterial {
            base_color: theme.buildings.commercial,
            ..default()
        });
        // Centre the blob over the city's content area
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_xyz(cx + city_size / 2.0, 0.025, cz + city_size / 2.0),
            LodRange { min_scale: LOD_CITIES.0, max_scale: LOD_CITIES.1 },
            Visibility::Hidden,
        ));
    }
}
