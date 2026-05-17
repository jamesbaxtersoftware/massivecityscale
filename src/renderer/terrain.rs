use bevy::prelude::*;
use crate::world_gen::WorldData;
use crate::theme::Theme;
use crate::lod::LodRange;
use crate::renderer::scale_consts::*;
use crate::renderer::PlanetRootEntity;

pub fn spawn_planet_sphere(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    theme: Res<Theme>,
    root: Res<PlanetRootEntity>,
) {
    // Radius 350 km, centre at Y=-350 → top at Y=0 (ground level). Camera is outside the sphere.
    let mesh = meshes.add(Sphere::new(SPHERE_RADIUS).mesh().uv(64, 36));
    let material = materials.add(StandardMaterial {
        base_color: theme.terrain.water,
        ..default()
    });
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_xyz(0.0, SPHERE_CENTER_Y, 0.0),
        LodRange { min_scale: LOD_PLANET.0, max_scale: LOD_PLANET.1 },
        Visibility::Hidden,
    )).set_parent(root.0);
}

pub fn spawn_sphere_continents(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    world: Res<WorldData>,
    theme: Res<Theme>,
    root: Res<PlanetRootEntity>,
) {
    let sphere_center = Vec3::new(0.0, SPHERE_CENTER_Y, 0.0);

    for continent in &world.planet.continents {
        for (cx, cz) in &continent.cells {
            let x = *cx as f32 * CELL_KM - WORLD_HALF_KM + CELL_KM / 2.0;
            let z = *cz as f32 * CELL_KM - WORLD_HALF_KM + CELL_KM / 2.0;

            let flat_dist_sq = x * x + z * z;
            if flat_dist_sq >= SPHERE_RADIUS * SPHERE_RADIUS {
                continue;
            }

            let y_sphere = SPHERE_CENTER_Y + (SPHERE_RADIUS * SPHERE_RADIUS - flat_dist_sq).sqrt();
            let surface_point = Vec3::new(x, y_sphere, z);
            let normal = (surface_point - sphere_center).normalize();
            let rotation = Quat::from_rotation_arc(Vec3::Y, normal);
            let position = surface_point + normal * 1.0;

            let mesh = meshes.add(Cuboid::new(CELL_KM * 0.9, 2.0, CELL_KM * 0.9));
            let material = materials.add(StandardMaterial {
                base_color: theme.terrain.land_continent,
                ..default()
            });
            commands.spawn((
                Mesh3d(mesh),
                MeshMaterial3d(material),
                Transform { translation: position, rotation, scale: Vec3::ONE },
                LodRange { min_scale: LOD_SPHERE_CONTINENTS.0, max_scale: LOD_SPHERE_CONTINENTS.1 },
                Visibility::Hidden,
            )).set_parent(root.0);
        }
    }
}
