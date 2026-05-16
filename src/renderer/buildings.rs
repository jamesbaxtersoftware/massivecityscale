use bevy::prelude::*;
use crate::world_gen::WorldData;
use crate::theme::Theme;
use crate::scale::SpaceTag;

pub fn spawn_street_buildings(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    world: Res<WorldData>,
    theme: Res<Theme>,
    space_query: Query<(Entity, &SpaceTag)>,
) {
    let street_entity = space_query.iter()
        .find(|(_, tag)| **tag == SpaceTag::Street)
        .map(|(e, _)| e);

    let Some(parent) = street_entity else { return };

    for city in &world.cities {
        for district in &city.districts {
            for block in &district.blocks {
                for building in &block.buildings {
                    let color = if building.height > 1.5 {
                        theme.buildings.commercial
                    } else {
                        theme.buildings.residential
                    };

                    let mesh = meshes.add(Cuboid::new(building.width, building.height, building.depth));
                    let material = materials.add(StandardMaterial {
                        base_color: color,
                        ..default()
                    });

                    commands.spawn((
                        Mesh3d(mesh),
                        MeshMaterial3d(material),
                        Transform::from_xyz(
                            building.x,
                            building.height / 2.0,
                            building.z,
                        ),
                    )).set_parent(parent);
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
    space_query: Query<(Entity, &SpaceTag)>,
) {
    let block_entity = space_query.iter()
        .find(|(_, tag)| **tag == SpaceTag::Block)
        .map(|(e, _)| e);

    let Some(parent) = block_entity else { return };

    for city in &world.cities {
        for district in &city.districts {
            for block in &district.blocks {
                let avg_height: f32 = if block.buildings.is_empty() {
                    0.5
                } else {
                    block.buildings.iter().map(|b| b.height).sum::<f32>() / block.buildings.len() as f32
                };

                let box_height = avg_height * 0.3;
                let mesh = meshes.add(Cuboid::new(0.7, box_height, 0.7));
                // Block level uses uniform color — individual height variation averages out at this scale
                let material = materials.add(StandardMaterial {
                    base_color: theme.buildings.residential,
                    ..default()
                });
                commands.spawn((
                    Mesh3d(mesh),
                    MeshMaterial3d(material),
                    Transform::from_xyz(block.x, box_height / 2.0, block.z),
                )).set_parent(parent);
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
    space_query: Query<(Entity, &SpaceTag)>,
) {
    let city_entity = space_query.iter()
        .find(|(_, tag)| **tag == SpaceTag::City)
        .map(|(e, _)| e);

    let Some(parent) = city_entity else { return };

    for city in &world.cities {
        let city_size = city.districts.len() as f32 * 0.3 + 1.0;
        let mesh = meshes.add(Cuboid::new(city_size, city_size * 0.1, city_size));
        let material = materials.add(StandardMaterial {
            base_color: theme.buildings.commercial,
            ..default()
        });
        // lat/lon are in planet grid coords (0..64); scale to city space (-16..16)
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_xyz(city.lat * 0.5 - 16.0, 0.0, city.lon * 0.5 - 16.0),
        )).set_parent(parent);
    }
}
