use std::collections::{HashMap, HashSet};

use serde::Deserialize;

use crate::wheel::Element;

/// Who/what a card resolves against. Simplified for the 1-human-vs-1-bot
/// MVP: there is always exactly one enemy, so no player index is needed.
/// Revisit when N-player support is added (AllEnemies/AllPlayers/etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind {
    SelfTarget,
    Enemy,
    /// Targets a card in the caster's own hand, chosen at play time (e.g.
    /// a future "sell" card). Not used by any MVP card yet.
    HandCard,
}

/// Composable stats: combo cards attach to a base card and their atk/def
/// simply sum. This is genuine addition, so there's no dispatch here --
/// the resolver in game.rs just sums the `combat` field across every
/// card in the attack/defense.
#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(default)]
pub struct CombatStats {
    pub atk: Option<i32>,
    pub def: Option<i32>,
}

/// One-off effects, dispatched by tag rather than composed. MVP set only
/// -- Afflict/status-effects are cut for now (see structure.md), since
/// they'd need a status-effect store on Player that isn't needed for a
/// dumb-bot MVP loop.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CardEffect {
    RestoreHealth { amount: u8 },
    RestoreMana { amount: u8 },
    RestoreMoney { amount: u8 },
    /// Direct, non-combat damage (a spell, not a weapon swing). Distinct
    /// from `CombatStats::atk`, which goes through the attack/defense
    /// exchange instead.
    DealDamage { amount: u8 },
    HealOrDamage {
        heal_chance: f32,
        heal_amount: u8,
        damage_amount: u8,
    },
}

fn default_true() -> bool {
    true
}
fn default_count() -> u32 {
    1
}

#[derive(Debug, Clone, Deserialize)]
pub struct CardDefinition {
    /// Filled in from the TOML table key when the mod is loaded --
    /// never present in the TOML itself.
    #[serde(skip)]
    pub id: String,
    #[serde(default)]
    pub target: Option<TargetKind>,
    #[serde(default)]
    pub combat: Option<CombatStats>,
    #[serde(default)]
    pub combo: bool,
    #[serde(default)]
    pub effect: Option<CardEffect>,
    #[serde(default = "default_true")]
    pub playable: bool,
    #[serde(default)]
    pub persistent: bool,
    #[serde(default)]
    pub price: u8,
    #[serde(default)]
    pub mana_cost: u8,
    #[serde(default)]
    pub elements: HashSet<Element>,
    /// String-keyed hook table (structure.md's "events" design). Only
    /// "on_death" is fired anywhere in the MVP engine.
    #[serde(default)]
    pub hooks: HashMap<String, Vec<CardEffect>>,
    /// How many copies of this card are in the single base pool. Multiple
    /// named deck categories (structure.md's `DeckCategory`) are deferred
    /// until something actually needs a second pool.
    #[serde(default = "default_count")]
    pub count: u32,
}

impl CardDefinition {
    /// True if this card can be played as an attack on your own turn.
    pub fn is_attack(&self) -> bool {
        self.playable
            && self
                .combat
                .and_then(|c| c.atk)
                .map_or(false, |atk| atk > 0)
    }

    /// True if this card can be played reactively as a defense.
    pub fn is_defense(&self) -> bool {
        self.playable
            && self
                .combat
                .and_then(|c| c.def)
                .map_or(false, |def| def > 0)
    }

    /// True if this card can be played on your own turn for its
    /// non-combat effect (heals, potions, etc).
    pub fn is_effect_card(&self) -> bool {
        self.playable && self.effect.is_some()
    }

    // todo: localization / art lookups live on CardRegistry, not here,
    // since they depend on the loaded locale/asset tables.
}
