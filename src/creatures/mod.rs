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

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Element {
    Grass,
    Water,
    Rock,
    Fire,
}

impl Element {
    /// The lingering ailment this element inflicts (flavour for a shared
    /// damage-over-time mechanic).
    pub fn ailment_label(self) -> &'static str {
        match self {
            Element::Grass => "Poisoned",
            Element::Water => "Drenched",
            Element::Rock => "Cracked",
            Element::Fire => "Burned",
        }
    }
}

/// Damage multiplier when `atk` hits `def` (classic triangle + rock).
pub fn effectiveness(atk: Element, def: Element) -> f32 {
    use Element::*;
    match (atk, def) {
        (Fire, Grass) | (Water, Fire) | (Water, Rock) | (Grass, Water) | (Grass, Rock)
        | (Rock, Fire) => 2.0,
        (Grass, Fire) | (Fire, Water) | (Fire, Rock) | (Water, Grass) | (Rock, Water)
        | (Rock, Grass) => 0.5,
        _ => 1.0,
    }
}

/// A battle move: display name, base power, SP cost, damage element, and whether
/// it inflicts that element's lingering ailment (damage over the next few turns).
#[derive(Clone, Copy)]
pub struct Move {
    pub name: &'static str,
    pub power: f32,
    pub sp_cost: u32,
    pub element: Element,
    pub ailment: bool,
}

impl CreatureKind {
    /// The lead creature's move set: a free typed Tackle, an SP special, and an
    /// SP ultimate that inflicts the element's ailment.
    pub fn moves(self) -> &'static [Move] {
        use Element::*;
        match self {
            CreatureKind::Grasshog => &[
                Move { name: "Tackle", power: 10.0, sp_cost: 0, element: Grass, ailment: false },
                Move { name: "Vine Lash", power: 18.0, sp_cost: 6, element: Grass, ailment: false },
                Move { name: "Leaf Storm", power: 26.0, sp_cost: 12, element: Grass, ailment: true },
            ],
            CreatureKind::Aquabud => &[
                Move { name: "Tackle", power: 10.0, sp_cost: 0, element: Water, ailment: false },
                Move { name: "Bubble", power: 18.0, sp_cost: 6, element: Water, ailment: false },
                Move { name: "Torrent", power: 26.0, sp_cost: 12, element: Water, ailment: true },
            ],
            CreatureKind::Rockfang => &[
                Move { name: "Tackle", power: 10.0, sp_cost: 0, element: Rock, ailment: false },
                Move { name: "Rock Throw", power: 18.0, sp_cost: 6, element: Rock, ailment: false },
                Move { name: "Boulder", power: 28.0, sp_cost: 13, element: Rock, ailment: true },
            ],
            CreatureKind::Flarehog => &[
                Move { name: "Tackle", power: 10.0, sp_cost: 0, element: Fire, ailment: false },
                Move { name: "Ember", power: 18.0, sp_cost: 6, element: Fire, ailment: false },
                Move { name: "Flamethrow", power: 26.0, sp_cost: 12, element: Fire, ailment: true },
            ],
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            CreatureKind::Grasshog => "Grasshog",
            CreatureKind::Aquabud => "Aquabud",
            CreatureKind::Rockfang => "Rockfang",
            CreatureKind::Flarehog => "Flarehog",
        }
    }
    pub fn from_name(s: &str) -> Option<Self> {
        match s {
            "Grasshog" => Some(CreatureKind::Grasshog),
            "Aquabud" => Some(CreatureKind::Aquabud),
            "Rockfang" => Some(CreatureKind::Rockfang),
            "Flarehog" => Some(CreatureKind::Flarehog),
            _ => None,
        }
    }
    pub fn element(self) -> Element {
        match self {
            CreatureKind::Grasshog => Element::Grass,
            CreatureKind::Aquabud => Element::Water,
            CreatureKind::Rockfang => Element::Rock,
            CreatureKind::Flarehog => Element::Fire,
        }
    }
    pub fn color(self) -> Color {
        match self {
            CreatureKind::Grasshog => Color::srgb(0.4, 0.7, 0.3),
            CreatureKind::Aquabud => Color::srgb(0.3, 0.6, 0.95),
            CreatureKind::Rockfang => Color::srgb(0.55, 0.5, 0.45),
            CreatureKind::Flarehog => Color::srgb(0.9, 0.45, 0.2),
        }
    }
    /// Weighted pool for a biome, so creatures suit the world you land on.
    fn biome_pool(biome: crate::onfoot::Biome) -> &'static [CreatureKind] {
        use crate::onfoot::Biome::*;
        use CreatureKind::*;
        match biome {
            Grass => &[Grasshog, Grasshog, Grasshog, Aquabud, Flarehog],
            Desert => &[Rockfang, Rockfang, Flarehog, Flarehog, Grasshog],
            Tundra => &[Aquabud, Aquabud, Aquabud, Rockfang, Grasshog],
            Volcanic => &[Flarehog, Flarehog, Flarehog, Rockfang, Aquabud],
            Alien => &[Grasshog, Aquabud, Rockfang, Flarehog],
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

/// On-foot consumables (reference starting counts).
#[derive(Resource)]
pub struct Inventory {
    pub capture_disc: u32,
    pub bait: u32,
    pub heal_spray: u32,
    pub revive: u32,
    pub flash_bomb: u32,
}
impl Default for Inventory {
    fn default() -> Self {
        Self { capture_disc: 23, bait: 12, heal_spray: 8, revive: 3, flash_bomb: 6 }
    }
}

/// A captured creature in the player's collection/party.
#[derive(Clone, Copy)]
pub struct CaughtCreature {
    pub kind: CreatureKind,
    pub level: u32,
}

/// The captured creatures (party head = active lead, shown beside you in battle).
#[derive(Resource)]
pub struct Collection {
    pub party: Vec<CaughtCreature>,
}
impl Default for Collection {
    fn default() -> Self {
        // Start with a Grasshog buddy so battles always have a lead creature.
        Self { party: vec![CaughtCreature { kind: CreatureKind::Grasshog, level: 5 }] }
    }
}

/// Player progression.
#[derive(Resource)]
pub struct PlayerStats {
    pub level: u32,
    pub exp: u32,
    pub hp: f32,
    pub max_hp: f32,
    pub sp: f32,
    pub max_sp: f32,
}
impl Default for PlayerStats {
    fn default() -> Self {
        Self { level: 1, exp: 0, hp: 50.0, max_hp: 50.0, sp: 30.0, max_sp: 30.0 }
    }
}

/// EXP needed to advance from `level` to the next.
pub fn exp_to_next(level: u32) -> u32 {
    50 + level * 50
}

/// Apply EXP, leveling up (carrying overflow) and growing stats. Returns the
/// number of levels gained so callers can surface a level-up notice.
pub fn gain_exp(stats: &mut PlayerStats, amount: u32) -> u32 {
    stats.exp += amount;
    let mut levels = 0;
    while stats.exp >= exp_to_next(stats.level) {
        stats.exp -= exp_to_next(stats.level);
        stats.level += 1;
        stats.max_hp += 6.0;
        stats.hp = stats.max_hp;
        stats.max_sp += 3.0;
        stats.sp = stats.max_sp;
        levels += 1;
    }
    levels
}

/// Crystals reward per successful capture.
pub const CAPTURE_REWARD: u32 = 40;

/// Capture probability 0..1: easier the more weakened and the lower the level.
pub fn capture_chance(hp: f32, max_hp: f32, level: u32) -> f32 {
    let weakened = (1.0 - hp / max_hp.max(1.0)).clamp(0.0, 1.0);
    let level_factor = (1.0 - level as f32 / 40.0).clamp(0.25, 1.0);
    ((0.08 + 0.72 * weakened) * level_factor).clamp(0.0, 1.0)
}

pub struct CreaturesPlugin;

impl Plugin for CreaturesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Engaged>()
            .init_resource::<Inventory>()
            .init_resource::<Collection>()
            .init_resource::<PlayerStats>()
            .add_systems(OnEnter(Mode::OnFoot), spawn_creatures)
            .add_systems(OnExit(Mode::OnFoot), despawn_creatures)
            // Roaming pauses during a battle.
            .add_systems(
                Update,
                (wander, update_engaged)
                    .run_if(in_state(Mode::OnFoot))
                    .run_if(in_state(crate::battle::Phase::Roam)),
            );
    }
}

