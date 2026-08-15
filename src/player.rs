/// Fixed hand size: 18 slots (col = i % 9, row = i / 9), matching the
/// hand grid in index.html. Slots are addressed directly rather than
/// hand being a loosely-ordered Vec, because miracles need to land in
/// specific back slots (see GameState::relocate_persistent_card).
pub const HAND_SLOTS: usize = 18;

pub struct Player {
    pub name: String,
    /// `None` = empty slot.
    pub hand: Vec<Option<String>>,
    pub health: u8,
    pub mana: u8,
    pub money: u8,
}

const MAX_HEALTH: u8 = 99;
const MAX_MANA: u8 = 99;
const MAX_MONEY: u8 = 99;

impl Player {
    /// Hand starts fully empty -- the opening draw needs the shared
    /// deck, so that's done by GameState::new right after construction.
    pub fn new(name: String) -> Player {
        Player {
            name,
            hand: vec![None; HAND_SLOTS],
            health: 40,
            mana: 20,
            money: 20,
        }
    }

    pub fn is_alive(&self) -> bool {
        self.health > 0
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
}
