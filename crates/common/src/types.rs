use serde::{Deserialize, Serialize};

/// A position on the tile grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct TilePosition {
    pub x: i32,
    pub y: i32,
}

impl TilePosition {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// Unique identifier for any entity in the game world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityId(pub u64);

/// Unique identifier for a connected player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlayerId(pub u64);

/// Server tick counter.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, PartialOrd, Ord,
)]
pub struct Tick(pub u64);

/// Primary class selection. Determines sprite, skills, and attribute lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, strum::EnumIter, strum::Display)]
pub enum PrimaryClass {
    Champion,
    Ranger,
    Monk,
    Elementalist,
    Illusionist,
    Cultist,
}

impl PrimaryClass {
    /// Returns the sprite filename for this class.
    pub fn sprite_file(self) -> &'static str {
        match self {
            PrimaryClass::Champion => "sprites/champion.png",
            PrimaryClass::Ranger => "sprites/ranger.png",
            PrimaryClass::Monk => "sprites/monk.png",
            PrimaryClass::Elementalist => "sprites/elementalist.png",
            PrimaryClass::Illusionist => "sprites/illusionist.png",
            PrimaryClass::Cultist => "sprites/cultist.png",
        }
    }
}

/// 8-directional movement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Direction {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
}

impl Direction {
    /// Whether this is a diagonal direction.
    pub fn is_diagonal(self) -> bool {
        matches!(
            self,
            Direction::NorthEast
                | Direction::SouthEast
                | Direction::SouthWest
                | Direction::NorthWest
        )
    }

    /// Returns the (dx, dy) offset for this direction.
    /// Y increases downward (screen space convention).
    pub fn offset(self) -> (i32, i32) {
        match self {
            Direction::North => (0, -1),
            Direction::NorthEast => (1, -1),
            Direction::East => (1, 0),
            Direction::SouthEast => (1, 1),
            Direction::South => (0, 1),
            Direction::SouthWest => (-1, 1),
            Direction::West => (-1, 0),
            Direction::NorthWest => (-1, -1),
        }
    }
}
