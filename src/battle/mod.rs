//! Turn-based battles. Engage a wild creature (ENTER when in reach) to enter a
//! Battle: each turn you [1] Attack, [2] throw a Capture Disc, or [3] Flee; the
//! creature retaliates. Weakening it raises the capture chance. Win by capturing.

use bevy::prelude::*;
use crate::creatures::{
    capture_chance, effectiveness, gain_exp, CaughtCreature, Collection, Creature, CreatureKind,
    Engaged, Inventory, PlayerStats, CAPTURE_REWARD,
};
use crate::hud::Wallet;
use crate::onfoot::Mode;

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Phase {
    #[default]
    Roam,
    Battle,
}

/// The active battle.
#[derive(Resource, Default)]
pub struct Battle {
    pub enemy: Option<Entity>,
    pub kind: Option<CreatureKind>,
    pub level: u32,
    pub hp: f32,
    pub max_hp: f32,
    /// Extra capture chance from Bait, this battle.
    pub bait_bonus: f32,
}

pub struct BattlePlugin;

impl Plugin for BattlePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<Phase>()
            .init_resource::<Battle>()
            .add_systems(
                Update,
                start_battle
                    .run_if(in_state(Mode::OnFoot))
                    .run_if(in_state(Phase::Roam)),
            )
            .add_systems(Update, battle_input.run_if(in_state(Phase::Battle)))
            .add_systems(OnEnter(Phase::Battle), spawn_battle_ui)
            .add_systems(OnExit(Phase::Battle), despawn_battle_ui)
            .add_systems(Update, update_battle_ui.run_if(in_state(Phase::Battle)));
    }
}

/// Damage the player's strike deals at a given level.
pub fn player_attack_damage(level: u32) -> f32 {
    10.0 + level as f32 * 3.0
}
/// Damage a wild creature deals back at a given level.
pub fn enemy_attack_damage(level: u32) -> f32 {
    4.0 + level as f32 * 1.5
}

fn start_battle(
    keys: Res<ButtonInput<KeyCode>>,
    engaged: Res<Engaged>,
    creatures: Query<&Creature>,
    mut battle: ResMut<Battle>,
    mut next: ResMut<NextState<Phase>>,
) {
    if !keys.just_pressed(KeyCode::Enter) {
        return;
    }
    let Some((e, c)) = engaged.0.and_then(|e| creatures.get(e).ok().map(|c| (e, c))) else {
        return;
    };
    *battle = Battle {
        enemy: Some(e),
        kind: Some(c.kind),
        level: c.level,
        hp: c.hp,
        max_hp: c.max_hp,
        bait_bonus: 0.0,
    };
    next.set(Phase::Battle);
}

// ── Battle message log (shown in the UI) ─────────────────────────────────────
#[derive(Resource, Default)]
struct BattleLog(String);

#[allow(clippy::too_many_arguments)]
fn battle_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut battle: ResMut<Battle>,
    mut stats: ResMut<PlayerStats>,
    mut inv: ResMut<Inventory>,
    mut collection: ResMut<Collection>,
    mut wallet: ResMut<Wallet>,
    mut log: ResMut<BattleLog>,
    mut next: ResMut<NextState<Phase>>,
    mut commands: Commands,
) {
    let kind = battle.kind.unwrap_or(CreatureKind::Grasshog);
    let mut acted = false;

    if keys.just_pressed(KeyCode::Digit1) {
        // Attack — the lead party creature's element decides effectiveness.
        let eff = collection
            .party
            .first()
            .map(|lead| effectiveness(lead.kind.element(), kind.element()))
            .unwrap_or(1.0);
        let dmg = player_attack_damage(stats.level) * eff;
        battle.hp = (battle.hp - dmg).max(0.0);
        let tag = match eff {
            e if e > 1.0 => "  It's super effective!",
            e if e < 1.0 => "  It's not very effective...",
            _ => "",
        };
        log.0 = format!("You strike for {dmg:.0}!{tag}");
        acted = true;
    } else if keys.just_pressed(KeyCode::Digit3) {
        // Heal Spray.
        if inv.heal_spray == 0 {
            log.0 = "No Heal Spray left!".into();
            return;
        }
        inv.heal_spray -= 1;
        stats.hp = (stats.hp + 25.0).min(stats.max_hp);
        log.0 = "You spray on a heal (+25 HP).".into();
        acted = true;
    } else if keys.just_pressed(KeyCode::Digit4) {
        // Bait — raises capture chance for the rest of the battle.
        if inv.bait == 0 {
            log.0 = "No Bait left!".into();
            return;
        }
        inv.bait -= 1;
        battle.bait_bonus = (battle.bait_bonus + 0.15).min(0.45);
        log.0 = format!("You toss Bait — the {kind:?} is intrigued.");
        acted = true;
    } else if keys.just_pressed(KeyCode::Digit2) {
        // Throw Capture Disc.
        if inv.capture_disc == 0 {
            log.0 = "Out of Capture Discs!".into();
            return;
        }
        inv.capture_disc -= 1;
        let chance =
            (capture_chance(battle.hp, battle.max_hp, battle.level) + battle.bait_bonus).clamp(0.0, 1.0);
        if rand::random::<f32>() < chance {
            if let Some(e) = battle.enemy {
                commands.entity(e).despawn();
            }
            collection.party.push(CaughtCreature { kind, level: battle.level });
            gain_exp(&mut stats, battle.level * 20);
            wallet.crystals += CAPTURE_REWARD;
            log.0 = format!("Gotcha! {kind:?} was caught!");
            next.set(Phase::Roam);
            return;
        }
        log.0 = format!("Aw, the {kind:?} broke free!");
        acted = true;
    } else if keys.just_pressed(KeyCode::Digit5) {
        log.0 = "You got away safely.".into();
        next.set(Phase::Roam);
        return;
    }

    if !acted {
        return;
    }

    // Enemy fainted from the attack (no capture).
    if battle.hp <= 0.0 {
        if let Some(e) = battle.enemy {
            commands.entity(e).despawn();
        }
        gain_exp(&mut stats, battle.level * 8);
        log.0 = format!("The {kind:?} fled, exhausted.");
        next.set(Phase::Roam);
        return;
    }

    // Enemy's turn.
    let edmg = enemy_attack_damage(battle.level);
    stats.hp = (stats.hp - edmg).max(0.0);
    log.0 = format!("{}  The wild {kind:?} hits back for {edmg:.0}!", log.0);
    if stats.hp <= 0.0 {
        stats.hp = stats.max_hp * 0.5;
        log.0 = "You were overwhelmed — retreated to safety!".into();
        next.set(Phase::Roam);
    }
}

