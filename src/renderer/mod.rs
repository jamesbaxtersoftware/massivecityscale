pub mod buildings;
pub mod terrain;
pub mod solar;

use bevy::prelude::*;

pub struct RendererPlugin;

impl Plugin for RendererPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (
            terrain::spawn_ground,
            buildings::spawn_street_buildings,
            buildings::spawn_block_level,
            buildings::spawn_city_level,
            terrain::spawn_water_plane,
            terrain::spawn_continent_patches,
            terrain::spawn_planet_sphere,
            solar::spawn_solar_system,
        ).chain());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;
    use crate::{world_gen::WorldGenPlugin, theme::ThemePlugin, camera::CameraPlugin, scale::ScalePlugin};

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
           .add_plugins(ScalePlugin)
           .add_plugins(RendererPlugin);
        app.update();
        let mesh_count = app.world_mut()
            .query::<&Mesh3d>()
            .iter(app.world())
            .count();
        assert!(mesh_count > 0, "no mesh entities were spawned");
    }
}
