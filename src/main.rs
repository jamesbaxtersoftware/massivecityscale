mod world_gen;
mod theme;
mod camera;

use bevy::prelude::*;
use world_gen::WorldGenPlugin;
use theme::ThemePlugin;
use camera::CameraPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(WorldGenPlugin)
        .add_plugins(ThemePlugin)
        .add_plugins(CameraPlugin)
        .run();
}
