use bevy::prelude::*;
use halestorm_common::protocol::{ClientMessage, ServerMessage};
use halestorm_common::transport::{ConnectionId, MessageInbox, MessageOutbox};
use halestorm_common::types::{EntityId, PlayerId, Tick, TilePosition};
use std::collections::HashMap;

/// Server-side plugin: processes client messages and runs the game simulation.
pub struct ServerGamePlugin;

impl Plugin for ServerGamePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MessageInbox<ClientMessage>>()
            .init_resource::<MessageOutbox<ServerMessage>>()
            .init_resource::<ServerState>()
            .insert_resource(Time::<Fixed>::from_seconds(1.0 / 20.0))
            .add_systems(FixedUpdate, process_messages);
    }
}

/// Tracks server-side state: accounts, players, entities.
#[derive(Resource, Default)]
pub struct ServerState {
    /// username -> hashed password (plain text for now, argon2 in WP5)
    accounts: HashMap<String, String>,
    /// connection -> authenticated player
    sessions: HashMap<ConnectionId, PlayerSession>,
    /// next entity id counter
    next_entity_id: u64,
    /// current server tick
    tick: Tick,
}

struct PlayerSession {
    _player_id: PlayerId,
    username: String,
    character: Option<CharacterData>,
    entity_id: Option<EntityId>,
}

struct CharacterData {
    _name: String,
    position: TilePosition,
}

impl ServerState {
    fn next_entity(&mut self) -> EntityId {
        let id = EntityId(self.next_entity_id);
        self.next_entity_id += 1;
        id
    }
}

fn process_messages(
    mut inbox: ResMut<MessageInbox<ClientMessage>>,
    mut outbox: ResMut<MessageOutbox<ServerMessage>>,
    mut state: ResMut<ServerState>,
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
                    // TODO: hash with argon2 in WP5
                    state.accounts.insert(username.clone(), password);
                    info!("Account created: {username}");
                    outbox.push(conn, ServerMessage::AccountCreated);
                }
            }

            ClientMessage::Login { username, password } => {
                match state.accounts.get(&username) {
                    Some(stored) if *stored == password => {
                        let player_id = PlayerId(conn.0);
                        state.sessions.insert(
                            conn,
                            PlayerSession {
                                _player_id: player_id,
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
                if let Some(session) = state.sessions.get_mut(&conn) {
                    let spawn = TilePosition::new(15, 10);
                    session.character = Some(CharacterData {
                        _name: name.clone(),
                        position: spawn,
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
                    // TODO: validate against collision map in WP4/5
                    character.position = target;
                    outbox.push(
                        conn,
                        ServerMessage::MoveConfirm {
                            tick,
                            position: target,
                        },
                    );
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
