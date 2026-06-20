use bevy::prelude::*;
use bevy::math::DVec3;

pub fn world_to_render(world: DVec3, origin: DVec3) -> Vec3 {
    (world - origin).as_vec3()
}

#[derive(Component, Clone, Copy, Debug)]
pub struct WorldPos(pub DVec3);

#[derive(Resource, Default)]
pub struct FloatingOrigin(pub DVec3);

/// Ordered phases every per-frame system slots into. Configured as a chain in
/// `OriginPlugin` so input → movement → origin publish → transform sync →
/// camera → streaming always run in that order.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum FrameSet {
    Input,
    Move,
    Origin,
    Sync,
    Camera,
    Stream,
}

/// Derive every entity's render `Transform` from its f64 `WorldPos` relative to
/// the current floating origin (the ship). Runs in `FrameSet::Sync`.
pub fn sync_transforms(
    origin: Res<FloatingOrigin>,
    mut q: Query<(&WorldPos, &mut Transform)>,
) {
    for (wp, mut tf) in &mut q {
        tf.translation = world_to_render(wp.0, origin.0);
    }
}

pub struct OriginPlugin;

impl Plugin for OriginPlugin {
    fn build(&self, app: &mut App) {
        use bevy::ecs::schedule::IntoSystemSetConfigs;
        app.init_resource::<FloatingOrigin>()
            .configure_sets(Update, (
                FrameSet::Input,
                FrameSet::Move,
                FrameSet::Origin,
                FrameSet::Sync,
                FrameSet::Camera,
                FrameSet::Stream,
            ).chain())
            .add_systems(Update, sync_transforms.in_set(FrameSet::Sync));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_pos_is_world_minus_origin() {
        let world = DVec3::new(1_000_000.5, 0.0, -2_000_000.25);
        let origin = DVec3::new(1_000_000.0, 0.0, -2_000_000.0);
        let r = world_to_render(world, origin);
        assert!((r - Vec3::new(0.5, 0.0, -0.25)).length() < 1e-4,
            "far-from-origin pair resolves to a small, precise offset");
    }

    #[test]
    fn ship_at_origin_renders_at_zero() {
        let p = DVec3::new(9_876_543.0, 12_345.0, -555_555.0);
        assert_eq!(world_to_render(p, p), Vec3::ZERO);
    }

    #[test]
    fn sync_transforms_places_entity_relative_to_origin() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_plugins(OriginPlugin);
        app.insert_resource(FloatingOrigin(DVec3::new(1_000_000.0, 0.0, 0.0)));

        let e = app.world_mut().spawn((
            WorldPos(DVec3::new(1_000_010.0, 0.0, 0.0)),
            Transform::default(),
        )).id();

        app.update();

        let tf = app.world().get::<Transform>(e).unwrap();
        assert!((tf.translation - Vec3::new(10.0, 0.0, 0.0)).length() < 1e-3,
            "entity renders 10 km from the origin");
    }
}
