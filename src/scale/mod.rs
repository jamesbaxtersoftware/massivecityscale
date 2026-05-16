pub mod spaces;
pub mod visibility;

use bevy::prelude::*;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpaceTag {
    Street,
    Block,
    Town,
    City,
    Country,
    World,
    Planet,
    SolarSystem,
}

impl SpaceTag {
    pub const ALL: [SpaceTag; 8] = [
        SpaceTag::Street,
        SpaceTag::Block,
        SpaceTag::Town,
        SpaceTag::City,
        SpaceTag::Country,
        SpaceTag::World,
        SpaceTag::Planet,
        SpaceTag::SolarSystem,
    ];

    pub fn zoom_center(&self) -> f32 {
        match self {
            SpaceTag::Street     => 0.0 / 7.0,
            SpaceTag::Block      => 1.0 / 7.0,
            SpaceTag::Town       => 2.0 / 7.0,
            SpaceTag::City       => 3.0 / 7.0,
            SpaceTag::Country    => 4.0 / 7.0,
            SpaceTag::World      => 5.0 / 7.0,
            SpaceTag::Planet     => 6.0 / 7.0,
            SpaceTag::SolarSystem => 7.0 / 7.0,
        }
    }

    pub fn opacity_for_zoom(&self, zoom: f32) -> f32 {
        let dist = (zoom - self.zoom_center()).abs();
        let fade_range = 1.5 / 7.0;
        (1.0 - dist / fade_range).clamp(0.0, 1.0)
    }
}

pub struct ScalePlugin;

impl Plugin for ScalePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spaces::spawn_spaces)
           .add_systems(Update, visibility::update_visibility);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;
    use crate::camera::ZoomLevel;

    #[test]
    fn street_is_fully_opaque_at_zoom_zero() {
        let opacity = SpaceTag::Street.opacity_for_zoom(0.0);
        assert!(opacity > 0.99, "street opacity at zoom 0 was {}", opacity);
    }

    #[test]
    fn solar_is_transparent_at_zoom_zero() {
        let opacity = SpaceTag::SolarSystem.opacity_for_zoom(0.0);
        assert!(opacity < 0.01, "solar opacity at zoom 0 was {}", opacity);
    }

    #[test]
    fn solar_is_fully_opaque_at_zoom_one() {
        let opacity = SpaceTag::SolarSystem.opacity_for_zoom(1.0);
        assert!(opacity > 0.99, "solar opacity at zoom 1 was {}", opacity);
    }

    #[test]
    fn eight_space_entities_spawned() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
           .insert_resource(ZoomLevel::default())
           .add_plugins(ScalePlugin);
        app.update();
        let count = app.world_mut().query::<&SpaceTag>().iter(app.world()).count();
        assert_eq!(count, 8);
    }
}
