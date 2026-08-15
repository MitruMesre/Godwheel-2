use std::collections::HashMap;

use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;

use crate::card::TargetKind;
use crate::play_card::trigger_effect;
use crate::player::{Player, HAND_SLOTS};
use crate::registry::CardRegistry;
use crate::wheel::{attack_display_color, DisplayColor, Element};

/// How many cards each side starts with.
pub const OPENING_HAND: usize = 9;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Human,
    Bot,
}

pub struct AttackResult {
    pub attacker: Side,
    pub target: Side,
    pub atk_total: i32,
    pub def_total: i32,
    /// Net HP change applied to `target`: positive means they lost HP,
    /// negative means they gained it (a heal going through cleanly).
    pub damage: i32,
    pub color: DisplayColor,
    pub defended_with: Vec<String>,
    pub target_died: bool,
    /// If `target` would have died but a persistent card's on_death
    /// hook saved them (e.g. Sun Amulet), this names that card.
    pub revived_by: Option<String>,
}

/// Owns both players, the shared deck, and the (dumb) bot AI. This is
/// the whole simulation for the MVP: single process, one human + one
/// bot, no networking. Host-authoritative multiplayer (structure.md)
/// slots in later by moving this struct behind a network boundary
/// without changing its API.
pub struct GameState {
    pub registry: CardRegistry,
    pub human: Player,
    pub bot: Player,
    deck: Vec<String>,
    discard: Vec<String>,
    rng: StdRng,
}

impl GameState {
    pub fn new(registry: CardRegistry) -> Self {
        let mut rng = StdRng::from_entropy();
        let mut deck: Vec<String> = registry
            .cards
            .values()
            .flat_map(|def| std::iter::repeat(def.id.clone()).take(def.count as usize))
            .collect();
        deck.shuffle(&mut rng);

        let mut game = GameState {
            registry,
            human: Player::new("You".to_string()),
            bot: Player::new("Bot".to_string()),
            deck,
            discard: Vec::new(),
            rng,
        };
        game.draw_into_front_slots(Side::Human, OPENING_HAND);
        game.draw_into_front_slots(Side::Bot, OPENING_HAND);
        game
    }

    pub fn player(&self, side: Side) -> &Player {
        match side {
            Side::Human => &self.human,
            Side::Bot => &self.bot,
        }
    }

    pub fn player_mut(&mut self, side: Side) -> &mut Player {
        match side {
            Side::Human => &mut self.human,
            Side::Bot => &mut self.bot,
        }
    }

    pub fn winner(&self) -> Option<Side> {
        if !self.human.is_alive() {
            Some(Side::Bot)
        } else if !self.bot.is_alive() {
            Some(Side::Human)
        } else {
            None
        }
    }

    // ---------- deck management ----------

    fn draw_one(&mut self) -> Option<String> {
        if self.deck.is_empty() {
            if self.discard.is_empty() {
                return None;
            }
            std::mem::swap(&mut self.deck, &mut self.discard);
            self.deck.shuffle(&mut self.rng);
        }
        self.deck.pop()
    }

    /// Draws up to `n` cards into the front-most empty slots (lowest
    /// index first). Stops early if the deck+discard run out.
    fn draw_into_front_slots(&mut self, side: Side, n: usize) {
        let mut remaining = n;
        let mut slot = 0;
        while remaining > 0 && slot < HAND_SLOTS {
            if self.player(side).hand[slot].is_none() {
                match self.draw_one() {
                    Some(card) => {
                        self.player_mut(side).hand[slot] = Some(card);
                        remaining -= 1;
                    }
                    None => break, // deck and discard both empty
                }
            }
            slot += 1;
        }
    }

    fn empty_slot_from_back(&self, side: Side) -> Option<usize> {
        self.player(side)
            .hand
            .iter()
            .enumerate()
            .rev()
            .find(|(_, c)| c.is_none())
            .map(|(i, _)| i)
    }

    /// Removes the cards in `slots` from `side`'s hand. Non-persistent
    /// cards go to the discard pile. Persistent cards ("miracles") are
    /// relocated to the back-most empty slot instead of being
    /// discarded -- that's what makes room for them to keep occupying a
    /// hand slot indefinitely. Afterward, draws `slots.len()` new cards
    /// into the front-most empty slots.
    fn discard_and_redraw(&mut self, side: Side, slots: &[usize]) {
        for &slot in slots {
            let Some(id) = self.player_mut(side).hand[slot].take() else {
                continue;
            };
            let persistent = self.registry.get(&id).map_or(false, |d| d.persistent);
            if persistent {
                if let Some(back_slot) = self.empty_slot_from_back(side) {
                    self.player_mut(side).hand[back_slot] = Some(id);
                }
                // If the hand is completely full, the miracle has
                // nowhere to go and is lost. Not specially handled --
                // an edge case that shouldn't come up with 18 slots.
            } else {
                self.discard.push(id);
            }
        }
        self.draw_into_front_slots(side, slots.len());
    }

