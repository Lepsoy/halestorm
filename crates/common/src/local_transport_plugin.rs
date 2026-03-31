use bevy::prelude::*;

use crate::local_transport::LocalChannels;
use crate::protocol::{ClientMessage, ServerMessage};
use crate::transport::{MessageInbox, MessageOutbox};

/// Bevy plugin that wires up in-process local transport between
/// an embedded server and client running in the same app.
pub struct LocalTransportPlugin;

impl Plugin for LocalTransportPlugin {
    fn build(&self, app: &mut App) {
        let channels = LocalChannels::new();
        app.insert_resource(LocalTransportChannels(channels))
            // Client-side inbox/outbox for ServerMessage/ClientMessage
            .init_resource::<MessageInbox<ServerMessage>>()
            .init_resource::<MessageOutbox<ClientMessage>>()
            // Transport systems: run after server fixed update, before client systems
            .add_systems(
                Update,
                (flush_client_to_server, flush_server_to_client).chain(),
            );
    }
}

#[derive(Resource)]
struct LocalTransportChannels(LocalChannels);

/// Move client outbox messages into the server inbox via channels.
fn flush_client_to_server(
    transport: Res<LocalTransportChannels>,
    mut client_outbox: ResMut<MessageOutbox<ClientMessage>>,
    mut server_inbox: ResMut<MessageInbox<ClientMessage>>,
) {
    crate::local_transport::send_client_messages(&transport.0, &mut client_outbox);
    crate::local_transport::receive_client_messages(&transport.0, &mut server_inbox);
}

/// Move server outbox messages into the client inbox via channels.
fn flush_server_to_client(
    transport: Res<LocalTransportChannels>,
    mut server_outbox: ResMut<MessageOutbox<ServerMessage>>,
    mut client_inbox: ResMut<MessageInbox<ServerMessage>>,
) {
    crate::local_transport::send_server_messages(&transport.0, &mut server_outbox);
    crate::local_transport::receive_server_messages(&transport.0, &mut client_inbox);
}
