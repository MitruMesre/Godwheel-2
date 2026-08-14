use std::collections::HashSet;

use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;

use crate::card::{CardEffect, TargetKind};
use crate::play_card::trigger_effect;
use crate::player::Player;
use crate::registry::CardRegistry;
use crate::wheel::{attack_display_color, DisplayColor, Element};

pub const HAND_SIZE: usize = 9;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Human,
    Bot,
}

pub struct AttackResult {
    pub attacker: Side,
    pub atk_total: i32,
    pub def_total: i32,
    pub damage: i32,
    pub color: DisplayColor,
    pub defended_with: Option<String>,
    pub defender_died: bool,
    /// If the defender would have died but a persistent card's on_death
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
        game.draw_cards(Side::Human, HAND_SIZE);
        game.draw_cards(Side::Bot, HAND_SIZE);
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

    fn draw_cards(&mut self, side: Side, n: usize) {
        let cap = self.player(side).max_hand_size();
        let room = cap.saturating_sub(self.player(side).hand.len());
        let n = n.min(room);
        let mut drawn = Vec::with_capacity(n);
        for _ in 0..n {
            match self.draw_one() {
                Some(card) => drawn.push(card),
                None => break, // deck and discard both empty
            }
        }
        self.player_mut(side).hand.extend(drawn);
    }

    /// Removes `played` from `side`'s hand (to discard, unless a card is
    /// `persistent`, in which case it goes right back to hand), then
    /// redraws the same number of cards.
    fn discard_and_redraw(&mut self, side: Side, played: &[String]) {
        for id in played {
            self.player_mut(side).remove_from_hand(id);
            let persistent = self.registry.get(id).map_or(false, |d| d.persistent);
            if persistent {
                self.player_mut(side).hand.push(id.clone());
            } else {
                self.discard.push(id.clone());
            }
        }
        self.draw_cards(side, played.len());
    }

    // ---------- describing hands (for the CLI / any future UI) ----------

    fn describe(&self, id: &str) -> String {
        let desc = self.registry.description_of(id);
        if desc.is_empty() {
            self.registry.name_of(id).to_string()
        } else {
            format!("{} -- {}", self.registry.name_of(id), desc)
        }
    }

    /// Cards `side` can play as an attack or a self/enemy effect on
    /// their own turn (i.e. everything except combo-only and
    /// defense-only cards).
    pub fn offensive_options(&self, side: Side) -> Vec<(usize, String)> {
        self.player(side)
            .hand
            .iter()
            .enumerate()
            .filter(|(_, id)| {
                self.registry.get(id).map_or(false, |d| {
                    !d.combo && (d.is_attack() || d.is_effect_card())
                })
            })
            .map(|(i, id)| (i, self.describe(id)))
            .collect()
    }

    /// Playable combo cards in `side`'s hand, to attach to a base attack.
    pub fn combo_options(&self, side: Side) -> Vec<(usize, String)> {
        self.player(side)
            .hand
            .iter()
            .enumerate()
            .filter(|(_, id)| self.registry.get(id).map_or(false, |d| d.playable && d.combo))
            .map(|(i, id)| (i, self.describe(id)))
            .collect()
    }