/// Build the multi-part creature model (body + head + eyes + horn + 4 legs) as
/// children of the current entity, tinted to `color` on `layer`. The model faces
/// -Z. Shared by wild spawns and the player's creature in battle so both match.
pub fn build_creature_children(
    cr: &mut ChildBuilder,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    color: Color,
    layer: &RenderLayers,
) {
    let body_mesh = meshes.add(Sphere::new(0.6).mesh().ico(3).unwrap());
    let head_mesh = meshes.add(Sphere::new(0.42).mesh().ico(3).unwrap());
    let eye_mesh = meshes.add(Sphere::new(0.1).mesh().ico(2).unwrap());
    let pupil_mesh = meshes.add(Sphere::new(0.05).mesh().ico(1).unwrap());
    let leg_mesh = meshes.add(Cuboid::new(0.18, 0.35, 0.18));
    let horn_mesh = meshes.add(Cone { radius: 0.14, height: 0.4 });
    let body_mat = materials.add(StandardMaterial { base_color: color, ..default() });
    let belly_mat = materials.add(StandardMaterial {
        base_color: color.mix(&Color::WHITE, 0.35),
        ..default()
    });
    let eye_mat = materials.add(StandardMaterial { base_color: Color::WHITE, ..default() });
    let pupil_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.05, 0.05, 0.08),
        ..default()
    });

    // Body (slightly squashed) + head (front-up).
    cr.spawn((
        layer.clone(),
        Mesh3d(body_mesh),
        MeshMaterial3d(body_mat.clone()),
        Transform::from_xyz(0.0, 0.55, 0.0).with_scale(Vec3::new(1.0, 0.85, 1.15)),
    ));
    cr.spawn((
        layer.clone(),
        Mesh3d(head_mesh),
        MeshMaterial3d(belly_mat),
        Transform::from_xyz(0.0, 0.95, -0.45),
    ));
    // Eyes (with pupils) on the front of the head.
    for sx in [-1.0_f32, 1.0] {
        cr.spawn((
            layer.clone(),
            Mesh3d(eye_mesh.clone()),
            MeshMaterial3d(eye_mat.clone()),
            Transform::from_xyz(sx * 0.16, 1.02, -0.78),
        ));
        cr.spawn((
            layer.clone(),
            Mesh3d(pupil_mesh.clone()),
            MeshMaterial3d(pupil_mat.clone()),
            Transform::from_xyz(sx * 0.16, 1.02, -0.84),
        ));
    }
    // Horn (a little flair).
    cr.spawn((
        layer.clone(),
        Mesh3d(horn_mesh),
        MeshMaterial3d(body_mat.clone()),
        Transform::from_xyz(0.0, 1.35, -0.4),
    ));
    // Four stubby legs.
    for (sx, sz) in [(-1.0_f32, -1.0_f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        cr.spawn((
            layer.clone(),
            Mesh3d(leg_mesh.clone()),
            MeshMaterial3d(body_mat.clone()),
            Transform::from_xyz(sx * 0.32, 0.18, sz * 0.3),
        ));
    }
}

