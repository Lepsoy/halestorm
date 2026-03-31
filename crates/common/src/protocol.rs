use serde::{Deserialize, Serialize};

use crate::types::{Direction, EntityId, PlayerId, Tick, TilePosition};

/// Messages sent from client to server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    CreateAccount {
        username: String,
        password: String,
    },
    Login {
        username: String,
        password: String,
    },
    CreateCharacter {
        name: String,
    },
    EnterWorld,
    MoveIntent {
        direction: Direction,
        tick: Tick,
    },
    Disconnect,
}

/// Messages sent from server to client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    AccountCreated,
    LoginSuccess {
        player_id: PlayerId,
    },
    LoginFailed {
        reason: String,
    },
    CharacterCreated {
        name: String,
        spawn_position: TilePosition,
    },
    EnterWorld {
        tick: Tick,
        entity_id: EntityId,
        position: TilePosition,
        map_id: String,
    },
    WorldSnapshot {
        tick: Tick,
        entities: Vec<EntityState>,
    },
    MoveConfirm {
        tick: Tick,
        position: TilePosition,
    },
    MoveReject {
        tick: Tick,
        position: TilePosition,
    },
}

/// State of a single entity as seen in a world snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityState {
    pub entity_id: EntityId,
    pub position: TilePosition,
    pub direction: Direction,
    pub moving: bool,
}
