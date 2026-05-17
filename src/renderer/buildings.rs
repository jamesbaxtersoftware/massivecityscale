use bevy::prelude::*;
use crate::world_gen::{WorldData, city::{CityData, DistrictData, BlockData}};
use crate::world_gen::city::BuildingData;
use crate::theme::Theme;
use crate::lod::LodRange;
use crate::renderer::scale_consts::*;

fn project_to_sphere(x: f32, z: f32, height_offset: f32) -> Option<(Vec3, Quat)> {
    let flat_dist_sq = x * x + z * z;
    if flat_dist_sq >= SPHERE_RADIUS * SPHERE_RADIUS {
        return None;
    }
    let sphere_center = Vec3::new(0.0, SPHERE_CENTER_Y, 0.0);
    let y_sphere = SPHERE_CENTER_Y + (SPHERE_RADIUS * SPHERE_RADIUS - flat_dist_sq).sqrt();
    let surface_point = Vec3::new(x, y_sphere, z);
    let normal = (surface_point - sphere_center).normalize();
    let rotation = Quat::from_rotation_arc(Vec3::Y, normal);
    let position = surface_point + normal * height_offset;
    Some((position, rotation))
}

fn city_km(city: &CityData) -> (f32, f32) {
    (
        city.lat * CELL_KM - WORLD_HALF_KM,
        city.lon * CELL_KM - WORLD_HALF_KM,
    )
}

fn building_world_xz(
    city: &CityData,
    district: &DistrictData,
    block: &BlockData,
    building: &BuildingData,
) -> (f32, f32) {
    let (cx, cz) = city_km(city);
    let dx = (district.x - city.lat) / 3.0 * DIST_SCALE;
    let dz = (district.z - city.lon) / 3.0 * DIST_SCALE;
    let bx = (block.x - district.x) * BLOCK_SCALE;
    let bz = (block.z - district.z) * BLOCK_SCALE;
    let ox = (building.x - block.x) * BLDG_POS_SCALE;
    let oz = (building.z - block.z) * BLDG_POS_SCALE;
    (cx + dx + bx + ox, cz + dz + bz + oz)
}

fn block_world_xz(city: &CityData, district: &DistrictData, block: &BlockData) -> (f32, f32) {
    let (cx, cz) = city_km(city);
    let dx = (district.x - city.lat) / 3.0 * DIST_SCALE;
    let dz = (district.z - city.lon) / 3.0 * DIST_SCALE;
    let bx = (block.x - district.x) * BLOCK_SCALE;
    let bz = (block.z - district.z) * BLOCK_SCALE;
    (cx + dx + bx, cz + dz + bz)
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
                    let (bx, bz) = building_world_xz(city, district, block, building);
                    let Some((translation, rotation)) = project_to_sphere(bx, bz, h / 2.0) else { continue };
                    let mesh = meshes.add(Cuboid::new(w, h, d));
                    let material = materials.add(StandardMaterial { base_color: color, ..default() });
                    commands.spawn((
                        Mesh3d(mesh),
                        MeshMaterial3d(material),
                        Transform { translation, rotation, scale: Vec3::ONE },
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
                    0.5 // fallback height for empty blocks (1 km raw → 0.05 km scaled)
                } else {
                    block.buildings.iter().map(|b| b.height).sum::<f32>()
                        / block.buildings.len() as f32
                };
                let box_height = avg_height * BLDG_H_SCALE;
                let (bx, bz) = block_world_xz(city, district, block);
                let Some((translation, rotation)) = project_to_sphere(bx, bz, box_height / 2.0) else { continue };
                let mesh = meshes.add(Cuboid::new(BLOCK_SCALE * 0.85, box_height, BLOCK_SCALE * 0.85)); // 0.85: gap between adjacent blocks
                let material = materials.add(StandardMaterial {
                    base_color: theme.buildings.residential,
                    ..default()
                });
                commands.spawn((
                    Mesh3d(mesh),
                    MeshMaterial3d(material),
                    Transform { translation, rotation, scale: Vec3::ONE },
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
        let x = cx + city_size / 2.0;
        let z = cz + city_size / 2.0;
        let Some((translation, rotation)) = project_to_sphere(x, z, 0.025) else { continue };
        let mesh = meshes.add(Cuboid::new(city_size, 0.05, city_size));
        let material = materials.add(StandardMaterial {
            base_color: theme.buildings.commercial,
            ..default()
        });
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform { translation, rotation, scale: Vec3::ONE },
            LodRange { min_scale: LOD_CITIES.0, max_scale: LOD_CITIES.1 },
            Visibility::Hidden,
        ));
    }
}
