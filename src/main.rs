mod world_gen;
mod theme;
mod camera;
mod scale;
mod renderer;

use bevy::prelude::*;
use world_gen::WorldGenPlugin;
use theme::ThemePlugin;
use camera::CameraPlugin;
use scale::ScalePlugin;
use renderer::RendererPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(WorldGenPlugin)
        .add_plugins(ThemePlugin)
        .add_plugins(CameraPlugin)
        .add_plugins(ScalePlugin)
        .add_plugins(RendererPlugin)
        .add_systems(Startup, spawn_light)
        .run();
}

fn spawn_light(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            illuminance: 10_000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(1.0, 2.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
