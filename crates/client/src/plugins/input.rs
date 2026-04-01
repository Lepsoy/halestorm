use bevy::prelude::*;
use halestorm_common::map::CollisionMap;
use halestorm_common::movement;
use halestorm_common::protocol::ClientMessage;
use halestorm_common::transport::MessageOutbox;
use halestorm_common::types::{Direction, Tick};

use super::game::ClientState;
use super::player::PlayerMovement;
use super::ui::GameScreen;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            handle_movement_input.run_if(in_state(GameScreen::InGame)),
        );
    }
}

fn handle_movement_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut outbox: ResMut<MessageOutbox<ClientMessage>>,
    mut state: ResMut<ClientState>,
    player_movement: Option<ResMut<PlayerMovement>>,
    collision_map: Option<Res<CollisionMap>>,
) {
    // Only handle input when in world and not mid-transition
    if state.phase != super::game::ClientPhase::InWorld {
        return;
    }
    let Some(mut player_mov) = player_movement else {
        return;
    };
    if player_mov.is_moving() {
        return;
    }

    let direction = read_direction(&keyboard);
    let Some(direction) = direction else {
        return;
    };

    let current_pos = state.predicted_position.unwrap_or_default();
    let target = movement::compute_target(current_pos, direction);

    // Client-side prediction: validate terrain
    let terrain_ok = collision_map
        .as_ref()
        .map(|cm| cm.is_walkable(target))
        .unwrap_or(true);

    if !terrain_ok {
        return;
    }

    // Check monster collision from latest snapshot
    if let Some((_tick, ref entities)) = state.latest_snapshot {
        use halestorm_common::protocol::EntityKind;
        let monster_blocking = entities.iter().any(|e| {
            matches!(e.kind, EntityKind::Monster { .. }) && e.position == target
        });
        if monster_blocking {
            return;
        }
    }

    // Predict locally
    state.predicted_position = Some(target);

    // Start visual movement
    player_mov.start_move(current_pos, target, direction);

    // Send to server
    let tick = state.last_confirmed_tick.unwrap_or(Tick(0));
    outbox.push(
        halestorm_common::transport::ConnectionId::LOCAL,
        ClientMessage::MoveIntent {
            direction,
            tick: Tick(tick.0 + 1),
        },
    );
}

fn read_direction(keyboard: &ButtonInput<KeyCode>) -> Option<Direction> {
    // Dedicated diagonal keys take priority
    if keyboard.pressed(KeyCode::KeyQ) {
        return Some(Direction::NorthWest);
    }
    if keyboard.pressed(KeyCode::KeyE) {
        return Some(Direction::NorthEast);
    }
    if keyboard.pressed(KeyCode::KeyZ) {
        return Some(Direction::SouthWest);
    }
    if keyboard.pressed(KeyCode::KeyC) {
        return Some(Direction::SouthEast);
    }

    let up = keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp);
    let down = keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown);
    let left = keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft);
    let right = keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight);

    match (up, down, left, right) {
        (true, false, false, false) => Some(Direction::North),
        (false, true, false, false) => Some(Direction::South),
        (false, false, true, false) => Some(Direction::West),
        (false, false, false, true) => Some(Direction::East),
        (true, false, true, false) => Some(Direction::NorthWest),
        (true, false, false, true) => Some(Direction::NorthEast),
        (false, true, true, false) => Some(Direction::SouthWest),
        (false, true, false, true) => Some(Direction::SouthEast),
        _ => None,
    }
}
