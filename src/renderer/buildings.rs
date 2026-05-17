use bevy::prelude::*;
use crate::world_gen::{WorldData, city::{CityData, DistrictData, BlockData, BuildingData}};
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


pub fn spawn_city_towers(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    world: Res<WorldData>,
    theme: Res<Theme>,
) {
    // One tall building-shaped tower per city — visible from globe, looks like a skyscraper marker
    let width = 3.0_f32;  // km
    let height = 12.0_f32; // km — ~3% of sphere radius, clearly readable on globe
    for city in &world.cities {
        let (cx, cz) = city_km(city);
        let Some((translation, rotation)) = project_to_sphere(cx, cz, height / 2.0) else { continue };
        let mesh = meshes.add(Cuboid::new(width, height, width));
        let material = materials.add(StandardMaterial {
            base_color: theme.buildings.commercial,
            ..default()
        });
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform { translation, rotation, scale: Vec3::ONE },
            LodRange { min_scale: LOD_BUILDINGS.0, max_scale: LOD_BUILDINGS.1 },
            Visibility::Hidden,
        ));
    }
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

