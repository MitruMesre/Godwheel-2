use std::collections::{HashMap, HashSet};

use crate::wheel::Element;

pub enum TargetKind {
    SelfTarget,
    Enemy { index: usize },
    AllEnemies,
    HandCard { index: usize },
    AllPlayers,
}

pub enum GamePhase {
    AttackPhase, // usable as part of an attack (weapons, potions, etc.)
    DefensePhase, //usable when defending an attack (defense, reflect, counterattack, etc.)
                 // maybe hooks go here?
}

// this is for instantiating decks that hold special cards
// for example, a card whose purpose is to draw from a special deck
// of powerful cards, that shouldn't appear in the "Base" deck.
pub enum DeckCategory {
    Base { cards: Vec<String> }, // I'm thinking I just hold ID, and then have an indexed lookup
}

pub struct CombatStats {
    atk: Option<i32>,
    def: Option<i32>,
    element: Option<Element>,
}

pub enum CardEffect {
    Afflict {
        affliction: Element,
    },
    DealDamageAndAfflictIfUnblocked {
        affliction: Element,
        damage_amount: u8,
    },
    RestoreHealth {
        amount: u8,
    },
    DealDamage {
        amount: u8,
    },
    HealOrDamage {
        heal_chance: f32,
        heal_amount: u8,
        damage_amount: u8,
    },
    TransferMoney {
        amount: u8,
    },
    Sell,
}

pub struct CardDefinition {
    id: String,
    target_data: Option<TargetKind>,
    combat: Option<CombatStats>,
    combo: bool,
    effect: Option<CardEffect>,
    playable: bool,
    persistent: bool,
    price: u8,
    mana_cost: u8,
    // hooks?
    elements: HashSet<Element>,
    copies_in_decks: HashMap<DeckCategory, u8>, // default: {base: 1}
}
// todo: new() with some defaults
// or rather, the toml is the only thing that's going to instantiate
// TODO: function to get name and description from localization
// TODO: function to get art from assets
