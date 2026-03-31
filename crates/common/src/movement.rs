use crate::types::{Direction, TilePosition};

/// Compute the target tile for a move in the given direction.
pub fn compute_target(from: TilePosition, direction: Direction) -> TilePosition {
    let (dx, dy) = direction.offset();
    TilePosition::new(from.x + dx, from.y + dy)
}

/// Validate a move: returns the target position if the tile is walkable, None otherwise.
pub fn validate_move(
    from: TilePosition,
    direction: Direction,
    is_walkable: impl Fn(TilePosition) -> bool,
) -> Option<TilePosition> {
    let target = compute_target(from, direction);
    if is_walkable(target) { Some(target) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_target_cardinal() {
        let origin = TilePosition::new(5, 5);
        assert_eq!(compute_target(origin, Direction::North), TilePosition::new(5, 4));
        assert_eq!(compute_target(origin, Direction::South), TilePosition::new(5, 6));
        assert_eq!(compute_target(origin, Direction::East), TilePosition::new(6, 5));
        assert_eq!(compute_target(origin, Direction::West), TilePosition::new(4, 5));
    }

    #[test]
    fn compute_target_diagonal() {
        let origin = TilePosition::new(5, 5);
        assert_eq!(compute_target(origin, Direction::NorthEast), TilePosition::new(6, 4));
        assert_eq!(compute_target(origin, Direction::SouthWest), TilePosition::new(4, 6));
    }

    #[test]
    fn validate_move_walkable() {
        let from = TilePosition::new(3, 3);
        let result = validate_move(from, Direction::East, |_| true);
        assert_eq!(result, Some(TilePosition::new(4, 3)));
    }

    #[test]
    fn validate_move_blocked() {
        let from = TilePosition::new(3, 3);
        let result = validate_move(from, Direction::East, |_| false);
        assert_eq!(result, None);
    }

    #[test]
    fn validate_move_selective_blocking() {
        let from = TilePosition::new(3, 3);
        let wall = TilePosition::new(4, 3);
        let result = validate_move(from, Direction::East, |pos| pos != wall);
        assert_eq!(result, None);

        let result = validate_move(from, Direction::West, |pos| pos != wall);
        assert_eq!(result, Some(TilePosition::new(2, 3)));
    }
}
