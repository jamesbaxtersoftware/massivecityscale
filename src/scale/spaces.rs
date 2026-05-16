use bevy::prelude::*;
use super::SpaceTag;

pub fn spawn_spaces(mut commands: Commands) {
    for tag in SpaceTag::ALL {
        commands.spawn((
            tag,
            Transform::default(),
            Visibility::default(),
        ));
    }
}
