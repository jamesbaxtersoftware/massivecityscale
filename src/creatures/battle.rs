use bevy::prelude::*;
use super::{BattleSession, PlayerCreature, Creature, WildMonster, Turn};
use super::spawn::{build_visual, spawn_creature_visual};
use super::battle_view::{spawn_hp_bar, BattleSide};
use crate::camera::zoom::OrbitState;

const BATTLE_CREATURE_SCALE: f32 = 40.0;

pub fn setup_battle(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut session: ResMut<BattleSession>,
    player: Res<PlayerCreature>,
    mut orbit: ResMut<OrbitState>,
    wild_q: Query<(&GlobalTransform, &Creature), With<WildMonster>>,
) {
    let Ok((wild_gt, wild)) = wild_q.get(session.wild_entity) else { return };
    let wild_pos = wild_gt.translation();

    // Frame the battle: pivot on the wild monster, fixed close distance.
    session.saved_orbit = Some(orbit.clone());
    orbit.pivot = wild_pos;
    orbit.distance = BATTLE_CREATURE_SCALE * 12.0;
    orbit.elevation = 0.2;

    session.wild_hp = wild.hp;
    session.player_hp = player.0.max_hp; // heal to full at battle start
    session.turn = Turn::Player;

    let cam = orbit.camera_pos();
    let fwd = (orbit.pivot - cam).normalize();
    let right = fwd.cross(Vec3::Y).normalize();
    let player_pos = wild_pos - fwd * BATTLE_CREATURE_SCALE * 4.0 + right * BATTLE_CREATURE_SCALE * 2.0;

    let vis = build_visual(player.0.creature_type(), BATTLE_CREATURE_SCALE, &mut meshes, &mut materials);
    let player_root = commands.spawn((
        Transform::from_translation(player_pos),
        GlobalTransform::default(),
        Visibility::Visible,
        BattleSide::Player,
    )).id();
    spawn_creature_visual(&mut commands, player_root, Transform::IDENTITY, &vis);
    session.view_entities.push(player_root);

    let pbar = spawn_hp_bar(&mut commands, &mut meshes, &mut materials,
        player_pos + Vec3::Y * BATTLE_CREATURE_SCALE * 2.0, BattleSide::Player);
    let wbar = spawn_hp_bar(&mut commands, &mut meshes, &mut materials,
        wild_pos + Vec3::Y * BATTLE_CREATURE_SCALE * 2.0, BattleSide::Wild);
    session.view_entities.push(pbar);
    session.view_entities.push(wbar);
}

pub fn teardown_battle(
    mut commands: Commands,
    mut session: ResMut<BattleSession>,
    mut orbit: ResMut<OrbitState>,
) {
    if let Some(saved) = session.saved_orbit.take() { *orbit = saved; }
    for e in session.view_entities.drain(..) {
        commands.entity(e).despawn_recursive();
    }
}
