use bevy::prelude::*;
use halestorm_common::protocol::{ClientMessage, ServerMessage};
use halestorm_common::transport::{ConnectionId, MessageInbox, MessageOutbox};
use halestorm_common::types::{EntityId, PlayerId, TilePosition};

/// Client-side game plugin: processes server messages and manages client state.
pub struct ClientGamePlugin;

impl Plugin for ClientGamePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ClientState>()
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
            }

            ServerMessage::LoginSuccess { player_id } => {
                info!("Logged in as player {:?}", player_id);
                state.player_id = Some(player_id);
                state.phase = ClientPhase::LoggedIn;
            }

            ServerMessage::LoginFailed { reason } => {
                warn!("Login failed: {reason}");
            }

            ServerMessage::CharacterCreated {
                name,
                spawn_position,
            } => {
                info!("Character '{name}' created at ({}, {})", spawn_position.x, spawn_position.y);
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
                state.phase = ClientPhase::InWorld;
            }

            ServerMessage::MoveConfirm { tick, position } => {
                state.position = Some(position);
                trace!("Move confirmed at tick {:?}: ({}, {})", tick, position.x, position.y);
            }

            ServerMessage::MoveReject { tick, position } => {
                state.position = Some(position);
                warn!("Move rejected at tick {:?}, snapping to ({}, {})", tick, position.x, position.y);
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
