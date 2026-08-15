use crate::card::CardEffect;
use crate::player::Player;

/// Applies a single one-off, non-combat `CardEffect` to `target`.
///
/// HP changes never come through here -- those are always
/// `combat.atk` (possibly negative) resolved by game.rs's combat path,
/// per design: a heal is just an attack. `RestoreHealth` is only ever
/// invoked from a hook (e.g. Sun Amulet's on_death), never from a
/// directly-played card.
pub fn trigger_effect(effect: &CardEffect, target: &mut Player) {
    match effect {
        CardEffect::RestoreHealth { amount } => target.change_hp(*amount as i32),
        CardEffect::RestoreMana { amount } => target.change_mana(*amount as i32),
        CardEffect::RestoreMoney { amount } => target.change_money(*amount as i32),
    }
}
