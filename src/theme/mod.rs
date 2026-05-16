pub mod themes;

use bevy::prelude::*;

#[derive(Clone, Debug)]
pub struct BuildingColors {
    pub residential: Color,
    pub commercial: Color,
    pub park: Color,
}

#[derive(Clone, Debug)]
pub struct TerrainColors {
    pub ground: Color,
    pub water: Color,
    pub land_continent: Color,
}

#[derive(Clone, Debug)]
pub struct SolarColors {
    pub sun: Color,
    pub planet: Color,
    pub orbital_ring: Color,
}

#[derive(Resource, Clone, Debug)]
pub struct Theme {
    pub name: &'static str,
    pub background: Color,
    pub buildings: BuildingColors,
    pub terrain: TerrainColors,
    pub solar: SolarColors,
}

pub struct ThemePlugin;

impl Plugin for ThemePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(themes::dark_neon())
           .add_systems(Update, cycle_theme_on_t);
    }
}

fn cycle_theme_on_t(
    keys: Res<ButtonInput<KeyCode>>,
    mut theme: ResMut<Theme>,
    mut clear_color: ResMut<ClearColor>,
) {
    if keys.just_pressed(KeyCode::KeyT) {
        let next = match theme.name {
            "dark_neon" => themes::clean_pastel(),
            "clean_pastel" => themes::earthy(),
            _ => themes::dark_neon(),
        };
        clear_color.0 = next.background;
        *theme = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;

    #[test]
    fn theme_plugin_inserts_default_theme() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
           .insert_resource(ClearColor(Color::BLACK))
           .insert_resource(ButtonInput::<KeyCode>::default())
           .add_plugins(ThemePlugin);
        app.update();
        let theme = app.world().resource::<Theme>();
        assert_eq!(theme.name, "dark_neon");
    }

    #[test]
    fn all_three_themes_construct_without_panic() {
        let _ = themes::dark_neon();
        let _ = themes::clean_pastel();
        let _ = themes::earthy();
    }
}
