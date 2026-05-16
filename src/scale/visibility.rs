use bevy::prelude::*;
use super::SpaceTag;
use crate::camera::ZoomLevel;

pub fn update_visibility(
    zoom: Res<ZoomLevel>,
    mut query: Query<(&SpaceTag, &mut Visibility)>,
) {
    if !zoom.is_changed() {
        return;
    }
    for (tag, mut vis) in query.iter_mut() {
        let opacity = tag.opacity_for_zoom(zoom.value);
        *vis = if opacity > 0.01 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}
