use std::collections::HashSet;

use bevy::prelude::Resource;

use crate::types::TilePosition;

/// Stores which tiles are blocked (walls, water, obstacles).
/// Used by both server (authoritative collision) and client (prediction).
#[derive(Debug, Clone, Resource, Default)]
pub struct CollisionMap {
    blocked: HashSet<TilePosition>,
    pub width: i32,
    pub height: i32,
}

impl CollisionMap {
    pub fn new(width: i32, height: i32) -> Self {
        Self {
            blocked: HashSet::new(),
            width,
            height,
        }
    }

    pub fn set_blocked(&mut self, pos: TilePosition) {
        self.blocked.insert(pos);
    }

    pub fn is_walkable(&self, pos: TilePosition) -> bool {
        // Out of bounds is not walkable
        if pos.x < 0 || pos.y < 0 || pos.x >= self.width || pos.y >= self.height {
            return false;
        }
        !self.blocked.contains(&pos)
    }

    pub fn blocked_count(&self) -> usize {
        self.blocked.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_map_is_walkable() {
        let map = CollisionMap::new(10, 10);
        assert!(map.is_walkable(TilePosition::new(5, 5)));
    }

    #[test]
    fn blocked_tile_is_not_walkable() {
        let mut map = CollisionMap::new(10, 10);
        map.set_blocked(TilePosition::new(3, 3));
        assert!(!map.is_walkable(TilePosition::new(3, 3)));
        assert!(map.is_walkable(TilePosition::new(3, 4)));
    }

    #[test]
    fn out_of_bounds_is_not_walkable() {
        let map = CollisionMap::new(10, 10);
        assert!(!map.is_walkable(TilePosition::new(-1, 5)));
        assert!(!map.is_walkable(TilePosition::new(5, -1)));
        assert!(!map.is_walkable(TilePosition::new(10, 5)));
        assert!(!map.is_walkable(TilePosition::new(5, 10)));
    }
}
