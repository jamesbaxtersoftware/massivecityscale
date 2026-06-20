pub mod gen;

use bevy::prelude::*;
use crate::origin::WorldPos;
use crate::palette::{planet_color, STAR_COLOR};
use crate::streaming::{LodBody, LodTier};

pub const WORLD_SEED: u64 = 42;

#[derive(Component)]
pub struct StarBody;

#[derive(Component)]
pub struct PlanetBody {
    pub radius: f32,
}

pub struct GalaxyPlugin;

impl Plugin for GalaxyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_galaxy);
    }
}

fn spawn_galaxy(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let data = gen::generate(WORLD_SEED);

    // Shared star mesh+material → Bevy batches identical handles (cheap "instancing").
    let star_mesh = meshes.add(Sphere::new(1.0).mesh().ico(1).unwrap());
    let star_mat = materials.add(StandardMaterial {
        base_color: STAR_COLOR,
        emissive: LinearRgba::rgb(2.0, 1.9, 1.5),
        unlit: true,
        ..default()
    });
    for star in &data.stars {
        commands.spawn((
            StarBody,
            WorldPos(star.pos),
            Mesh3d(star_mesh.clone()),
            MeshMaterial3d(star_mat.clone()),
            Transform::default(),
        ));
    }

    for planet in &data.planets {
        commands.spawn((
            PlanetBody { radius: planet.radius as f32 },
            WorldPos(planet.pos),
            Mesh3d(meshes.add(Sphere::new(planet.radius as f32).mesh().ico(4).unwrap())),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: planet_color(planet.kind),
                ..default()
            })),
            Transform::default(),
            LodBody { tier: LodTier::Point },
        ));
    }

}
