use bevy::prelude::*;
use halestorm_common::protocol::{ClientMessage, ServerMessage};
use halestorm_common::transport::{ConnectionId, MessageInbox, MessageOutbox};
use halestorm_common::types::{EntityId, PlayerId, Tick, TilePosition};

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

            ServerMessage::LoginSuccess { player_id } => {
                info!("Logged in as player {:?}", player_id);
                state.player_id = Some(player_id);
                state.phase = ClientPhase::LoggedIn;
            }

            ServerMessage::LoginFailed { reason } => {
                warn!("Login failed: {reason}");
                state.status_message = Some(reason);
            }

            ServerMessage::CharacterCreated {
                name,
                spawn_position,
            } => {
                info!(
                    "Character '{name}' created at ({}, {})",
                    spawn_position.x, spawn_position.y
                );
            }

            ServerMessage::EnterWorld {
                tick,
                entity_id,
                position,
                map_id,
            } => {
                info!(
                    "Entered world: map={map_id}, entity={:?}, pos=({}, {}), tick={:?}",
                    entity_id, position.x, position.y, tick
                );
                state.entity_id = Some(entity_id);
                state.position = Some(position);
                state.predicted_position = Some(position);
                state.last_confirmed_tick = Some(tick);
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

            ServerMessage::WorldSnapshot { .. } => {
                // TODO: entity interpolation in WP7
            }
        }
    }
}

/// Helper to send a client message via the outbox.
pub fn send_message(outbox: &mut MessageOutbox<ClientMessage>, msg: ClientMessage) {
    outbox.push(ConnectionId::LOCAL, msg);
}
