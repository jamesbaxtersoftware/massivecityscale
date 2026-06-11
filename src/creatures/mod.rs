pub mod spawn;
pub mod pick;
pub mod starter;
pub mod battle;
pub mod battle_view;

use bevy::prelude::*;
use bevy::ecs::schedule::IntoSystemConfigs;

// ── Types ──────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CreatureType { Fire, Water }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Species { Emberling, Tideling }

impl Species {
    pub fn creature_type(self) -> CreatureType {
        match self {
            Species::Emberling => CreatureType::Fire,
            Species::Tideling => CreatureType::Water,
        }
    }
    pub fn name(self) -> &'static str {
        match self { Species::Emberling => "Emberling", Species::Tideling => "Tideling" }
    }
    /// Two moves per species: [weak, strong].
    pub fn moves(self) -> [Move; 2] {
        match self {
            Species::Emberling => [
                Move { name: "Ember", power: 8 },
                Move { name: "Flare", power: 14 },
            ],
            Species::Tideling => [
                Move { name: "Splash", power: 8 },
                Move { name: "Torrent", power: 14 },
            ],
        }
    }
    pub fn base_max_hp(self) -> i32 { 30 }
    pub fn base_attack(self) -> i32 { 10 }
    pub fn of_type(t: CreatureType) -> Species {
        match t { CreatureType::Fire => Species::Emberling, CreatureType::Water => Species::Tideling }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Move { pub name: &'static str, pub power: i32 }

#[derive(Component, Clone, Debug)]
pub struct Creature {
    pub species: Species,
    pub level:   u32,
    pub xp:      u32,
    pub hp:      i32,
    pub max_hp:  i32,
    pub attack:  i32,
}

impl Creature {
    pub fn new(species: Species, level: u32) -> Self {
        // +5 max_hp and +2 attack per level above 1.
        let lv = level.max(1);
        let max_hp = species.base_max_hp() + (lv as i32 - 1) * 5;
        let attack = species.base_attack() + (lv as i32 - 1) * 2;
        Self { species, level: lv, xp: 0, hp: max_hp, max_hp, attack }
    }
    pub fn creature_type(&self) -> CreatureType { self.species.creature_type() }
    pub fn heal_full(&mut self) { self.hp = self.max_hp; }
    pub fn is_fainted(&self) -> bool { self.hp <= 0 }
}

// ── Components ──────────────────────────────────────────────────────────────────

/// Element a planet is themed as; added to each planet entity at spawn.
#[derive(Component, Clone, Copy, Debug)]
pub struct PlanetType(pub CreatureType);

/// Marks a wild (clickable, battleable) monster entity.
#[derive(Component)]
pub struct WildMonster;

/// Sphere used for click raycasting against non-planet entities.
#[derive(Component)]
pub struct ClickSphere { pub radius: f32 }

// ── Resources ───────────────────────────────────────────────────────────────────

#[derive(Resource)]
pub struct PlayerCreature(pub Creature);

#[derive(Resource, Default)]
pub struct Collection(pub Vec<Creature>);

#[derive(Resource)]
pub struct Inventory { pub force_fields: u32 }

impl Default for Inventory {
    fn default() -> Self { Self { force_fields: 3 } }
}

// ── Battle session ───────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Turn { Player, Enemy }

#[derive(Resource)]
pub struct BattleSession {
    pub wild_entity: Entity,
    pub turn: Turn,
    /// Entities spawned only for the battle view (player creature, HP bars),
    /// despawned on teardown.
    pub view_entities: Vec<Entity>,
    /// Snapshot of camera framing to restore when the battle ends.
    pub saved_orbit: Option<crate::camera::zoom::OrbitState>,
    pub player_hp: i32,
    pub wild_hp: i32,
}

impl BattleSession {
    pub fn new(wild_entity: Entity) -> Self {
        Self {
            wild_entity,
            turn: Turn::Player,
            view_entities: Vec::new(),
            saved_orbit: None,
            player_hp: 0,
            wild_hp: 0,
        }
    }
}

// ── State ────────────────────────────────────────────────────────────────────────

#[derive(States, Default, Clone, Eq, PartialEq, Hash, Debug)]
pub enum GameState {
    #[default]
    StarterSelect,
    Exploring,
    Battle,
}

// ── Plugin ───────────────────────────────────────────────────────────────────────

pub struct CreaturesPlugin;

// ── Free functions ───────────────────────────────────────────────────────────────

pub fn type_multiplier(attacker: CreatureType, defender: CreatureType) -> f32 {
    use CreatureType::*;
    match (attacker, defender) {
        (Water, Fire) => 2.0,
        (Fire, Water) => 0.5,
        _ => 1.0,
    }
}

/// Classify a planet color: red channel >= blue channel -> Fire, else Water.
pub fn type_from_color(c: Color) -> CreatureType {
    let s = c.to_srgba();
    if s.red >= s.blue { CreatureType::Fire } else { CreatureType::Water }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn water_beats_fire_fire_weak_to_water() {
        assert_eq!(type_multiplier(CreatureType::Water, CreatureType::Fire), 2.0);
        assert_eq!(type_multiplier(CreatureType::Fire, CreatureType::Water), 0.5);
    }

    #[test]
    fn same_type_is_neutral() {
        assert_eq!(type_multiplier(CreatureType::Fire, CreatureType::Fire), 1.0);
        assert_eq!(type_multiplier(CreatureType::Water, CreatureType::Water), 1.0);
    }

    #[test]
    fn warm_colors_are_fire_cool_are_water() {
        // lava orange
        assert_eq!(type_from_color(Color::srgb(0.95, 0.28, 0.05)), CreatureType::Fire);
        // ocean blue
        assert_eq!(type_from_color(Color::srgb(0.10, 0.38, 0.90)), CreatureType::Water);
        // ice cyan (blue dominant)
        assert_eq!(type_from_color(Color::srgb(0.35, 0.82, 0.88)), CreatureType::Water);
    }
}

impl Plugin for CreaturesPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
           .init_resource::<Collection>()
           .init_resource::<Inventory>()
           .add_systems(Startup, spawn::spawn_wild_monsters.after(crate::renderer::solar::spawn_solar_system));
    }
}
