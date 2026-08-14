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
    pub fn new(name: String) -> Player {
        let hand = Vec::new();
        // TODO: draw 9 cards
        Player {
            name,
            hand,
            health: 40,
            mana: 20,
            money: 20,
        }
    }
    fn change_value(value: &mut u8, delta: i8, max: u8) {
        if delta >= 0 {
            *value = value.saturating_add(delta as u8).min(max);
        } else {
            *value = value.saturating_sub((-delta) as u8);
        }
    }

    pub fn change_hp(&mut self, delta: i8) {
        Self::change_value(&mut self.health, delta, MAX_HEALTH);
    }

    pub fn change_mana(&mut self, delta: i8) {
        Self::change_value(&mut self.mana, delta, MAX_MANA);
    }

    pub fn change_money(&mut self, delta: i8) {
        Self::change_value(&mut self.money, delta, MAX_MONEY);
    }
}

// todo: draw cards... maybe a hand struct?
// not sure if deck is an actual deck you draw from,
// or if it's just a weighted table that you get a copy of a card from

// axes for each aspect pair