    // ---------- describing hands (for the CLI / wasm UI) ----------

    fn describe(&self, id: &str) -> String {
        let desc = self.registry.description_of(id);
        if desc.is_empty() {
            self.registry.name_of(id).to_string()
        } else {
            format!("{} -- {}", self.registry.name_of(id), desc)
        }
    }

    /// Cards `side` can play as the base of an attack (weapon, self-heal,
    /// or a non-combat effect card) -- i.e. everything except combo-only
    /// and defense-only cards.
    pub fn offensive_options(&self, side: Side) -> Vec<(usize, String)> {
        self.player(side)
            .hand
            .iter()
            .enumerate()
            .filter_map(|(slot, maybe_id)| {
                let id = maybe_id.as_ref()?;
                let def = self.registry.get(id)?;
                (!def.combo && (def.is_combat_card() || def.is_effect_card()))
                    .then(|| (slot, self.describe(id)))
            })
            .collect()
    }

    /// Playable combo cards in `side`'s hand, to attach to a base attack.
    pub fn combo_options(&self, side: Side) -> Vec<(usize, String)> {
        self.player(side)
            .hand
            .iter()
            .enumerate()
            .filter_map(|(slot, maybe_id)| {
                let id = maybe_id.as_ref()?;
                let def = self.registry.get(id)?;
                (def.playable && def.combo).then(|| (slot, self.describe(id)))
            })
            .collect()
    }

    /// Cards `side` can play as (part of) a defense. Any number of
    /// these can be combined freely -- there's no base+combo
    /// restriction on defense (that model is attack-only). Exclusive
    /// defenses (e.g. a "reflect" that can't be combined with anything
    /// else) aren't modeled in the MVP.
    pub fn defensive_options(&self, side: Side) -> Vec<(usize, String)> {
        self.player(side)
            .hand
            .iter()
            .enumerate()
            .filter_map(|(slot, maybe_id)| {
                let id = maybe_id.as_ref()?;
                let def = self.registry.get(id)?;
                def.is_defense().then(|| (slot, self.describe(id)))
            })
            .collect()
    }

    // ---------- combat math ----------

    fn combat_totals(&self, ids: &[String]) -> (i32, Vec<Element>) {
        let mut atk = 0;
        let mut elements = Vec::new();
        for id in ids {
            if let Some(def) = self.registry.get(id) {
                if let Some(a) = def.combat.and_then(|c| c.atk) {
                    atk += a;
                }
                elements.extend(def.elements.iter().copied());
            }
        }
        (atk, elements)
    }

    fn defense_total(&self, ids: &[String]) -> i32 {
        ids.iter()
            .filter_map(|id| self.registry.get(id).and_then(|d| d.combat).and_then(|c| c.def))
            .sum()
    }

    fn ids_at(&self, side: Side, slots: &[usize]) -> Vec<String> {
        slots
            .iter()
            .filter_map(|&s| self.player(side).hand.get(s).and_then(|c| c.clone()))
            .collect()
    }

    /// Validates `indices[0]` as a non-combo attack base and the rest as
    /// combo cards, returning `(slot, id)` pairs for each.
    fn take_attack_cards(
        &self,
        side: Side,
        indices: &[usize],
    ) -> Result<Vec<(usize, String)>, String> {
        if indices.is_empty() {
            return Err("no cards selected".into());
        }
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for (i, &slot) in indices.iter().enumerate() {
            if !seen.insert(slot) {
                return Err("duplicate card selected".into());
            }
            let id = self
                .player(side)
                .hand
                .get(slot)
                .and_then(|c| c.clone())
                .ok_or_else(|| "empty or invalid slot".to_string())?;
            let def = self
                .registry
                .get(&id)
                .ok_or_else(|| format!("unknown card id {id}"))?;
            if i == 0 {
                if def.combo {
                    return Err("the first card can't be a combo-only card".into());
                }
                if !def.is_combat_card() {
                    return Err(format!("{} isn't playable as an attack", self.registry.name_of(&id)));
                }
            } else if !def.combo {
                return Err(format!("{} isn't a combo card", self.registry.name_of(&id)));
            }
            out.push((slot, id));
        }
        Ok(out)
    }

    fn apply_damage_and_check_death(&mut self, side: Side, damage: i32) -> (bool, Option<String>) {
        self.player_mut(side).change_hp(-damage);
        self.check_death_hooks(side)
    }

