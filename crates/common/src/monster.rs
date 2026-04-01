use serde::{Deserialize, Serialize};

/// Monster type definitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MonsterKind {
    Goblin,
}

impl MonsterKind {
    pub fn sprite_file(self) -> &'static str {
        match self {
            MonsterKind::Goblin => "sprites/goblin.png",
        }
    }

    pub fn max_hp(self) -> i32 {
        match self {
            MonsterKind::Goblin => 50,
        }
    }

    pub fn attack_power(self) -> i32 {
        match self {
            MonsterKind::Goblin => 5,
        }
    }

    pub fn aggro_range(self) -> i32 {
        match self {
            MonsterKind::Goblin => 6,
        }
    }

    pub fn leash_range(self) -> i32 {
        match self {
            MonsterKind::Goblin => 15,
        }
    }

    pub fn move_speed_ticks(self) -> u32 {
        match self {
            // Moves every N server ticks (20 ticks/sec, so 4 = every 200ms)
            MonsterKind::Goblin => 4,
        }
    }
}
