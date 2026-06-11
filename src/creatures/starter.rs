use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use super::{CreatureType, Species, Creature, PlayerCreature, GameState, ClickSphere};
use super::spawn::{build_visual, spawn_creature_visual};
use super::pick::{ortho_pick_ray, ray_sphere_t};
use crate::camera::zoom::OrbitState;

#[derive(Component)]
pub struct StarterChoice(pub CreatureType);

#[derive(Component)]
pub struct StarterRoot;

const STARTER_SCALE: f32 = 60.0;

/// Spawn the two starter creatures floating in front of the camera.
pub fn spawn_starters(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    orbit: Res<OrbitState>,
) {
    let cam = orbit.camera_pos();
    let fwd = (orbit.pivot - cam).normalize();
    let right = fwd.cross(Vec3::Y).normalize();
    let mid = cam + fwd * (orbit.distance * 0.5);

    for (i, t) in [CreatureType::Fire, CreatureType::Water].into_iter().enumerate() {
        let side = if i == 0 { -1.0 } else { 1.0 };
        let pos = mid + right * side * STARTER_SCALE * 2.0;
        let vis = build_visual(t, STARTER_SCALE, &mut meshes, &mut materials);
        let root = commands.spawn((
            Transform::from_translation(pos),
            GlobalTransform::default(),
            Visibility::Visible,
            StarterChoice(t),
            StarterRoot,
            ClickSphere { radius: STARTER_SCALE * 1.5 },
        )).id();
        spawn_creature_visual(&mut commands, root, Transform::IDENTITY, &vis);
    }
}

pub fn pick_starter(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    proj_query: Query<&Projection, With<Camera3d>>,
    orbit: Res<OrbitState>,
    choices: Query<(&GlobalTransform, &ClickSphere, &StarterChoice)>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if !mouse_buttons.just_pressed(MouseButton::Left) { return; }
    let Ok(Projection::Orthographic(ortho)) = proj_query.get_single() else { return };
    let Ok(window) = windows.get_single() else { return };
    let Some((origin, fwd)) = ortho_pick_ray(window, ortho.scale, &orbit) else { return };

    let mut best: Option<(CreatureType, f32)> = None;
    for (gt, cs, choice) in &choices {
        if let Some(t) = ray_sphere_t(origin, fwd, gt.translation(), cs.radius) {
            if best.map_or(true, |(_, bt)| t < bt) { best = Some((choice.0, t)); }
        }
    }
    if let Some((t, _)) = best {
        commands.insert_resource(PlayerCreature(Creature::new(Species::of_type(t), 5)));
        next_state.set(GameState::Exploring);
    }
}

pub fn despawn_starters(mut commands: Commands, roots: Query<Entity, With<StarterRoot>>) {
    for e in &roots { commands.entity(e).despawn_recursive(); }
}
