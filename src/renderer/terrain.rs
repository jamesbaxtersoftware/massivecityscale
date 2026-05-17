use bevy::prelude::*;
use crate::world_gen::WorldData;
use crate::theme::Theme;
use crate::lod::LodRange;
use crate::renderer::scale_consts::*;

pub fn spawn_ocean(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    theme: Res<Theme>,
) {
    // Large flat plane covering the whole planet surface (640 km) plus margin
    let mesh = meshes.add(Plane3d::default().mesh().size(700.0, 700.0));
    let material = materials.add(StandardMaterial {
        base_color: theme.terrain.water,
        ..default()
    });
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_xyz(0.0, -0.01, 0.0),
        LodRange { min_scale: LOD_OCEAN.0, max_scale: LOD_OCEAN.1 },
        Visibility::Visible,
    ));
}

pub fn spawn_continent_patches(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    world: Res<WorldData>,
    theme: Res<Theme>,
) {
    for continent in &world.planet.continents {
        for (cx, cz) in &continent.cells {
            // Each grid cell is CELL_KM × CELL_KM km; offset to centre at origin
            let x = *cx as f32 * CELL_KM - WORLD_HALF_KM + CELL_KM / 2.0;
            let z = *cz as f32 * CELL_KM - WORLD_HALF_KM + CELL_KM / 2.0;
            let mesh = meshes.add(Cuboid::new(CELL_KM, 0.02, CELL_KM));
            let material = materials.add(StandardMaterial {
                base_color: theme.terrain.land_continent,
                ..default()
            });
            commands.spawn((
                Mesh3d(mesh),
                MeshMaterial3d(material),
                Transform::from_xyz(x, 0.01, z),
                LodRange { min_scale: LOD_CONTINENTS.0, max_scale: LOD_CONTINENTS.1 },
                Visibility::Hidden,
            ));
        }
    }
}

pub fn spawn_planet_sphere(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    theme: Res<Theme>,
) {
    // Sphere radius slightly larger than WORLD_HALF_KM so it peeks around the flat map
    let mesh = meshes.add(Sphere::new(350.0).mesh().uv(32, 18));
    let material = materials.add(StandardMaterial {
        base_color: theme.solar.planet,
        ..default()
    });
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_xyz(0.0, -30.0, 0.0),
        LodRange { min_scale: LOD_PLANET.0, max_scale: LOD_PLANET.1 },
        Visibility::Hidden,
    ));
}
