use bevy::prelude::*;
use rand::Rng;
use rand_chacha::ChaCha8Rng;
use super::{BattleSession, PlayerCreature, Creature, WildMonster, Turn, Inventory, Collection, GameState, apply_xp, xp_reward, damage, trap_chance};
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

#[derive(Resource)]
pub struct BattleRng(pub ChaCha8Rng);

/// Player turn: keys 1/2 attack with moves[0]/[1]; F throws a force field.
pub fn battle_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut session: ResMut<BattleSession>,
    mut player: ResMut<PlayerCreature>,
    mut inventory: ResMut<Inventory>,
    mut collection: ResMut<Collection>,
    mut wild_q: Query<&mut Creature, With<WildMonster>>,
    mut commands: Commands,
    mut rng: ResMut<BattleRng>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if session.turn != Turn::Player { return; }
    let Ok(mut wild) = wild_q.get_mut(session.wild_entity) else { return };

    let move_idx = if keys.just_pressed(KeyCode::Digit1) { Some(0) }
                   else if keys.just_pressed(KeyCode::Digit2) { Some(1) }
                   else { None };
    let throw = keys.just_pressed(KeyCode::KeyF);

    if let Some(idx) = move_idx {
        let mv = player.0.species.moves()[idx];
        let roll = rng.0.gen_range(0.85_f32..=1.0);
        let dmg = damage(&player.0, &wild, mv, roll);
        session.wild_hp -= dmg;
        wild.hp = session.wild_hp;
        if session.wild_hp <= 0 {
            apply_xp(&mut player.0, xp_reward(wild.level));
            commands.entity(session.wild_entity).despawn_recursive();
            next_state.set(GameState::Exploring);
            return;
        }
        session.turn = Turn::Enemy;
    } else if throw {
        if inventory.force_fields == 0 { return; }
        inventory.force_fields -= 1;
        let p = trap_chance(session.wild_hp, wild.max_hp, 0.2);
        if rng.0.gen_range(0.0_f32..1.0) < p {
            let mut caught = wild.clone();
            caught.hp = caught.max_hp;
            collection.0.push(caught);
            commands.entity(session.wild_entity).despawn_recursive();
            player.0.heal_full();
            next_state.set(GameState::Exploring);
            return;
        }
        session.turn = Turn::Enemy; // failed throw wastes the turn
    }
}

/// Enemy turn: the wild monster attacks with a random move.
pub fn enemy_turn(
    mut session: ResMut<BattleSession>,
    mut player: ResMut<PlayerCreature>,
    wild_q: Query<&Creature, With<WildMonster>>,
    mut rng: ResMut<BattleRng>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if session.turn != Turn::Enemy { return; }
    let Ok(wild) = wild_q.get(session.wild_entity) else { return };
    let mv = wild.species.moves()[rng.0.gen_range(0..2)];
    let roll = rng.0.gen_range(0.85_f32..=1.0);
    let dmg = damage(wild, &player.0, mv, roll);
    session.player_hp -= dmg;
    if session.player_hp <= 0 {
        player.0.heal_full();
        next_state.set(GameState::Exploring);
        return;
    }
    session.turn = Turn::Player;
}
