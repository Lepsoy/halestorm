use halestorm_common::local_transport::LocalChannels;
use halestorm_common::protocol::{ClientMessage, ServerMessage};
use halestorm_common::transport::{ConnectionId, MessageInbox, MessageOutbox};
use halestorm_common::types::{Direction, Tick, TilePosition};

#[test]
fn local_transport_client_to_server_roundtrip() {
    let channels = LocalChannels::new();
    let mut client_outbox = MessageOutbox::<ClientMessage>::default();
    let mut server_inbox = MessageInbox::<ClientMessage>::default();

    // Client sends a message
    client_outbox.push(
        ConnectionId::LOCAL,
        ClientMessage::Login {
            username: "player1".into(),
            password: "pass".into(),
        },
    );

    // Flush through local transport
    halestorm_common::local_transport::send_client_messages(&channels, &mut client_outbox);
    halestorm_common::local_transport::receive_client_messages(&channels, &mut server_inbox);

    // Server should receive it
    let messages: Vec<_> = server_inbox.drain().collect();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].0, ConnectionId::LOCAL);
    match &messages[0].1 {
        ClientMessage::Login { username, .. } => assert_eq!(username, "player1"),
        _ => panic!("wrong message type"),
    }
}

#[test]
fn local_transport_server_to_client_roundtrip() {
    let channels = LocalChannels::new();
    let mut server_outbox = MessageOutbox::<ServerMessage>::default();
    let mut client_inbox = MessageInbox::<ServerMessage>::default();

    server_outbox.push(
        ConnectionId::LOCAL,
        ServerMessage::MoveConfirm {
            tick: Tick(42),
            position: TilePosition::new(5, 10),
        },
    );

    halestorm_common::local_transport::send_server_messages(&channels, &mut server_outbox);
    halestorm_common::local_transport::receive_server_messages(&channels, &mut client_inbox);

    let messages: Vec<_> = client_inbox.drain().collect();
    assert_eq!(messages.len(), 1);
    match &messages[0].1 {
        ServerMessage::MoveConfirm { tick, position } => {
            assert_eq!(*tick, Tick(42));
            assert_eq!(*position, TilePosition::new(5, 10));
        }
        _ => panic!("wrong message type"),
    }
}

#[test]
fn local_transport_multiple_messages_preserved_order() {
    let channels = LocalChannels::new();
    let mut outbox = MessageOutbox::<ClientMessage>::default();
    let mut inbox = MessageInbox::<ClientMessage>::default();

    for i in 0..5 {
        outbox.push(
            ConnectionId::LOCAL,
            ClientMessage::MoveIntent {
                direction: Direction::North,
                tick: Tick(i),
            },
        );
    }

    halestorm_common::local_transport::send_client_messages(&channels, &mut outbox);
    halestorm_common::local_transport::receive_client_messages(&channels, &mut inbox);

    let messages: Vec<_> = inbox.drain().collect();
    assert_eq!(messages.len(), 5);
    for (i, (_, msg)) in messages.iter().enumerate() {
        match msg {
            ClientMessage::MoveIntent { tick, .. } => assert_eq!(*tick, Tick(i as u64)),
            _ => panic!("wrong message type"),
        }
    }
}

#[test]
fn outbox_broadcast_sends_to_all_connections() {
    let mut outbox = MessageOutbox::<ServerMessage>::default();
    let connections = vec![ConnectionId(0), ConnectionId(1), ConnectionId(2)];

    outbox.broadcast(
        &connections,
        ServerMessage::MoveConfirm {
            tick: Tick(1),
            position: TilePosition::new(0, 0),
        },
    );

    let messages: Vec<_> = outbox.drain().collect();
    assert_eq!(messages.len(), 3);
    assert_eq!(messages[0].0, ConnectionId(0));
    assert_eq!(messages[1].0, ConnectionId(1));
    assert_eq!(messages[2].0, ConnectionId(2));
}

#[test]
fn inbox_drain_empties_queue() {
    let mut inbox = MessageInbox::<ClientMessage>::default();
    inbox.push(ConnectionId::LOCAL, ClientMessage::Disconnect);
    inbox.push(ConnectionId::LOCAL, ClientMessage::Disconnect);

    assert!(!inbox.is_empty());
    let _: Vec<_> = inbox.drain().collect();
    assert!(inbox.is_empty());
}
