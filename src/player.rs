pub struct Player {
    pub name: String,
    pub hand: Vec<String>, // card IDs
    pub health: u8,
    pub mana: u8,
    pub money: u8,
}

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
}

// todo: draw cards... maybe a hand struct?
// axes for each aspect pair