    fn check_death_hooks(&mut self, side: Side) -> (bool, Option<String>) {
        if self.player(side).is_alive() {
            return (false, None);
        }
        match self.fire_on_death_hooks(side) {
            Some(name) => (false, Some(name)), // revived
            None => (true, None),              // still dead
        }
    }

    /// Fires the first "on_death" hook found among `side`'s hand cards
    /// (e.g. Sun Amulet), consuming that card entirely (it does not go
    /// to discard -- it's just gone). Stops as soon as one brings the
    /// player back above 0 HP.
    fn fire_on_death_hooks(&mut self, side: Side) -> Option<String> {
        let hand_snapshot: Vec<(usize, String)> = self
            .player(side)
            .hand
            .iter()
            .enumerate()
            .filter_map(|(i, c)| c.clone().map(|id| (i, id)))
            .collect();
        for (slot, id) in hand_snapshot {
            let Some(on_death) = self
                .registry
                .get(&id)
                .and_then(|d| d.hooks.get("on_death"))
                .cloned()
            else {
                continue;
            };
            self.player_mut(side).hand[slot] = None;
            for effect in &on_death {
                let target = match side {
                    Side::Human => &mut self.human,
                    Side::Bot => &mut self.bot,
                };
                trigger_effect(effect, target);
            }
            if self.player(side).is_alive() {
                return Some(self.registry.name_of(&id).to_string());
            }
        }
        None
    }

    // ---------- shared combat resolution ----------

    /// `manual_defense`: `Some(slots)` when the *target* is a human who
    /// interactively chose their defense cards (used by
    /// `bot_resolve_turn`); `None` to let the bot AI choose
    /// automatically (used by `human_play`, where the target is always
    /// the bot). Ignored entirely when `target == attacker` (you can't
    /// defend against your own card).
    fn play_and_resolve(
        &mut self,
        attacker: Side,
        indices: &[usize],
        target: Side,
        manual_defense: Option<Vec<usize>>,
    ) -> Result<AttackResult, String> {
        let cards = self.take_attack_cards(attacker, indices)?;
        let ids: Vec<String> = cards.iter().map(|(_, id)| id.clone()).collect();
        let (atk, elements) = self.combat_totals(&ids);
        let played_slots: Vec<usize> = cards.iter().map(|(s, _)| *s).collect();
        self.discard_and_redraw(attacker, &played_slots);

        let (def_total, defended_with) = if target == attacker {
            (0, Vec::new())
        } else {
            let defense_slots = match manual_defense {
                Some(slots) => slots,
                None => {
                    let ids = self.bot_choose_defense(atk);
                    self.slots_for_ids(target, &ids)
                }
            };
            let defense_ids = self.ids_at(target, &defense_slots);
            let total = self.defense_total(&defense_ids);
            let names: Vec<String> = defense_ids
                .iter()
                .map(|id| self.registry.name_of(id).to_string())
                .collect();
            if !defense_slots.is_empty() {
                self.discard_and_redraw(target, &defense_slots);
            }
            (total, names)
        };

        let damage = atk - def_total;
        let (target_died, revived_by) = self.apply_damage_and_check_death(target, damage);

        Ok(AttackResult {
            attacker,
            target,
            atk_total: atk,
            def_total,
            damage,
            color: attack_display_color(elements.iter()),
            defended_with,
            target_died,
            revived_by,
        })
    }

    /// Maps card ids back to hand-slot indices, first-match greedy
    /// (handles duplicate ids in the same hand).
    fn slots_for_ids(&self, side: Side, ids: &[String]) -> Vec<usize> {
        let hand = &self.player(side).hand;
        let mut used = std::collections::HashSet::new();
        let mut slots = Vec::new();
        for id in ids {
            if let Some(slot) = hand
                .iter()
                .enumerate()
                .position(|(i, h)| !used.contains(&i) && h.as_deref() == Some(id.as_str()))
            {
                used.insert(slot);
                slots.push(slot);
            }
        }
        slots
    }

    // ---------- human actions ----------

    /// Plays an attack: `indices[0]` is the base card, the rest are
    /// combo cards, resolved against `target` (either side -- attacking
    /// yourself, or "attacking" the bot with a heal, are both legal).
    /// The bot auto-defends when it's the target.
    pub fn human_play(&mut self, indices: &[usize], target: Side) -> Result<AttackResult, String> {
        self.play_and_resolve(Side::Human, indices, target, None)
    }