// ── Battle UI ────────────────────────────────────────────────────────────────
#[derive(Component)]
struct BattleUi;
#[derive(Component)]
struct EnemyLine;
#[derive(Component)]
struct LogLine;
#[derive(Component)]
struct PlayerLine;
#[derive(Component)]
struct MenuLine;

fn spawn_battle_ui(mut commands: Commands) {
    commands.init_resource::<BattleLog>();
    commands
        .spawn((
            BattleUi,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::all(Val::Px(40.0)),
                ..default()
            },
        ))
        .with_children(|c| {
            // Enemy (top).
            c.spawn((
                EnemyLine,
                Text::new(""),
                TextFont { font_size: 26.0, ..default() },
                TextColor(Color::srgb(1.0, 0.9, 0.85)),
            ));
            // Message + menu (bottom block).
            c.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                ..default()
            })
            .with_children(|b| {
                b.spawn((
                    LogLine,
                    Text::new(""),
                    TextFont { font_size: 22.0, ..default() },
                    TextColor(Color::srgb(0.95, 0.97, 1.0)),
                ));
                b.spawn((
                    PlayerLine,
                    Text::new(""),
                    TextFont { font_size: 18.0, ..default() },
                    TextColor(Color::srgb(0.7, 0.85, 1.0)),
                ));
                b.spawn((
                    MenuLine,
                    Text::new(""),
                    TextFont { font_size: 22.0, ..default() },
                    TextColor(Color::srgb(1.0, 0.9, 0.3)),
                ));
            });
        });
}

fn despawn_battle_ui(mut commands: Commands, q: Query<Entity, With<BattleUi>>) {
    for e in &q {
        commands.entity(e).despawn_recursive();
    }
}

#[allow(clippy::type_complexity)]
fn update_battle_ui(
    battle: Res<Battle>,
    stats: Res<PlayerStats>,
    inv: Res<Inventory>,
    log: Res<BattleLog>,
    mut q: ParamSet<(
        Query<&mut Text, With<EnemyLine>>,
        Query<&mut Text, With<LogLine>>,
        Query<&mut Text, With<PlayerLine>>,
        Query<&mut Text, With<MenuLine>>,
    )>,
) {
    let kind = battle.kind.unwrap_or(CreatureKind::Grasshog);
    if let Ok(mut t) = q.p0().get_single_mut() {
        t.0 = format!(
            "Wild {kind:?}   Lv{}\nHP {:.0}/{:.0}",
            battle.level, battle.hp, battle.max_hp
        );
    }
    if let Ok(mut t) = q.p1().get_single_mut() {
        t.0 = log.0.clone();
    }
    if let Ok(mut t) = q.p2().get_single_mut() {
        t.0 = format!("YOU   HP {:.0}/{:.0}   SP {:.0}/{:.0}", stats.hp, stats.max_hp, stats.sp, stats.max_sp);
    }
    if let Ok(mut t) = q.p3().get_single_mut() {
        t.0 = format!(
            "[1] Attack    [2] Capture Disc ({})    [3] Heal ({})    [4] Bait ({})    [5] Flee",
            inv.capture_disc, inv.heal_spray, inv.bait
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attack_scales_with_level_and_enemy_hits_back() {
        assert!(player_attack_damage(10) > player_attack_damage(1));
        assert!(enemy_attack_damage(10) > enemy_attack_damage(1));
        assert!(player_attack_damage(5) > 0.0 && enemy_attack_damage(5) > 0.0);
    }
}