fn spawn_creatures(
    mut commands: Commands,
    biome: Res<crate::onfoot::LandedBiome>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let pool = CreatureKind::biome_pool(biome.0);
    let layer = RenderLayers::layer(SURFACE_LAYER);
    let mut rng = rand::thread_rng();
    for _ in 0..COUNT {
        let kind = pool[rng.gen_range(0..pool.len())];
        let level = rng.gen_range(2..18);
        let max_hp = 20.0 + level as f32 * 4.0;
        let (cx, cz) = (rng.gen_range(-FIELD..FIELD), rng.gen_range(-FIELD..FIELD));
        let pos = Vec3::new(cx, crate::onfoot::surface_height(cx, cz), cz);
        commands
            .spawn((
                Creature {
                    kind,
                    level,
                    hp: max_hp,
                    max_hp,
                    wander_target: pos,
                    wander_timer: rng.gen_range(0.0..3.0),
                },
                layer.clone(),
                Transform::from_translation(pos),
                Visibility::default(),
            ))
            .with_children(|cr| {
                build_creature_children(cr, &mut meshes, &mut materials, kind.color(), &layer);
            });
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
            tf.translation.y = crate::onfoot::surface_height(tf.translation.x, tf.translation.z);
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

    #[test]
    fn capture_chance_rises_as_weakened_and_is_bounded() {
        let full = capture_chance(40.0, 40.0, 5);
        let weak = capture_chance(2.0, 40.0, 5);
        assert!(weak > full, "weaker creature is easier to catch");
        for hp in [40.0, 20.0, 0.0] {
            let c = capture_chance(hp, 40.0, 5);
            assert!((0.0..=1.0).contains(&c), "chance stays in 0..1");
        }
    }

    #[test]
    fn capture_chance_drops_with_level() {
        let low = capture_chance(10.0, 40.0, 3);
        let high = capture_chance(10.0, 40.0, 35);
        assert!(low > high, "higher-level creatures are harder to catch");
    }

    #[test]
    fn type_effectiveness_triangle() {
        assert_eq!(effectiveness(Element::Fire, Element::Grass), 2.0);
        assert_eq!(effectiveness(Element::Grass, Element::Fire), 0.5);
        assert_eq!(effectiveness(Element::Fire, Element::Fire), 1.0);
        assert_eq!(effectiveness(Element::Water, Element::Fire), 2.0);
    }

    #[test]
    fn every_creature_has_a_free_basic_move() {
        for k in [
            CreatureKind::Grasshog,
            CreatureKind::Aquabud,
            CreatureKind::Rockfang,
            CreatureKind::Flarehog,
        ] {
            let mvs = k.moves();
            assert!(!mvs.is_empty(), "{k:?} has moves");
            assert_eq!(mvs[0].sp_cost, 0, "{k:?}'s first move is free");
            assert!(mvs.iter().any(|m| m.sp_cost > 0), "{k:?} has an SP special");
        }
    }

    #[test]
    fn gain_exp_levels_up_and_carries_overflow() {
        let mut s = PlayerStats::default();
        let start_hp = s.max_hp;
        // Enough to cross at least one level boundary with leftover.
        gain_exp(&mut s, exp_to_next(1) + 10);
        assert_eq!(s.level, 2, "leveled up");
        assert_eq!(s.exp, 10, "overflow carried");
        assert!(s.max_hp > start_hp, "stats grew");
    }
}
