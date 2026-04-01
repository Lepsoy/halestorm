use bevy::prelude::*;
use halestorm_common::map::CollisionMap;
use halestorm_common::monster::MonsterKind;
use halestorm_common::types::{Direction, EntityId, TilePosition};
use std::collections::{HashMap, HashSet};

use super::game::ServerState;

pub struct MonsterPlugin;

impl Plugin for MonsterPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MonsterState>()
            .add_systems(Startup, spawn_monsters_deferred)
            .add_systems(Update, try_spawn_monsters)
            .add_systems(FixedUpdate, update_monster_ai);
    }
}

#[derive(Resource, Default)]
pub struct MonsterState {
    pub monsters: HashMap<EntityId, Monster>,
}

pub struct Monster {
    pub entity_id: EntityId,
    pub kind: MonsterKind,
    pub position: TilePosition,
    pub spawn_position: TilePosition,
    pub direction: Direction,
    pub hp: i32,
    pub max_hp: i32,
    pub ai: AiState,
    pub move_cooldown: u32,
}

#[allow(dead_code)]
pub enum AiState {
    Idle { wander_timer: u32 },
    Chasing { target_pos: TilePosition },
    Returning,
}

/// Marker to trigger monster spawning after map is loaded.
#[derive(Resource)]
pub struct SpawnMonstersFlag;

fn spawn_monsters_deferred(mut commands: Commands) {
    commands.insert_resource(SpawnMonstersFlag);
}

/// System that spawns monsters once the collision map is available.
pub fn try_spawn_monsters(
    mut commands: Commands,
    flag: Option<Res<SpawnMonstersFlag>>,
    collision_map: Option<Res<CollisionMap>>,
    mut monster_state: ResMut<MonsterState>,
    mut server_state: ResMut<ServerState>,
) {
    if flag.is_none() || collision_map.is_none() {
        return;
    }

    let collision_map = collision_map.unwrap();

    let spawn_points = vec![
        TilePosition::new(65, 15),
        TilePosition::new(75, 30),
        TilePosition::new(60, 25),
        TilePosition::new(47, 42),
        TilePosition::new(52, 38),
        TilePosition::new(13, 80),
        TilePosition::new(22, 85),
        TilePosition::new(35, 63),
        TilePosition::new(65, 63),
    ];

    for base in &spawn_points {
        for offset in &[(0, 0), (1, 1), (-1, 1)] {
            let pos = TilePosition::new(base.x + offset.0, base.y + offset.1);
            if !collision_map.is_walkable(pos) {
                continue;
            }
            let entity_id = server_state.next_entity();
            let kind = MonsterKind::Goblin;
            monster_state.monsters.insert(
                entity_id,
                Monster {
                    entity_id,
                    kind,
                    position: pos,
                    spawn_position: pos,
                    direction: Direction::South,
                    hp: kind.max_hp(),
                    max_hp: kind.max_hp(),
                    ai: AiState::Idle { wander_timer: 0 },
                    move_cooldown: 0,
                },
            );
        }
    }

    info!("Spawned {} monsters", monster_state.monsters.len());
    commands.remove_resource::<SpawnMonstersFlag>();
}

