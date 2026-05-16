mod world_gen;
mod theme;
mod camera;
mod scale;

use bevy::prelude::*;
use world_gen::WorldGenPlugin;
use theme::ThemePlugin;
use camera::CameraPlugin;
use scale::ScalePlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(WorldGenPlugin)
        .add_plugins(ThemePlugin)
        .add_plugins(CameraPlugin)
        .add_plugins(ScalePlugin)
        .run();
}
