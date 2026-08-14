use std::collections::HashMap;

pub enum TargetKind {
    SelfTarget,
    Enemy { index: usize },
    AllEnemies,
    HandCard { index: usize },
}

pub enum Element {
    Fire,
    Water,
    Earth,
    Wind,
}

pub struct TargetData {
    starting_target: TargetKind,
    count: u8,
}

pub struct CombatStats {
    atk: Option<i32>,
    def: Option<i32>,
    element: Option<Element>,
}

pub struct CardDef {
    id: String,
    target_data: TargetData,
    combat: Option<CombatStats>,
    combo: bool,
    on_play: Option<LuaFunction>,
    hooks: HashMap<String, LuaFunction>,
    // ...
}
