mod world_gen;

use bevy::prelude::*;
use world_gen::WorldGenPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(WorldGenPlugin)
        .run();
}
