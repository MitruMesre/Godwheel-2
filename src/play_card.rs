// todo: this is intended to be a lookup table for effects
use crate::card::CardEffect;
use crate::player::Player;
use crate::wheel::Element;

fn trigger_effect(effect: CardEffect, caster_id: u8, target_id: u8) -> () {
    let caster: &mut Player = todo!();
    let target: &mut Player = todo!();
    match effect {
        CardEffect::Afflict { affliction } => todo!(),
        CardEffect::DealDamageAndAfflictIfUnblocked {
            affliction,
            damage_amount,
        } => todo!(),
        CardEffect::RestoreHealth { amount } => {
            assert!(
                amount <= i8::MAX as u8,
                "health delta exceeds i8 range in play_card::trigger_effect::RestoreHealth"
            );
            target.change_hp(amount as i8)
        }
        CardEffect::DealDamage { amount } => {
            assert!(
                amount <= i8::MAX as u8,
                "health delta exceeds i8 range in play_card::trigger_effect::DealDamage"
            );
            target.change_hp(-(amount as i8))
        }
        CardEffect::HealOrDamage {
            heal_chance,
            heal_amount,
            damage_amount,
        } => {
            let heal = true; // todo: rng
            if heal {
                trigger_effect(
                    CardEffect::RestoreHealth {
                        amount: heal_amount,
                    },
                    caster_id,
                    target_id,
                );
            } else {
                trigger_effect(
                    CardEffect::DealDamage {
                        amount: damage_amount,
                    },
                    caster_id,
                    target_id,
                );
            }
        }
        CardEffect::RestoreMana { amount } => todo!(),
        CardEffect::RestoreMoney { amount } => todo!(),
    }
}
