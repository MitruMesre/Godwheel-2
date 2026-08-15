use std::collections::{HashMap, HashSet};

use serde::Deserialize;

use crate::wheel::Element;

/// A *hint* for what a bot should default to targeting with this card --
/// nothing more. Humans are always free to target either side with any
/// playable card (attack yourself, "heal" an enemy, etc. -- all
/// intended). Only the bot AI reads this field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind {
    SelfTarget,
    Enemy,
}

/// Composable stats: combo cards attach to a base card and their atk/def
/// simply sum. `atk` is signed -- positive is a weapon/spell (damage),
/// negative is a heal. There is no separate "heal" mechanic: a heal is
/// just an attack with negative atk, aimed at yourself (or, if you're
/// feeling generous/mean, at someone else). Composition is genuine
/// addition, so there's no dispatch here -- the resolver in game.rs
/// just sums `combat` across every card in the attack/defense.
#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(default)]
pub struct CombatStats {
    pub atk: Option<i32>,
    pub def: Option<i32>,
}

/// One-off, non-combat effects. Deliberately small: HP changes never
/// belong here (that's what negative `combat.atk` is for) -- this is
/// only for things combat resolution doesn't touch.
///
/// `RestoreHealth` is the one exception, and it's *not* meant to be
/// used on a directly playable card: it exists for hooks (Sun Amulet's
/// on_death) which fire outside the normal play/target/defend pipeline
/// entirely, so they can't be expressed as an attack.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CardEffect {
    RestoreHealth { amount: u8 },
    RestoreMana { amount: u8 },
    RestoreMoney { amount: u8 },
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
    /// Bot-only targeting hint. See `TargetKind`.
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
    /// A "miracle" in Godwheel vocabulary: stays in your hand instead of
    /// being discarded when played (see game.rs's hand-slot handling).
    #[serde(default)]
    pub persistent: bool,
    #[serde(default)]
    pub price: u8,
    #[serde(default)]
    pub mana_cost: u8,
    #[serde(default)]
    pub elements: HashSet<Element>,
    /// String-keyed hook table. Only "on_death" is fired anywhere in
    /// the MVP engine.
    #[serde(default)]
    pub hooks: HashMap<String, Vec<CardEffect>>,
    /// How many copies of this card are in the single base pool.
    /// Multiple named deck categories are deferred until something
    /// actually needs a second pool.
    #[serde(default = "default_count")]
    pub count: u32,
}

impl CardDefinition {
    /// True if this card can be played as the base of an attack (which
    /// includes self-heals -- any signed atk counts, not just positive).
    pub fn is_combat_card(&self) -> bool {
        self.playable && self.combat.and_then(|c| c.atk).is_some()
    }

    /// True if this card can be played reactively as (part of) a
    /// defense. Any number of these can be stacked together freely --
    /// there's no combo restriction on defense, unlike attacks.
    pub fn is_defense(&self) -> bool {
        self.playable
            && self
                .combat
                .and_then(|c| c.def)
                .map_or(false, |def| def > 0)
    }

    /// True if this is a non-combat effect card (mana/money potions).
    pub fn is_effect_card(&self) -> bool {
        self.playable && matches!(self.effect, Some(CardEffect::RestoreMana { .. }) | Some(CardEffect::RestoreMoney { .. }))
    }
}
