use rand::Rng;

use crate::card::CardEffect;
use crate::player::Player;

/// Applies a single one-off CardEffect to `target` (RNG for the
/// probabilistic ones comes from `rng`). This only handles the
/// non-combat effect track -- atk/def combat resolution lives in
/// game.rs, since it needs both players plus the attacker/defender
/// exchange, not just one target.
pub fn trigger_effect(effect: &CardEffect, target: &mut Player, rng: &mut impl Rng) {
    match effect {
        CardEffect::RestoreHealth { amount } => target.change_hp(*amount as i32),
        CardEffect::RestoreMana { amount } => target.change_mana(*amount as i32),
        CardEffect::RestoreMoney { amount } => target.change_money(*amount as i32),
        CardEffect::DealDamage { amount } => target.change_hp(-(*amount as i32)),
        CardEffect::HealOrDamage {
            heal_chance,
            heal_amount,
            damage_amount,
        } => {
            let heal = rng.gen::<f32>() < *heal_chance;
            if heal {
                target.change_hp(*heal_amount as i32);
            } else {
                target.change_hp(-(*damage_amount as i32));
            }
        }
    }
}
