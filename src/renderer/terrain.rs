use bevy::prelude::*;
use crate::theme::Theme;
use crate::scale::SpaceTag;

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
