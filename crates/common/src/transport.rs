use std::collections::VecDeque;

use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};

/// Identifies a connection. For local transport there is exactly one (LOCAL).
/// For network transport, each quinn connection gets a unique id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConnectionId(pub u64);

impl ConnectionId {
    /// The connection id used for the embedded single-player client.
    pub const LOCAL: ConnectionId = ConnectionId(0);
}

/// Inbound message queue. Transport implementations push messages here;
/// game systems drain and process them.
#[derive(Debug, Resource)]
pub struct MessageInbox<M> {
    messages: VecDeque<(ConnectionId, M)>,
}

impl<M> Default for MessageInbox<M> {
    fn default() -> Self {
        Self {
            messages: VecDeque::new(),
        }
    }
}

impl<M> MessageInbox<M> {
    pub fn push(&mut self, connection: ConnectionId, message: M) {
        self.messages.push_back((connection, message));
    }

    pub fn drain(&mut self) -> impl Iterator<Item = (ConnectionId, M)> + '_ {
        self.messages.drain(..)
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}

/// Outbound message queue. Game systems push messages here;
/// transport implementations drain and send them.
#[derive(Debug, Resource)]
pub struct MessageOutbox<M> {
    messages: VecDeque<(ConnectionId, M)>,
}

impl<M> Default for MessageOutbox<M> {
    fn default() -> Self {
        Self {
            messages: VecDeque::new(),
        }
    }
}

impl<M> MessageOutbox<M> {
    pub fn push(&mut self, connection: ConnectionId, message: M) {
        self.messages.push_back((connection, message));
    }

    /// Send a message to all connections in the provided list.
    pub fn broadcast(&mut self, connections: &[ConnectionId], message: M)
    where
        M: Clone,
    {
        for &conn in connections {
            self.messages.push_back((conn, message.clone()));
        }
    }

    pub fn drain(&mut self) -> impl Iterator<Item = (ConnectionId, M)> + '_ {
        self.messages.drain(..)
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}
