//! Simple save/load of player progress (collection, items, stats, crystals) to a
//! plain-text file. F5 saves, F9 loads; a save auto-loads on startup. The
//! serialize/deserialize pair is pure and round-trip tested.

use bevy::prelude::*;
use std::collections::HashMap;
use crate::creatures::{CaughtCreature, Collection, CreatureKind, Inventory, PlayerStats};
use crate::hud::Wallet;

const SAVE_PATH: &str = "galaxy_roamer.save";

#[derive(Clone, PartialEq, Debug, Default)]
pub struct SaveData {
    pub level: u32,
    pub exp: u32,
    pub hp: f32,
    pub max_hp: f32,
    pub sp: f32,
    pub max_sp: f32,
    pub crystals: u32,
    pub disc: u32,
    pub bait: u32,
    pub heal: u32,
    pub revive: u32,
    pub flash: u32,
    pub antidote: u32,
    pub party: Vec<(CreatureKind, u32)>,
}

pub fn serialize(d: &SaveData) -> String {
    let party = d
        .party
        .iter()
        .map(|(k, l)| format!("{}:{}", k.name(), l))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "level={}\nexp={}\nhp={}\nmaxhp={}\nsp={}\nmaxsp={}\ncrystals={}\ndisc={}\nbait={}\nheal={}\nrevive={}\nflash={}\nantidote={}\nparty={}\n",
        d.level, d.exp, d.hp, d.max_hp, d.sp, d.max_sp, d.crystals, d.disc, d.bait, d.heal,
        d.revive, d.flash, d.antidote, party
    )
}

pub fn deserialize(s: &str) -> Option<SaveData> {
    let mut m: HashMap<&str, &str> = HashMap::new();
    for line in s.lines() {
        if let Some((k, v)) = line.split_once('=') {
            m.insert(k.trim(), v.trim());
        }
    }
    let u = |k: &str| m.get(k).and_then(|v| v.parse::<u32>().ok());
    let f = |k: &str| m.get(k).and_then(|v| v.parse::<f32>().ok());
    let party = m
        .get("party")
        .map(|p| {
            p.split(',')
                .filter(|e| !e.is_empty())
                .filter_map(|e| {
                    let (name, lvl) = e.split_once(':')?;
                    Some((CreatureKind::from_name(name)?, lvl.parse::<u32>().ok()?))
                })
                .collect()
        })
        .unwrap_or_default();
    Some(SaveData {
        level: u("level")?,
        exp: u("exp")?,
        hp: f("hp")?,
        max_hp: f("maxhp")?,
        sp: f("sp")?,
        max_sp: f("maxsp")?,
        crystals: u("crystals")?,
        disc: u("disc")?,
        bait: u("bait")?,
        heal: u("heal")?,
        revive: u("revive")?,
        flash: u("flash")?,
        antidote: u("antidote").unwrap_or(0),
        party,
    })
}

pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, auto_load)
            .add_systems(Update, (save_input, load_input));
    }
}

fn gather(stats: &PlayerStats, wallet: &Wallet, inv: &Inventory, col: &Collection) -> SaveData {
    SaveData {
        level: stats.level,
        exp: stats.exp,
        hp: stats.hp,
        max_hp: stats.max_hp,
        sp: stats.sp,
        max_sp: stats.max_sp,
        crystals: wallet.crystals,
        disc: inv.capture_disc,
        bait: inv.bait,
        heal: inv.heal_spray,
        revive: inv.revive,
        flash: inv.flash_bomb,
        antidote: inv.antidote,
        party: col.party.iter().map(|c| (c.kind, c.level)).collect(),
    }
}

fn apply(
    d: &SaveData,
    stats: &mut PlayerStats,
    wallet: &mut Wallet,
    inv: &mut Inventory,
    col: &mut Collection,
) {
    stats.level = d.level;
    stats.exp = d.exp;
    stats.hp = d.hp;
    stats.max_hp = d.max_hp;
    stats.sp = d.sp;
    stats.max_sp = d.max_sp;
    wallet.crystals = d.crystals;
    inv.capture_disc = d.disc;
    inv.bait = d.bait;
    inv.heal_spray = d.heal;
    inv.revive = d.revive;
    inv.flash_bomb = d.flash;
    inv.antidote = d.antidote;
    col.party = d.party.iter().map(|(k, l)| CaughtCreature { kind: *k, level: *l }).collect();
}

fn save_input(
    keys: Res<ButtonInput<KeyCode>>,
    stats: Res<PlayerStats>,
    wallet: Res<Wallet>,
    inv: Res<Inventory>,
    col: Res<Collection>,
) {
    if keys.just_pressed(KeyCode::F5) {
        let _ = std::fs::write(SAVE_PATH, serialize(&gather(&stats, &wallet, &inv, &col)));
        info!("game saved");
    }
}

fn load_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut stats: ResMut<PlayerStats>,
    mut wallet: ResMut<Wallet>,
    mut inv: ResMut<Inventory>,
    mut col: ResMut<Collection>,
) {
    if keys.just_pressed(KeyCode::F9) {
        if let Ok(s) = std::fs::read_to_string(SAVE_PATH) {
            if let Some(d) = deserialize(&s) {
                apply(&d, &mut stats, &mut wallet, &mut inv, &mut col);
                info!("game loaded");
            }
        }
    }
}

fn auto_load(
    mut stats: ResMut<PlayerStats>,
    mut wallet: ResMut<Wallet>,
    mut inv: ResMut<Inventory>,
    mut col: ResMut<Collection>,
) {
    if let Ok(s) = std::fs::read_to_string(SAVE_PATH) {
        if let Some(d) = deserialize(&s) {
            apply(&d, &mut stats, &mut wallet, &mut inv, &mut col);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_round_trips() {
        let d = SaveData {
            level: 7,
            exp: 120,
            hp: 56.0,
            max_hp: 62.0,
            sp: 37.0,
            max_sp: 39.0,
            crystals: 240,
            disc: 19,
            bait: 11,
            heal: 6,
            revive: 3,
            flash: 4,
            antidote: 5,
            party: vec![
                (CreatureKind::Flarehog, 12),
                (CreatureKind::Aquabud, 8),
            ],
        };
        let back = deserialize(&serialize(&d)).expect("parses");
        assert_eq!(back, d);
    }

    #[test]
    fn deserialize_rejects_garbage() {
        assert!(deserialize("not a save").is_none());
    }
}
