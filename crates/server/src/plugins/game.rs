use bevy::prelude::*;
use halestorm_common::local_transport_plugin::LocalTransportSet;
use halestorm_common::map::CollisionMap;
use halestorm_common::protocol::{ClientMessage, EntityState, ServerMessage};
use halestorm_common::transport::{ConnectionId, MessageInbox, MessageOutbox};
use halestorm_common::types::{Direction, EntityId, PlayerId, Tick, TilePosition};
use std::collections::HashMap;

/// Server-side plugin: processes client messages and runs the game simulation.
pub struct ServerGamePlugin;

impl Plugin for ServerGamePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MessageInbox<ClientMessage>>()
            .init_resource::<MessageOutbox<ServerMessage>>()
            .init_resource::<ServerState>()
            .insert_resource(Time::<Fixed>::from_seconds(1.0 / 20.0))
            .add_systems(
                FixedUpdate,
                (process_messages, broadcast_world_snapshot)
                    .chain()
                    .after(LocalTransportSet::ClientToServer)
                    .before(LocalTransportSet::ServerToClient),
            );
    }
}

/// Tracks server-side state: accounts, players, entities.
#[derive(Resource, Default)]
pub struct ServerState {
    /// username -> hashed password (plain text for now, argon2 later)
    accounts: HashMap<String, String>,
    /// connection -> authenticated player session
    sessions: HashMap<ConnectionId, PlayerSession>,
    /// next entity id counter
    next_entity_id: u64,
    /// next player id counter
    next_player_id: u64,
    /// current server tick
    tick: Tick,
    /// configured spawn point (set from map data)
    pub spawn_point: TilePosition,
}

#[allow(dead_code)]
struct PlayerSession {
    player_id: PlayerId,
    username: String,
    character: Option<CharacterData>,
    entity_id: Option<EntityId>,
}

#[allow(dead_code)]
struct CharacterData {
    name: String,
    position: TilePosition,
    direction: Direction,
    moving: bool,
}

impl ServerState {
    fn next_entity(&mut self) -> EntityId {
        let id = EntityId(self.next_entity_id);
        self.next_entity_id += 1;
        id
    }

    fn next_player(&mut self) -> PlayerId {
        let id = PlayerId(self.next_player_id);
        self.next_player_id += 1;
        id
    }
}

fn process_messages(
    mut inbox: ResMut<MessageInbox<ClientMessage>>,
    mut outbox: ResMut<MessageOutbox<ServerMessage>>,
    mut state: ResMut<ServerState>,
    collision_map: Option<Res<CollisionMap>>,
) {
    state.tick = Tick(state.tick.0 + 1);

    let messages: Vec<_> = inbox.drain().collect();
    for (conn, msg) in messages {
        match msg {
            ClientMessage::CreateAccount { username, password } => {
                if state.accounts.contains_key(&username) {
                    outbox.push(
                        conn,
                        ServerMessage::LoginFailed {
                            reason: "Username already exists".into(),
                        },
                    );
                } else {
                    // TODO: hash with argon2
                    state.accounts.insert(username.clone(), password);
                    info!("Account created: {username}");
                    outbox.push(conn, ServerMessage::AccountCreated);
                }
            }

            ClientMessage::Login { username, password } => {
                match state.accounts.get(&username) {
                    Some(stored) if *stored == password => {
                        let player_id = state.next_player();
                        state.sessions.insert(
                            conn,
                            PlayerSession {
                                player_id,
                                username: username.clone(),
                                character: None,
                                entity_id: None,
                            },
                        );
                        info!("Player logged in: {username}");
                        outbox.push(conn, ServerMessage::LoginSuccess { player_id });
                    }
                    _ => {
                        outbox.push(
                            conn,
                            ServerMessage::LoginFailed {
                                reason: "Invalid username or password".into(),
                            },
                        );
                    }
                }
            }

            ClientMessage::CreateCharacter { name } => {
                let spawn = state.spawn_point;
                if let Some(session) = state.sessions.get_mut(&conn) {
                    session.character = Some(CharacterData {
                        name: name.clone(),
                        position: spawn,
                        direction: Direction::South,
                        moving: false,
                    });
                    info!("Character created: {name}");
                    outbox.push(
                        conn,
                        ServerMessage::CharacterCreated {
                            name,
                            spawn_position: spawn,
                        },
                    );
                }
            }

            ClientMessage::EnterWorld => {
                let entity_id = state.next_entity();
                if let Some(session) = state.sessions.get_mut(&conn)
                    && let Some(ref character) = session.character
                {
                    let position = character.position;
                    session.entity_id = Some(entity_id);
                    info!(
                        "Player {} entering world at ({}, {})",
                        session.username, position.x, position.y
                    );
                    outbox.push(
                        conn,
                        ServerMessage::EnterWorld {
                            tick: state.tick,
                            entity_id,
                            position,
                            map_id: "test_map".into(),
                        },
                    );
                }
            }

            ClientMessage::MoveIntent { direction, tick } => {
                if let Some(session) = state.sessions.get_mut(&conn)
                    && let Some(ref mut character) = session.character
                {
                    let target = halestorm_common::movement::compute_target(
                        character.position,
                        direction,
                    );

                    let walkable = collision_map
                        .as_ref()
                        .map(|cm| cm.is_walkable(target))
                        .unwrap_or(true);

                    if walkable {
                        character.position = target;
                        character.direction = direction;
                        character.moving = true;
                        outbox.push(
                            conn,
                            ServerMessage::MoveConfirm {
                                tick,
                                position: target,
                            },
                        );
                    } else {
                        outbox.push(
                            conn,
                            ServerMessage::MoveReject {
                                tick,
                                position: character.position,
                            },
                        );
                    }
                }
            }

            ClientMessage::Disconnect => {
                if let Some(session) = state.sessions.remove(&conn) {
                    info!("Player disconnected: {}", session.username);
                }
            }
        }
    }
}

fn broadcast_world_snapshot(
    mut outbox: ResMut<MessageOutbox<ServerMessage>>,
    state: Res<ServerState>,
) {
    if state.sessions.is_empty() {
        return;
    }

    let entities: Vec<EntityState> = state
        .sessions
        .values()
        .filter_map(|session| {
            let character = session.character.as_ref()?;
            Some(EntityState {
                entity_id: session.entity_id?,
                position: character.position,
                direction: character.direction,
                moving: character.moving,
            })
        })
        .collect();

    let connections: Vec<ConnectionId> = state.sessions.keys().copied().collect();
    outbox.broadcast(
        &connections,
        ServerMessage::WorldSnapshot {
            tick: state.tick,
            entities,
        },
    );
}
