use bevy::prelude::*;
use crate::theme::Theme;
use crate::scale::SpaceTag;

pub fn spawn_solar_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    theme: Res<Theme>,
    space_query: Query<(Entity, &SpaceTag)>,
) {
    let solar_entity = space_query.iter()
        .find(|(_, tag)| **tag == SpaceTag::SolarSystem)
        .map(|(e, _)| e);

    let Some(parent) = solar_entity else { return };

    // Sun
    let sun_mesh = meshes.add(Sphere::new(80.0).mesh().uv(32, 18));
    let sun_mat = materials.add(StandardMaterial {
        base_color: theme.solar.sun,
        emissive: LinearRgba::from(theme.solar.sun) * 2.0,
        ..default()
    });
    commands.spawn((
        Mesh3d(sun_mesh),
        MeshMaterial3d(sun_mat),
        Transform::from_xyz(-800.0, 0.0, 0.0),
    )).set_parent(parent);

    // Orbital ring — thin torus: major_radius = distance from center, minor_radius = tube thickness
    let ring_mesh = meshes.add(
        Torus { minor_radius: 5.0, major_radius: 490.0 }.mesh().minor_resolution(4).major_resolution(64)
    );
    let ring_mat = materials.add(StandardMaterial {
        base_color: theme.solar.orbital_ring,
        ..default()
    });
    commands.spawn((
        Mesh3d(ring_mesh),
        MeshMaterial3d(ring_mat),
        Transform::from_xyz(-800.0, 0.0, 0.0),
    )).set_parent(parent);

    // Planet dot at orbit position
    let planet_mesh = meshes.add(Sphere::new(20.0).mesh().uv(16, 12));
    let planet_mat = materials.add(StandardMaterial {
        base_color: theme.solar.planet,
        ..default()
    });
    commands.spawn((
        Mesh3d(planet_mesh),
        MeshMaterial3d(planet_mat),
        Transform::from_xyz(-320.0, 0.0, 0.0),
    )).set_parent(parent);
}
