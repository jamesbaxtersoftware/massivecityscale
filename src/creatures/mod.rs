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

/// `roll` is a damage multiplier in [0.85, 1.0]; pass 1.0 for deterministic tests.
pub fn damage(attacker: &Creature, defender: &Creature, mv: Move, roll: f32) -> i32 {
    let mult = type_multiplier(attacker.creature_type(), defender.creature_type());
    let base = mv.power as f32 * (attacker.attack as f32 / 10.0) * mult * roll;
    base.round().max(1.0) as i32
}

pub fn xp_reward(wild_level: u32) -> u32 { wild_level * 20 }
pub fn xp_to_next(level: u32) -> u32 { level * 50 }

pub fn apply_xp(c: &mut Creature, gained: u32) {
    c.xp += gained;
    while c.xp >= xp_to_next(c.level) {
        c.xp -= xp_to_next(c.level);
        c.level += 1;
        c.max_hp += 5;
        c.attack += 2;
    }
    c.hp = c.max_hp;
}

pub fn trap_chance(hp: i32, max_hp: i32, base: f32) -> f32 {
    let frac = (hp.max(0) as f32) / (max_hp.max(1) as f32);
    (base + (1.0 - frac)).clamp(0.0, 1.0)
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

    #[test]
    fn damage_scales_with_type_and_power() {
        let fire = Creature::new(Species::Emberling, 5);
        let water = Creature::new(Species::Tideling, 5);
        let strong = Move { name: "Torrent", power: 14 };
        // water -> fire is x2; deterministic roll = 1.0
        let d = damage(&water, &fire, strong, 1.0);
        // fire -> water is x0.5
        let d2 = damage(&fire, &water, Move { name: "Flare", power: 14 }, 1.0);
        assert!(d > d2, "super-effective should beat not-very-effective: {d} vs {d2}");
        assert!(d >= 1);
    }

    #[test]
    fn xp_levels_up_and_raises_stats() {
        let mut c = Creature::new(Species::Emberling, 1);
        let (hp0, atk0) = (c.max_hp, c.attack);
        apply_xp(&mut c, 1000);
        assert!(c.level > 1);
        assert!(c.max_hp > hp0 && c.attack > atk0);
        assert_eq!(c.hp, c.max_hp, "level-up heals to full");
    }

    #[test]
    fn trap_chance_rises_as_hp_falls_and_is_clamped() {
        let full = trap_chance(30, 30, 0.2);
        let low = trap_chance(1, 30, 0.2);
        assert!(low > full);
        assert!((0.0..=1.0).contains(&full));
        assert!((0.0..=1.0).contains(&low));
        assert_eq!(trap_chance(0, 30, 0.2), 1.0_f32.min(0.2 + 1.0).min(1.0));
    }
}

impl Plugin for CreaturesPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
           .init_resource::<Collection>()
           .init_resource::<Inventory>()
           .add_systems(Startup, spawn::spawn_wild_monsters.after(crate::renderer::solar::spawn_solar_system))
           .add_systems(OnEnter(GameState::StarterSelect), starter::spawn_starters)
           .add_systems(OnExit(GameState::StarterSelect), starter::despawn_starters)
           .add_systems(Update, starter::pick_starter.run_if(in_state(GameState::StarterSelect)))
           .add_systems(Update, pick::pick_wild_monster.run_if(in_state(GameState::Exploring)))
           .add_systems(OnEnter(GameState::Battle), battle::setup_battle)
           .add_systems(OnExit(GameState::Battle), battle::teardown_battle)
           .add_systems(Update, (
               battle_view::billboard_hp_bars,
               battle_view::update_hp_bars,
           ).run_if(in_state(GameState::Battle)));
    }
}
