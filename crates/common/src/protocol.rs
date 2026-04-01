use serde::{Deserialize, Serialize};

use crate::monster::MonsterKind;
use crate::types::{Direction, EntityId, PlayerId, PrimaryClass, Tick, TilePosition};

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
        class: PrimaryClass,
    },
    SelectCharacter {
        character_id: u64,
    },
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
        characters: Vec<CharacterInfo>,
    },
    LoginFailed {
        reason: String,
    },
    CharacterCreated {
        name: String,
        class: PrimaryClass,
        spawn_position: TilePosition,
        /// Updated character list after creation.
        characters: Vec<CharacterInfo>,
    },
    EnterWorld {
        tick: Tick,
        entity_id: EntityId,
        position: TilePosition,
        map_id: String,
        class: PrimaryClass,
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

/// Summary info about a character, sent in login response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterInfo {
    pub id: u64,
    pub name: String,
    pub class: PrimaryClass,
}

/// What kind of entity this is.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntityKind {
    Player { class: PrimaryClass },
    Monster { kind: MonsterKind, hp: i32, max_hp: i32 },
}

/// State of a single entity as seen in a world snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityState {
    pub entity_id: EntityId,
    pub position: TilePosition,
    pub direction: Direction,
    pub moving: bool,
    pub kind: EntityKind,
}
