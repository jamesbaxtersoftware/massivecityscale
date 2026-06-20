use bevy::prelude::*;

mod galaxy;
mod origin;
mod palette;
mod ship;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Galaxy Roamer".into(),
                resolution: (1280.0, 720.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(origin::OriginPlugin)
        .add_plugins(ship::ShipPlugin)
        .insert_resource(ClearColor(Color::srgb(0.01, 0.01, 0.03)))
        .add_systems(Startup, setup)
        .add_systems(Update, quit_on_escape)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        DirectionalLight { illuminance: 10_000.0, shadows_enabled: false, ..default() },
        Transform::from_xyz(1.0, 2.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn quit_on_escape(keys: Res<ButtonInput<KeyCode>>, mut exit: EventWriter<AppExit>) {
    if keys.just_pressed(KeyCode::Escape) {
        exit.send(AppExit::Success);
    }
}
