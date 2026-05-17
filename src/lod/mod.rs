use bevy::prelude::*;
use crate::camera::ZoomLevel;

#[derive(Component, Debug, Clone, Copy)]
pub struct LodRange {
    pub min_scale: f32,
    pub max_scale: f32,
}

pub struct LodPlugin;

impl Plugin for LodPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_lod_visibility);
    }
}

fn update_lod_visibility(
    zoom: Res<ZoomLevel>,
    mut query: Query<(&LodRange, &mut Visibility)>,
) {
    let scale = zoom.to_ortho_scale();
    for (lod, mut vis) in query.iter_mut() {
        *vis = if scale >= lod.min_scale && scale <= lod.max_scale {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;
    use crate::camera::ZoomLevel;

    #[test]
    fn entity_visible_within_lod_range() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
           .insert_resource(ZoomLevel { value: 0.0 }) // fully zoomed in
           .add_plugins(LodPlugin);

        let entity = app.world_mut().spawn((
            LodRange { min_scale: 0.0, max_scale: 0.05 },
            Visibility::Hidden,
        )).id();

        app.update();

        let vis = app.world().get::<Visibility>(entity).unwrap();
        assert_eq!(*vis, Visibility::Visible);
    }

    #[test]
    fn entity_hidden_above_lod_range() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
           .insert_resource(ZoomLevel { value: 1.0 }) // fully zoomed out
           .add_plugins(LodPlugin);

        let entity = app.world_mut().spawn((
            LodRange { min_scale: 0.0, max_scale: 0.03 }, // buildings range
            Visibility::Visible,
        )).id();

        app.update();

        let vis = app.world().get::<Visibility>(entity).unwrap();
        assert_eq!(*vis, Visibility::Hidden);
    }

    #[test]
    fn entity_hidden_below_lod_range() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
           .insert_resource(ZoomLevel { value: 0.0 }) // fully zoomed in
           .add_plugins(LodPlugin);

        let entity = app.world_mut().spawn((
            LodRange { min_scale: 3.0, max_scale: 20.0 }, // solar range
            Visibility::Visible,
        )).id();

        app.update();

        let vis = app.world().get::<Visibility>(entity).unwrap();
        assert_eq!(*vis, Visibility::Hidden);
    }
}
