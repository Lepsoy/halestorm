use halestorm_common::map::CollisionMap;
use halestorm_common::map_loader;
use halestorm_common::movement;
use halestorm_common::types::{Direction, TilePosition};

#[test]
fn collision_map_boundary_edges() {
    let map = CollisionMap::new(5, 5);
    // All corners walkable
    assert!(map.is_walkable(TilePosition::new(0, 0)));
    assert!(map.is_walkable(TilePosition::new(4, 4)));
    assert!(map.is_walkable(TilePosition::new(0, 4)));
    assert!(map.is_walkable(TilePosition::new(4, 0)));
    // Just outside
    assert!(!map.is_walkable(TilePosition::new(5, 0)));
    assert!(!map.is_walkable(TilePosition::new(0, 5)));
    assert!(!map.is_walkable(TilePosition::new(-1, 0)));
    assert!(!map.is_walkable(TilePosition::new(0, -1)));
}

#[test]
fn movement_into_blocked_tile_rejected() {
    let mut map = CollisionMap::new(10, 10);
    map.set_blocked(TilePosition::new(6, 5));

    let from = TilePosition::new(5, 5);
    let result = movement::validate_move(from, Direction::East, |pos| map.is_walkable(pos));
    assert_eq!(result, None);
}

#[test]
fn movement_around_blocked_tile_works() {
    let mut map = CollisionMap::new(10, 10);
    map.set_blocked(TilePosition::new(6, 5));

    let from = TilePosition::new(5, 5);
    // Can go north to avoid the wall
    let result = movement::validate_move(from, Direction::North, |pos| map.is_walkable(pos));
    assert_eq!(result, Some(TilePosition::new(5, 4)));
    // Can go NorthEast diagonally
    let result = movement::validate_move(from, Direction::NorthEast, |pos| map.is_walkable(pos));
    assert_eq!(result, Some(TilePosition::new(6, 4)));
}

#[test]
fn movement_off_map_rejected() {
    let map = CollisionMap::new(10, 10);
    let from = TilePosition::new(0, 0);
    let result = movement::validate_move(from, Direction::North, |pos| map.is_walkable(pos));
    assert_eq!(result, None); // y=-1 is out of bounds

    let result = movement::validate_move(from, Direction::West, |pos| map.is_walkable(pos));
    assert_eq!(result, None); // x=-1 is out of bounds
}

#[test]
fn test_map_loads_and_has_expected_structure() {
    let content =
        std::fs::read_to_string("../../assets/maps/test_map.tmj").expect("test map exists");
    let map = map_loader::parse_tmj(&content).expect("valid map");

    assert_eq!(map.width, 30);
    assert_eq!(map.height, 20);
    assert_eq!(map.tile_size, 32);
    assert_eq!(map.spawn_point, TilePosition::new(15, 10));
    assert_eq!(map.ground_tiles.len(), (map.width * map.height) as usize);
    assert_eq!(map.wall_tiles.len(), (map.width * map.height) as usize);
}

#[test]
fn test_map_walls_block_correctly() {
    let content =
        std::fs::read_to_string("../../assets/maps/test_map.tmj").expect("test map exists");
    let map = map_loader::parse_tmj(&content).expect("valid map");

    // Border walls
    assert!(!map.collision_map.is_walkable(TilePosition::new(0, 0)));
    assert!(!map.collision_map.is_walkable(TilePosition::new(29, 0)));
    assert!(!map.collision_map.is_walkable(TilePosition::new(0, 10)));

    // Interior wall at y=5, x=5..10
    assert!(!map.collision_map.is_walkable(TilePosition::new(7, 5)));

    // Open area
    assert!(map.collision_map.is_walkable(TilePosition::new(10, 10)));
    assert!(map.collision_map.is_walkable(TilePosition::new(15, 10))); // spawn

    // Water at y=18,19
    assert!(!map.collision_map.is_walkable(TilePosition::new(10, 19)));
}

#[test]
fn test_map_spawn_is_walkable() {
    let content =
        std::fs::read_to_string("../../assets/maps/test_map.tmj").expect("test map exists");
    let map = map_loader::parse_tmj(&content).expect("valid map");
    assert!(map.collision_map.is_walkable(map.spawn_point));
}

#[test]
fn movement_validated_against_test_map() {
    let content =
        std::fs::read_to_string("../../assets/maps/test_map.tmj").expect("test map exists");
    let map = map_loader::parse_tmj(&content).expect("valid map");

    // From spawn, moving south should work (open grass)
    let result = movement::validate_move(map.spawn_point, Direction::South, |pos| {
        map.collision_map.is_walkable(pos)
    });
    assert_eq!(result, Some(TilePosition::new(15, 11)));

    // From top-left open area, moving north into border wall should fail
    let from = TilePosition::new(5, 1);
    let result = movement::validate_move(from, Direction::North, |pos| {
        map.collision_map.is_walkable(pos)
    });
    assert_eq!(result, None);
}

#[test]
fn all_eight_directions_from_center() {
    let map = CollisionMap::new(20, 20);
    let center = TilePosition::new(10, 10);

    let dirs = [
        (Direction::North, TilePosition::new(10, 9)),
        (Direction::NorthEast, TilePosition::new(11, 9)),
        (Direction::East, TilePosition::new(11, 10)),
        (Direction::SouthEast, TilePosition::new(11, 11)),
        (Direction::South, TilePosition::new(10, 11)),
        (Direction::SouthWest, TilePosition::new(9, 11)),
        (Direction::West, TilePosition::new(9, 10)),
        (Direction::NorthWest, TilePosition::new(9, 9)),
    ];

    for (dir, expected) in dirs {
        let result = movement::validate_move(center, dir, |pos| map.is_walkable(pos));
        assert_eq!(result, Some(expected), "Failed for direction {:?}", dir);
    }
}