    /// Plays a single non-combat effect card (mana/money potion).
    /// Always self-targeted -- these aren't attacks.
    pub fn human_effect(&mut self, slot: usize) -> Result<String, String> {
        let id = self
            .player(Side::Human)
            .hand
            .get(slot)
            .and_then(|c| c.clone())
            .ok_or_else(|| "empty or invalid slot".to_string())?;
        let (effect, name) = {
            let def = self
                .registry
                .get(&id)
                .ok_or_else(|| format!("unknown card id {id}"))?;
            if !def.is_effect_card() {
                return Err(format!("{} isn't an effect card", self.registry.name_of(&id)));
            }
            (def.effect.clone().unwrap(), self.registry.name_of(&id).to_string())
        };

        self.discard_and_redraw(Side::Human, &[slot]);
        trigger_effect(&effect, &mut self.human);
        self.check_death_hooks(Side::Human);

        Ok(format!("You used {name}."))
    }

    // ---------- bot AI (deliberately simple) ----------

    /// Closest-subset-sum defense: picks the combination of `side`'s DEF
    /// cards whose summed def lands as close as possible to
    /// `incoming_atk` (over or under -- whichever is closer). Bounded,
    /// polynomial-time subset-sum DP; fine at hand-of-18 scale.
    /// "Reflect"-style exclusive defenses aren't modeled in the MVP.
    fn bot_choose_defense(&self, incoming_atk: i32) -> Vec<String> {
        let defenders: Vec<(String, i32)> = self
            .bot
            .hand
            .iter()
            .filter_map(|c| c.as_ref())
            .filter_map(|id| {
                let d = self.registry.get(id)?.combat.and_then(|c| c.def)?;
                (d > 0).then(|| (id.clone(), d))
            })
            .collect();

        let mut best: HashMap<i32, Vec<usize>> = HashMap::new();
        best.insert(0, Vec::new());
        for (i, (_, d)) in defenders.iter().enumerate() {
            let existing: Vec<(i32, Vec<usize>)> =
                best.iter().map(|(s, v)| (*s, v.clone())).collect();
            for (s, combo) in existing {
                let new_sum = s + d;
                best.entry(new_sum).or_insert_with(|| {
                    let mut v = combo.clone();
                    v.push(i);
                    v
                });
            }
        }

        let chosen_sum = best
            .keys()
            .min_by_key(|&&s| (s - incoming_atk).abs())
            .copied()
            .unwrap_or(0);
        best.get(&chosen_sum)
            .map(|indices| indices.iter().map(|&i| defenders[i].0.clone()).collect())
            .unwrap_or_default()
    }

    /// The bot's own-turn decision: pick a random playable weapon
    /// (non-combo, any signed atk), attach every combo card currently
    /// in hand, and target per that base card's `TargetKind` hint
    /// (defaulting to the enemy). Returns `(slots, target, atk_total)`,
    /// or `None` if the bot has nothing playable. Doesn't mutate hand
    /// state -- call `bot_resolve_turn` to actually play it.
    pub fn bot_plan_turn(&mut self) -> Option<(Vec<usize>, Side, i32)> {
        let bases: Vec<usize> = self
            .bot
            .hand
            .iter()
            .enumerate()
            .filter_map(|(slot, maybe_id)| {
                let id = maybe_id.as_ref()?;
                let def = self.registry.get(id)?;
                (!def.combo && def.is_combat_card()).then_some(slot)
            })
            .collect();
        let &base_slot = bases.choose(&mut self.rng)?;
        let base_id = self.bot.hand[base_slot].clone()?;
        let target = match self.registry.get(&base_id).and_then(|d| d.target) {
            Some(TargetKind::SelfTarget) => Side::Bot,
            _ => Side::Human,
        };

        let combo_slots: Vec<usize> = self
            .bot
            .hand
            .iter()
            .enumerate()
            .filter_map(|(slot, maybe_id)| {
                let id = maybe_id.as_ref()?;
                let def = self.registry.get(id)?;
                (def.playable && def.combo).then_some(slot)
            })
            .collect();

        let mut slots = vec![base_slot];
        slots.extend(&combo_slots);
        let ids = self.ids_at(Side::Bot, &slots);
        let (atk, _elements) = self.combat_totals(&ids);

        Some((slots, target, atk))
    }

    /// Actually plays the move `bot_plan_turn` described. `target` and
    /// `slots` should be exactly what that call returned.
    /// `human_defense` is the set of hand slots the human chose to
    /// defend with, if `target == Side::Human` (pass an empty slice for
    /// "no defense" or if `target == Side::Bot`, where it's unused).
    pub fn bot_resolve_turn(
        &mut self,
        slots: &[usize],
        target: Side,
        human_defense: &[usize],
    ) -> Result<AttackResult, String> {
        let manual = (target == Side::Human).then(|| human_defense.to_vec());
        self.play_and_resolve(Side::Bot, slots, target, manual)
    }
}
