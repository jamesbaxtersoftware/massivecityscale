mod world_gen;
mod theme;

use bevy::prelude::*;
use world_gen::WorldGenPlugin;
use theme::ThemePlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(WorldGenPlugin)
        .add_plugins(ThemePlugin)
        .run();
}
