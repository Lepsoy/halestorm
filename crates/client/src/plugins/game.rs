use bevy::prelude::*;
use halestorm_common::protocol::{CharacterInfo, ClientMessage, EntityState, ServerMessage};
use halestorm_common::transport::{ConnectionId, MessageInbox, MessageOutbox};
use halestorm_common::types::{EntityId, PlayerId, PrimaryClass, Tick, TilePosition};

/// Client-side game plugin: processes server messages and manages client state.
pub struct ClientGamePlugin;

impl Plugin for ClientGamePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ClientState>()
            .init_resource::<MessageInbox<ServerMessage>>()
            .init_resource::<MessageOutbox<ClientMessage>>()
            .add_systems(Update, process_server_messages);
    }
}

/// Tracks the client's game state.
#[derive(Resource, Default)]
pub struct ClientState {
    pub phase: ClientPhase,
    pub player_id: Option<PlayerId>,
    pub entity_id: Option<EntityId>,
    pub position: Option<TilePosition>,
    pub predicted_position: Option<TilePosition>,
    pub last_confirmed_tick: Option<Tick>,
    /// The player's class (determines sprite)
    pub class: Option<PrimaryClass>,
    /// Characters available on this account
    pub characters: Vec<CharacterInfo>,
    /// Latest world snapshot from server.
    pub latest_snapshot: Option<(Tick, Vec<EntityState>)>,
    /// Set when a character was just created (triggers return to select screen)
    pub character_just_created: bool,
    /// Last status message for UI display (login errors, etc.)
    pub status_message: Option<String>,
    /// Whether account was just created (for UI feedback)
    pub account_created: bool,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub enum ClientPhase {
    #[default]
    Login,
    LoggedIn,
    InWorld,
}

fn process_server_messages(
    mut inbox: ResMut<MessageInbox<ServerMessage>>,
    mut state: ResMut<ClientState>,
) {
    for (_conn, msg) in inbox.drain() {
        match msg {
            ServerMessage::AccountCreated => {
                info!("Account created successfully");
                state.account_created = true;
                state.status_message = Some("Account created! You can now log in.".into());
            }

            ServerMessage::LoginSuccess {
                player_id,
                characters,
            } => {
                info!("Logged in as player {:?} ({} characters)", player_id, characters.len());
                state.player_id = Some(player_id);
                state.characters = characters;
                state.phase = ClientPhase::LoggedIn;
            }

            ServerMessage::LoginFailed { reason } => {
                warn!("Login failed: {reason}");
                state.status_message = Some(reason);
            }

            ServerMessage::CharacterCreated {
                name,
                class,
                spawn_position,
                characters,
            } => {
                info!(
                    "Character '{name}' ({class}) created at ({}, {})",
                    spawn_position.x, spawn_position.y
                );
                state.characters = characters;
                state.character_just_created = true;
            }

            ServerMessage::EnterWorld {
                tick,
                entity_id,
                position,
                map_id,
                class,
            } => {
                info!(
                    "Entered world: map={map_id}, entity={:?}, pos=({}, {}), tick={:?}, class={class}",
                    entity_id, position.x, position.y, tick
                );
                state.entity_id = Some(entity_id);
                state.position = Some(position);
                state.predicted_position = Some(position);
                state.last_confirmed_tick = Some(tick);
                state.class = Some(class);
                state.phase = ClientPhase::InWorld;
            }

            ServerMessage::MoveConfirm { tick, position } => {
                state.position = Some(position);
                state.last_confirmed_tick = Some(tick);
                // If prediction matches, no correction needed
                if state.predicted_position == Some(position) {
                    trace!("Move confirmed (prediction correct) tick={:?}", tick);
                } else {
                    // Server disagrees with prediction — snap to authoritative position
                    debug!(
                        "Move confirmed (correction) tick={:?}: ({}, {})",
                        tick, position.x, position.y
                    );
                    state.predicted_position = Some(position);
                }
            }

            ServerMessage::MoveReject { tick, position } => {
                state.position = Some(position);
                state.predicted_position = Some(position);
                state.last_confirmed_tick = Some(tick);
                warn!(
                    "Move rejected tick={:?}, snapping to ({}, {})",
                    tick, position.x, position.y
                );
            }

            ServerMessage::WorldSnapshot { tick, entities } => {
                state.latest_snapshot = Some((tick, entities));
            }
        }
    }
}

/// Helper to send a client message via the outbox.
pub fn send_message(outbox: &mut MessageOutbox<ClientMessage>, msg: ClientMessage) {
    outbox.push(ConnectionId::LOCAL, msg);
}
