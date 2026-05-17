pub mod scale_consts;
pub mod buildings;
pub mod terrain;
pub mod solar;

use bevy::ecs::schedule::IntoSystemConfigs;
use bevy::prelude::App;
use bevy::prelude::Plugin;
use bevy::prelude::Startup;

pub struct RendererPlugin;

impl Plugin for RendererPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (
            terrain::spawn_planet_sphere,
            solar::spawn_solar_system,
            buildings::spawn_city_towers,
            buildings::spawn_street_buildings,
            terrain::spawn_sphere_continents,
        ).chain());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;
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