fn update_monster_ai(
    mut monster_state: ResMut<MonsterState>,
    server_state: Res<ServerState>,
    collision_map: Option<Res<CollisionMap>>,
) {
    let Some(collision_map) = collision_map else {
        return;
    };

    // Collect player positions for aggro and collision
    let player_positions: Vec<TilePosition> = server_state
        .sessions
        .values()
        .filter_map(|s| s.character.as_ref().map(|c| c.position))
        .collect();

    // Build occupied tile set: players + all monsters
    let mut occupied: HashSet<TilePosition> = player_positions.iter().copied().collect();
    for monster in monster_state.monsters.values() {
        occupied.insert(monster.position);
    }

    let monster_ids: Vec<EntityId> = monster_state.monsters.keys().copied().collect();

    for id in monster_ids {
        let Some(monster) = monster_state.monsters.get_mut(&id) else {
            continue;
        };

        if monster.move_cooldown > 0 {
            monster.move_cooldown -= 1;
            continue;
        }

        let aggro_range = monster.kind.aggro_range();
        let leash_range = monster.kind.leash_range();
        let speed = monster.kind.move_speed_ticks();

        match &monster.ai {
            AiState::Idle { wander_timer } => {
                // Check for nearby players
                if let Some(&target) =
                    find_nearest_player(monster.position, &player_positions, aggro_range)
                {
                    monster.ai = AiState::Chasing { target_pos: target };
                    continue;
                }

                let timer = *wander_timer;
                if timer == 0 {
                    let dir = random_direction();
                    let target =
                        halestorm_common::movement::compute_target(monster.position, dir);
                    if can_move_to(target, monster.position, &collision_map, &occupied)
                        && distance(target, monster.spawn_position) < leash_range
                    {
                        occupied.remove(&monster.position);
                        monster.position = target;
                        occupied.insert(target);
                        monster.direction = dir;
                        monster.move_cooldown = speed * 2;
                    }
                    monster.ai = AiState::Idle {
                        wander_timer: rand_range(10, 40),
                    };
                } else {
                    monster.ai = AiState::Idle {
                        wander_timer: timer - 1,
                    };
                }
            }

            AiState::Chasing { .. } => {
                if distance(monster.position, monster.spawn_position) > leash_range {
                    monster.ai = AiState::Returning;
                    continue;
                }

                if let Some(&new_target) =
                    find_nearest_player(monster.position, &player_positions, aggro_range + 2)
                {
                    // Stop adjacent to player (melee range) — don't walk on top of them
                    if distance(monster.position, new_target) > 1
                        && let Some(dir) = direction_toward(monster.position, new_target)
                    {
                        let next = halestorm_common::movement::compute_target(
                            monster.position,
                            dir,
                        );
                        if can_move_to(next, monster.position, &collision_map, &occupied) {
                            occupied.remove(&monster.position);
                            monster.position = next;
                            occupied.insert(next);
                            monster.direction = dir;
                        }
                    }
                    monster.ai = AiState::Chasing {
                        target_pos: new_target,
                    };
                    monster.move_cooldown = speed;
                } else {
                    monster.ai = AiState::Returning;
                }
            }

            AiState::Returning => {
                if distance(monster.position, monster.spawn_position) <= 1 {
                    monster.ai = AiState::Idle { wander_timer: 10 };
                    continue;
                }

                if let Some(dir) = direction_toward(monster.position, monster.spawn_position) {
                    let next =
                        halestorm_common::movement::compute_target(monster.position, dir);
                    if can_move_to(next, monster.position, &collision_map, &occupied) {
                        occupied.remove(&monster.position);
                        monster.position = next;
                        occupied.insert(next);
                        monster.direction = dir;
                    }
                }
                monster.move_cooldown = speed;
            }
        }
    }
}

/// Check if a tile is free: walkable terrain and not occupied by another entity.
fn can_move_to(
    target: TilePosition,
    current: TilePosition,
    collision_map: &CollisionMap,
    occupied: &HashSet<TilePosition>,
) -> bool {
    if !collision_map.is_walkable(target) {
        return false;
    }
    // Occupied by another entity (not ourselves)
    if target != current && occupied.contains(&target) {
        return false;
    }
    true
}

fn find_nearest_player(
    pos: TilePosition,
    players: &[TilePosition],
    range: i32,
) -> Option<&TilePosition> {
    players
        .iter()
        .filter(|p| distance(pos, **p) <= range)
        .min_by_key(|p| distance(pos, **p))
}

fn distance(a: TilePosition, b: TilePosition) -> i32 {
    (a.x - b.x).abs().max((a.y - b.y).abs())
}

fn direction_toward(from: TilePosition, to: TilePosition) -> Option<Direction> {
    let dx = (to.x - from.x).signum();
    let dy = (to.y - from.y).signum();
    match (dx, dy) {
        (0, -1) => Some(Direction::North),
        (1, -1) => Some(Direction::NorthEast),
        (1, 0) => Some(Direction::East),
        (1, 1) => Some(Direction::SouthEast),
        (0, 1) => Some(Direction::South),
        (-1, 1) => Some(Direction::SouthWest),
        (-1, 0) => Some(Direction::West),
        (-1, -1) => Some(Direction::NorthWest),
        _ => None,
    }
}

fn random_direction() -> Direction {
    let dirs = [
        Direction::North,
        Direction::NorthEast,
        Direction::East,
        Direction::SouthEast,
        Direction::South,
        Direction::SouthWest,
        Direction::West,
        Direction::NorthWest,
    ];
    dirs[rand_range(0, 7) as usize]
}

fn rand_range(min: u32, max: u32) -> u32 {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    min + (t % (max - min + 1))
}
