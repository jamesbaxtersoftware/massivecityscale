use bevy::prelude::*;
use crate::theme::Theme;
use crate::scale::SpaceTag;
use crate::world_gen::{WorldData, planet::GRID_SIZE};

pub fn spawn_ground(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    theme: Res<Theme>,
    space_query: Query<(Entity, &SpaceTag)>,
) {
    let street_entity = space_query.iter()
        .find(|(_, tag)| **tag == SpaceTag::Street)
        .map(|(e, _)| e);

    let Some(parent) = street_entity else { return };

    let mesh = meshes.add(Plane3d::default().mesh().size(200.0, 200.0));
    let material = materials.add(StandardMaterial {
        base_color: theme.terrain.ground,
        ..default()
    });

    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_xyz(0.0, 0.0, 0.0),
    )).set_parent(parent);
}

pub fn spawn_water_plane(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    theme: Res<Theme>,
    space_query: Query<(Entity, &SpaceTag)>,
) {
    let world_entity = space_query.iter()
        .find(|(_, tag)| **tag == SpaceTag::World)
        .map(|(e, _)| e);

    let Some(parent) = world_entity else { return };

    let mesh = meshes.add(Plane3d::default().mesh().size(300.0, 300.0));
    let material = materials.add(StandardMaterial {
        base_color: theme.terrain.water,
        ..default()
    });

    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_xyz(0.0, -0.5, 0.0),
    )).set_parent(parent);
}

pub fn spawn_continent_patches(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    world: Res<WorldData>,
    theme: Res<Theme>,
    space_query: Query<(Entity, &SpaceTag)>,
) {
    let world_entity = space_query.iter()
        .find(|(_, tag)| **tag == SpaceTag::World)
        .map(|(e, _)| e);

    let Some(parent) = world_entity else { return };

    let cell_size = 2.0;
    let offset = GRID_SIZE as f32 * cell_size / 2.0;

    for continent in &world.planet.continents {
        for (cx, cz) in &continent.cells {
            let mesh = meshes.add(Cuboid::new(cell_size, 0.1, cell_size));
            let material = materials.add(StandardMaterial {
                base_color: theme.terrain.land_continent,
                ..default()
            });
            commands.spawn((
                Mesh3d(mesh),
                MeshMaterial3d(material),
                Transform::from_xyz(
                    *cx as f32 * cell_size - offset,
                    0.05,
                    *cz as f32 * cell_size - offset,
                ),
            )).set_parent(parent);
        }
    }
}

pub fn spawn_planet_sphere(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    theme: Res<Theme>,
    space_query: Query<(Entity, &SpaceTag)>,
) {
    let planet_entity = space_query.iter()
        .find(|(_, tag)| **tag == SpaceTag::Planet)
        .map(|(e, _)| e);

    let Some(parent) = planet_entity else { return };

    let mesh = meshes.add(Sphere::new(40.0).mesh().uv(32, 18));
    let material = materials.add(StandardMaterial {
        base_color: theme.solar.planet,
        ..default()
    });
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_xyz(0.0, 0.0, 0.0),
    )).set_parent(parent);
}