    /// Cards `side` can play reactively to defend against an attack.
    pub fn defensive_options(&self, side: Side) -> Vec<(usize, String)> {
        self.player(side)
            .hand
            .iter()
            .enumerate()
            .filter(|(_, id)| self.registry.get(id).map_or(false, |d| d.is_defense()))
            .map(|(i, id)| (i, self.describe(id)))
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

    fn take_attack_cards(&self, side: Side, indices: &[usize]) -> Result<Vec<String>, String> {
        if indices.is_empty() {
            return Err("no cards selected".into());
        }
        let hand = &self.player(side).hand;
        let mut seen = HashSet::new();
        let mut ids = Vec::new();
        for (i, &idx) in indices.iter().enumerate() {
            if !seen.insert(idx) {
                return Err("duplicate card selected".into());
            }
            let id = hand
                .get(idx)
                .ok_or_else(|| "card index out of range".to_string())?;
            let def = self
                .registry
                .get(id)
                .ok_or_else(|| format!("unknown card id {id}"))?;
            if i == 0 {
                if def.combo {
                    return Err("the first card can't be a combo-only card".into());
                }
                if !def.is_attack() {
                    return Err(format!("{} isn't an attack card", self.registry.name_of(id)));
                }
            } else if !def.combo {
                return Err(format!("{} isn't a combo card", self.registry.name_of(id)));
            }
            ids.push(id.clone());
        }
        Ok(ids)
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
    /// (e.g. Sun Amulet), consuming that card. Stops as soon as one
    /// brings the player back above 0 HP.
    fn fire_on_death_hooks(&mut self, side: Side) -> Option<String> {
        let hand_snapshot: Vec<String> = self.player(side).hand.clone();
        for id in hand_snapshot {
            let Some(on_death) = self
                .registry
                .get(&id)
                .and_then(|d| d.hooks.get("on_death"))
                .cloned()
            else {
                continue;
            };
            self.player_mut(side).remove_from_hand(&id);
            for effect in &on_death {
                let target = match side {
                    Side::Human => &mut self.human,
                    Side::Bot => &mut self.bot,
                };
                trigger_effect(effect, target, &mut self.rng);
            }
            if self.player(side).is_alive() {
                return Some(self.registry.name_of(&id).to_string());
            }
        }
        None
    }

    // ---------- human actions ----------

    /// Plays an attack (base card at `indices[0]` plus any combo cards
    /// at the rest of `indices`) against the bot. The bot's defense is
    /// decided automatically.
    pub fn human_attack(&mut self, indices: &[usize]) -> Result<AttackResult, String> {
        let ids = self.take_attack_cards(Side::Human, indices)?;
        let (atk, elements) = self.combat_totals(&ids);
        self.discard_and_redraw(Side::Human, &ids);

        let defense_id = self.bot_choose_defense();
        let def_total = defense_id
            .as_ref()
            .and_then(|id| self.registry.get(id))
            .and_then(|d| d.combat)
            .and_then(|c| c.def)
            .unwrap_or(0);
        if let Some(id) = &defense_id {
            self.discard_and_redraw(Side::Bot, std::slice::from_ref(id));
        }

        let damage = (atk - def_total).max(0);
        let (defender_died, revived_by) = self.apply_damage_and_check_death(Side::Bot, damage);

        Ok(AttackResult {
            attacker: Side::Human,
            atk_total: atk,
            def_total,
            damage,
            color: attack_display_color(elements.iter()),
            defended_with: defense_id.map(|id| self.registry.name_of(&id).to_string()),
            defender_died,
            revived_by,
        })
    }

    /// Plays a single (non-combo) effect card, resolving it against its
    /// declared target (self, or the bot).
    pub fn human_effect(&mut self, index: usize) -> Result<String, String> {
        let id = self
            .player(Side::Human)
            .hand
            .get(index)
            .cloned()
            .ok_or_else(|| "card index out of range".to_string())?;
        let (target_side, effect, name) = {
            let def = self
                .registry
                .get(&id)
                .ok_or_else(|| format!("unknown card id {id}"))?;
            if !def.is_effect_card() {
                return Err(format!("{} isn't an effect card", self.registry.name_of(&id)));
            }
            let target_side = match def.target {
                Some(TargetKind::Enemy) => Side::Bot,
                _ => Side::Human,
            };
            (
                target_side,
                def.effect.clone().unwrap(),
                self.registry.name_of(&id).to_string(),
            )
        };

        self.discard_and_redraw(Side::Human, std::slice::from_ref(&id));
        {
            let target = match target_side {
                Side::Human => &mut self.human,
                Side::Bot => &mut self.bot,
            };
            trigger_effect(&effect, target, &mut self.rng);
        }
        self.check_death_hooks(target_side);

        Ok(format!("You used {name}."))
    }

    // ---------- bot AI (deliberately simple) ----------

    fn bot_choose_defense(&self) -> Option<String> {
        self.bot
            .hand
            .iter()
            .filter(|id| self.registry.get(*id).map_or(false, |d| d.is_defense()))
            .max_by_key(|id| {
                self.registry
                    .get(*id)
                    .and_then(|d| d.combat)
                    .and_then(|c| c.def)
                    .unwrap_or(0)
            })
            .cloned()
    }

    /// Index of a healing effect card in the bot's hand, if its HP is
    /// low enough to bother.
    pub fn bot_plan_heal(&self) -> Option<usize> {
        if self.bot.health > 15 {
            return None;
        }
        self.bot.hand.iter().position(|id| {
            self.registry.get(id).map_or(false, |d| {
                d.is_effect_card() && matches!(d.effect, Some(CardEffect::RestoreHealth { .. }))
            })
        })
    }

    /// Index of the bot's best single attack card, if it has one.
    pub fn bot_plan_attack(&self) -> Option<usize> {
        self.bot
            .hand
            .iter()
            .enumerate()
            .filter(|(_, id)| self.registry.get(id.as_str()).map_or(false, |d| d.is_attack()))
            .max_by_key(|(_, id)| {
                self.registry
                    .get(id.as_str())
                    .and_then(|d| d.combat)
                    .and_then(|c| c.atk)
                    .unwrap_or(0)
            })
            .map(|(i, _)| i)
    }

    pub fn bot_use_effect(&mut self, index: usize) -> Result<String, String> {
        let id = self
            .bot
            .hand
            .get(index)
            .cloned()
            .ok_or_else(|| "bad card index".to_string())?;
        let effect = self
            .registry
            .get(&id)
            .and_then(|d| d.effect.clone())
            .ok_or_else(|| format!("{id} isn't an effect card"))?;
        let name = self.registry.name_of(&id).to_string();

        self.discard_and_redraw(Side::Bot, std::slice::from_ref(&id));
        trigger_effect(&effect, &mut self.bot, &mut self.rng);

        Ok(format!("Bot used {name}."))
    }

    /// The bot attacks with the card at `bot_index`; `human_defense_index`
    /// is the index (if any) of the defense card the human chose to
    /// respond with.
    pub fn resolve_bot_attack(
        &mut self,
        bot_index: usize,
        human_defense_index: Option<usize>,
    ) -> Result<AttackResult, String> {
        let bot_id = self
            .bot
            .hand
            .get(bot_index)
            .cloned()
            .ok_or_else(|| "bad card index".to_string())?;
        let (atk, elements) = self.combat_totals(std::slice::from_ref(&bot_id));
        self.discard_and_redraw(Side::Bot, std::slice::from_ref(&bot_id));

        let defense_id = human_defense_index.and_then(|i| self.human.hand.get(i).cloned());
        let def_total = defense_id
            .as_ref()
            .and_then(|id| self.registry.get(id))
            .and_then(|d| d.combat)
            .and_then(|c| c.def)
            .unwrap_or(0);
        if let Some(id) = &defense_id {
            self.discard_and_redraw(Side::Human, std::slice::from_ref(id));
        }

        let damage = (atk - def_total).max(0);
        let (defender_died, revived_by) = self.apply_damage_and_check_death(Side::Human, damage);

        Ok(AttackResult {
            attacker: Side::Bot,
            atk_total: atk,
            def_total,
            damage,
            color: attack_display_color(elements.iter()),
            defended_with: defense_id.map(|id| self.registry.name_of(&id).to_string()),
            defender_died,
            revived_by,
        })
    }
}
