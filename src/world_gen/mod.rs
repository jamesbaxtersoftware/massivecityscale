pub mod planet;
pub mod city;
pub mod street;

use bevy::prelude::*;

pub const WORLD_SEED: u64 = 42;

#[derive(Resource)]
pub struct WorldData {
    pub planet: planet::PlanetData,
    pub cities: Vec<city::CityData>,
}

pub struct WorldGenPlugin;

impl Plugin for WorldGenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, generate_world);
    }
}

fn generate_world(mut commands: Commands) {
    let planet = planet::generate(WORLD_SEED);
    let mut cities = city::generate(&planet, WORLD_SEED);
    street::populate_all_blocks(&mut cities, WORLD_SEED);
    commands.insert_resource(WorldData { planet, cities });
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;

    #[test]
    fn world_gen_plugin_inserts_resource() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
           .add_plugins(WorldGenPlugin);
        app.update();
        assert!(app.world().get_resource::<WorldData>().is_some());
    }

    #[test]
    fn generated_world_has_cities_with_buildings() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
           .add_plugins(WorldGenPlugin);
        app.update();
        let world = app.world().resource::<WorldData>();
        assert!(!world.cities.is_empty());
        let first_city = &world.cities[0];
        let first_block = &first_city.districts[0].blocks[0];
        assert!(!first_block.buildings.is_empty());
    }
}
