use crossbeam_channel::{Receiver, Sender, unbounded};

use crate::protocol::{ClientMessage, ServerMessage};
use crate::transport::{ConnectionId, MessageInbox, MessageOutbox};

/// Channels for in-process communication between embedded server and client.
/// Created once at startup and shared via Bevy resources.
pub struct LocalChannels {
    /// Client writes here, server reads from the receiver.
    pub client_to_server_tx: Sender<ClientMessage>,
    pub client_to_server_rx: Receiver<ClientMessage>,
    /// Server writes here, client reads from the receiver.
    pub server_to_client_tx: Sender<ServerMessage>,
    pub server_to_client_rx: Receiver<ServerMessage>,
}

impl LocalChannels {
    pub fn new() -> Self {
        let (c2s_tx, c2s_rx) = unbounded();
        let (s2c_tx, s2c_rx) = unbounded();
        Self {
            client_to_server_tx: c2s_tx,
            client_to_server_rx: c2s_rx,
            server_to_client_tx: s2c_tx,
            server_to_client_rx: s2c_rx,
        }
    }
}

impl Default for LocalChannels {
    fn default() -> Self {
        Self::new()
    }
}

/// Drains the client→server channel into the server's MessageInbox.
pub fn receive_client_messages(
    channels: &LocalChannels,
    inbox: &mut MessageInbox<ClientMessage>,
) {
    for msg in channels.client_to_server_rx.try_iter() {
        inbox.push(ConnectionId::LOCAL, msg);
    }
}

/// Drains the server's MessageOutbox into the server→client channel.
pub fn send_server_messages(
    channels: &LocalChannels,
    outbox: &mut MessageOutbox<ServerMessage>,
) {
    for (_conn, msg) in outbox.drain() {
        // In local mode, there's only one client — ignore connection id.
        let _ = channels.server_to_client_tx.send(msg);
    }
}

/// Drains the server→client channel into the client's MessageInbox.
pub fn receive_server_messages(
    channels: &LocalChannels,
    inbox: &mut MessageInbox<ServerMessage>,
) {
    for msg in channels.server_to_client_rx.try_iter() {
        inbox.push(ConnectionId::LOCAL, msg);
    }
}

/// Drains the client's MessageOutbox into the client→server channel.
pub fn send_client_messages(
    channels: &LocalChannels,
    outbox: &mut MessageOutbox<ClientMessage>,
) {
    for (_conn, msg) in outbox.drain() {
        let _ = channels.client_to_server_tx.send(msg);
    }
}
