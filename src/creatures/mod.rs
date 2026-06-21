//! Wild creatures on the surface: spawn per landing, wander, and flag the nearest
//! one within reach as "engaged" (the capture loop builds on this). Creatures live
//! only while on foot.

use bevy::prelude::*;
use bevy::render::view::RenderLayers;
use rand::Rng;
use crate::onfoot::{Avatar, Mode, SURFACE_LAYER};

const COUNT: usize = 6;
const FIELD: f32 = 35.0;
const WANDER_SPEED: f32 = 4.0;
const ENGAGE_RANGE: f32 = 9.0;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum CreatureKind {
    Grasshog,
    Aquabud,
    Rockfang,
    Flarehog,
}

impl CreatureKind {
    pub fn color(self) -> Color {
        match self {
            CreatureKind::Grasshog => Color::srgb(0.4, 0.7, 0.3),
            CreatureKind::Aquabud => Color::srgb(0.3, 0.6, 0.95),
            CreatureKind::Rockfang => Color::srgb(0.55, 0.5, 0.45),
            CreatureKind::Flarehog => Color::srgb(0.9, 0.45, 0.2),
        }
    }
    fn pick(n: u32) -> Self {
        match n % 4 {
            0 => CreatureKind::Grasshog,
            1 => CreatureKind::Aquabud,
            2 => CreatureKind::Rockfang,
            _ => CreatureKind::Flarehog,
        }
    }
}

#[derive(Component)]
#[allow(dead_code)] // kind/level/hp/max_hp consumed by the capture loop (next task)
pub struct Creature {
    pub kind: CreatureKind,
    pub level: u32,
    pub hp: f32,
    pub max_hp: f32,
    wander_target: Vec3,
    wander_timer: f32,
}

/// The creature currently within reach of the avatar (for the capture loop).
#[derive(Resource, Default)]
pub struct Engaged(pub Option<Entity>);

pub struct CreaturesPlugin;

impl Plugin for CreaturesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Engaged>()
            .add_systems(OnEnter(Mode::OnFoot), spawn_creatures)
            .add_systems(OnExit(Mode::OnFoot), despawn_creatures)
            .add_systems(Update, (wander, update_engaged).run_if(in_state(Mode::OnFoot)));
    }
}

fn spawn_creatures(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let body = meshes.add(Capsule3d::new(0.7, 0.9));
    let mut rng = rand::thread_rng();
    for i in 0..COUNT {
        let kind = CreatureKind::pick(rng.gen());
        let level = rng.gen_range(2..18);
        let max_hp = 20.0 + level as f32 * 4.0;
        let pos = Vec3::new(
            rng.gen_range(-FIELD..FIELD),
            1.0,
            rng.gen_range(-FIELD..FIELD),
        );
        commands.spawn((
            Creature {
                kind,
                level,
                hp: max_hp,
                max_hp,
                wander_target: pos,
                wander_timer: rng.gen_range(0.0..3.0),
            },
            RenderLayers::layer(SURFACE_LAYER),
            Mesh3d(body.clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: kind.color(),
                ..default()
            })),
            Transform::from_translation(pos + Vec3::Y * (i as f32 * 0.0)),
        ));
    }
}

fn despawn_creatures(mut commands: Commands, q: Query<Entity, With<Creature>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

/// Random-walk wander across the field.
fn wander(time: Res<Time>, mut q: Query<(&mut Creature, &mut Transform)>) {
    let dt = time.delta_secs();
    let mut rng = rand::thread_rng();
    for (mut c, mut tf) in &mut q {
        c.wander_timer -= dt;
        if c.wander_timer <= 0.0 {
            c.wander_target = Vec3::new(
                rng.gen_range(-FIELD..FIELD),
                1.0,
                rng.gen_range(-FIELD..FIELD),
            );
            c.wander_timer = rng.gen_range(2.0..5.0);
        }
        let to = c.wander_target - tf.translation;
        let flat = Vec3::new(to.x, 0.0, to.z);
        if flat.length() > 1.0 {
            let step = flat.normalize() * WANDER_SPEED * dt;
            tf.translation += step;
            tf.rotation = Quat::from_rotation_arc(Vec3::NEG_Z, flat.normalize());
        }
    }
}

/// Mark the nearest creature within reach of the avatar as engaged.
fn update_engaged(
    mut engaged: ResMut<Engaged>,
    avatar: Query<&Transform, With<Avatar>>,
    creatures: Query<(Entity, &Transform), With<Creature>>,
) {
    let Ok(a) = avatar.get_single() else {
        engaged.0 = None;
        return;
    };
    let mut best: Option<(f32, Entity)> = None;
    for (e, tf) in &creatures {
        let d = tf.translation.distance(a.translation);
        if d <= ENGAGE_RANGE && best.map_or(true, |(bd, _)| d < bd) {
            best = Some((d, e));
        }
    }
    engaged.0 = best.map(|(_, e)| e);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engage_range_picks_nearest_in_reach() {
        // pure check of the selection rule used in update_engaged
        let avatar = Vec3::ZERO;
        let candidates = [
            (Vec3::new(20.0, 0.0, 0.0), 1u32), // out of range
            (Vec3::new(5.0, 0.0, 0.0), 2u32),  // in range, nearer
            (Vec3::new(8.0, 0.0, 0.0), 3u32),  // in range, farther
        ];
        let mut best: Option<(f32, u32)> = None;
        for (p, id) in candidates {
            let d = p.distance(avatar);
            if d <= ENGAGE_RANGE && best.map_or(true, |(bd, _)| d < bd) {
                best = Some((d, id));
            }
        }
        assert_eq!(best.map(|(_, id)| id), Some(2));
    }
}
