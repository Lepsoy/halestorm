use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use bevy::prelude::*;
use halestorm_common::local_transport_plugin::LocalTransportSet;
use halestorm_common::map::CollisionMap;
use halestorm_common::protocol::{CharacterInfo, ClientMessage, EntityKind, EntityState, ServerMessage};
use halestorm_common::transport::{ConnectionId, MessageInbox, MessageOutbox};
use halestorm_common::types::{Direction, EntityId, PrimaryClass, Tick, TilePosition};
use std::collections::HashMap;

use super::persistence::Database;

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

/// Tracks server-side state: sessions and entities.
#[derive(Resource, Default)]
pub struct ServerState {
    /// connection -> authenticated player session
    pub(crate) sessions: HashMap<ConnectionId, PlayerSession>,
    /// next entity id counter
    next_entity_id: u64,
    /// current server tick
    pub(crate) tick: Tick,
    /// configured spawn point (set from map data)
    pub spawn_point: TilePosition,
}

#[allow(dead_code)]
pub(crate) struct PlayerSession {
    pub(crate) player_id: i64,
    pub(crate) username: String,
    pub(crate) character: Option<CharacterData>,
    pub(crate) entity_id: Option<EntityId>,
    pub(crate) character_db_id: Option<i64>,
}

#[allow(dead_code)]
pub(crate) struct CharacterData {
    pub(crate) name: String,
    pub(crate) class: PrimaryClass,
    pub(crate) position: TilePosition,
    pub(crate) direction: Direction,
    pub(crate) moving: bool,
}

impl ServerState {
    pub fn next_entity(&mut self) -> EntityId {
        let id = EntityId(self.next_entity_id);
        self.next_entity_id += 1;
        id
    }
}

fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| format!("Hash failed: {e}"))?;
    Ok(hash.to_string())
}

