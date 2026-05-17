use bevy::prelude::*;
use crate::theme::Theme;
use crate::lod::LodRange;
use crate::renderer::scale_consts::LOD_SOLAR;

pub fn spawn_solar_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    theme: Res<Theme>,
) {
    // Sun — large emissive sphere to one side of the orbital ring
    let sun_mesh = meshes.add(Sphere::new(800.0).mesh().uv(32, 18)); // 800 km radius ≈ small star
    let sun_mat = materials.add(StandardMaterial {
        base_color: theme.solar.sun,
        emissive: LinearRgba::from(theme.solar.sun) * 2.0, // 2× base color for visible glow
        ..default()
    });
    commands.spawn((
        Mesh3d(sun_mesh),
        MeshMaterial3d(sun_mat),
        Transform::from_xyz(-5000.0, 0.0, 0.0), // 5000 km to the left of planet origin
        LodRange { min_scale: LOD_SOLAR.0, max_scale: LOD_SOLAR.1 },
        Visibility::Hidden,
    ));

    // Orbital ring centred at world origin
    let ring_mesh = meshes.add(
        Torus { minor_radius: 50.0, major_radius: 3000.0 } // tube: 50 km thick, orbit: 3000 km radius
            .mesh()
            .minor_resolution(4)
            .major_resolution(64),
    );
    let ring_mat = materials.add(StandardMaterial {
        base_color: theme.solar.orbital_ring,
        ..default()
    });
    commands.spawn((
        Mesh3d(ring_mesh),
        MeshMaterial3d(ring_mat),
        Transform::IDENTITY,
        LodRange { min_scale: LOD_SOLAR.0, max_scale: LOD_SOLAR.1 },
        Visibility::Hidden, // LodPlugin sets visible when ortho_scale in [3.0, 20.0]
    ));

    // Planet dot on the orbital ring
    let planet_mesh = meshes.add(Sphere::new(250.0).mesh().uv(16, 12)); // 250 km radius
    let planet_mat = materials.add(StandardMaterial {
        base_color: theme.solar.planet,
        ..default()
    });
    commands.spawn((
        Mesh3d(planet_mesh),
        MeshMaterial3d(planet_mat),
        Transform::from_xyz(0.0, 0.0, 3000.0), // at the top of the orbital ring
        LodRange { min_scale: LOD_SOLAR.0, max_scale: LOD_SOLAR.1 },
        Visibility::Hidden,
    ));
}
