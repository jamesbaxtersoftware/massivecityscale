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
