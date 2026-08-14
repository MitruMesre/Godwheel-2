pub struct Player {
    pub name: String,
    pub hand: Vec<String>, // card IDs
    pub health: u8,
    pub mana: u8,
    pub money: u8,
}

const MAX_HAND_SIZE: u8 = 18;
const MAX_HEALTH: u8 = 99;
const MAX_MANA: u8 = 99;
const MAX_MONEY: u8 = 99;

impl Player {
    /// Starts with an empty hand -- drawing the opening hand needs the
    /// shared deck, so that's done by GameState::new right after
    /// construction, not here.
    pub fn new(name: String) -> Player {
        Player {
            name,
            hand: Vec::new(),
            health: 40,
            mana: 20,
            money: 20,
        }
    }

    pub fn is_alive(&self) -> bool {
        self.health > 0
    }

    pub fn max_hand_size(&self) -> usize {
        MAX_HAND_SIZE as usize
    }

    fn change_value(value: &mut u8, delta: i32, max: u8) {
        if delta >= 0 {
            *value = value.saturating_add(delta as u8).min(max);
        } else {
            *value = value.saturating_sub((-delta) as u8);
        }
    }

    pub fn change_hp(&mut self, delta: i32) {
        Self::change_value(&mut self.health, delta, MAX_HEALTH);
    }

    pub fn change_mana(&mut self, delta: i32) {
        Self::change_value(&mut self.mana, delta, MAX_MANA);
    }

    pub fn change_money(&mut self, delta: i32) {
        Self::change_value(&mut self.money, delta, MAX_MONEY);
    }

    /// Removes the first occurrence of `card_id` from hand, if present.
    pub fn remove_from_hand(&mut self, card_id: &str) -> bool {
        if let Some(pos) = self.hand.iter().position(|id| id == card_id) {
            self.hand.remove(pos);
            true
        } else {
            false
        }
    }
}
