pub mod scale_consts;
pub mod buildings;
pub mod terrain;
pub mod solar;

use bevy::ecs::schedule::IntoSystemConfigs;
use bevy::prelude::*;

#[derive(Resource)]
pub struct PlanetRootEntity(pub Entity);

pub struct RendererPlugin;

impl Plugin for RendererPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (
            spawn_planet_root,
            terrain::spawn_planet_sphere,
            solar::spawn_solar_system,
            solar::spawn_starfield,
            buildings::spawn_city_towers,
            buildings::spawn_street_buildings,
            terrain::spawn_sphere_continents,
        ).chain())
        .add_systems(Update, (solar::orbit_bodies, solar::center_starfield_on_camera));
    }
}

/// Spawns the invisible root transform that all home-planet geometry is parented to.
/// Runs first in the Startup chain; subsequent systems read PlanetRootEntity.
fn spawn_planet_root(mut commands: Commands) {
    // Orbit at 5 000 km from sun, θ=0 → initial world pos ≈ (0, 0, 0)
    let orbit = solar::OrbitalBody::new(5_000.0, 0.0, 0.04);
    let pos   = orbit.position();
    let id    = commands.spawn((
        Transform::from_translation(pos),
        GlobalTransform::default(),
        orbit,
        solar::HomePlanet,
    )).id();
    commands.insert_resource(PlanetRootEntity(id));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{world_gen::WorldGenPlugin, theme::ThemePlugin, camera::CameraPlugin, lod::LodPlugin};

    #[test]
    fn renderer_plugin_spawns_mesh_entities() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
           .add_plugins(bevy::input::InputPlugin)
           .add_plugins(AssetPlugin::default())
           .init_asset::<Mesh>()
           .init_asset::<StandardMaterial>()
           .insert_resource(ClearColor(Color::BLACK))
           .add_plugins(WorldGenPlugin)
           .add_plugins(ThemePlugin)
           .add_plugins(CameraPlugin)
           .add_plugins(LodPlugin)
           .add_plugins(RendererPlugin);
        app.update();
        let mesh_count = app.world_mut()
            .query::<&Mesh3d>()
            .iter(app.world())
            .count();
        assert!(mesh_count > 0, "no mesh entities were spawned");
    }
}