fn verify_password(password: &str, hash: &str) -> bool {
    let parsed = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

fn parse_class(s: &str) -> PrimaryClass {
    match s {
        "Champion" => PrimaryClass::Champion,
        "Ranger" => PrimaryClass::Ranger,
        "Monk" => PrimaryClass::Monk,
        "Elementalist" => PrimaryClass::Elementalist,
        "Illusionist" => PrimaryClass::Illusionist,
        "Cultist" => PrimaryClass::Cultist,
        _ => PrimaryClass::Champion,
    }
}

fn process_messages(
    mut inbox: ResMut<MessageInbox<ClientMessage>>,
    mut outbox: ResMut<MessageOutbox<ServerMessage>>,
    mut state: ResMut<ServerState>,
    collision_map: Option<Res<CollisionMap>>,
    db: Option<Res<Database>>,
    monster_state: Option<Res<super::monsters::MonsterState>>,
) {
    state.tick = Tick(state.tick.0 + 1);

    let messages: Vec<_> = inbox.drain().collect();
    for (conn, msg) in messages {
        match msg {
            ClientMessage::CreateAccount { username, password } => {
                let Some(ref db) = db else {
                    outbox.push(conn, ServerMessage::LoginFailed {
                        reason: "Database not available".into(),
                    });
                    continue;
                };

                if db.get_account(&username).is_some() {
                    outbox.push(conn, ServerMessage::LoginFailed {
                        reason: "Username already exists".into(),
                    });
                    continue;
                }

                match hash_password(&password) {
                    Ok(hash) => match db.create_account(&username, &hash) {
                        Ok(_) => {
                            info!("Account created: {username}");
                            outbox.push(conn, ServerMessage::AccountCreated);
                        }
                        Err(e) => {
                            outbox.push(conn, ServerMessage::LoginFailed {
                                reason: format!("Failed to create account: {e}"),
                            });
                        }
                    },
                    Err(e) => {
                        outbox.push(conn, ServerMessage::LoginFailed {
                            reason: format!("Internal error: {e}"),
                        });
                    }
                }
            }

            ClientMessage::Login { username, password } => {
                let Some(ref db) = db else {
                    outbox.push(conn, ServerMessage::LoginFailed {
                        reason: "Database not available".into(),
                    });
                    continue;
                };

                match db.get_account(&username) {
                    Some(account) if verify_password(&password, &account.password_hash) => {
                        // List all characters for this account
                        let characters: Vec<CharacterInfo> = db
                            .get_characters(account.id)
                            .into_iter()
                            .map(|c| CharacterInfo {
                                id: c.id as u64,
                                name: c.name,
                                class: parse_class(&c.class),
                            })
                            .collect();

                        state.sessions.insert(
                            conn,
                            PlayerSession {
                                player_id: account.id,
                                username: username.clone(),
                                character: None,
                                entity_id: None,
                                character_db_id: None,
                            },
                        );
                        info!("Player logged in: {username} ({} characters)", characters.len());
                        outbox.push(
                            conn,
                            ServerMessage::LoginSuccess {
                                player_id: halestorm_common::types::PlayerId(account.id as u64),
                                characters,
                            },
                        );
                    }
                    _ => {
                        outbox.push(conn, ServerMessage::LoginFailed {
                            reason: "Invalid username or password".into(),
                        });
                    }
                }
            }

            ClientMessage::CreateCharacter { name, class } => {
                let spawn = state.spawn_point;
                let Some(ref db) = db else { continue };
                if let Some(session) = state.sessions.get_mut(&conn) {
                    match db.create_character(
                        session.player_id,
                        &name,
                        &class.to_string(),
                        spawn,
                    ) {
                        Ok(_char_id) => {
                            info!("Character created: {name} ({class})");
                            // Fetch updated character list
                            let characters: Vec<CharacterInfo> = db
                                .get_characters(session.player_id)
                                .into_iter()
                                .map(|c| CharacterInfo {
                                    id: c.id as u64,
                                    name: c.name,
                                    class: parse_class(&c.class),
                                })
                                .collect();
                            outbox.push(
                                conn,
                                ServerMessage::CharacterCreated {
                                    name,
                                    class,
                                    spawn_position: spawn,
                                    characters,
                                },
                            );
                        }
                        Err(e) => {
                            warn!("Failed to create character: {e}");
                        }
                    }
                }
            }

            ClientMessage::SelectCharacter { character_id } => {
                let Some(ref db) = db else { continue };
                let entity_id = state.next_entity();
                if let Some(session) = state.sessions.get_mut(&conn) {
                    // Verify character belongs to this account
                    if let Some(c) = db.get_character_by_id(character_id as i64) {
                        if c.account_id != session.player_id {
                            warn!("Player {} tried to select another account's character", session.username);
                            continue;
                        }
                        let class = parse_class(&c.class);
                        let position = TilePosition::new(c.position_x, c.position_y);
                        session.character = Some(CharacterData {
                            name: c.name.clone(),
                            class,
                            position,
                            direction: Direction::South,
                            moving: false,
                        });
                        session.character_db_id = Some(c.id);
                        session.entity_id = Some(entity_id);
                        info!(
                            "Player {} selected character '{}' ({class}) at ({}, {})",
                            session.username, c.name, position.x, position.y
                        );
                        outbox.push(
                            conn,
                            ServerMessage::EnterWorld {
                                tick: state.tick,
                                entity_id,
                                position,
                                map_id: "test_map".into(),
                                class,
                            },
                        );
                    }
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

                    let terrain_ok = collision_map
                        .as_ref()
                        .map(|cm| cm.is_walkable(target))
                        .unwrap_or(true);

                    // Check monster collision — players can't walk through monsters
                    let monster_blocking = monster_state
                        .as_ref()
                        .map(|ms| ms.monsters.values().any(|m| m.position == target))
                        .unwrap_or(false);

                    if terrain_ok && !monster_blocking {
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
                    // Save position on disconnect
                    if let (Some(db), Some(char_id), Some(character)) =
                        (&db, session.character_db_id, &session.character)
                    {
                        db.save_character_position(char_id, character.position);
                        info!(
                            "Saved position for {}: ({}, {})",
                            session.username, character.position.x, character.position.y
                        );
                    }
                    info!("Player disconnected: {}", session.username);
                }
            }
        }
    }
}

fn broadcast_world_snapshot(
    mut outbox: ResMut<MessageOutbox<ServerMessage>>,
    state: Res<ServerState>,
    monster_state: Option<Res<super::monsters::MonsterState>>,
) {
    if state.sessions.is_empty() {
        return;
    }

    let mut entities: Vec<EntityState> = state
        .sessions
        .values()
        .filter_map(|session| {
            let character = session.character.as_ref()?;
            Some(EntityState {
                entity_id: session.entity_id?,
                position: character.position,
                direction: character.direction,
                moving: character.moving,
                kind: EntityKind::Player {
                    class: character.class,
                },
            })
        })
        .collect();

    // Add monsters
    if let Some(ref ms) = monster_state {
        for monster in ms.monsters.values() {
            entities.push(EntityState {
                entity_id: monster.entity_id,
                position: monster.position,
                direction: monster.direction,
                moving: false,
                kind: EntityKind::Monster {
                    kind: monster.kind,
                    hp: monster.hp,
                    max_hp: monster.max_hp,
                },
            });
        }
    }

    let connections: Vec<ConnectionId> = state.sessions.keys().copied().collect();
    outbox.broadcast(
        &connections,
        ServerMessage::WorldSnapshot {
            tick: state.tick,
            entities,
        },
    );
}
