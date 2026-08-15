//! The browser-facing API: everything crosses the JS/wasm boundary as a
//! JSON string. This is deliberately not a fine-grained wasm-bindgen
//! type surface -- with no compiler available while writing this, a
//! single well-tested serialization boundary is a lot less risky than
//! marshaling a dozen individual Vec<struct> types through
//! wasm-bindgen's own type system. `index.html` calls these methods and
//! JSON.parses the results.

use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::game::{AttackResult, GameState, Side};
use crate::registry::CardRegistry;
use crate::wheel::DisplayColor;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HandCardView {
    slot: usize,
    id: String,
    name: String,
    desc: String,
    atk: Option<i32>,
    def: Option<i32>,
    combo: bool,
    persistent: bool,
    mana_cost: u8,
    price: u8,
    /// Playable on your own turn as the base of an attack, or as an
    /// effect card. False for combo-only and defense-only cards.
    playable_as_base: bool,
    is_defense: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PlayerView {
    name: String,
    hp: u8,
    mp: u8,
    gold: u8,
    hand: Vec<HandCardView>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OpponentView {
    name: String,
    hp: u8,
    mp: u8,
    gold: u8,
    /// The bot's hand contents are hidden from the human -- only the
    /// count is exposed, matching normal Godfield-style fog of war.
    hand_count: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StateView {
    you: PlayerView,
    bot: OpponentView,
    /// "you" | "bot" | null
    winner: Option<String>,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct PlayResult {
    ok: bool,
    error: Option<String>,
    log: Option<String>,
    atk: Option<i32>,
    def: Option<i32>,
    damage: Option<i32>,
    color: Option<String>,
    defended_with: Option<Vec<String>>,
    target_died: Option<bool>,
    revived_by: Option<String>,
}

impl PlayResult {
    fn from_attack(result: Result<AttackResult, String>) -> PlayResult {
        match result {
            Ok(res) => PlayResult {
                ok: true,
                atk: Some(res.atk_total),
                def: Some(res.def_total),
                damage: Some(res.damage),
                color: Some(match res.color {
                    DisplayColor::Neutral => "neutral".to_string(),
                    DisplayColor::Element(e) => format!("{e:?}").to_lowercase(),
                }),
                defended_with: Some(res.defended_with),
                target_died: Some(res.target_died),
                revived_by: res.revived_by,
                ..Default::default()
            },
            Err(e) => PlayResult {
                ok: false,
                error: Some(e),
                ..Default::default()
            },
        }
    }

    fn from_log(result: Result<String, String>) -> PlayResult {
        match result {
            Ok(log) => PlayResult {
                ok: true,
                log: Some(log),
                ..Default::default()
            },
            Err(e) => PlayResult {
                ok: false,
                error: Some(e),
                ..Default::default()
            },
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BotPlan {
    has_move: bool,
    slots: Vec<usize>,
    /// "you" | "bot"
    target: String,
    atk: i32,
    card_names: Vec<String>,
}

fn to_json<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string())
}

fn parse_slots(json: &str) -> Vec<usize> {
    serde_json::from_str::<Vec<u32>>(json)
        .unwrap_or_default()
        .into_iter()
        .map(|s| s as usize)
        .collect()
}

fn build_state_view(game: &GameState) -> StateView {
    let you_hand: Vec<HandCardView> = game
        .human
        .hand
        .iter()
        .enumerate()
        .filter_map(|(slot, maybe_id)| {
            let id = maybe_id.as_ref()?;
            let def = game.registry.get(id)?;
            Some(HandCardView {
                slot,
                id: id.clone(),
                name: game.registry.name_of(id).to_string(),
                desc: game.registry.description_of(id).to_string(),
                atk: def.combat.and_then(|c| c.atk),
                def: def.combat.and_then(|c| c.def),
                combo: def.combo,
                persistent: def.persistent,
                mana_cost: def.mana_cost,
                price: def.price,
                playable_as_base: !def.combo && (def.is_combat_card() || def.is_effect_card()),
                is_defense: def.is_defense(),
            })
        })
        .collect();

    StateView {
        you: PlayerView {
            name: game.human.name.clone(),
            hp: game.human.health,
            mp: game.human.mana,
            gold: game.human.money,
            hand: you_hand,
        },
        bot: OpponentView {
            name: game.bot.name.clone(),
            hp: game.bot.health,
            mp: game.bot.mana,
            gold: game.bot.money,
            hand_count: game.bot.hand.iter().filter(|c| c.is_some()).count(),
        },
        winner: game.winner().map(|w| match w {
            Side::Human => "you".to_string(),
            Side::Bot => "bot".to_string(),
        }),
    }
}

#[wasm_bindgen]
pub struct WasmGame {
    game: GameState,
}

#[wasm_bindgen]
impl WasmGame {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmGame {
        WasmGame {
            game: GameState::new(CardRegistry::load_embedded_base()),
        }
    }

    /// Full renderable snapshot: your hand in detail, the bot's HP/MP/
    /// gold/hand-count only, and the winner (if any).
    pub fn state(&self) -> String {
        to_json(&build_state_view(&self.game))
    }

    /// Plays `base_slot` (+ the slots in `combo_slots_json`, a JSON
    /// array of numbers) against `target` ("self" or "enemy"). The bot
    /// auto-defends.
    pub fn play(&mut self, base_slot: u32, combo_slots_json: &str, target: &str) -> String {
        let mut indices = vec![base_slot as usize];
        indices.extend(parse_slots(combo_slots_json));
        let target_side = if target == "enemy" { Side::Bot } else { Side::Human };
        to_json(&PlayResult::from_attack(self.game.human_play(&indices, target_side)))
    }

    /// Plays a non-combat effect card (mana/money potion) at `slot`.
    /// Always self-targeted.
    pub fn play_effect(&mut self, slot: u32) -> String {
        to_json(&PlayResult::from_log(self.game.human_effect(slot as usize)))
    }

    /// Asks the bot to decide its move, without playing it yet -- lets
    /// the UI show "Bot attacks with X for N" and collect a defense
    /// choice from the human before `bot_resolve` actually applies it.
    pub fn bot_plan(&mut self) -> String {
        let plan = self.game.bot_plan_turn();
        let view = match plan {
            None => BotPlan {
                has_move: false,
                slots: Vec::new(),
                target: String::new(),
                atk: 0,
                card_names: Vec::new(),
            },
            Some((slots, target, atk)) => {
                let card_names = slots
                    .iter()
                    .filter_map(|&s| self.game.player(Side::Bot).hand.get(s).and_then(|c| c.clone()))
                    .map(|id| self.game.registry.name_of(&id).to_string())
                    .collect();
                BotPlan {
                    has_move: true,
                    slots,
                    target: match target {
                        Side::Human => "you".to_string(),
                        Side::Bot => "bot".to_string(),
                    },
                    atk,
                    card_names,
                }
            }
        };
        to_json(&view)
    }

    /// Resolves the move `bot_plan` described. `slots_json` and
    /// `target` should be exactly what `bot_plan` returned;
    /// `defense_slots_json` is the human's chosen defense (a JSON array
    /// of numbers, possibly empty), used only when `target == "you"`.
    pub fn bot_resolve(&mut self, slots_json: &str, target: &str, defense_slots_json: &str) -> String {
        let slots = parse_slots(slots_json);
        let defense = parse_slots(defense_slots_json);
        let target_side = if target == "you" { Side::Human } else { Side::Bot };
        to_json(&PlayResult::from_attack(
            self.game.bot_resolve_turn(&slots, target_side, &defense),
        ))
    }
}

impl Default for WasmGame {
    fn default() -> Self {
        WasmGame::new()
    }
}
